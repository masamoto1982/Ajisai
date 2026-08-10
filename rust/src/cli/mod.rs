//! Headless `ajisai` CLI: the agent-facing write → run → read-structured-error
//! loop, entirely in a terminal.
//!
//! Commands (see `docs/dev/agent-cli-output-contract.md` for the `--json`
//! output contract):
//!
//! ```text
//! ajisai run <file.ajisai> [--json]
//! ajisai check <file.ajisai> [--json]     # tokenize + parse + resolve, no execution
//! ajisai contract <file.ajisai> [--json]  # report inferred word contracts, no execution
//! ajisai test <file-or-dir> [--json]       # execute `#@` host test directives
//! ajisai repl [--json]                     # persistent interactive session
//! ajisai version [--json]
//! ```
//!
//! Exit codes: 0 = success, 1 = language error (diagnosis emitted),
//! 2 = CLI usage error. With `--json`, stdout carries exactly one JSON
//! document and nothing else (pipe-safe); usage errors go to stderr.
//!
//! This module is observational: it feeds source text to the existing
//! interpreter and serializes the existing diagnostic structures. It defines
//! no language semantics (canonical source: `SPECIFICATION.html`).

pub mod agent_api;
mod contract_decl;
mod contract_linearity;
mod contract_report;
mod repl;
mod report;
mod run_render;
#[cfg(test)]
mod step_limit_tests;
mod test_runner;

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use crate::interpreter::{HostEffect, Interpreter};
use crate::types::Token;
use report::Report;

const USAGE: &str = "Usage: ajisai <command> [options]

Commands:
  run <file.ajisai> [--json] [--step-limit <N>]
                                  Execute a program file
  check <file.ajisai> [--json] [--contract]
                                  Tokenize, parse and resolve only (no
                                  execution). With --contract, also check each
                                  `#:contract` declaration against the contract
                                  inferred from the Core Words it calls
  contract <file.ajisai> [--json] Report each user word's inferred contract
                                  (arity, purity, NIL, determinism) plus a
                                  paste-ready `#:contract` line (no execution)
  test <file-or-dir> [--json]     Run test files, checking each program against
                                  its `#@` directive comments (status/stack/
                                  output/error). Exit 1 if any test fails
  repl [--json]                   Interactive session; stack and definitions
                                  persist. :help for commands, :quit to leave
  version [--json]                Print version information

Options:
  --json                          Emit one JSON document (pipe-safe)
  --contract                      With `check`: verify `#:contract` word
                                  declarations against the inferred contract
                                  (exit 1 on a contradiction). The check is
                                  conservative: an unanalyzable body is
                                  reported as `cannot verify`, never as passed
  --step-limit <N>                With `run`: override the execution step
                                  budget. N is a positive integer; default:
                                  100000. A host safety control, not a language
                                  semantic

Exit codes:
  0  success
  1  language error (structured diagnosis emitted)
  2  CLI usage error";

/// CLI entry point. Returns the process exit code.
pub fn run(args: &[String]) -> i32 {
    let Some((command, rest)) = args.split_first() else {
        eprintln!("{}", USAGE);
        return 2;
    };
    let mut json = false;
    let mut contract = false;
    let mut step_limit: Option<usize> = None;
    let mut positional: Vec<&str> = Vec::new();
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--contract" => contract = true,
            "--step-limit" => match iter.next().and_then(|value| value.parse::<usize>().ok()) {
                Some(parsed) if parsed > 0 => step_limit = Some(parsed),
                _ => {
                    eprintln!("--step-limit expects a positive integer\n\n{}", USAGE);
                    return 2;
                }
            },
            flag if flag.starts_with('-') => {
                eprintln!("Unknown option: {}\n\n{}", flag, USAGE);
                return 2;
            }
            path => positional.push(path),
        }
    }
    let opts = Opts {
        json,
        contract,
        step_limit,
    };
    match (command.as_str(), positional.as_slice()) {
        ("run", [path]) => cmd_run(path, &opts),
        ("check", [path]) => cmd_check(path, &opts),
        ("contract", [path]) => cmd_contract(path, &opts),
        ("test", [path]) => test_runner::cmd_test(path, &opts),
        ("repl", []) => repl::cmd_repl(&opts),
        ("version", []) => cmd_version(json),
        _ => {
            eprintln!("{}", USAGE);
            2
        }
    }
}

