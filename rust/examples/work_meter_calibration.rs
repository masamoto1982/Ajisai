//! What the work meter charges per millisecond, on every path that charges —
//! and what it charges for time that no ceiling prices at all.
//!
//! Run with:  `cargo run --release --example work_meter_calibration`
//!
//! The meter exists so a runaway computation is *refused* rather than measured:
//! `max_numeric_work` is a stand-in for "too long". That only works if every
//! path charges at roughly the same rate, because a ceiling calibrated against
//! one rate means something different on another. `ALGEBRAIC_PAIR_UNITS` exists
//! for exactly that reason — the algebraic path once ran 1,400x cheaper per
//! unit than the rational one, so a ceiling tuned for either was nonsense for
//! the other.
//!
//! That constant was chosen by a measurement nobody could re-run. This is that
//! measurement, committed, so "re-measure when either path's representation
//! changes" becomes an instruction someone can follow. It is deliberately *not*
//! a test: a wall clock in CI is flaky, and the absolute numbers are a property
//! of one machine and one `num-bigint`. What the suite pins instead is the
//! ratio between paths, under bounds loose enough to survive a slow runner
//! (`interpreter/work_meter_calibration_tests.rs`).
//!
//! Two sections, because there turned out to be two different problems.

use std::time::Instant;

use ajisai_core::interpreter::{Interpreter, RuntimeLimits};

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

/// Ceilings lifted out of the way: this measures the price, not the limit.
fn unbounded() -> RuntimeLimits {
    RuntimeLimits {
        max_materialized_elements: 10_000_000,
        max_source_bytes: 64 * 1024 * 1024,
        max_numeric_literal_digits: 1_000_000,
        max_numeric_work: u64::MAX,
        max_collection_work: u64::MAX,
        max_bigint_bits: u64::MAX,
        max_algebraic_terms: usize::MAX,
    }
}

struct Measurement {
    name: String,
    units: u64,
    run_millis: f64,
    /// Time spent rendering the result *after* execution finished. The meter
    /// prices computation and nothing else, so whatever lands here is time no
    /// ceiling can see.
    render_millis: f64,
}

impl Measurement {
    fn rate(&self) -> f64 {
        if self.run_millis <= 0.0 {
            return f64::INFINITY;
        }
        self.units as f64 / self.run_millis
    }
}

fn measure(name: String, source: &str) -> Measurement {
    measure_after_setup(name, "", source)
}

/// Build the operand with `setup`, then time `source` alone.
///
/// The distinction is not cosmetic, and getting it wrong is how this harness
/// under-reported a rate the host profile was later derived from. A rate is
/// `charged units / elapsed ms`, so any time inside the measured interval that
/// charges *this* meter nothing drags the rate down. `[ 0 99999 ] RANGE`
/// materializes 100,000 elements — real milliseconds, and real
/// `collectionWork` — while charging `numericWork` almost nothing. Timing it
/// together with the twenty additions that follow made the dense-lane path
/// look ~20% cheaper per millisecond than it is (6,153 vs 7,374 units/ms
/// measured back to back on one container), and a floor rate that reads low
/// derives a budget that is too small.
///
/// `execute` keeps the stack across calls and resets the counters
/// (`execution_loop.rs`), so the second call sees the first call's operand and
/// counts only its own work. This is the same split
/// `collection_word_calibration::measure` has always used; the two harnesses
/// disagreeing was itself the defect.
fn measure_after_setup(name: String, setup: &str, source: &str) -> Measurement {
    let mut interp = Interpreter::new();
    interp.set_runtime_limits(unbounded());
    interp.set_max_execution_steps(usize::MAX);

    if !setup.is_empty() {
        if let Err(error) = block_on(interp.execute(setup)) {
            panic!("setup `{setup}` for `{name}` must succeed, got: {error:?}");
        }
    }

    let started = Instant::now();
    let outcome = block_on(interp.execute(source));
    let run_millis = started.elapsed().as_secs_f64() * 1000.0;
    if let Err(error) = outcome {
        panic!("`{name}` must complete to be measurable, got: {error:?}");
    }

    let render_started = Instant::now();
    let rendered = ajisai_core::types::display::render_stack(interp.get_stack());
    let render_millis = render_started.elapsed().as_secs_f64() * 1000.0;
    std::hint::black_box(rendered);

    Measurement {
        name,
        units: interp.numeric_work_used(),
        run_millis,
        render_millis,
    }
}

