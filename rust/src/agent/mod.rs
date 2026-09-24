//! Host-neutral agent boundary: pure computation over Ajisai source with no
//! filesystem, terminal or process I/O, so it compiles for every host target
//! that enables the `std` feature (native and wasm32 alike). The native CLI
//! (`crate::cli`) and the WASM one-shot entry point
//! (`crate::wasm_interpreter_bindings`) both render through this module so
//! they observe the identical stack, NIL flow, diagnostics, output and
//! runtime-metrics envelope (`docs/dev/agent-cli-output-contract.md`).

pub mod api;
#[cfg(test)]
mod check_locals_tests;
pub(crate) mod contract_cost;
pub(crate) mod contract_decl;
#[cfg(test)]
mod contract_decl_tests;
pub(crate) mod contract_gap;
pub(crate) mod contract_report;
#[cfg(test)]
mod contract_report_tests;
mod error_stack;
#[cfg(test)]
mod error_stack_tests;
pub(crate) mod execution_receipt;
#[cfg(test)]
mod execution_receipt_tests;
pub(crate) mod observation_digest;
#[cfg(test)]
mod observation_digest_tests;
pub(crate) mod outcome_report;
#[cfg(test)]
mod outcome_report_tests;
#[cfg(test)]
mod profile_liveness_tests;
pub(crate) mod report;
#[cfg(test)]
mod resource_usage_tests;
pub(crate) mod run_render;

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::interpreter::{HostEffect, Interpreter};
use crate::types::{Token, Value};
use observation_digest::{observation_digest, ObservationDigestInput};
use report::Report;
use std::collections::HashMap;

/// Options shared across agent operations. `json` only matters to the native
/// CLI's own text-vs-JSON command rendering; the agent operations in this
/// module ignore it.
pub(crate) struct Opts {
    pub json: bool,
    /// `check`: verify `#:contract` word declarations against the inferred
    /// contract.
    pub contract: bool,
    /// `compute`: execution step budget override. `None` keeps the
    /// interpreter default.
    pub step_limit: Option<usize>,
}

pub(crate) fn error_report(
    interp: &Interpreter,
    diagnosis: &DebugDiagnosis,
    category: Option<&ErrorCategory>,
    message: String,
    output: Vec<String>,
    trace: Vec<crate::interpreter::error_flow_trace::ErrorFlowEvent>,
    // `Some(source)` builds an execution receipt (`compute`'s callers);
    // `None` skips it (`check`'s callers, which never execute and so have
    // nothing to receipt — see `Report::receipt`'s doc comment).
    source: Option<&str>,
) -> Report {
    // Every error gets the position, not only the ones raised by a Word: the
    // execution loop attaches it to the traced diagnosis, and this covers the
    // rest (a malformed vector literal, a source-entry limit) from the cursor
    // the interpreter still holds.
    let diagnosis = diagnosis
        .clone()
        .with_source_position(interp.current_source_position());
    let ai = diagnosis.ai_payload(category, None, None, None);
    // The residue a failed run was holding is not worth the diagnosis that
    // explains it — see `agent::error_stack`.
    let residue = error_stack::elided_error_stack(interp);
    // The digest is taken over the real stack, not the wire-budgeted residue:
    // `stack`/`stackDisplay` may elide values for `responseBytes`, but the
    // observation itself is what the interpreter actually holds.
    let error_category = category.map(ErrorCategory::as_protocol_str);
    let digest = observation_digest(ObservationDigestInput {
        status: "error",
        stack: &stack_values(interp),
        output: &output,
        user_words: &user_word_identities(interp),
        error_category,
    });
    let resource_usage = interp.resource_usage();
    let receipt = source.map(|source| {
        execution_receipt::build_receipt(
            source,
            interp.runtime_limits(),
            interp.max_execution_steps(),
            "error",
            &resource_usage,
            &digest,
        )
    });
    Report {
        status: "error",
        stack: residue.stack,
        stack_display: residue.stack_display,
        output,
        message: Some(message),
        diagnosis: Some(diagnosis),
        ai_diagnostic: Some(ai),
        error_flow_trace: trace,
        runtime_metrics: interp.runtime_metrics(),
        resource_usage,
        contract_decls: None,
        stack_elided: residue.elided,
        observation_digest: digest,
        receipt,
    }
}

