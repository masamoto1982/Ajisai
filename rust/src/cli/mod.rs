//! Headless `ajisai` CLI: the agent-facing write → run → read-structured-error
//! loop, entirely in a terminal.
//!
//! Commands (see `docs/dev/agent-cli-output-contract.md` for the JSON
//! output contract):
//!
//! ```text
//! ajisai run <file.ajisai>                 # human-readable
//! ajisai check <file.ajisai>               # tokenize + parse + resolve, no execution
//! ajisai contract <file.ajisai>            # report inferred word contracts, no execution
//! ajisai agent <operation> <file.ajisai>   # the one JSON boundary for host adapters
//!   operations: compute, check, infer-contracts, outcomes
//! ajisai test <file-or-dir> [--json]       # execute `#@` host test directives
//! ajisai repl [--json]                     # persistent interactive session
//! ajisai version [--json]
//! ```
//!
//! Exit codes: 0 = success, 1 = language error (diagnosis emitted),
//! 2 = CLI usage error. `agent` (and `test`/`repl`/`version` with `--json`)
//! write exactly one JSON document to stdout and nothing else (pipe-safe);
//! usage errors go to stderr. `run`, `check` and `contract` are the same
//! operations for a person to read: their machine-readable form is `agent`,
//! so they take no `--json` of their own.
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
use crate::agent::{block_on, contract_report, LimitProfile, Opts};

const USAGE: &str = "Usage: ajisai <command> [options]

Commands:
  run <file.ajisai> [--step-limit <N>]
                                  Execute a program file
  check <file.ajisai> [--contract]
                                  Tokenize, parse and resolve only (no
                                  execution). With --contract, also check each
                                  `#:contract` declaration against the contract
                                  inferred from the Core Words it calls
  contract <file.ajisai>          Report each user word's inferred contract
                                  (arity, purity, NIL, determinism) plus a
                                  paste-ready `#:contract` line (no execution)
  agent <operation> <file.ajisai|-> [--limits <agent|trusted>] [--step-limit <N>]
                                  The one source-to-JSON host boundary. Operations:
                                  compute, check, infer-contracts, outcomes. `-`
                                  reads the program from standard input, so an
                                  embedding host needs no temporary file. `check`
                                  always verifies `#:contract` declarations
  test <file-or-dir> [--json]     Run test files, checking each program against
                                  its `#@` directive comments (status/stack/
                                  output/error). Exit 1 if any test fails
  repl [--json]                   Interactive session; stack and definitions
                                  persist. :help for commands, :quit to leave
  version [--json]                Print version information