/// `(√p₁+√q₁)·(√p₂+√q₂)·…` — the multiquadratic cascade, which doubles its
/// term count with every factor.
fn algebraic_product(factors: usize) -> String {
    const PAIRS: [(u32, u32); 6] = [(2, 3), (5, 7), (11, 13), (17, 19), (23, 29), (31, 37)];
    let mut source = String::new();
    for (index, (left, right)) in PAIRS.iter().take(factors).enumerate() {
        source.push_str(&format!("{left} SQRT {right} SQRT +"));
        if index > 0 {
            source.push_str(" *");
        }
        source.push(' ');
    }
    source
}

/// Steps per millisecond for a dispatch-bound loop: no `Fraction` wider than
/// a machine word, no algebraic term, nothing any other meter prices.
/// `executionSteps` is the *only* ceiling such a path can ever reach — the
/// same role `dense tensor lanes` plays for `numericWork` — so the *slowest*
/// such path's rate is what `DEFAULT_MAX_EXECUTION_STEPS` should be sized
/// against for a host whose ceiling is a time budget rather than a fixed
/// round number. Two shapes, because dispatch cost is not uniform: a flat
/// arithmetic loop is cheap, and a user-word call (dictionary lookup, frame
/// push) is dearer — `DOWN_PROBE` in `cli/step_limit_tests.rs` is exactly
/// this second shape, and it is the one that sets the floor.
///
/// Read via `Interpreter::resource_usage().execution_steps` rather than
/// computed from the source text, so it counts what the interpreter actually
/// charged (one step per word, including the leading literal push) instead of
/// what the calibration author assumed it would.
fn steps_per_ms(source: &str) -> (u64, f64, f64) {
    let mut interp = Interpreter::new();
    interp.set_runtime_limits(unbounded());
    interp.set_max_execution_steps(usize::MAX);

    let started = Instant::now();
    let outcome = block_on(interp.execute(source));
    let run_millis = started.elapsed().as_secs_f64() * 1000.0;
    if let Err(error) = outcome {
        panic!("steps_per_ms must complete, got: {error:?}");
    }
    let steps = interp.resource_usage().execution_steps;
    let rate = if run_millis > 0.0 {
        steps as f64 / run_millis
    } else {
        f64::INFINITY
    };
    (steps, run_millis, rate)
}

/// A flat loop of `reps` cheap machine-word additions.
fn add_loop(reps: usize) -> String {
    format!("1{}", " 7 +".repeat(reps))
}

/// A trampolined tail-recursive countdown, same construction as
/// `cli::step_limit_tests::DOWN_PROBE`: every iteration is a user-word call
/// (dictionary lookup, guard check, frame push) rather than a bare
/// arithmetic op, so it prices dispatch overhead the flat loop cannot see.
fn down_probe(n: usize) -> String {
    format!(
        "{{\n{{ [ 0 ] > | [ 1 ] - DOWN }}\n{{ IDLE | [ 'done' ] }} COND\n}} 'DOWN' DEF\n{n} DOWN"
    )
}