/// The stack, bottom to top, as owned `Value`s — the raw material
/// `observation_digest` encodes from. Cloning is cheap: `Value`'s heavy
/// payloads (`Vector`, `Tensor`, `Text`, `CodeBlock`) are all reference
/// counted.
pub(crate) fn stack_values(interp: &Interpreter) -> Vec<Value> {
    interp.get_stack().to_vec()
}

/// `(normalized word name, content identity)` for every user word, sorted by
/// name — the shape `ObservationDigestInput::user_words` requires.
pub(crate) fn user_word_identities(interp: &Interpreter) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = interp
        .user_words
        .keys()
        .map(|name| {
            let identity = interp.word_identity(name).cloned().unwrap_or_default();
            (name.clone(), identity)
        })
        .collect();
    pairs.sort();
    pairs
}

pub(crate) fn print_payloads(interp: &Interpreter) -> Vec<String> {
    interp
        .host_effects()
        .iter()
        .map(|effect| match effect {
            HostEffect::Print(payload) => payload.clone(),
        })
        .collect()
}

pub(crate) fn stack_display(interp: &Interpreter) -> Vec<String> {
    // One shared rendering (LANG.OBSERVATION.PROTOCOL) for every observation
    // surface.
    crate::types::display::render_stack(interp.get_stack())
}

/// execution — this only front-loads the same failure for `check`.
pub(crate) fn check_structure(tokens: &[Token]) -> Result<(), String> {
    let mut depth: usize = 0;
    for token in tokens {
        match token {
            Token::VectorStart => depth += 1,
            Token::VectorEnd => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| "Unexpected vector end".to_string())?;
            }
            _ => {}
        }
    }
    if depth > 0 {
        return Err("Unclosed vector".to_string());
    }
    Ok(())
}

pub(crate) fn normalize_word(symbol: &str) -> String {
    symbol.to_uppercase()
}

/// The outcome of best-effort static word resolution.
pub(crate) struct ResolvedWords {
    /// Unknown words in first-appearance order, deduplicated.
    pub unknown: Vec<String>,
    /// Names this file defines for itself. Carried out alongside the unknown
    /// list so a "did you mean" for a misspelled call can consider the very
    /// definitions the same source introduces — nothing else knows them, since
    /// static checking never executes the `DEF`.
    pub locally_defined: Vec<String>,
    /// Unknown words that some *other* frame of the same file binds: a name
    /// written inside a DEF body but bound at the top level, or the reverse.
    /// The runtime refuses these with its "bound in another frame" message,
    /// and `check` says the same thing instead of a bare "unknown word".
    pub bound_elsewhere: Vec<String>,
}

