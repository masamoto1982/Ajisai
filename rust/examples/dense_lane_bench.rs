//! What a Word costs per lane of a dense tensor it does not read.
//!
//! Two questions ran after every core word — "is the top of the stack a
//! collection?" and "did this Word produce a reasoned absence?" — and both were
//! answered through `Value::as_vector_view`, whose `Cow` is `Owned` for a
//! `Tensor`. Each one rebuilt the whole buffer as boxed per-lane `Value`s and
//! dropped it again, so the *bookkeeping* around an element-wise op cost more
//! than the op, and cost it per Word rather than per failure.
//!
//! This measures the part that shows it: one wide tensor, then the same
//! broadcast multiply applied k times in a single line. Per-Word overhead that
//! scales with the lane count shows up as a flat per-element cost that refuses
//! to amortize as k grows; overhead that does not shows up as the literal's
//! one-time parse being divided away.
//!
//! Run with:  `cargo run --release --example dense_lane_bench`

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

const LANES: usize = 4096;

fn interpreter() -> Interpreter {
    let mut interp = Interpreter::new();
    interp.set_max_execution_steps(1_000_000_000);
    let mut limits = *interp.runtime_limits();
    limits.max_materialized_elements = 100_000_000;
    limits.max_numeric_work = u64::MAX;
    limits.max_collection_work = u64::MAX;
    interp.set_runtime_limits(limits);
    interp
}

fn time(source: &str, reps: u32) -> std::time::Duration {
    let mut interp = interpreter();
    for _ in 0..2 {
        block_on(interp.execute(source)).expect("bench source must compute");
        interp.update_stack(Vec::new());
    }
    let start = Instant::now();
    for _ in 0..reps {
        block_on(interp.execute(source)).expect("bench source must compute");
        interp.update_stack(Vec::new());
    }
    start.elapsed()
}

fn main() {
    let lanes: String = (0..LANES)
        .map(|k| (k % 97).to_string())
        .collect::<Vec<_>>()
        .join(" ");

    println!("== per-Word cost over a {LANES}-lane dense tensor ==\n");
    println!("  Words in line    total per run     per element-op");
    for words in [1usize, 2, 4, 8, 16, 32] {
        let source = format!("[ {lanes} ] {}", "2 MUL ".repeat(words));
        let reps = 40;
        let per_run = time(&source, reps).as_nanos() as f64 / reps as f64;
        println!(
            "  {words:>5} x 2 MUL   {:>9.1} us     {:>8.2} ns",
            per_run / 1e3,
            per_run / (LANES * words) as f64
        );
    }
    println!(
        "\n  A per-element cost that keeps falling as the Word count rises is the\n  \
         literal's one-time parse amortizing. A flat one is per-Word work that\n  \
         scales with the lane count — which is what materializing the tensor for\n  \
         a predicate question looked like."
    );
}
