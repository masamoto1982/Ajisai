//! Headless `ajisai` CLI: the agent-facing write → run → read-structured-error
//! loop, entirely in a terminal.
//!
//! Commands (see `docs/dev/agent-cli-output-contract.md` for the `--json`
//! output contract):
//!
//! ```text
//! ajisai run <file.ajisai> [--json]
//! ajisai check <file.ajisai> [--json]   # tokenize + parse + resolve, no execution
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

mod explain;
#[cfg(test)]
mod explain_tests;
mod host;
mod report;
#[cfg(test)]
mod report_tests;

use explain::{Explanation, Lang};

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{
    CauseClass, DebugCheck, DebugDiagnosis, ErrorLocus, ErrorLocusKind, ErrorPhase,
};
use crate::interpreter::{HostEffect, Interpreter, RuntimeMetrics};
use crate::types::display::format_with_hint;
use crate::types::{Interpretation, Token};
use report::Report;
use std::sync::Arc;

const USAGE: &str = "Usage: ajisai <command> [options]

Commands:
  run <file.ajisai> [--json]      Execute a program file
  check <file.ajisai> [--json]    Tokenize, parse and resolve only (no execution)
  version [--json]                Print version information

Options:
  --json                          Emit one JSON document (pipe-safe)
  --explain                       Add a plain-language explanation of any
                                  diagnosis (headline, next step, details)
  --lang <ja|en>                  Language for --explain output (default: ja)

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
    let mut want_explain = false;
    let mut lang = Lang::Ja;
    let mut positional: Vec<&str> = Vec::new();
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--explain" => want_explain = true,
            "--lang" => match iter.next().map(String::as_str).and_then(Lang::parse) {
                Some(parsed) => lang = parsed,
                None => {
                    eprintln!("--lang expects one of: ja, en\n\n{}", USAGE);
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
        explain: want_explain,
        lang,
    };
    match (command.as_str(), positional.as_slice()) {
        ("run", [path]) => cmd_run(path, &opts),
        ("check", [path]) => cmd_check(path, &opts),
        ("version", []) => cmd_version(json),
        _ => {
            eprintln!("{}", USAGE);
            2
        }
    }
}

/// Parsed CLI options shared by `run` and `check`.
struct Opts {
    json: bool,
    explain: bool,
    lang: Lang,
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

