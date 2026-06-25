//! Empirical A/B benchmark for internal tail-call elimination (the "internal
//! GOTO" backward-jump trampoline).
//!
//! Run with:  `cargo run --release --example tail_call_bench`
//!
//! It exercises a guarded tail-recursive word (`DOWN`, a countdown that recurses
//! in the tail of a `COND` clause) under two configurations of the *same*
//! interpreter:
//!   * ON  — tail-call elimination engaged (default).
//!   * OFF — legacy native recursion (`set_tail_call_enabled(false)`).
//!
//! Two things are measured:
//!   1. Reach: how deep each configuration can recurse before failing. OFF is
//!      capped by `MAX_USER_WORD_DEPTH` (256); ON is bounded only by the
//!      execution step budget, in O(1) native stack.
//!   2. Per-iteration cost at an equal depth both can complete (250).

use std::time::Instant;

use ajisai_core::interpreter::Interpreter;

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

const DEF: &str = "{\n  { [ 0 ] > | [ 1 ] - DOWN }\n  { IDLE | [ 0 ] } COND\n} 'DOWN' DEF";

/// Run `[ depth ] DOWN DROP` once on a freshly-prepared interpreter and report
/// whether it completed.
fn run_once(tail_call: bool, depth: u64, max_steps: usize) -> Result<(), String> {
    let mut interp = Interpreter::new();
    interp.set_tail_call_enabled(tail_call);
    interp.set_max_execution_steps(max_steps);
    block_on(interp.execute(DEF)).map_err(|e| e.to_string())?;
    block_on(interp.execute(&format!("[ {} ] DOWN", depth))).map_err(|e| e.to_string())
}

/// Exact largest depth that completes within `[0, cap]`, by binary search.
/// A generous step budget ensures the depth guard (OFF) or the step budget
/// itself (ON) — not an accidental low budget — is what bounds the result.
fn max_reach(tail_call: bool, cap: u64) -> u64 {
    let steps = (cap as usize).saturating_mul(64).saturating_add(1_000_000);
    if run_once(tail_call, cap, steps).is_ok() {
        return cap;
    }
    let (mut lo, mut hi) = (0u64, cap); // lo completes, hi fails
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if run_once(tail_call, mid, steps).is_ok() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

fn time_loops(tail_call: bool, cond_dispatch: bool, depth: u64, reps: u32) -> std::time::Duration {
    time_loops_full(tail_call, cond_dispatch, true, depth, reps)
}

fn time_loops_full(
    tail_call: bool,
    cond_dispatch: bool,
    compiled_clause: bool,
    depth: u64,
    reps: u32,
) -> std::time::Duration {
    let steps = 100_000_000;
    // Prepare one interpreter; re-run the same loop `reps` times in-process.
    let mut interp = Interpreter::new();
    interp.set_tail_call_enabled(tail_call);
    interp.set_cond_dispatch_enabled(cond_dispatch);
    interp.set_compiled_clause_enabled(compiled_clause);
    interp.set_max_execution_steps(steps);
    block_on(interp.execute(DEF)).unwrap();
    let line = format!("[ {} ] DOWN", depth);
    let t0 = Instant::now();
    for _ in 0..reps {
        block_on(interp.execute(&line)).unwrap();
        interp.update_stack(Vec::new()); // discard the loop's result value
    }
    t0.elapsed()
}

fn main() {
    println!("== internal GOTO (tail-call elimination) A/B bench ==\n");

    println!("-- Reach (deepest guarded tail recursion that completes) --");
    let off_reach = max_reach(false, 1_000_000);
    let on_reach = max_reach(true, 1_000_000);
    println!("  OFF (native recursion):  {off_reach:>9}   (capped by MAX_USER_WORD_DEPTH guard)");
    println!("  ON  (backward jump):     {on_reach:>9}   (O(1) native stack; step-budget bounded)");
    println!(
        "  ratio:                   {:>9}x deeper\n",
        on_reach / off_reach.max(1)
    );

    let depth = 250u64;
    let reps = 2000u32;
    let iters = (depth as u128) * (reps as u128);
    let ns = |d: std::time::Duration| d.as_nanos() as f64 / iters as f64;

    println!("-- Tail-call: native recursion vs backward jump (depth 250) --");
    // Warm up plan caches.
    let _ = time_loops(false, true, depth, 50);
    let _ = time_loops(true, true, depth, 50);
    let tc_off = time_loops(false, true, depth, reps);
    let tc_on = time_loops(true, true, depth, reps);
    println!("  {reps} loops × depth {depth} = {iters} iterations");
    println!(
        "  tail-call OFF: {:>8.3} ms  ({:.1} ns/iter)",
        tc_off.as_secs_f64() * 1e3,
        ns(tc_off)
    );
    println!(
        "  tail-call ON : {:>8.3} ms  ({:.1} ns/iter)",
        tc_on.as_secs_f64() * 1e3,
        ns(tc_on)
    );
    println!(
        "  speedup: {:.2}x\n",
        tc_off.as_secs_f64() / tc_on.as_secs_f64()
    );

    println!(
        "-- COND dispatch: dynamic collect vs precompiled jump table (depth 250, tail-call ON) --"
    );
    let _ = time_loops(true, false, depth, 50);
    let cd_off = time_loops(true, false, depth, reps);
    let cd_on = time_loops(true, true, depth, reps);
    println!(
        "  dispatch OFF (dynamic): {:>8.3} ms  ({:.1} ns/iter)",
        cd_off.as_secs_f64() * 1e3,
        ns(cd_off)
    );
    println!(
        "  dispatch ON  (jump tbl): {:>8.3} ms  ({:.1} ns/iter)",
        cd_on.as_secs_f64() * 1e3,
        ns(cd_on)
    );
    println!(
        "  speedup: {:.2}x\n",
        cd_off.as_secs_f64() / cd_on.as_secs_f64()
    );

    println!(
        "-- Compiled clause body: interpreted vs compiled (depth 250, tail-call + dispatch ON) --"
    );
    let _ = time_loops_full(true, true, false, depth, 50);
    let cc_off = time_loops_full(true, true, false, depth, reps);
    let cc_on = time_loops_full(true, true, true, depth, reps);
    println!(
        "  clause OFF (interpreted): {:>8.3} ms  ({:.1} ns/iter)",
        cc_off.as_secs_f64() * 1e3,
        ns(cc_off)
    );
    println!(
        "  clause ON  (compiled):    {:>8.3} ms  ({:.1} ns/iter)",
        cc_on.as_secs_f64() * 1e3,
        ns(cc_on)
    );
    println!(
        "  speedup: {:.2}x",
        cc_off.as_secs_f64() / cc_on.as_secs_f64()
    );
}
