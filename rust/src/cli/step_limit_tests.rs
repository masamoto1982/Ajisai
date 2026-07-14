//! Native tests for `ajisai run --step-limit <N>`: the host-configurable
//! execution step budget (water level, SPECIFICATION.html §5.3). The budget
//! is a runtime safety control, not language semantics, so these tests only
//! assert *whether* `ExecutionLimitExceeded` is raised — raising the budget
//! lets a legitimate guarded tail recursion run to completion, lowering it
//! sandboxes even a short program — plus the CLI usage-error contract
//! (exit 2 for zero / non-numeric values). Deliberately not part of the
//! conformance suite: conformance must not depend on any budget value.

use crate::error::AjisaiError;
use crate::interpreter::Interpreter;
use std::sync::Arc;

/// Guarded tail-recursive countdown. 200,000 iterations exceed the default
/// 100,000-step budget but complete under a raised one — the trampoline
/// itself is O(1) native stack, so only the step budget stops it.
const DOWN_PROBE: &str = "{
{ [ 0 ] > | [ 1 ] - DOWN }
{ IDLE | [ 'done' ] } COND
} 'DOWN' DEF
200000 DOWN";

/// Write `source` to a unique temp file and return its path.
fn write_program(name: &str, source: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "ajisai-step-limit-{}-{}.ajisai",
        std::process::id(),
        name
    ));
    std::fs::write(&path, source).expect("temp program must be writable");
    path
}

fn run_cli(args: &[&str]) -> i32 {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    super::run(&args)
}

#[test]
fn down_probe_exceeds_default_budget_without_step_limit() {
    let path = write_program("default", DOWN_PROBE);
    let code = run_cli(&["run", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 1, "200000 DOWN must exceed the default 100,000 budget");
}

#[test]
fn down_probe_succeeds_with_raised_step_limit() {
    let path = write_program("raised", DOWN_PROBE);
    let code = run_cli(&["run", path.to_str().unwrap(), "--step-limit", "1000000"]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0, "200000 DOWN must complete under --step-limit 1000000");
}

/// Twelve word executions (a step counts a *word* execution, not a literal),
/// so this trips a 10-step budget but is far below the 100,000 default.
const SIMPLE_PROGRAM: &str =
    "[ 1 ] [ 1 ] + [ 1 ] + [ 1 ] + [ 1 ] + [ 1 ] + [ 1 ] + \
     [ 1 ] + [ 1 ] + [ 1 ] + [ 1 ] + [ 1 ] + [ 1 ] +";

#[test]
fn lowered_step_limit_sandboxes_a_simple_program() {
    let path = write_program("lowered", SIMPLE_PROGRAM);
    let code = run_cli(&["run", path.to_str().unwrap(), "--step-limit", "10"]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 1, "a lowered budget must stop even a simple program");
}

/// The lowered budget must fail with `ExecutionLimitExceeded` specifically
/// (the sandbox use case), not some other error the CLI exit code would mask.
#[test]
fn lowered_budget_raises_execution_limit_exceeded() {
    let mut interp = Interpreter::with_host(Arc::new(super::host::CliHostEnv));
    interp.set_max_execution_steps(10);
    let err = super::block_on(interp.execute(SIMPLE_PROGRAM))
        .expect_err("a 10-step budget must stop this program");
    assert!(
        matches!(err, AjisaiError::ExecutionLimitExceeded { limit: 10 }),
        "expected ExecutionLimitExceeded {{ limit: 10 }}, got: {err}"
    );
}

#[test]
fn step_limit_zero_is_a_usage_error() {
    let path = write_program("zero", "[ 1 ]");
    let code = run_cli(&["run", path.to_str().unwrap(), "--step-limit", "0"]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 2);
}

#[test]
fn step_limit_non_numeric_is_a_usage_error() {
    let path = write_program("nonnum", "[ 1 ]");
    let code = run_cli(&["run", path.to_str().unwrap(), "--step-limit", "many"]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 2);
}

#[test]
fn step_limit_missing_value_is_a_usage_error() {
    let path = write_program("missing", "[ 1 ]");
    let code = run_cli(&["run", path.to_str().unwrap(), "--step-limit"]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 2);
}
