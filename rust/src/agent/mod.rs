//! Host-neutral agent boundary: pure computation over Ajisai source with no
//! filesystem, terminal or process I/O, so it compiles for every host target
//! that enables the `std` feature (native and wasm32 alike). The native CLI
//! (`crate::cli`) and the WASM one-shot entry point
//! (`crate::wasm_interpreter_bindings`) both render through this module so
//! they observe the identical stack, NIL flow, diagnostics, output and
//! runtime-metrics envelope (`docs/dev/agent-cli-output-contract.md`).

pub mod api;
pub(crate) mod contract_cost;
pub(crate) mod contract_decl;
#[cfg(test)]
mod contract_decl_tests;
pub(crate) mod contract_gap;
mod contract_linearity;
pub(crate) mod contract_report;
mod error_stack;
#[cfg(test)]
mod error_stack_tests;
mod observation_digest;
#[cfg(test)]
mod observation_digest_tests;
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
    _opts: &Opts,
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
        resource_usage: interp.resource_usage(),
        contract_decls: None,
        stack_elided: residue.elided,
        observation_digest: digest,
    }
}

/// The stack, bottom to top, as owned `Value`s — the raw material
/// `observation_digest` encodes from. Cloning is cheap: `Value`'s heavy
/// payloads (`Vector`, `Tensor`, `Text`, `CodeBlock`) are all reference
/// counted.
pub(crate) fn stack_values(interp: &Interpreter) -> Vec<Value> {
    interp
        .get_stack()
        .iter_slots()
        .map(|(value, _role)| value.clone())
        .collect()
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
    // One shared `(value, role)` rendering (SPEC §12) for every observation
    // surface; the `Stack` owns aligned values and roles, so no snapshot/
    // realignment step is needed here.
    crate::types::display::render_stack(interp.get_stack())
}

/// execution — this only front-loads the same failure for `check`.
pub(crate) fn check_structure(tokens: &[Token]) -> Result<(), String> {
    let mut vector_depth: i64 = 0;
    let mut block_depth: i64 = 0;
    for token in tokens {
        match token {
            Token::VectorStart => vector_depth += 1,
            Token::VectorEnd => {
                vector_depth -= 1;
                if vector_depth < 0 {
                    return Err("Unexpected vector end".to_string());
                }
            }
            Token::BlockStart => block_depth += 1,
            Token::BlockEnd => {
                block_depth -= 1;
                if block_depth < 0 {
                    return Err("Unexpected code block end".to_string());
                }
            }
            _ => {}
        }
    }
    if vector_depth > 0 {
        return Err("Unclosed vector".to_string());
    }
    if block_depth > 0 {
        return Err("Unclosed code block".to_string());
    }
    Ok(())
}

pub(crate) fn normalize_word(symbol: &str) -> String {
    match symbol {
        "%" => "MOD".to_string(),
        "&" => "AND".to_string(),
        _ => symbol.to_uppercase(),
    }
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
}

/// Best-effort static resolution: a word resolves when it is a builtin, a
/// canonical alias, or a word the file itself defines via DEF.
pub(crate) fn resolve_words(interp: &Interpreter, tokens: &[Token]) -> ResolvedWords {
    use std::collections::HashSet;

    let mut locally_known: HashSet<String> = HashSet::new();
    // Pre-pass: `'NAME' DEF` definitions anywhere in the file (definitions may
    // be referenced before they appear, e.g. mutual recursion between user
    // words).
    for (i, token) in tokens.iter().enumerate() {
        let Token::String(text) = token else {
            continue;
        };
        let next_words: Vec<String> = tokens[i + 1..]
            .iter()
            .filter(|t| !matches!(t, Token::LineBreak))
            .take(2)
            .filter_map(|t| match t {
                Token::Symbol(s) => Some(normalize_word(s)),
                _ => None,
            })
            .collect();
        if next_words.iter().any(|w| w == "DEF") {
            locally_known.insert(text.to_uppercase());
        }
    }

    let mut unknown: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for token in tokens {
        let Token::Symbol(symbol) = token else {
            continue;
        };
        let normalized = normalize_word(symbol);
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(&normalized);
        let resolved = interp.core_vocabulary.contains_key(canonical.as_ref())
            || crate::coreword_registry::get_coreword_metadata(&canonical).is_some()
            || locally_known.contains(canonical.as_ref());
        if !resolved && seen.insert(canonical.to_string()) {
            unknown.push(canonical.into_owned());
        }
    }
    let mut locally_defined: Vec<String> = locally_known.into_iter().collect();
    locally_defined.sort();
    ResolvedWords {
        unknown,
        locally_defined,
    }
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
