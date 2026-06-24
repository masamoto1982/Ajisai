//! A/B benchmark for compile-time literal-vector lowering
//! (`CompiledOp::PushVectorLiteral`).
//!
//! Run with:  `cargo run --release --example vector_literal_bench`
//!
//! A word doing element-wise vector arithmetic on literal vectors used to fall
//! back to the interpreter for its whole line (any vector token was a
//! `FallbackToken`). With lowering on, the literal vectors are prebuilt once and
//! the line runs compiled. We time the same word with lowering ON vs OFF.

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

// Element-wise add/sub/mul over 8-element literal vectors.
const DEF: &str =
    "{ [ 1 2 3 4 5 6 7 8 ] [ 8 7 6 5 4 3 2 1 ] + [ 2 2 2 2 2 2 2 2 ] * [ 1 1 1 1 1 1 1 1 ] - } 'VWORK' DEF";

fn time(vector_literal: bool, reps: u32) -> std::time::Duration {
    let mut interp = Interpreter::new();
    interp.set_vector_literal_enabled(vector_literal);
    interp.set_max_execution_steps(100_000_000);
    block_on(interp.execute(DEF)).unwrap();
    let t0 = Instant::now();
    for _ in 0..reps {
        block_on(interp.execute("VWORK")).unwrap();
        interp.update_stack(Vec::new());
    }
    t0.elapsed()
}

fn main() {
    println!("== literal-vector lowering A/B bench ==\n");
    let reps = 200_000u32;
    // Warm up plan caches.
    let _ = time(false, 1000);
    let _ = time(true, 1000);
    let off = time(false, reps);
    let on = time(true, reps);
    let per = |d: std::time::Duration| d.as_nanos() as f64 / reps as f64;
    println!("  {reps} calls of an 8-wide vector add/mul/sub word");
    println!(
        "  lowering OFF (interpreter): {:>8.3} ms  ({:.0} ns/call)",
        off.as_secs_f64() * 1e3,
        per(off)
    );
    println!(
        "  lowering ON  (compiled):    {:>8.3} ms  ({:.0} ns/call)",
        on.as_secs_f64() * 1e3,
        per(on)
    );
    println!("  speedup: {:.2}x", off.as_secs_f64() / on.as_secs_f64());
}