/// Parsed CLI options shared across commands.
struct Opts {
    json: bool,
    /// `check --contract`: verify `#:contract` declarations against the
    /// contract inferred from the called Core Words. Only `check` reads it.
    contract: bool,
    /// Execution step budget override. `None` keeps the interpreter default
    /// (`DEFAULT_MAX_EXECUTION_STEPS`); only `run` executes, so only `run`
    /// reads it.
    step_limit: Option<usize>,
}

fn cmd_version(json: bool) -> i32 {
    let version = env!("CARGO_PKG_VERSION");
    if json {
        let doc = serde_json::json!({
            "schemaVersion": report::SCHEMA_VERSION,
            "status": "ok",
            "version": version,
        });
        println!("{}", pretty(&doc));
    } else {
        println!("ajisai {}", version);
    }
    0
}

fn cmd_run(path: &str, opts: &Opts) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("ajisai: cannot read {}: {}", path, e);
            return 2;
        }
    };

    let response = block_on(agent_api::compute(
        &source,
        agent_api::ComputeOptions {
            step_limit: opts.step_limit,
        },
    ));
    emit(response.report(), opts);
    response.exit_code()
}

fn error_report(
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

/// Read one `key=value` evidence entry.
fn evidence_value<'a>(evidence: &'a [String], key: &str) -> Option<&'a str> {
    evidence
        .iter()
        .find_map(|entry| entry.strip_prefix(key)?.strip_prefix('='))
}

fn print_payloads(interp: &Interpreter) -> Vec<String> {
    interp
        .host_effects()
        .iter()
        .map(|effect| match effect {
            HostEffect::Print(payload) => payload.clone(),
        })
        .collect()
}

fn stack_display(interp: &Interpreter) -> Vec<String> {
    // One shared `(value, role)` rendering (SPEC §12) for every observation
    // surface; the `Stack` owns aligned values and roles, so no snapshot/
    // realignment step is needed here.
    crate::types::display::render_stack(interp.get_stack())
}

fn cmd_check(path: &str, opts: &Opts) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("ajisai: cannot read {}: {}", path, e);
            return 2;
        }
    };
    if opts.json {
        let response = agent_api::check(&source, opts.contract);
        println!("{}", pretty(&response.to_json()));
        return response.exit_code();
    }
    let interp = Interpreter::new();

    let tokens = match crate::tokenizer::tokenize(&source) {
        Ok(tokens) => tokens,
        Err(message) => {
            let diagnosis = DebugDiagnosis::from_error_category(
                ErrorPhase::Tokenize,
                None,
                Some(&ErrorCategory::MalformedSource),
                None,
                0,
                0,
                Some(message.clone()),
            );
            emit(
                &error_report(
                    &interp,
                    &diagnosis,
                    None,
                    message,
                    Vec::new(),
                    Vec::new(),
                    opts,
                ),
                opts,
            );
            return 1;
        }
    };

    if let Err(message) = check_structure(&tokens) {
        let category = ErrorCategory::StructureError;
        let diagnosis = DebugDiagnosis::from_error_category(
            ErrorPhase::ParseStructure,
            None,
            Some(&category),
            None,
            0,
            0,
            Some(message.clone()),
        );
        emit(
            &error_report(
                &interp,
                &diagnosis,
                Some(&category),
                message,
                Vec::new(),
                Vec::new(),
                opts,
            ),
            opts,
        );
        return 1;
    }

    let unknown = resolve_words(&interp, &tokens);
    if let Some(first) = unknown.first() {
        let message = format!("Unknown words: {}", unknown.join(", "));
        let category = ErrorCategory::UnknownWord;
        let mut diagnosis = DebugDiagnosis::from_error_category(
            ErrorPhase::ResolveWord,
            Some(first),
            Some(&category),
            None,
            0,
            0,
            Some(format!("Unknown word: {}", first)),
        );
        diagnosis
            .evidence
            .push(format!("unknownWords={}", unknown.join(",")));
        emit(
            &error_report(
                &interp,
                &diagnosis,
                Some(&category),
                message,
                Vec::new(),
                Vec::new(),
                opts,
            ),
            opts,
        );
        return 1;
    }

    // Per-word `#:contract` declarations, checked against the contract inferred
    // from the called Core Words without executing any body. A violated
    // declaration exits 1; a "cannot verify" note does not.
    let contract_decls = if opts.contract {
        Some(contract_decl::check_contract_decls(&source))
    } else {
        None
    };
    let contract_failed = contract_decls
        .as_ref()
        .map(|check| check.violated)
        .unwrap_or(false);

    let status = if contract_failed { "fail" } else { "ok" };
    println!("{}: {} ({} tokens)", status, path, tokens.len());
    if let Some(check) = &contract_decls {
        for finding in &check.findings {
            eprintln!("  [{}] {}", finding.severity.as_str(), finding.message);
        }
    }
    if contract_failed {
        1
    } else {
        0
    }
}

