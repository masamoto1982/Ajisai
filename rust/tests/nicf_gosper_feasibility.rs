//! NICF feasibility, part ②.2: can a *Gosper node* emit nearest-integer
//! continued-fraction terms cheaply, reusing the production Möbius
//! coefficient mechanics?
//!
//! NOT a conformance test and NOT product code. This prototype answers the
//! one question that gates promotion of SPEC §4.2.5 / §7.4.1.1 (PR #999):
//! the abstract term-count win was already measured (≈22–28%,
//! `nicf_feasibility.rs`); what remained unknown was whether the *production
//! representation* — a Gosper/Möbius transform that emits result terms by
//! narrowing an interval as it ingests operand terms — can emit NICF terms
//! without prohibitive extra bookkeeping, and without ingesting more operand
//! terms than the RCF emitter does.
//!
//! Method. We mirror the production unary Gosper stepper
//! (`step_mobius` in continued_fraction.rs): a Möbius value
//! M(x) = (a·x + b)/(c·x + d) over x ∈ [1, ∞), fed operand partial quotients
//! from the *real* `CfIter` (via the public `partial_quotients_bounded`).
//! We implement two emitters over the identical coefficient machinery:
//!   * RCF: emit q when floor(M) agrees at both interval endpoints, then
//!          (a,b,c,d) ← (c, d, a−q·c, b−q·d).
//!   * NICF: emit q when round(M) (nearest integer, half-down) agrees at both
//!          endpoints, then the SAME update (a,b,c,d) ← (c, d, a−q·c, b−q·d).
//!          The continuation v' = 1/(v−q) carries a negative remainder's sign
//!          through the reciprocal on its own, so the next round() yields a
//!          negative term automatically — no ε sign bookkeeping is needed in
//!          the coefficients. The ONLY delta from RCF is the round-vs-floor
//!          emit test. (An earlier draft of this prototype added an ε sign
//!          multiply on update and it produced sign-flipped terms; removing it
//!          made every case match the oracle — see RESULT below.)
//!
//! Validation. For each value v we compare the NICF terms emitted by the
//! Möbius stepper against an INDEPENDENT oracle: the exact-rational NICF of a
//! high-precision rational approximation of v (same algorithm as
//! `nicf_feasibility.rs`). Agreement on a long prefix demonstrates the
//! Gosper-path NICF emission is correct. We also count operand-term ingests
//! per emitted result term, NICF vs RCF, to confirm the cost is comparable.
//!
//! Run: `cargo test --test nicf_gosper_feasibility -- --ignored --nocapture`
//! `#[ignore]` so it never runs in the normal suite / CI.
//!
//! RESULT (the ②.2 verdict — FEASIBLE):
//!   * Correctness: all 8 Gosper/Möbius cases emit NICF terms matching the
//!     independent exact-rational oracle on the full 18-term prefix (18/18).
//!   * Bookkeeping: the feared "extra Möbius bookkeeping" does NOT exist. The
//!     coefficient update on emit is BYTE-FOR-BYTE IDENTICAL to RCF:
//!     (a,b,c,d) ← (c, d, a−q·c, b−q·d). A negative NICF remainder makes the
//!     reciprocal negative, so the next round() yields a negative term on its
//!     own — no ε sign tracking in the coefficients. The ONLY RCF→NICF change
//!     is the emit predicate: round()-agreement across endpoints instead of
//!     floor()-agreement. That is a one-line change to step_mobius /
//!     bihom_emit_candidate.
//!   * Cost: NICF ingests ≈1.46× the operand terms PER EMITTED RESULT TERM
//!     (209 vs 143 for 18 terms), because NICF terms are larger and need the
//!     interval narrowed further before they pin. This composes favorably
//!     with the §999 result that NICF needs ≈28% FEWER result terms to decide
//!     a comparison: the budget is spent on result terms, and you need fewer
//!     of them. No blow-up; cost stays within a small constant factor.
//!   Conclusion: the open implementation prerequisite gating promotion of
//!   SPEC §4.2.5 / §7.4.1.1 is satisfied for the Rational / AlgebraicSqrt /
//!   unary-Gosper(Möbius) path. (Binary bihom uses the same emit/update shape;
//!   bihom_emit_candidate's four-corner floor-agreement generalizes to
//!   round-agreement identically.)

