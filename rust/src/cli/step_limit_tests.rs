//! Native tests for `ajisai run --step-limit <N>`: the host-configurable
//! execution step budget (water level, SPECIFICATION.html §5.3). The budget
//! is a runtime safety control, not language semantics, so these tests only
//! assert *whether* `ExecutionLimitExceeded` is raised — raising the budget
//! lets a legitimate guarded tail recursion run to completion, lowering it
//! sandboxes even a short program — plus the CLI usage-error contract
//! (exit 2 for zero / non-numeric values). Deliberately not part of the
//! conformance suite: conformance must not depend on any budget value.

use crate::error::AjisaiError;
use crate::interpreter::{Interpreter, DEFAULT_MAX_EXECUTION_STEPS};

/// Guarded tail-recursive countdown. `n` iterations cost `n + 2` steps
/// (measured: `n=100_000` charges 100_002) — the guard check, decrement and
/// tail call collapse to one charged step per iteration, plus two for the
/// literal push and the terminal branch; the trampoline itself is O(1)
/// native stack, so only the step budget stops it.
///
/// 200,000 was chosen when the default budget was 100,000: any n past 50,000
/// would have exceeded it. It no longer does — the default is now derived
/// from a host time budget (`runtime_limits::DEFAULT_MAX_EXECUTION_STEPS`,
/// `docs/dev/host-profile-derivation-handoff.md` §4) and grew to the tens of
/// millions, so 200,000 completes comfortably under it. Kept as the fixed
/// probe for the "raised limit" direction below, where the point is that an
/// explicit `--step-limit` still works, not that it is large.
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

/// The CLI's default step budget really is `DEFAULT_MAX_EXECUTION_STEPS`, not
/// some other number nobody re-checks. Structural rather than a full run:
/// `DEFAULT_MAX_EXECUTION_STEPS` is now in the tens of millions
/// (`runtime_limits::DEFAULT_MAX_EXECUTION_STEPS`), and actually exhausting a
/// budget that size costs wall time proportional to dispatch speed by
/// construction — there is no cheap operand that reaches a step *count* the
/// way one wide BigInt reaches a work ceiling. The `#[ignore]`d test further
/// down proves the real default end to end for whoever runs it deliberately.
#[test]
fn cli_run_without_step_limit_uses_the_documented_default() {
    assert_eq!(
        Interpreter::new().max_execution_steps(),
        DEFAULT_MAX_EXECUTION_STEPS,
        "`ajisai run` with no `--step-limit` falls through to \
         `Interpreter::new()`'s default, which must be the documented one"
    );
}

#[test]
fn down_probe_succeeds_with_an_explicit_step_limit() {
    // 1,000,000 is no longer a *raised* limit — `DEFAULT_MAX_EXECUTION_STEPS`
    // is far larger now — but the point of this test was always that an
    // explicit `--step-limit` is honored, not that the number is big.
    let path = write_program("explicit", DOWN_PROBE);
    let code = run_cli(&["run", path.to_str().unwrap(), "--step-limit", "1000000"]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        code, 0,
        "200000 DOWN must complete under an explicit --step-limit 1000000"
    );
}

/// The real, end-to-end version of the test above: just over
/// `DEFAULT_MAX_EXECUTION_STEPS` iterations of the trampoline, with no
/// `--step-limit`, must still exceed the real default. At the measured floor
/// of ~773-890 steps/ms (release; far slower in the debug build this
/// normally runs under) this takes on the order of a minute, so it is
/// `#[ignore]`d rather than run on every `cargo test`. Run deliberately with
/// `cargo test -- --ignored` after touching `DEFAULT_MAX_EXECUTION_STEPS` or
/// the interpreter's dispatch path.
#[test]
#[ignore = "exhausting the real default costs real wall time by construction; run explicitly"]
fn down_probe_exceeds_the_real_default_budget_without_step_limit() {
    // Each `DOWN` iteration costs ~1 step, not 2 — the guard/decrement/tail
    // call collapse to one charged step apiece, measured empirically
    // (`n=100_000` charges 100_002 steps). `+ 10` covers the fixed overhead
    // (the literal push, the `DEF`, the terminal `IDLE`/`'done'` branch) with
    // margin to spare.
    let iterations = DEFAULT_MAX_EXECUTION_STEPS + 10;
    let probe = format!(
        "{{\n{{ [ 0 ] > | [ 1 ] - DOWN }}\n{{ IDLE | [ 'done' ] }} COND\n}} 'DOWN' DEF\n{iterations} DOWN"
    );
    let path = write_program("real-default", &probe);
    let code = run_cli(&["run", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        code, 1,
        "{iterations} DOWN must exceed the real default budget of \
         {DEFAULT_MAX_EXECUTION_STEPS} steps"
    );
}

/// Twelve word executions (a step counts a *word* execution, not a literal),
/// so this trips a 10-step budget but is far below the default either way.
const SIMPLE_PROGRAM: &str = "[ 1 ] [ 1 ] + [ 1 ] + [ 1 ] + [ 1 ] + [ 1 ] + [ 1 ] + \
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
    let mut interp = Interpreter::new();
    interp.set_max_execution_steps(10);
    let err = crate::agent::block_on(interp.execute(SIMPLE_PROGRAM))
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
