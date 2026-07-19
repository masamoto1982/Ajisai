//! `ajisai test` — host-side test runner (Phase 8A).
//!
//! Runs Ajisai test files and checks each program's result against
//! expectations declared as `#@` **directive comments**. It adds NO language
//! word (no `ASSERT` in Core): the expectations live in host-read comments that
//! the interpreter ignores as ordinary `#` comments (SPEC §3.4), so the test
//! harness stays strictly separate from language semantics (§15.1). The runner
//! drives the production Core, the same execution path as `run`.
//!
//! Directives (one per line, anywhere in the file):
//!
//! ```text
//! #@ status ok | error     expected outcome (default: ok)
//! #@ stack  <display>       expected final stack, space-joined display strings
//! #@ output <line>          expected PRINT payload (repeatable; full list must match)
//! #@ error  <substring>     the run must fail with a message containing <substring>
//!                           (implies `status error`)
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::interpreter::Interpreter;

use super::{block_on, host, print_payloads, stack_display, Opts};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedStatus {
    Ok,
    Error,
}

/// Expectations parsed from a test file's `#@` directives.
#[derive(Debug, Default)]
struct Expectations {
    /// `None` means the default (success).
    status: Option<ExpectedStatus>,
    /// Expected space-joined stack display; `None` leaves the stack unchecked.
    stack: Option<String>,
    /// Expected `PRINT` payloads in order; `Some` means the full list must match.
    output: Option<Vec<String>>,
    /// Substring the error message must contain (implies `status error`).
    error_contains: Option<String>,
    /// Malformed directives — reported as failures so typos never pass silently.
    directive_errors: Vec<String>,
}

fn parse_directives(source: &str) -> Expectations {
    let mut exp = Expectations::default();
    for raw in source.lines() {
        let Some(body) = raw.trim_start().strip_prefix("#@") else {
            continue;
        };
        let body = body.trim();
        if body.is_empty() {
            exp.directive_errors
                .push("empty `#@` directive".to_string());
            continue;
        }
        let (keyword, value) = match body.split_once(char::is_whitespace) {
            Some((k, v)) => (k, v.trim()),
            None => (body, ""),
        };
        match keyword {
            "status" => match value {
                "ok" => exp.status = Some(ExpectedStatus::Ok),
                "error" => exp.status = Some(ExpectedStatus::Error),
                other => exp
                    .directive_errors
                    .push(format!("unknown status `{other}` (expected ok|error)")),
            },
            "stack" => exp.stack = Some(value.to_string()),
            "output" => exp
                .output
                .get_or_insert_with(Vec::new)
                .push(value.to_string()),
            "error" => {
                exp.error_contains = Some(value.to_string());
                exp.status = Some(ExpectedStatus::Error);
            }
            other => exp
                .directive_errors
                .push(format!("unknown directive `{other}`")),
        }
    }
    exp
}

/// The result of running one test file.
#[derive(Debug)]
struct TestOutcome {
    name: String,
    /// Empty when the test passed.
    failures: Vec<String>,
}

impl TestOutcome {
    fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Run one test program and check it against its directives. Pure w.r.t. the
/// filesystem, so it is testable without temp files.
fn run_test_source(name: &str, source: &str) -> TestOutcome {
    let exp = parse_directives(source);
    let mut failures = exp.directive_errors.clone();

    let mut interp = Interpreter::with_host(Arc::new(host::CliHostEnv));
    let result = block_on(interp.execute(source));
    let error_message = result.as_ref().err().map(|e| e.to_string());
    let actual_status = if result.is_ok() {
        ExpectedStatus::Ok
    } else {
        ExpectedStatus::Error
    };

    let expected_status = exp.status.unwrap_or(ExpectedStatus::Ok);
    if actual_status != expected_status {
        failures.push(match expected_status {
            ExpectedStatus::Ok => format!(
                "expected success but the program failed: {}",
                error_message.clone().unwrap_or_default()
            ),
            ExpectedStatus::Error => "expected an error but the program succeeded".to_string(),
        });
    }

    if let Some(substring) = &exp.error_contains {
        match &error_message {
            Some(message) if message.contains(substring) => {}
            Some(message) => failures.push(format!(
                "error message `{message}` does not contain `{substring}`"
            )),
            // A missing error is already reported by the status mismatch above.
            None => {}
        }
    }

    if let Some(expected_stack) = &exp.stack {
        let actual = stack_display(&interp).join(" ");
        if &actual != expected_stack {
            failures.push(format!(
                "stack mismatch\n      expected: {expected_stack}\n      actual:   {actual}"
            ));
        }
    }

    if let Some(expected_output) = &exp.output {
        let actual = print_payloads(&interp);
        if &actual != expected_output {
            failures.push(format!(
                "output mismatch\n      expected: {expected_output:?}\n      actual:   {actual:?}"
            ));
        }
    }

    TestOutcome {
        name: name.to_string(),
        failures,
    }
}

/// Collect `*.ajisai` files under a directory, recursively, in a deterministic
/// (sorted) order.
fn collect_dir(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_dir(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "ajisai") {
            out.push(path);
        }
    }
    Ok(())
}