    // Tokenize separately first so a lexical failure is reported with the
    // accurate `tokenize` phase (execute() folds it into a generic error).
    if let Err(message) = crate::tokenizer::tokenize(&source) {
        let diagnosis = DebugDiagnosis::from_error_category(
            ErrorPhase::Tokenize,
            None,
            None,
            None,
            0,
            0,
            Some(message.clone()),
        );
        let interp = Interpreter::new();
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

    let mut interp = Interpreter::with_host(Arc::new(host::CliHostEnv));
    let result = block_on(interp.execute(&source));
    let trace = interp.drain_error_flow_trace();
    let output = print_payloads(&interp);

    match result {
        Ok(()) => {
            let explanation = nil_explanation(&trace, opts);
            let report = Report {
                status: "ok",
                stack: report::stack_json(&interp),
                stack_display: stack_display(&interp),
                output,
                message: None,
                diagnosis: None,
                ai_diagnostic: None,
                error_flow_trace: trace,
                runtime_metrics: interp.runtime_metrics(),
                explanation,
            };
            emit(&report, opts);
            0
        }
        Err(err) => {
            let message = err.to_string();
            let stack_len = interp.get_stack().len();
            let diagnosis = missing_capability_diagnosis(&interp, &message)
                .or_else(|| trace.iter().rev().find_map(|event| event.diagnosis.clone()))
                .unwrap_or_else(|| DebugDiagnosis::from_error(&err, None, stack_len, stack_len));
            let category = ErrorCategory::from_error(&err);
            emit(
                &error_report(
                    &interp,
                    &diagnosis,
                    Some(&category),
                    message,
                    output,
                    trace,
                    opts,
                ),
                opts,
            );
            1
        }
    }
}

fn error_report(
    interp: &Interpreter,
    diagnosis: &DebugDiagnosis,
    category: Option<&ErrorCategory>,
    message: String,
    output: Vec<String>,
    trace: Vec<crate::interpreter::error_flow_trace::ErrorFlowEvent>,
    opts: &Opts,
) -> Report {
    let ai = diagnosis.ai_payload(category, None, None, None);
    let explanation = opts
        .explain
        .then(|| explain::explain(diagnosis, Some(&ai.recoverability), None, opts.lang));
    Report {
        status: "error",
        stack: report::stack_json(interp),
        stack_display: stack_display(interp),
        output,
        message: Some(message),
        diagnosis: Some(diagnosis.clone()),
        ai_diagnostic: Some(ai),
        error_flow_trace: trace,
        runtime_metrics: interp.runtime_metrics(),
        explanation,
    }
}

/// On a *successful* run a value may still have bubbled to NIL (SPEC Bubble
/// Rule). When `--explain` is set, project the last such NIL event so the
/// user sees the absence in plain language. The actionable advice for a value
/// that bubbled and flows downstream is to decide a fallback (`VENT`) or add a
/// branch, so this projects `handleUnknownOrNil` regardless of the underlying
/// domain cause.
fn nil_explanation(
    trace: &[crate::interpreter::error_flow_trace::ErrorFlowEvent],
    opts: &Opts,
) -> Option<Explanation> {
    if !opts.explain {
        return None;
    }
    trace.iter().rev().find_map(|event| {
        let diagnosis = event.diagnosis.as_ref()?;
        let nil_reason = event
            .absence
            .as_ref()
            .and_then(|absence| absence.reason.as_ref())
            .map(|reason| reason.as_protocol_str());
        Some(explain::explain(
            diagnosis,
            Some("handleUnknownOrNil"),
            nil_reason,
            opts.lang,
        ))
    })
}

fn print_payloads(interp: &Interpreter) -> Vec<String> {
    interp
        .host_effects()
        .iter()
        .filter_map(|effect| match effect {
            HostEffect::Print(payload) => Some(payload.clone()),
            _ => None,
        })
        .collect()
}

fn stack_display(interp: &Interpreter) -> Vec<String> {
    let hints = interp.collect_stack_hints();
    interp
        .get_stack()
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let hint = hints.get(i).copied().unwrap_or(Interpretation::Unassigned);
            format_with_hint(value, hint)
        })
        .collect()
}

/// When a Hosted word failed because this terminal host does not provide its
/// capability (no audio device, no serial port, ...), the interpreter emitted
/// a structured `Diagnostic` host effect before consuming anything. Surface
/// it as the top-level diagnosis: `why: environment`, locus
/// `hostEnvironment` — a property of the execution environment, not of the
/// program (§2.5 of the CLI work order).
fn missing_capability_diagnosis(interp: &Interpreter, message: &str) -> Option<DebugDiagnosis> {
    if !message.contains("requires missing host capability") {
        return None;
    }
    let (word, capability) = interp.host_effects().iter().rev().find_map(|effect| {
        let HostEffect::Diagnostic(payload) = effect else {
            return None;
        };
        let parsed: serde_json::Value = serde_json::from_str(payload).ok()?;
        if parsed.get("op")?.as_str()? != "missingCapability" {
            return None;
        }
        Some((
            parsed.get("word")?.as_str()?.to_string(),
            parsed.get("capability")?.as_str()?.to_string(),
        ))
    })?;
    let module = word.split_once('@').map(|(m, _)| m.to_string());
    Some(DebugDiagnosis {
        when: ErrorPhase::HostIo,
        where_: ErrorLocus {
            kind: ErrorLocusKind::HostEnvironment,
            word: Some(word.clone()),
            module,
            dictionary: None,
        },
        why: CauseClass::Environment,
        summary: format!(
            "hostIo / {} / environment (missing host capability {})",
            word, capability
        ),
        evidence: vec![format!("missingCapability={}", capability)],
        next_checks: vec![
            DebugCheck {
                label: "Check host capability".to_string(),
                detail: format!(
                    "この実行環境（ajisai CLI）は capability '{}' を提供していない",
                    capability
                ),
            },
            DebugCheck {
                label: "Check execution host".to_string(),
                detail: format!(
                    "{} を実行するには該当 capability を持つホスト（GUI/Tauri 等）を使う",
                    word
                ),
            },
            DebugCheck {
                label: "Check program intent".to_string(),
                detail: "CLI 上で完結させる場合は該当 word の使用を避ける".to_string(),
            },
        ],
        agreed_prefix: None,
    })
}