/// `ajisai contract <file>`: report each user word's inferred contract
/// (`interpreter::word_contract`), the reporting companion to `check --contract`
/// (P2). Registers definitions and imports without executing any word body or
/// top-level code. Observational — a well-formed file always exits 0.
fn cmd_contract(path: &str, opts: &Opts) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("ajisai: cannot read {}: {}", path, e);
            return 2;
        }
    };
    if opts.json {
        let response = agent_api::infer_contracts(&source);
        println!("{}", pretty(response.contracts()));
    } else {
        let reports = contract_report::report_contracts(&source);
        if reports.is_empty() {
            println!("{}: no user words defined", path);
        }
        for r in &reports {
            println!(
                "{} : {} {} {} {} {} [{}]",
                r.name, r.arity, r.purity, r.nil, r.determinism, r.space, r.confidence
            );
            if !r.effects.is_empty() {
                println!("    effects: {}", r.effects.join(", "));
            }
            println!("    {}", r.suggested);
        }
    }
    0
}

/// execution — this only front-loads the same failure for `check`.
fn check_structure(tokens: &[Token]) -> Result<(), String> {
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

fn normalize_word(symbol: &str) -> String {
    match symbol {
        "%" => "MOD".to_string(),
        "&" => "AND".to_string(),
        _ => symbol.to_uppercase(),
    }
}

/// Best-effort static resolution: a word resolves when it is a builtin, a
/// canonical alias, or a word the file itself defines via DEF. Returns unknown
/// words in first-appearance order, deduplicated.
fn resolve_words(interp: &Interpreter, tokens: &[Token]) -> Vec<String> {
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

fn emit(report: &Report, opts: &Opts) {
    if opts.json {
        println!("{}", pretty(&report.to_json()));
        return;
    }
    for line in &report.output {
        println!("{}", line);
    }
    if report.status == "ok" {
        if report.stack_display.is_empty() {
            println!("stack: (empty)");
        } else {
            println!("stack: {}", report.stack_display.join(" "));
        }
        return;
    }
    if let Some(message) = &report.message {
        eprintln!("error: {}", message);
    }
    if let Some(diagnosis) = &report.diagnosis {
        eprintln!("diagnosis: {}", diagnosis.summary);
        // The stack lengths either side of the failure. They are already in the
        // structured `evidence`, and reading them is most of the work of
        // telling "the word was called with too few operands" apart from "the
        // word consumed more than it should have" — so print them next to the
        // summary rather than only in `--json`.
        if let (Some(line), Some(column)) = (
            evidence_value(&diagnosis.evidence, "sourceLine"),
            evidence_value(&diagnosis.evidence, "sourceColumn"),
        ) {
            eprintln!("  at line {}, column {}", line, column);
        }
        // The Words the failure happened *inside*, innermost first. The
        // position above is the top-level token that reached the failure — a
        // block and a Word body are each their own token stream with no source
        // of their own — so this is what says the rest: which Word failed, and
        // which construct it was written in.
        if let Some(inside) = evidence_value(&diagnosis.evidence, "insideWords") {
            eprintln!("  inside {}", inside.replace(',', ", "));
        }
        for line in &diagnosis.evidence {
            if line.starts_with("stackLen") {
                eprintln!("  {}", line);
            }
        }
        for check in &diagnosis.next_checks {
            eprintln!("  - {}: {}", check.label, check.detail);
        }
    }
}

/// Poll the interpreter future to completion. `Interpreter::execute` is
/// `async` for the WASM host's benefit but contains no await points on the
/// native path, so a no-op waker is sufficient; the yield is a safety valve.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
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

fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}
