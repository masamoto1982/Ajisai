//! Minimal JSON-record workload probe used to A/B Record layout interning
//! across revisions (uses only APIs stable across the comparison).
//!
//! Run with:  `cargo run --release --example record_shape_bench`

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

fn main() {
    // Section 1: construction + successor + key read (intern cost visibility).
    let line = r#"'{"alpha": 1, "beta": 2, "gamma": 3, "delta": 4}' JSON@PARSE 'epsilon' [ 5 ] JSON@SET 'gamma' JSON@GET"#;
    let eq_line = r#"'{"alpha": 1, "beta": 2, "gamma": 3, "delta": 4}' JSON@PARSE '{"alpha": 1, "beta": 2, "gamma": 3, "delta": 4}' JSON@PARSE ="#;

    let reps = 10_000u32;
    let mut best_ns = f64::MAX;
    for _ in 0..3 {
        let mut interp = Interpreter::new();
        interp.set_max_execution_steps(100_000_000);
        block_on(interp.execute("'json' IMPORT")).unwrap();
        let t0 = Instant::now();
        for _ in 0..reps {
            block_on(interp.execute(line)).unwrap();
            block_on(interp.execute(eq_line)).unwrap();
            interp.update_stack(Vec::new());
        }
        let ns = t0.elapsed().as_nanos() as f64 / f64::from(reps);
        if ns < best_ns {
            best_ns = ns;
        }
    }
    println!("json record roundtrip best of 3: {best_ns:.1} ns/op-pair ({reps} reps/round)");

    // Section 2: record Value clone cost (the layout-sharing target — the
    // per-instance layout copy vs a shared-shape pointer bump). Uses a wide
    // record so the layout dominates the clone.
    let wide: String = {
        let fields: Vec<String> = (0..32).map(|i| format!("\"key{i:02}\": {i}")).collect();
        format!("'{{{}}}' JSON@PARSE", fields.join(", "))
    };
    let mut interp = Interpreter::new();
    interp.set_max_execution_steps(100_000_000);
    block_on(interp.execute("'json' IMPORT")).unwrap();
    block_on(interp.execute(&wide)).unwrap();
    let record = interp.get_stack()[0].clone();
    let clone_reps = 2_000_000u32;
    let mut best_clone_ns = f64::MAX;
    for _ in 0..3 {
        let t0 = Instant::now();
        let mut sink = 0usize;
        for _ in 0..clone_reps {
            let copy = record.clone();
            sink = sink.wrapping_add(&copy as *const _ as usize);
        }
        std::hint::black_box(sink);
        let ns = t0.elapsed().as_nanos() as f64 / f64::from(clone_reps);
        if ns < best_clone_ns {
            best_clone_ns = ns;
        }
    }
    println!("32-field record Value clone best of 3: {best_clone_ns:.1} ns/clone");
}
