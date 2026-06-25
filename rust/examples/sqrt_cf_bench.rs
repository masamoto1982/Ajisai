//! A/B benchmark for the i128 `SqrtSmall` fast path in the continued-fraction
//! core. Run with: `cargo run --release --example sqrt_cf_bench`
//!
//! It expands square-root CFs and compares surds — the work that drives every
//! irrational guard/observation — with the fast path on vs off (the BigInt
//! `Sqrt` state). Results are identical either way; this measures only speed.

use std::time::Instant;

use ajisai_core::types::continued_fraction::{set_sqrt_small_fast_path, ExactReal};
use ajisai_core::types::fraction::Fraction;

fn surd(num: i64, den: i64) -> ExactReal {
    ExactReal::from_sqrt_rational(Fraction::new(num.into(), den.into()))
        .expect("non-negative radicand")
}

fn live_surds() -> Vec<ExactReal> {
    (2..=40i64)
        .filter(|n| (*n as f64).sqrt().fract() != 0.0) // skip perfect squares
        .map(|n| surd(n, 1))
        .collect()
}

/// Möbius transforms `r ± √n` / `r·√n` — the `√ ⊕ rational` arithmetic.
fn mobius_values() -> Vec<ExactReal> {
    let mut out = Vec::new();
    for n in [2i64, 3, 5, 6, 7, 10, 13, 17] {
        let root = surd(n, 1);
        for (rn, rd) in [(1, 2), (3, 1), (-2, 3), (5, 4)] {
            let r = ExactReal::Rational(Fraction::new(rn.into(), rd.into()));
            out.push(r.add(&root));
            out.push(root.sub(&r));
            out.push(r.mul(&root));
        }
    }
    out
}

/// Expand many surds to `budget` CF terms, and compare each adjacent pair.
fn work(reps: u32, budget: usize) -> u64 {
    let surds = live_surds();
    let mobius = mobius_values();
    let mut acc = 0u64;
    for _ in 0..reps {
        for m in &mobius {
            acc += m.partial_quotients_bounded(budget).len() as u64;
        }
        for s in &surds {
            acc += s.partial_quotients_bounded(budget).len() as u64;
        }
        for w in surds.windows(2) {
            if w[0].cmp_with_budget(&w[1], 256).is_some() {
                acc += 1;
            }
        }
    }
    acc
}

fn time(enabled: bool, reps: u32, budget: usize) -> std::time::Duration {
    set_sqrt_small_fast_path(enabled);
    let _ = work(50, budget); // warm up
    let t0 = Instant::now();
    let acc = work(reps, budget);
    let dt = t0.elapsed();
    std::hint::black_box(acc);
    dt
}

fn main() {
    println!("== CF i128 fast-path A/B bench (Sqrt + Möbius) ==\n");
    let reps = 1500u32;
    let budget = 64usize;
    let off = time(false, reps, budget);
    let on = time(true, reps, budget);
    println!("  {reps} reps × (expand ~36 surds + ~96 Möbius transforms to {budget} terms, + compare pairs)");
    println!(
        "  fast path OFF (BigInt): {:>8.1} ms",
        off.as_secs_f64() * 1e3
    );
    println!(
        "  fast path ON  (i128):   {:>8.1} ms",
        on.as_secs_f64() * 1e3
    );
    println!("  speedup: {:.2}x", off.as_secs_f64() / on.as_secs_f64());
}
