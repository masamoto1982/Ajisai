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
//! ajisai agent <operation> <file.ajisai>   # common JSON envelope for host adapters
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

mod repl;
#[cfg(test)]
mod step_limit_tests;
mod test_runner;

use crate::agent::api as agent_api;
use crate::agent::report::{self, Report};
use crate::agent::{
    block_on, check_structure, contract_decl, contract_report, error_report, resolve_words, Opts,
};
use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use crate::interpreter::Interpreter;

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
  agent <operation> <file.ajisai|->
                                  Stable source-to-JSON host boundary. Operations:
                                  compute, check, infer-contracts. `-` reads the
                                  program from standard input, so an embedding
                                  host needs no temporary file
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
                                  the host's derived step budget
                                  (interpreter::DEFAULT_MAX_EXECUTION_STEPS,
                                  docs/dev/mcp-host-profiles.md). A host
                                  safety control, not a language semantic

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
            // A bare `-` is the conventional name for standard input, not an
            // option; every command that takes a path accepts it as one.
            "-" => positional.push("-"),
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
        ("agent", [operation, path]) => cmd_agent(operation, path, &opts),
        ("test", [path]) => test_runner::cmd_test(path, &opts),
        ("repl", []) => repl::cmd_repl(&opts),
        ("version", []) => cmd_version(json),
        _ => {
            eprintln!("{}", USAGE);
            2
        }
    }
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
            runtime_limits: None,
        },
    ));
    emit(response.report(), opts);
    response.exit_code()
}

/// Read one `key=value` evidence entry.
fn evidence_value<'a>(evidence: &'a [String], key: &str) -> Option<&'a str> {
    evidence
        .iter()
        .find_map(|entry| entry.strip_prefix(key)?.strip_prefix('='))
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

    let resolved = resolve_words(&interp, &tokens);
    let unknown = &resolved.unknown;
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
        diagnosis.with_user_vocabulary(resolved.locally_defined.iter().map(String::as_str));
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
            println!(
                "    cost: steps={} numeric={} collection={}",
                r.cost_steps, r.cost_numeric, r.cost_collection
            );
            println!("    {}", r.suggested);
        }
    }
    0
}

/// Common JSON boundary consumed by host adapters. Unlike the compatibility
/// CLI commands, every operation returns the same top-level envelope shape.
fn cmd_agent(operation: &str, path: &str, opts: &Opts) -> i32 {
    // `-` reads the program from standard input. A host adapter that already
    // holds the source in memory should not have to invent a temporary file to
    // hand it over: writing one needs a writable temporary directory, leaves
    // the program on disk for as long as the call runs, and buys nothing the
    // pipe does not already give.
    let source = if path == "-" {
        use std::io::Read;
        let mut buffer = String::new();
        match std::io::stdin().read_to_string(&mut buffer) {
            Ok(_) => buffer,
            Err(e) => {
                eprintln!("ajisai: cannot read standard input: {}", e);
                return 2;
            }
        }
    } else {
        match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(e) => {
                eprintln!("ajisai: cannot read {}: {}", path, e);
                return 2;
            }
        }
    };
    let (document, exit_code) = match operation {
        "compute" => {
            let response = block_on(agent_api::compute(
                &source,
                agent_api::ComputeOptions {
                    step_limit: opts.step_limit,
                    runtime_limits: Some(agent_api::LOCAL_AGENT_RUNTIME_LIMITS),
                },
            ));
            (response.to_json(), response.exit_code())
        }
        "check" => {
            let response = agent_api::check(&source, true);
            (response.to_json(), response.exit_code())
        }
        "infer-contracts" => (agent_api::infer_contracts(&source).to_json(), 0),
        _ => {
            eprintln!("unknown agent operation: {operation}");
            return 2;
        }
    };
    println!("{}", pretty(&document));
    exit_code
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
        if !diagnosis.candidates.is_empty() {
            eprintln!("  did you mean: {}", diagnosis.candidates.join(", "));
        }
        if let Some(facts) = &diagnosis.resource_limit {
            match facts.observed {
                Some(observed) => eprintln!(
                    "  limit {}: {} exceeds {}",
                    facts.resource, observed, facts.limit
                ),
                None => eprintln!("  limit {}: {}", facts.resource, facts.limit),
            }
        }
        // The terminal is an English surface; the same checks reach a Japanese
        // reader through the `ja` locale of the JSON envelope.
        for check in &diagnosis.next_checks {
            eprintln!("  - {}: {}", check.title.en, check.detail.en);
        }
    }
}

fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}
