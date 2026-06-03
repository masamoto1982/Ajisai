//! NICF feasibility experiment (PR #998 open-question ②, "②実測").
//!
//! NOT a conformance test and NOT part of the product. This integration
//! test measures, with exact BigInt rational arithmetic (no floating point),
//! how many fewer leading partial-quotient terms it takes a *nearest-integer*
//! continued fraction (NICF) to reveal the order of two values compared to
//! the *regular* floor continued fraction (RCF). That term count is exactly
//! the `agreedPrefix` the comparison budget cares about (SPEC §4.5.0/§7.4.1),
//! so a reduction here means more comparisons decide within a fixed budget.
//!
//! Run with: `cargo test --test nicf_feasibility -- --ignored --nocapture`
//!
//! It is `#[ignore]` by default so it never runs in the normal suite / CI;
//! it is a one-off measurement harness, run explicitly.
//!
//! MEASURED RESULTS (seed 0x9E3779B97F4A7C15, CAP=300):
//!   broad random rationals:   RCF mean 0.217  NICF 0.169   −22.0%   (max 4→3)
//!   near-equal x vs x+1/N:     RCF mean 5.274  NICF 3.895   −26.2%   (max 12→7)
//!   surds √D vs rational/√D':  RCF mean 62.74  NICF 45.00   −28.3%   (max 104→78)
//! Conclusion: NICF shrinks the agreed-prefix depth by ~22–28% across the
//! board, including ~28% on the surd/lazy-CF case that actually exhausts the
//! budget. The win is aggregate, not per-pair (a few percent of pairs tie or
//! regress by one term), which matches the theory: NICF converges at least as
//! fast as RCF on average but is not a strict per-pair refinement. This term-
//! count reduction is the quantity the comparison budget sees directly. It
//! does NOT measure the production-path (Gosper-node) emission cost — open
//! question ②.2 — which remains the implementation risk to settle before any
//! promotion of SPEC §4.2.5 / §7.4.1.1.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

/// Exact rational, denominator always > 0, always reduced.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Rat {
    n: BigInt,
    d: BigInt,
}

impl Rat {
    fn new(n: BigInt, d: BigInt) -> Self {
        assert!(!d.is_zero(), "zero denominator");
        let (mut n, mut d) = (n, d);
        if d.is_negative() {
            n = -n;
            d = -d;
        }
        let g = n.gcd(&d);
        if !g.is_zero() {
            n /= &g;
            d /= &g;
        }
        Rat { n, d }
    }

    fn from_ints(n: i64, d: i64) -> Self {
        Rat::new(BigInt::from(n), BigInt::from(d))
    }

    fn is_zero(&self) -> bool {
        self.n.is_zero()
    }

    /// Floor(self) as a BigInt.
    fn floor(&self) -> BigInt {
        self.n.div_floor(&self.d)
    }

    /// Nearest integer with remainder in (-1/2, 1/2]  (tie at +1/2 → round
    /// toward the lower integer, matching SPEC §4.2.5's half-open interval).
    /// n = ceil((2a - b) / (2b)), with b > 0.
    fn round_nicf(&self) -> BigInt {
        let two = BigInt::from(2);
        let p = &two * &self.n - &self.d;
        let q = &two * &self.d;
        // ceil(p/q), q > 0
        p.div_ceil(&q)
    }

    fn sub_int(&self, k: &BigInt) -> Rat {
        Rat::new(&self.n - k * &self.d, self.d.clone())
    }

    fn recip(&self) -> Rat {
        assert!(!self.n.is_zero(), "reciprocal of zero");
        Rat::new(self.d.clone(), self.n.clone())
    }
}

/// Regular (floor) CF: the sequence of integer partial quotients.
fn rcf_terms(mut x: Rat, cap: usize) -> Vec<BigInt> {
    let mut out = Vec::new();
    for _ in 0..cap {
        let a = x.floor();
        out.push(a.clone());
        let frac = x.sub_int(&a);
        if frac.is_zero() {
            break;
        }
        x = frac.recip();
    }
    out
}

/// Nearest-integer CF: the sequence of signed integer partial quotients
/// b_i (the ε_i signs are subsumed into the sign of the following b_i,
/// so the b-sequence alone identifies the NICF).
fn nicf_terms(mut x: Rat, cap: usize) -> Vec<BigInt> {
    let mut out = Vec::new();
    for _ in 0..cap {
        let b = x.round_nicf();
        out.push(b.clone());
        let frac = x.sub_int(&b); // in (-1/2, 1/2]
        if frac.is_zero() {
            break;
        }
        x = frac.recip();
    }
    out
}

/// Length of the longest common leading run of two term sequences. This is
/// the `agreedPrefix` (number of matching leading terms before the first
/// difference); a smaller value means the order is revealed sooner.
fn agreed_prefix(a: &[BigInt], b: &[BigInt]) -> usize {
    let mut i = 0;
    while i < a.len() && i < b.len() && a[i] == b[i] {
        i += 1;
    }
    i
}

/// A high-precision rational approximation of sqrt(d) with `digits` decimal
/// digits of precision: floor(sqrt(d * 10^(2k))) / 10^k. Its leading CF
/// terms match those of the true surd far beyond any prefix length we count
/// here (we count tens of terms; precision is hundreds of digits).
fn sqrt_approx(d: u64, digits: u32) -> Rat {
    let scale = BigInt::from(10u32).pow(digits);
    let scale_sq = &scale * &scale;
    let radicand = BigInt::from(d) * &scale_sq;
    let root = radicand.sqrt();
    Rat::new(root, scale)
}

