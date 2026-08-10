//! Host-neutral agent boundary: pure computation over Ajisai source with no
//! filesystem, terminal or process I/O, so it compiles for every host target
//! that enables the `std` feature (native and wasm32 alike). The native CLI
//! (`crate::cli`) and the WASM one-shot entry point
//! (`crate::wasm_interpreter_bindings`) both render through this module so
//! they observe the identical stack, NIL flow, diagnostics, output and
//! runtime-metrics envelope (`docs/dev/agent-cli-output-contract.md`).

pub mod api;
pub(crate) mod contract_decl;
mod contract_linearity;
pub(crate) mod contract_report;
pub(crate) mod report;
pub(crate) mod run_render;

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::interpreter::{HostEffect, Interpreter};
use crate::types::Token;
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
    Report {
        status: "error",
        stack: report::stack_json(interp),
        stack_display: stack_display(interp),
        output,
        message: Some(message),
        diagnosis: Some(diagnosis),
        ai_diagnostic: Some(ai),
        error_flow_trace: trace,
        runtime_metrics: interp.runtime_metrics(),
        contract_decls: None,
    }
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

/// Best-effort static resolution: a word resolves when it is a builtin, a
/// canonical alias, or a word the file itself defines via DEF. Returns unknown
/// words in first-appearance order, deduplicated.
pub(crate) fn resolve_words(interp: &Interpreter, tokens: &[Token]) -> Vec<String> {
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
    unknown
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