fn cmd_check(path: &str, opts: &Opts) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("ajisai: cannot read {}: {}", path, e);
            return 2;
        }
    };
    let interp = Interpreter::new();

    let tokens = match crate::tokenizer::tokenize(&source) {
        Ok(tokens) => tokens,
        Err(message) => {
            let diagnosis = DebugDiagnosis::from_error_category(
                ErrorPhase::Tokenize,
                None,
                None,
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

    if opts.json {
        let report = Report {
            status: "ok",
            stack: serde_json::Value::Array(Vec::new()),
            stack_display: Vec::new(),
            output: Vec::new(),
            message: None,
            diagnosis: None,
            ai_diagnostic: None,
            error_flow_trace: Vec::new(),
            runtime_metrics: RuntimeMetrics::default(),
            explanation: None,
        };
        println!("{}", pretty(&report.to_json()));
    } else {
        println!("ok: {} ({} tokens)", path, tokens.len());
    }
    0
}

/// Static bracket balance for vector literals and code blocks. Purely
/// structural; the runtime performs the authoritative check during
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
/// canonical alias, a word the file itself defines via DEF, a word imported
/// from a module the file IMPORTs, or a qualified `DICT@WORD` reference into
/// a user dictionary (runtime state, accepted statically). Returns unknown
/// words in first-appearance order, deduplicated.
fn resolve_words(interp: &Interpreter, tokens: &[Token]) -> Vec<String> {
    use std::collections::HashSet;

    let mut locally_known: HashSet<String> = HashSet::new();
    // Pre-pass: `'NAME' DEF` definitions and `'MODULE' IMPORT[-ONLY]`
    // imports anywhere in the file (definitions may be referenced before
    // they appear, e.g. mutual recursion between user words).
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
        if next_words
            .iter()
            .any(|w| w == "IMPORT" || w == "IMPORT-ONLY")
        {
            let module = text.to_uppercase();
            if let Some(catalog) = crate::interpreter::modules::module_catalog_words(&module) {
                for word in catalog {
                    locally_known.insert(word.short_name.to_uppercase());
                }
            }
        }
    }

    let modules: HashSet<String> = crate::interpreter::modules::available_module_names()
        .into_iter()
        .map(|name| name.to_uppercase())
        .collect();

    let mut unknown: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for token in tokens {
        let Token::Symbol(symbol) = token else {
            continue;
        };
        let normalized = normalize_word(symbol);
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(&normalized);
        let resolved = if let Some((module, short)) = canonical.split_once('@') {
            if modules.contains(module) {
                crate::coreword_registry::get_coreword_metadata(&canonical).is_some()
                    || crate::interpreter::modules::module_catalog_words(module)
                        .map(|catalog| {
                            catalog
                                .iter()
                                .any(|w| w.short_name.eq_ignore_ascii_case(short))
                        })
                        .unwrap_or(false)
            } else {
                // A user-dictionary reference (DICT@WORD); dictionaries are
                // runtime state, so accept statically.
                true
            }
        } else {
            interp.core_vocabulary.contains_key(canonical.as_ref())
                || crate::coreword_registry::get_coreword_metadata(&canonical).is_some()
                || locally_known.contains(canonical.as_ref())
        };
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
        // A value bubbled to NIL on an otherwise successful run; surface it as
        // an advisory note when --explain is set (L0 plain language).
        if let Some(explanation) = &report.explanation {
            eprintln!("note: {} {}", explanation.headline, explanation.next_step);
        }
        return;
    }
    if let Some(message) = &report.message {
        eprintln!("error: {}", message);
    }
    // L0: when --explain is set, lead with the plain-language projection and
    // the L2 detail list. Otherwise fall back to the raw machine summary.
    if let Some(explanation) = &report.explanation {
        eprintln!("{}", explanation.headline);
        eprintln!("{}", explanation.next_step);
        for detail in &explanation.details {
            eprintln!("  - {}", detail);
        }
    } else if let Some(diagnosis) = &report.diagnosis {
        eprintln!("diagnosis: {}", diagnosis.summary);
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