use ajisai_core::types::continued_fraction::ExactReal;
use ajisai_core::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

// ───────────────────────── exact-rational NICF oracle ─────────────────────
// (Independent of the Möbius stepper; same algorithm as nicf_feasibility.rs.)

fn rat_round_half_down(num: &BigInt, den: &BigInt) -> BigInt {
    // den > 0. Nearest integer with remainder in (-1/2, 1/2]; tie (frac == 1/2)
    // rounds DOWN. b = ceil((2·num − den) / (2·den)).
    assert!(den.is_positive());
    let two = BigInt::from(2);
    let p = &two * num - den;
    let q = &two * den;
    p.div_ceil(&q)
}

/// Exact-rational NICF term sequence of num/den (den != 0), capped.
fn oracle_nicf_terms(mut num: BigInt, mut den: BigInt, cap: usize) -> Vec<BigInt> {
    if den.is_negative() {
        num = -num;
        den = -den;
    }
    let mut out = Vec::new();
    for _ in 0..cap {
        let b = rat_round_half_down(&num, &den);
        out.push(b.clone());
        // remainder r = num/den − b = (num − b·den)/den, in (-1/2, 1/2]
        let rn = &num - &b * &den;
        if rn.is_zero() {
            break;
        }
        // next tail = 1/r = den/rn ; renormalize so denominator > 0
        num = den;
        den = rn;
        if den.is_negative() {
            num = -num;
            den = -den;
        }
        let g = num.gcd(&den);
        if !g.is_zero() {
            num /= &g;
            den /= &g;
        }
    }
    out
}

/// floor(sqrt(d)*10^digits)/10^digits as (num, den) — a high-precision
/// rational under-approximation of sqrt(d).
fn sqrt_rational(d: u64, digits: u32) -> (BigInt, BigInt) {
    let scale = BigInt::from(10u32).pow(digits);
    let root = (BigInt::from(d) * &scale * &scale).sqrt();
    (root, scale)
}

// ───────────────────── Möbius stepper over a real CfIter ──────────────────
// Operand partial quotients come from the production engine; we pull them
// lazily from a precomputed RCF prefix (which `partial_quotients_bounded`
// produced via the real Gosper/Sqrt steppers).

struct OperandStream {
    terms: Vec<BigInt>,
    pos: usize,
}
impl OperandStream {
    fn new(terms: Vec<BigInt>) -> Self {
        OperandStream { terms, pos: 0 }
    }
    /// Next RCF partial quotient of the operand, or None if the prefix is
    /// exhausted (treated as "operand done" — we cap prefixes well beyond
    /// what any emit needs).
    fn next(&mut self) -> Option<BigInt> {
        let t = self.terms.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Rcf,
    Nicf,
}

struct MobiusStepper {
    a: BigInt,
    b: BigInt,
    c: BigInt,
    d: BigInt,
    x: OperandStream,
    x_done: bool,
    ingests: usize, // operand terms consumed (the cost metric)
}

impl MobiusStepper {
    fn new(a: i64, b: i64, c: i64, d: i64, x: OperandStream) -> Self {
        MobiusStepper {
            a: BigInt::from(a),
            b: BigInt::from(b),
            c: BigInt::from(c),
            d: BigInt::from(d),
            x,
            x_done: false,
            ingests: 0,
        }
    }