fn main() {
    let wide = "9".repeat(4096);

    println!("── what the meter charges, per path ─────────────────────────────\n");
    println!(
        "Every source parses its wide literal *once*, through a user word, and\n\
         applies it N times. Writing the literal per operation instead measures\n\
         the O(n²) decimal->BigInt parse — 126 µs for 4096 digits here — which is\n\
         `numericLiteralDigits`' business, not the meter's, and swamps the\n\
         operation being measured.\n"
    );
    let charged = vec![
        // The path the unit is named after: one bignum multiply per charged
        // limb-multiply.
        measure(
            "scalar wide MUL x20 (4096-digit)".into(),
            &format!("{{ {wide} * }} 'M' DEF 1{}", " M".repeat(20)),
        ),
        // Rational addition: cross-multiply plus gcd, and measurably dearer
        // than the multiplication above at the same width.
        measure(
            "scalar wide ADD x200 (4096-digit)".into(),
            &format!("{{ {wide} + }} 'W' DEF 1{}", " W".repeat(200)),
        ),
        // Dense `i64` lanes: one charged unit per lane, and a lane here is a
        // machine-word add inside a tensor kernel. This is the *floor* path —
        // the slowest charged rate, and therefore the one a host time budget
        // divides by — so it is the row that most needs the operand build kept
        // out of the timed interval, and enough repetitions that what remains
        // is the add and not the measurement's own edges.
        measure_after_setup(
            "dense tensor lanes (100k x i64 add x200)".into(),
            "[ 0 99999 ] RANGE",
            &" 1 +".repeat(200),
        ),
        // A scalar operation on machine-word values. Its cost is dominated by
        // interpreter dispatch, which `executionSteps` prices, not this meter.
        measure(
            "scalar small ADD x200".into(),
            &format!("1{}", " 7 +".repeat(200)),
        ),
        measure(
            "algebraic cascade (4 two-radical factors)".into(),
            &algebraic_product(4),
        ),
    ];
    println!(
        "{:<46} {:>13} {:>9} {:>13}",
        "path", "units", "run ms", "units/ms"
    );
    for measurement in &charged {
        println!(
            "{:<46} {:>13} {:>9.1} {:>13.0}",
            measurement.name,
            measurement.units,
            measurement.run_millis,
            measurement.rate()
        );
    }
    let rates: Vec<f64> = charged.iter().map(Measurement::rate).collect();
    let slowest = rates.iter().cloned().fold(f64::INFINITY, f64::min);
    let fastest = rates.iter().cloned().fold(0.0_f64, f64::max);
    println!(
        "\nspread: {:.0}x between the cheapest and the dearest charged unit.\n\
         A low units/ms is the dangerous direction: it is time the ceiling does\n\
         not see. The two cheap rows are bounded elsewhere — a scalar operation\n\
         by `executionSteps`, a lane by `maxMaterializedElements` — so what\n\
         matters most is that no *unbounded* path sits far below the rest.",
        fastest / slowest
    );

    println!("\n── what nothing charges: rendering the answer ───────────────────\n");
    println!(
        "The meter prices computation. A multiquadratic value is cheap to build\n\
         and expensive to *write down*: `stackDisplay` is a continued fraction,\n\
         and expanding one needs enclosure refinement that grows with the term\n\
         count. No ceiling prices that, so `wallTimeMs` is the only thing that\n\
         catches it — and it catches it as a timeout, which names nothing.\n"
    );
    println!(
        "{:>8} {:>8} {:>10} {:>9} {:>12}",
        "factors", "terms", "units", "run ms", "render ms"
    );
    for factors in 1..=6 {
        let measurement = measure(
            format!("algebraic product of {factors}"),
            &algebraic_product(factors),
        );
        println!(
            "{:>8} {:>8} {:>10} {:>9.1} {:>12.1}",
            factors,
            1_usize << factors,
            measurement.units,
            measurement.run_millis,
            measurement.render_millis
        );
    }

    println!("\n── executionSteps floor: dispatch-bound steps/ms ────────────────\n");
    println!(
        "`executionSteps` prices word count, not the internal work a word does —\n\
         every other meter above already prices that. The only path\n\
         `executionSteps` alone has to bound is a long loop of the cheapest\n\
         possible word, the way `dense tensor lanes` is the only path\n\
         `numericWork` alone has to bound. Repeated at growing rep counts to\n\
         show the rate has settled rather than trusting one noisy sample.\n"
    );
    println!(
        "{:<24} {:>10} {:>9} {:>12}",
        "shape", "steps", "run ms", "steps/ms"
    );
    let mut floor = f64::INFINITY;
    for (label, reps) in [
        ("add loop", 50_000usize),
        ("add loop", 200_000),
        ("add loop", 1_000_000),
    ] {
        let (steps, run_millis, rate) = steps_per_ms(&add_loop(reps));
        floor = floor.min(rate);
        println!("{label:<24} {steps:>10} {run_millis:>9.1} {rate:>12.0}");
    }
    for n in [50_000usize, 200_000, 1_000_000] {
        let (steps, run_millis, rate) = steps_per_ms(&down_probe(n));
        floor = floor.min(rate);
        println!(
            "{:<24} {steps:>10} {run_millis:>9.1} {rate:>12.0}",
            "DOWN trampoline"
        );
    }
    println!("\nfloor: {floor:.0} steps/ms on this container, this build.");
}