fn render_report(outcomes: &[TestOutcome], opts: &Opts) {
    let passed = outcomes.iter().filter(|o| o.passed()).count();
    let failed = outcomes.len() - passed;

    if opts.json {
        let results: Vec<serde_json::Value> = outcomes
            .iter()
            .map(|o| {
                serde_json::json!({
                    "name": o.name,
                    "passed": o.passed(),
                    "failures": o.failures,
                })
            })
            .collect();
        let doc = serde_json::json!({
            "schemaVersion": super::report::SCHEMA_VERSION,
            "status": if failed == 0 { "ok" } else { "error" },
            "total": outcomes.len(),
            "passed": passed,
            "failed": failed,
            "results": results,
        });
        println!("{}", super::pretty(&doc));
    } else {
        for outcome in outcomes {
            if outcome.passed() {
                println!("PASS {}", outcome.name);
            } else {
                println!("FAIL {}", outcome.name);
                for failure in &outcome.failures {
                    println!("    {failure}");
                }
            }
        }
        println!(
            "\n{} test{}: {} passed, {} failed",
            outcomes.len(),
            if outcomes.len() == 1 { "" } else { "s" },
            passed,
            failed
        );
    }
}

/// `ajisai test <file-or-dir>`: run each test file and check its `#@`
/// directives. A file is run whatever its extension; a directory is walked for
/// `*.ajisai` files. Exit 0 when every test passes, 1 when any fails, 2 on a
/// usage or read error.
pub(crate) fn cmd_test(path_arg: &str, opts: &Opts) -> i32 {
    let path = Path::new(path_arg);
    if !path.exists() {
        eprintln!("ajisai: no such file or directory: {}", path_arg);
        return 2;
    }

    let mut files: Vec<PathBuf> = Vec::new();
    if path.is_dir() {
        if let Err(e) = collect_dir(path, &mut files) {
            eprintln!("ajisai: cannot read {}: {}", path_arg, e);
            return 2;
        }
        if files.is_empty() {
            eprintln!("ajisai test: no .ajisai files under {}", path_arg);
            return 2;
        }
    } else {
        files.push(path.to_path_buf());
    }

    let outcomes: Vec<TestOutcome> = files
        .iter()
        .map(|file| {
            let name = file.display().to_string();
            match fs::read_to_string(file) {
                Ok(source) => run_test_source(&name, &source),
                Err(e) => TestOutcome {
                    name,
                    failures: vec![format!("cannot read: {e}")],
                },
            }
        })
        .collect();

    let all_passed = outcomes.iter().all(TestOutcome::passed);
    render_report(&outcomes, opts);
    if all_passed {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passing_test_with_stack_and_output() {
        let src = "#@ stack [ 5/1 ]\n#@ output [ 9/1 ]\n[ 9 ] PRINT [ 2 ] [ 3 ] +";
        let outcome = run_test_source("t", src);
        assert!(outcome.passed(), "failures: {:?}", outcome.failures);
    }

    #[test]
    fn stack_mismatch_fails() {
        let src = "#@ stack [ 6/1 ]\n[ 2 ] [ 3 ] +";
        let outcome = run_test_source("t", src);
        assert!(!outcome.passed());
        assert!(outcome.failures[0].contains("stack mismatch"));
    }

    #[test]
    fn output_mismatch_fails() {
        let src = "#@ output nope\n[ 1 ] PRINT";
        let outcome = run_test_source("t", src);
        assert!(!outcome.passed());
        assert!(outcome.failures[0].contains("output mismatch"));
    }

    #[test]
    fn default_status_is_success() {
        // No directives: the program must merely run without error.
        let outcome = run_test_source("t", "[ 1 ] [ 2 ] +");
        assert!(outcome.passed());
    }

    #[test]
    fn unexpected_error_fails_by_default() {
        let outcome = run_test_source("t", "NOSUCHWORD");
        assert!(!outcome.passed());
        assert!(outcome.failures[0].contains("expected success"));
    }

    #[test]
    fn expected_error_passes() {
        let outcome = run_test_source("t", "#@ status error\nNOSUCHWORD");
        assert!(outcome.passed(), "failures: {:?}", outcome.failures);
    }

    #[test]
    fn error_substring_matches() {
        let outcome = run_test_source("t", "#@ error Unknown word\nNOSUCHWORD");
        assert!(outcome.passed(), "failures: {:?}", outcome.failures);
    }

    #[test]
    fn error_substring_mismatch_fails() {
        let outcome = run_test_source("t", "#@ error totally different\nNOSUCHWORD");
        assert!(!outcome.passed());
        assert!(outcome.failures[0].contains("does not contain"));
    }

    #[test]
    fn expected_error_but_success_fails() {
        let outcome = run_test_source("t", "#@ status error\n[ 1 ]");
        assert!(!outcome.passed());
        assert!(outcome.failures[0].contains("expected an error"));
    }

    #[test]
    fn unknown_directive_is_reported() {
        let outcome = run_test_source("t", "#@ bogus whatever\n[ 1 ]");
        assert!(!outcome.passed());
        assert!(outcome
            .failures
            .iter()
            .any(|f| f.contains("unknown directive")));
    }

    #[test]
    fn ordinary_comments_are_not_directives() {
        // A plain `#` comment is not a `#@` directive and must be ignored.
        let outcome = run_test_source("t", "# stack [ 9/9 ]\n#@ stack [ 5/1 ]\n[ 2 ] [ 3 ] +");
        assert!(outcome.passed(), "failures: {:?}", outcome.failures);
    }
}
