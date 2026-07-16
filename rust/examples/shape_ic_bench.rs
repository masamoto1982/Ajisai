//! Empirical A/B benchmark for the call-site shape inline cache
//! (hidden-class-style builtin call specialization, `shape_ic.rs`) and the
//! compile-time builtin key pre-resolution it rides on.
//!
//! Run with:  `cargo run --release --example shape_ic_bench`
//!
//! It drives the same guarded tail-recursive countdown loop as
//! `tail_call_bench` (per iteration: one comparison, one subtraction, one
//! COND dispatch) with the shape IC toggled ON/OFF on the same interpreter
//! configuration, and reports the IC hit/miss counters. A second section
//! measures Record layout interning: same-layout record construction and
//! equality with the shared `Arc<RecordShape>`.

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

struct LoopTiming {
    elapsed: std::time::Duration,
    ic_hits: u64,
    ic_misses: u64,
}

fn time_loops(shape_ic: bool, depth: u64, reps: u32) -> LoopTiming {
    let mut interp = Interpreter::new();
    interp.set_shape_ic_enabled(shape_ic);
    interp.set_max_execution_steps(100_000_000);
    block_on(interp.execute(DEF)).unwrap();
    let line = format!("[ {} ] DOWN", depth);
    let t0 = Instant::now();
    for _ in 0..reps {
        block_on(interp.execute(&line)).unwrap();
        interp.update_stack(Vec::new());
    }
    LoopTiming {
        elapsed: t0.elapsed(),
        ic_hits: interp.runtime_metrics().shape_ic_hit_count,
        ic_misses: interp.runtime_metrics().shape_ic_miss_count,
    }
}

fn json_record_roundtrip(reps: u32) -> std::time::Duration {
    // Build many same-layout records through JSON@SET (record construction +
    // layout intern + key lookup), the workload record-shape sharing targets.
    let mut interp = Interpreter::new();
    interp.set_max_execution_steps(100_000_000);
    block_on(interp.execute("'json' IMPORT")).unwrap();
    let t0 = Instant::now();
    for _ in 0..reps {
        block_on(interp.execute(r#"'{"a": 1, "b": 2}' JSON@PARSE 'c' [ 3 ] JSON@SET"#)).unwrap();
        interp.update_stack(Vec::new());
    }
    t0.elapsed()
}

fn main() {
    println!("== call-site shape IC A/B bench ==\n");

    let depth = 250u64;
    let reps = 2000u32;
    let iters = u64::from(reps) * depth;
    println!("-- countdown loop ({reps} loops x depth {depth} = {iters} iterations) --");
    let off = time_loops(false, depth, reps);
    let on = time_loops(true, depth, reps);
    let off_ns = off.elapsed.as_nanos() as f64 / iters as f64;
    let on_ns = on.elapsed.as_nanos() as f64 / iters as f64;
    println!(
        "  shape IC OFF: {:>9.3} ms  ({off_ns:.1} ns/iter, hits {}, misses {})",
        off.elapsed.as_secs_f64() * 1e3,
        off.ic_hits,
        off.ic_misses
    );
    println!(
        "  shape IC ON : {:>9.3} ms  ({on_ns:.1} ns/iter, hits {}, misses {})",
        on.elapsed.as_secs_f64() * 1e3,
        on.ic_hits,
        on.ic_misses
    );
    println!("  speedup: {:.2}x\n", off_ns / on_ns);

    println!("-- JSON record set (same-layout records; interned shapes) --");
    let reps = 20_000u32;
    let elapsed = json_record_roundtrip(reps);
    println!(
        "  {reps} JSON@SET record builds: {:.3} ms  ({:.1} ns/op)",
        elapsed.as_secs_f64() * 1e3,
        elapsed.as_nanos() as f64 / f64::from(reps)
    );
}