Options:
  --json                          With `test`/`repl`/`version`: emit JSON
                                  (pipe-safe). `agent` always emits JSON
  --contract                      With `check`: verify `#:contract` word
                                  declarations against the inferred contract
                                  (exit 1 on a contradiction). The check is
                                  conservative: an unanalyzable body is
                                  reported as `cannot verify`, never as passed
  --limits <agent|trusted>        With `agent compute`/`agent outcomes`: the
                                  resource ceilings. `agent` (default) is the
                                  tighter profile for untrusted, generated
                                  programs; `trusted` is the interpreter
                                  default `run` uses
  --step-limit <N>                With `run`/`agent compute`/`agent outcomes`:
                                  override the execution step
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
    let mut limits = LimitProfile::Agent;
    let mut limits_given = false;
    let mut step_limit: Option<usize> = None;
    let mut positional: Vec<&str> = Vec::new();
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--contract" => contract = true,
            "--limits" => match iter.next().map(String::as_str) {
                Some("agent") => (limits, limits_given) = (LimitProfile::Agent, true),
                Some("trusted") => (limits, limits_given) = (LimitProfile::Trusted, true),
                _ => {
                    eprintln!("--limits expects `agent` or `trusted`\n\n{}", USAGE);
                    return 2;
                }
            },
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
        limits,
    };
    if json && matches!(command.as_str(), "run" | "check" | "contract") {
        // Name the exact equivalent, profile included: `run` executes under
        // the trusted ceilings, which `agent compute` applies only on request.
        let equivalent = match command.as_str() {
            "run" => "`ajisai agent compute --limits trusted`",
            "check" => "`ajisai agent check` (it always verifies `#:contract` declarations)",
            _ => "`ajisai agent infer-contracts`",
        };
        eprintln!(
            "`{command}` is the human-readable form; its JSON form is {equivalent}\n\n{USAGE}"
        );
        return 2;
    }
    // A flag the command does not read would be accepted and silently
    // ignored — `run file --limits agent` running under the trusted ceilings
    // regardless — so it is refused instead.
    let agent_op = |op: &str| command == "agent" && positional.first() == Some(&op);
    let misplaced = [
        (
            "--limits",
            limits_given,
            agent_op("compute") || agent_op("outcomes"),
        ),
        (
            "--step-limit",
            step_limit.is_some(),
            command == "run" || agent_op("compute") || agent_op("outcomes"),
        ),
        ("--contract", contract, command == "check"),
    ];
    for (flag, given, applies) in misplaced {
        if given && !applies {
            let where_ = if command == "agent" {
                format!("agent {}", positional.first().copied().unwrap_or(""))
            } else {
                command.to_string()
            };
            eprintln!("`{flag}` does not apply to `{where_}`\n\n{USAGE}");
            return 2;
        }
    }
    match (command.as_str(), positional.as_slice()) {
        ("run", [path]) => cmd_run(path, &opts),
        ("check", [path]) => cmd_check(path, &opts),
        ("contract", [path]) => cmd_contract(path),
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
    emit(response.report());
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
    // The same check `agent check` answers, read out for a person: one
    // implementation, two renderings.
    let response = agent_api::check(&source, opts.contract);
    let report = response.report();
    if report.diagnosis.is_some() {
        emit(report);
        return response.exit_code();
    }
    let status = if report.status == "ok" { "ok" } else { "fail" };
    println!("{}: {}", status, path);
    let findings = report
        .contract_decls
        .as_ref()
        .and_then(|decls| decls["findings"].as_array());
    for finding in findings.into_iter().flatten() {
        eprintln!(
            "  [{}] {}",
            finding["severity"].as_str().unwrap_or(""),
            finding["message"].as_str().unwrap_or("")
        );
    }
    response.exit_code()
}

/// `ajisai contract <file>`: report each user word's inferred contract
/// (`interpreter::word_contract`), the reporting companion to `check --contract`
/// (P2). Registers definitions and imports without executing any word body or
/// top-level code. Observational — a well-formed file always exits 0.
fn cmd_contract(path: &str) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("ajisai: cannot read {}: {}", path, e);
            return 2;
        }
    };
    {
        let reports = contract_report::report_contracts(&source);
        if reports.is_empty() {
            println!("{}: no user words defined", path);
        }
        for r in &reports {
            let count = |n: Option<u16>| n.map_or("variable".to_string(), |n| n.to_string());
            println!(
                "{} : inputs={} outputs={} partiality={} purity={} determinism={} [{}]",
                r.name,
                count(r.inputs),
                count(r.outputs),
                r.partiality,
                r.purity,
                r.determinism,
                r.confidence
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

/// The one JSON boundary consumed by host adapters: every operation returns
/// the same top-level envelope shape.
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
            let response = block_on(agent_api::compute(&source, compute_options(opts)));
            (response.to_json(), response.exit_code())
        }
        "check" => {
            let response = agent_api::check(&source, true);
            (response.to_json(), response.exit_code())
        }
        "infer-contracts" => {
            let response = agent_api::infer_contracts(&source);
            (response.to_json(), response.exit_code())
        }
        "outcomes" => (
            agent_api::predict_outcomes(&source, compute_options(opts)).to_json(),
            0,
        ),
        _ => {
            eprintln!("unknown agent operation: {operation}");
            return 2;
        }
    };
    println!("{}", pretty(&document));
    exit_code
}

/// The ceilings an `agent compute`/`agent outcomes` runs under, from the
/// command line: the step budget and the chosen limit profile.
fn compute_options(opts: &Opts) -> agent_api::ComputeOptions {
    agent_api::ComputeOptions {
        step_limit: opts.step_limit,
        runtime_limits: match opts.limits {
            LimitProfile::Agent => Some(agent_api::LOCAL_AGENT_RUNTIME_LIMITS),
            LimitProfile::Trusted => None,
        },
    }
}

fn emit(report: &Report) {
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