/// Best-effort static resolution: a word resolves when it is a builtin, a
/// canonical alias, a word the file itself defines via DEF, or a name a
/// `BIND` in the same frame region binds.
///
/// Frame regions follow `bindings.rs`'s rule: a binding is reachable in the
/// frame that made it and in the blocks written there, never inside a Word
/// called from it. Statically, a Word body is the block a `] 'NAME' DEF`
/// closes; every token outside such a body belongs to the run's own frame,
/// and every token inside one belongs to that body's frame. Blocks a Core
/// Word evaluates (`EXEC`, `MAP`, `FOLD`) are transparent at runtime, so this
/// does not open a region for them — which is also why order within a region
/// does not matter: a block may be bound first and evaluated later.
pub(crate) fn resolve_words(interp: &Interpreter, tokens: &[Token]) -> ResolvedWords {
    use std::collections::{HashMap, HashSet};

    let mut defined: HashSet<String> = HashSet::new();
    // Pre-pass: `'NAME' DEF` definitions anywhere in the file (definitions may
    // be referenced before they appear, e.g. mutual recursion between user
    // words).
    for (i, token) in tokens.iter().enumerate() {
        let Token::String(text) = token else {
            continue;
        };
        let next_words: Vec<String> = tokens[i + 1..]
            .iter()
            .take(2)
            .filter_map(|t| match t {
                Token::Symbol(s) => Some(normalize_word(s)),
                _ => None,
            })
            .collect();
        if next_words.iter().any(|w| w == "DEF") {
            defined.insert(text.to_uppercase());
        }
    }

    // Region of each token: 0 is the run's frame; a DEF body's region is the
    // index of its opening `[`, and nested DEF bodies get their own.
    let regions = frame_regions(tokens);

    // Every `BIND` makes bindings rather than Words — one name (`'N' BIND`)
    // or several (`[ 'A' 'B' ] BIND`). They are not in the dictionary, and
    // `check` resolves without running, so without this every bound name read
    // as an unknown Word and `check` refused programs that run.
    let mut bound: HashMap<usize, HashSet<String>> = HashMap::new();
    for (i, token) in tokens.iter().enumerate() {
        if !matches!(token, Token::Symbol(s) if normalize_word(s) == "BIND") {
            continue;
        }
        let region = regions[i];
        match i.checked_sub(1).map(|j| (j, &tokens[j])) {
            Some((_, Token::String(name))) => {
                bound.entry(region).or_default().insert(name.to_uppercase());
            }
            Some((close, Token::VectorEnd)) => {
                let mut depth = 0usize;
                for t in tokens[..=close].iter().rev() {
                    match t {
                        Token::VectorEnd => depth += 1,
                        Token::VectorStart => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        Token::String(name) if depth == 1 => {
                            bound.entry(region).or_default().insert(name.to_uppercase());
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    let mut unknown: Vec<String> = Vec::new();
    let mut bound_elsewhere: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (i, token) in tokens.iter().enumerate() {
        let Token::Symbol(symbol) = token else {
            continue;
        };
        let normalized = normalize_word(symbol);
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(&normalized);
        let bound_here = bound
            .get(&regions[i])
            .is_some_and(|names| names.contains(canonical.as_ref()));
        let resolved = interp.core_vocabulary.contains_key(canonical.as_ref())
            || crate::coreword_registry::get_coreword_metadata(&canonical).is_some()
            || defined.contains(canonical.as_ref())
            || bound_here;
        if !resolved && seen.insert(canonical.to_string()) {
            if bound
                .values()
                .any(|names| names.contains(canonical.as_ref()))
            {
                bound_elsewhere.push(canonical.to_string());
            }
            unknown.push(canonical.into_owned());
        }
    }
    let mut locally_defined: Vec<String> = defined
        .into_iter()
        .chain(bound.into_values().flatten())
        .collect();
    locally_defined.sort();
    locally_defined.dedup();
    ResolvedWords {
        unknown,
        locally_defined,
        bound_elsewhere,
    }
}

/// The frame region of every token (see [`resolve_words`]). Assumes the
/// bracket structure already passed [`check_structure`].
fn frame_regions(tokens: &[Token]) -> Vec<usize> {
    // Match every `[` to its `]` first, so a block can be recognised as a
    // DEF body from its opening side.
    let mut close_of: HashMap<usize, usize> = HashMap::new();
    let mut open_stack: Vec<usize> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::VectorStart => open_stack.push(i),
            Token::VectorEnd => {
                if let Some(open) = open_stack.pop() {
                    close_of.insert(open, i);
                }
            }
            _ => {}
        }
    }
    let is_def_body = |open: usize| -> bool {
        let Some(&close) = close_of.get(&open) else {
            return false;
        };
        matches!(tokens.get(close + 1), Some(Token::String(_)))
            && matches!(tokens.get(close + 2), Some(Token::Symbol(s)) if normalize_word(s) == "DEF")
    };

    let mut regions = Vec::with_capacity(tokens.len());
    // (region id, index of the `]` that ends it)
    let mut region_stack: Vec<(usize, usize)> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        while region_stack.last().is_some_and(|(_, end)| *end < i) {
            region_stack.pop();
        }
        if matches!(token, Token::VectorStart) && is_def_body(i) {
            region_stack.push((i + 1, close_of[&i]));
        }
        regions.push(region_stack.last().map_or(0, |(id, _)| *id));
    }
    regions
}

/// Poll the interpreter future to completion. `Interpreter::execute` is
/// `async` for the WASM host's benefit but contains no await points on either
/// the native or the one-shot WASM agent path (both drive it to completion
/// synchronously), so a no-op waker is sufficient; the yield is a safety
/// valve.
pub(crate) fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll};
    let mut fut = Box::pin(fut);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