    /// Emit one term in the given mode, or None if no further term is
    /// determinable within a generous local ingest budget.
    fn emit(&mut self, mode: Mode) -> Option<BigInt> {
        let mut budget = 4000usize;
        loop {
            // Endpoints of M(x) over x ∈ [1, ∞): at x=1 → (a+b)/(c+d),
            // at x=∞ → a/c. Emit when both map to the same integer under
            // the mode's rounding and there's no pole in [1, ∞).
            if !self.c.is_zero() && !(&self.c + &self.d).is_zero() {
                let cd = &self.c + &self.d;
                let same_sign = (self.c.is_positive() && cd.is_positive())
                    || (self.c.is_negative() && cd.is_negative());
                if same_sign {
                    let (q_inf, q_one) = match mode {
                        Mode::Rcf => (
                            self.a.div_floor(&self.c),
                            (&self.a + &self.b).div_floor(&cd),
                        ),
                        Mode::Nicf => {
                            // Normalize each endpoint so denominator > 0,
                            // then nearest-integer (half-down).
                            let near = |mut n: BigInt, mut d: BigInt| {
                                if d.is_negative() {
                                    n = -n;
                                    d = -d;
                                }
                                rat_round_half_down(&n, &d)
                            };
                            (
                                near(self.a.clone(), self.c.clone()),
                                near(&self.a + &self.b, cd.clone()),
                            )
                        }
                    };
                    if q_inf == q_one {
                        let q = q_inf;
                        // Update is IDENTICAL for RCF and NICF:
                        //   v' = 1/(v − q)  ⇒  (a,b,c,d) ← (c, d, a−q·c, b−q·d).
                        // For NICF the remainder v−q may be negative; the
                        // reciprocal is then negative and the next emit's
                        // round() yields a negative term automatically — no
                        // ε sign bookkeeping in the coefficient update is
                        // needed. The ONLY RCF→NICF difference is the emit
                        // test above (round-agreement vs floor-agreement).
                        let (na, nb) = (self.c.clone(), self.d.clone());
                        let nc = &self.a - &q * &self.c;
                        let nd = &self.b - &q * &self.d;
                        self.a = na;
                        self.b = nb;
                        self.c = nc;
                        self.d = nd;
                        return Some(q);
                    }
                }
            }

            if self.x_done {
                return None; // would collapse to a rational tail; prefix enough
            }
            if budget == 0 {
                return None;
            }
            budget -= 1;

            match self.x.next() {
                Some(p) => {
                    self.ingests += 1;
                    // Ingest p: (a,b,c,d) ← (a·p + b, a, c·p + d, c)
                    let na = &self.a * &p + &self.b;
                    let nb = self.a.clone();
                    let nc = &self.c * &p + &self.d;
                    let nd = self.c.clone();
                    self.a = na;
                    self.b = nb;
                    self.c = nc;
                    self.d = nd;
                }
                None => {
                    self.x_done = true;
                }
            }
        }
    }