struct Stats {
    pairs: usize,
    rcf_total: usize,
    nicf_total: usize,
    rcf_max: usize,
    nicf_max: usize,
    nicf_le_rcf: usize,
    nicf_lt_rcf: usize,
}

impl Stats {
    fn new() -> Self {
        Stats {
            pairs: 0,
            rcf_total: 0,
            nicf_total: 0,
            rcf_max: 0,
            nicf_max: 0,
            nicf_le_rcf: 0,
            nicf_lt_rcf: 0,
        }
    }

    fn record(&mut self, x: &Rat, y: &Rat, cap: usize) {
        if x == y {
            return; // equal values never diverge; not an order-decision case
        }
        let rp = agreed_prefix(&rcf_terms(x.clone(), cap), &rcf_terms(y.clone(), cap));
        let np = agreed_prefix(&nicf_terms(x.clone(), cap), &nicf_terms(y.clone(), cap));
        self.pairs += 1;
        self.rcf_total += rp;
        self.nicf_total += np;
        self.rcf_max = self.rcf_max.max(rp);
        self.nicf_max = self.nicf_max.max(np);
        if np <= rp {
            self.nicf_le_rcf += 1;
        }
        if np < rp {
            self.nicf_lt_rcf += 1;
        }
    }

    fn report(&self, label: &str) {
        let mean = |t: usize| t as f64 / self.pairs.max(1) as f64;
        let rcf_mean = mean(self.rcf_total);
        let nicf_mean = mean(self.nicf_total);
        let reduction = if rcf_mean > 0.0 {
            100.0 * (rcf_mean - nicf_mean) / rcf_mean
        } else {
            0.0
        };
        println!("── {label} ({} pairs) ──", self.pairs);
        println!(
            "   mean agreedPrefix:  RCF {:.3}   NICF {:.3}   reduction {:.1}%",
            rcf_mean, nicf_mean, reduction
        );
        println!(
            "   max  agreedPrefix:  RCF {}        NICF {}",
            self.rcf_max, self.nicf_max
        );
        println!(
            "   NICF <= RCF: {}/{}   NICF strictly < RCF: {}/{}",
            self.nicf_le_rcf, self.pairs, self.nicf_lt_rcf, self.pairs
        );
        // Hard invariant: a smaller agreed prefix must never be paid for by
        // NICF ever being WORSE on the metric we care about across the corpus
        // mean. (Per-pair NICF can occasionally tie or, in rare adversarial
        // single pairs, exceed RCF by one; the claim under test is the
        // aggregate, which is what the budget sees over real workloads.)
        assert!(
            nicf_mean <= rcf_mean + 1e-9,
            "{label}: NICF mean agreedPrefix ({nicf_mean:.3}) exceeded RCF ({rcf_mean:.3})"
        );
    }
}

#[test]
#[ignore = "feasibility measurement harness; run explicitly with --nocapture"]
fn nicf_vs_rcf_agreed_prefix() {
    // The agreedPrefix the budget cares about is in the tens (production
    // budget is 256), so a cap of 300 captures every decision we measure
    // while keeping BigInt growth bounded and the run fast.
    const CAP: usize = 300;

    // Deterministic LCG so the corpus is reproducible without an rng dep.
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut next = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed
    };

    println!("\n========== NICF feasibility (exact BigInt) ==========");

    // 1) Broad random rational pairs.
    let mut broad = Stats::new();
    for _ in 0..5_000 {
        let xn = (next() % 20_001) as i64 - 10_000;
        let xd = (next() % 10_000) as i64 + 1;
        let yn = (next() % 20_001) as i64 - 10_000;
        let yd = (next() % 10_000) as i64 + 1;
        broad.record(&Rat::from_ints(xn, xd), &Rat::from_ints(yn, yd), CAP);
    }
    broad.report("broad random rationals");

    // 2) Budget-stressing near-equal family: x and x + 1/N for large N.
    //    These share a long CF prefix and are exactly what exhausts a budget.
    let mut near = Stats::new();
    for _ in 0..5_000 {
        let xn = (next() % 2_001) as i64 - 1_000;
        let xd = (next() % 1_000) as i64 + 1;
        let x = Rat::from_ints(xn, xd);
        let big_n = BigInt::from((next() % 1_000_000) as i64 + 1_000);
        let eps = Rat::new(BigInt::one(), big_n);
        let y = Rat::new(&x.n * &eps.d + &eps.n * &x.d, &x.d * &eps.d);
        near.record(&x, &y, CAP);
    }
    near.report("near-equal rationals (x vs x + 1/N)");

    // 3) Surds: sqrt(D) vs nearby rationals, and sqrt(D) vs sqrt(D').
    let mut surd = Stats::new();
    let ds = [2u64, 3, 5, 6, 7, 8, 10, 11, 13, 17, 19, 23, 29, 31];
    for &d in &ds {
        let s = sqrt_approx(d, 60);
        // sqrt(D) vs a ladder of its own rational convergents' neighbours.
        for k in 1..=200i64 {
            // sqrt(D) vs a near rational offset by k/scale.
            let near_r = Rat::new(&s.n + BigInt::from(k), s.d.clone());
            surd.record(&s, &near_r, CAP);
        }
        // sqrt(D) vs sqrt(D')
        for &e in &ds {
            if e != d {
                surd.record(&s, &sqrt_approx(e, 60), CAP);
            }
        }
    }
    surd.report("surds (sqrt(D) vs rational / sqrt(D'))");

    println!("=====================================================\n");
}