    fn emit_n(&mut self, mode: Mode, n: usize) -> Vec<BigInt> {
        let mut out = Vec::new();
        for _ in 0..n {
            match self.emit(mode) {
                Some(q) => out.push(q),
                None => break,
            }
        }
        out
    }
}

/// A unary-Gosper test value: M(a,b,c,d) applied to sqrt(d_radicand).
struct Case {
    label: &'static str,
    a: i64,
    b: i64,
    c: i64,
    d: i64,
    radicand: u64,
    /// Exact value as (num,den) coefficients of (a·√r + b)/(c·√r + d) — used
    /// to build the oracle's high-precision rational via √r ≈ p/q.
    eval_oracle: fn(p: &BigInt, q: &BigInt, a: i64, b: i64, c: i64, d: i64) -> (BigInt, BigInt),
}

fn eval_mobius_of_sqrt(p: &BigInt, q: &BigInt, a: i64, b: i64, c: i64, d: i64) -> (BigInt, BigInt) {
    // value = (a·(p/q) + b) / (c·(p/q) + d) = (a·p + b·q) / (c·p + d·q)
    let num = BigInt::from(a) * p + BigInt::from(b) * q;
    let den = BigInt::from(c) * p + BigInt::from(d) * q;
    (num, den)
}

#[test]
#[ignore = "Gosper-NICF feasibility prototype; run explicitly with --ignored --nocapture"]
fn gosper_can_emit_nicf_terms() {
    const EMIT: usize = 18; // result terms to emit and validate
    const OP_PREFIX: usize = 4000; // operand RCF prefix length (≫ enough)
    const ORACLE_DIGITS: u32 = 400;

    let cases = [
        Case {
            label: "√2 + 1",
            a: 1,
            b: 1,
            c: 0,
            d: 1,
            radicand: 2,
            eval_oracle: eval_mobius_of_sqrt,
        },
        Case {
            label: "√2 − 1",
            a: 1,
            b: -1,
            c: 0,
            d: 1,
            radicand: 2,
            eval_oracle: eval_mobius_of_sqrt,
        },
        Case {
            label: "1 / √2",
            a: 0,
            b: 1,
            c: 1,
            d: 0,
            radicand: 2,
            eval_oracle: eval_mobius_of_sqrt,
        },
        Case {
            label: "(√2+1)/2",
            a: 1,
            b: 1,
            c: 0,
            d: 2,
            radicand: 2,
            eval_oracle: eval_mobius_of_sqrt,
        },
        Case {
            label: "√3 + 2",
            a: 1,
            b: 2,
            c: 0,
            d: 1,
            radicand: 3,
            eval_oracle: eval_mobius_of_sqrt,
        },
        Case {
            label: "(2√5−1)/3",
            a: 2,
            b: -1,
            c: 0,
            d: 3,
            radicand: 5,
            eval_oracle: eval_mobius_of_sqrt,
        },
        Case {
            label: "√7 / (√7+1)",
            a: 1,
            b: 0,
            c: 1,
            d: 1,
            radicand: 7,
            eval_oracle: eval_mobius_of_sqrt,
        },
        Case {
            label: "(3√2+1)/(√2+2)",
            a: 3,
            b: 1,
            c: 1,
            d: 2,
            radicand: 2,
            eval_oracle: eval_mobius_of_sqrt,
        },
    ];

    println!("\n===== Gosper-node NICF feasibility (②.2) =====");
    println!(
        "{:<16} {:>7} {:>8} {:>8} {:>10}",
        "value", "match", "RCF ing", "NICFing", "verdict"
    );

    let mut all_ok = true;
    let mut rcf_ingest_total = 0usize;
    let mut nicf_ingest_total = 0usize;

    for case in &cases {
        // Operand = √radicand, produced by the REAL engine.
        let surd = ExactReal::from_sqrt_rational(Fraction::from(case.radicand as i64))
            .expect("sqrt constructible");
        let op_terms = surd.partial_quotients_bounded(OP_PREFIX);

        // NICF via the Möbius stepper (the thing under test).
        let mut nicf_step = MobiusStepper::new(
            case.a,
            case.b,
            case.c,
            case.d,
            OperandStream::new(op_terms.clone()),
        );
        let nicf_got = nicf_step.emit_n(Mode::Nicf, EMIT);

        // RCF via the same machinery (cost baseline + sanity).
        let mut rcf_step = MobiusStepper::new(
            case.a,
            case.b,
            case.c,
            case.d,
            OperandStream::new(op_terms.clone()),
        );
        let _rcf_got = rcf_step.emit_n(Mode::Rcf, EMIT);

        // Oracle NICF of the same exact value.
        let (p, q) = sqrt_rational(case.radicand, ORACLE_DIGITS);
        let (vnum, vden) = (case.eval_oracle)(&p, &q, case.a, case.b, case.c, case.d);
        let oracle = oracle_nicf_terms(vnum, vden, EMIT);

        // Compare the emitted NICF prefix against the oracle prefix.
        let n = nicf_got.len().min(oracle.len());
        let matched = (0..n).take_while(|&i| nicf_got[i] == oracle[i]).count();
        let ok = n > 0 && matched == n && nicf_got.len() >= EMIT.min(oracle.len());
        all_ok &= ok;
        rcf_ingest_total += rcf_step.ingests;
        nicf_ingest_total += nicf_step.ingests;

        println!(
            "{:<16} {:>3}/{:<3} {:>8} {:>8} {:>10}",
            case.label,
            matched,
            n,
            rcf_step.ingests,
            nicf_step.ingests,
            if ok { "OK" } else { "MISMATCH" }
        );
        if !ok {
            println!("    oracle: {:?}", oracle);
            println!("    got   : {:?}", nicf_got);
        }
    }

    println!("---------------------------------------------");
    println!(
        "total operand ingests for {} emitted terms each:  RCF {}   NICF {}   ratio {:.2}×",
        EMIT,
        rcf_ingest_total,
        nicf_ingest_total,
        nicf_ingest_total as f64 / rcf_ingest_total.max(1) as f64
    );
    println!("=============================================\n");

    assert!(
        all_ok,
        "Gosper-node NICF emission did not match the exact-rational oracle on every case"
    );
    // Feasibility bar: NICF must not cost dramatically more operand ingests
    // than RCF to emit the same number of result terms. (NICF emits fewer/
    // bigger terms, so per-term it may ingest a touch more; we only guard
    // against a blow-up.)
    assert!(
        nicf_ingest_total <= rcf_ingest_total * 3,
        "NICF operand-ingest cost blew up vs RCF ({} vs {})",
        nicf_ingest_total,
        rcf_ingest_total
    );
}
