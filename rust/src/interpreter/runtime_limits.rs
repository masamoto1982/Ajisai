//! Unified resource-control limits for internal computation cost (CS5).
//!
//! The execution-step budget (`Interpreter::max_execution_steps`, charged once
//! per word in `execute_builtin.rs`) prices *how many words* run, but not the
//! expensive work performed **inside** a single word: algebraic term×term
//! products, reciprocal/conjugate recursion, sign/bounds precision doubling,
//! BigInt blow-up, huge materializations, and huge numeric-literal parses. A
//! word that loops internally counts as one step, so those costs bypass the
//! step budget entirely. Ajisai must stay exact but return a **diagnosable**
//! runtime failure at a resource ceiling rather than an approximation,
//! wraparound, panic, OOM, or WASM trap.
//!
//! [`RuntimeLimits`] gathers those ceilings in one place. It lives on the
//! interpreter (and every child runtime inherits a copy — it is **not** a
//! global), and small limits can be injected in tests to fire a guard without
//! actually allocating or computing anything huge.
//!
//! Limits are a safety control, never value semantics: conformance results must
//! not depend on a specific limit value, and all conformance must pass under
//! the documented defaults (SPEC §2.5).

use crate::error::{AjisaiError, ResourceLimit, Result};
use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;

/// Default cap on elements a single generative built-in (`RANGE`, `FILL`,
/// `RESHAPE`, …) may materialize in one call. Mirrors the historical
/// `MAX_MATERIALIZED_ELEMENTS` constant; each generated `Value` costs a few
/// hundred bytes, so one million elements bounds a call to a few hundred MiB
/// rather than a multi-gigabyte OOM abort.
pub const DEFAULT_MAX_MATERIALIZED_ELEMENTS: usize = 1_000_000;

/// Default cap on the byte length of a single source program handed to
/// `execute`, checked before tokenization allocates per-character buffers.
///
/// The default is deliberately generous (64 MiB): machine-generated programs
/// are legitimately several megabytes (the perf-benchmark's largest chain is
/// ~1.77 MB), so the *default* only rejects genuinely pathological input while
/// keeping the char-buffer allocation bounded. Memory-constrained hosts — the
/// WASM playground in particular — should inject a tighter `max_source_bytes`
/// via `Interpreter::set_runtime_limits`; that is exactly why the limit is a
/// per-interpreter injectable field rather than a global.
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;

/// Default cap on the digit count of a single numeric literal in source. A
/// 4096-digit integer is astronomically large for any legitimate program,
/// while the ceiling stops a megabyte-long literal from driving an expensive
/// BigInt parse (`Fraction::from_str`) before the value is ever built.
pub const DEFAULT_MAX_NUMERIC_LITERAL_DIGITS: usize = 4_096;

/// Default cap on accumulated internal numeric work units charged through the
/// work meter (algebraic products, reciprocal recursion, precision doubling,
/// enclosure refinement).
pub const DEFAULT_MAX_NUMERIC_WORK: u64 = 1_000_000_000;

/// Width of the machine word the bignum arithmetic underneath actually works
/// in. Work is priced in limb×limb products because that is what a bignum
/// multiply costs.
pub const WORK_LIMB_BITS: u64 = 64;

/// Limbs needed to hold `bits`, never zero — every operation costs at least one.
///
/// This is the unit the work meter counts in, and the reason it counts in this
/// unit rather than in operations: the meter used to charge a *term-pair*
/// count, which prices the loop and ignores the size of the numbers inside it.
/// Multiplying two 4096-digit integers and two single-digit ones both charged
/// one unit while differing by orders of magnitude in cost, so the meter could
/// not bound the thing it exists to bound.
pub fn work_limbs(bits: u64) -> u64 {
    bits.div_ceil(WORK_LIMB_BITS).max(1)
}

/// What one algebraic term pair costs, in limb-multiply units.
///
/// A Tier 1 term pair is not one bignum multiply. It multiplies two
/// coefficients *and* two radicands, then square-free-decomposes the product
/// against a basis that grows with the number of distinct primes in play, and
/// inserts the result into an ordered term map. Measured against the rational
/// path — which is one bignum multiply and therefore the meter's natural unit —
/// a term pair costs roughly three orders of magnitude more. Without this
/// factor the two paths ran on the same meter at wildly different prices, so a
/// ceiling calibrated for one was meaningless for the other.
///
/// Empirical, and rounded to a power of two. It was chosen by measuring both
/// paths on one container: the rational chain charged ~80,000 units/ms and the
/// algebraic cascade ~56 units/ms, a ratio near 1,400. The exact figure is a
/// property of that machine and this `num-bigint`, and the meter only needs to
/// be right to within the order of magnitude that keeps a ceiling meaningful
/// for both paths. It is deliberately *not* pinned by a timing test: a wall
/// clock in CI would be flaky, and a stale constant is a worse failure than an
/// unpinned one only if nobody re-measures — so re-measure when either path's
/// representation changes.
pub const ALGEBRAIC_PAIR_UNITS: u64 = 1_024;

/// The work a binary bignum operation of these two operand widths costs.
///
/// Multiplication and division are limb×limb (schoolbook is what `num-bigint`
/// uses at these sizes); addition and subtraction are linear in the wider
/// operand. Deliberately an *upper bound* on the real cost and deliberately
/// cheap to compute: it is charged before the operation runs, so a runaway
/// computation is refused rather than measured.
pub fn binary_numeric_work(left_bits: u64, right_bits: u64, multiplicative: bool) -> u64 {
    let left = work_limbs(left_bits);
    let right = work_limbs(right_bits);
    if multiplicative {
        left.saturating_mul(right)
    } else {
        left.max(right)
    }
}

/// What the work meter needs to know about one operand, independent of the
/// shape it arrived in.
///
/// The meter used to read a `Fraction` and stop there, which made it a meter on
/// the *representation* rather than on the arithmetic: `2 3 *` was charged and
/// `[ 2 ] 3 *` was free, because the second one leaves the scalar path and
/// every other path charged nothing. Whether an operand is stored as a scalar,
/// a one-element vector or an N-lane tensor is an internal decision
/// (LANG.AUTHORITY.FREEDOM says it is unobservable), and a safety control whose
/// price turns on an unobservable decision is not a control. A broadcast
/// performs one operation per lane, so lanes are what it is charged for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperandWork {
    /// Numeric leaves the operand carries. A scalar is one lane.
    pub lanes: u64,
    /// The widest leaf, in bits.
    pub bits: u64,
    /// The largest algebraic term count among the leaves, or 0 when every leaf
    /// is rational.
    pub terms: u64,
}

impl OperandWork {
    /// The measure of a single rational leaf.
    pub const fn leaf(bits: u64) -> Self {
        Self {
            lanes: 1,
            bits,
            terms: 0,
        }
    }

    /// Combine sibling leaves: lanes add, width and term count take the worst.
    pub fn join(self, other: Self) -> Self {
        Self {
            lanes: self.lanes.saturating_add(other.lanes),
            bits: self.bits.max(other.bits),
            terms: self.terms.max(other.terms),
        }
    }
}

/// The work a binary arithmetic operation costs across every lane a broadcast
/// will touch.
///
/// `pair_units` is what one lane pair costs *above* the bignum operation: 1 for
/// a rational pair, and the term-pair count times [`ALGEBRAIC_PAIR_UNITS`] for
/// an algebraic one. A scalar operation is `lanes = 1` of this and is charged
/// exactly what the scalar-only meter charged before, which is what keeps the
/// existing calibration — and the ordinary-work regression pinned against it —
/// meaningful across this change.
pub fn broadcast_numeric_work(
    left: OperandWork,
    right: OperandWork,
    multiplicative: bool,
    pair_units: u64,
) -> u64 {
    // Broadcast repeats the narrower operand across the wider one, so the
    // wider lane count is how many operations actually run.
    let lanes = left.lanes.max(right.lanes).max(1);
    binary_numeric_work(left.bits, right.bits, multiplicative)
        .saturating_mul(pair_units)
        .saturating_mul(lanes)
}

/// Default cap on the bit length of a BigInt arithmetic result. ~300k decimal
/// digits — generous for exact rationals, but bounded so a doubling cascade
/// cannot blow up to gigabytes. Consumed by the work meter in the follow-up.
pub const DEFAULT_MAX_BIGINT_BITS: u64 = 1_000_000;

/// Default cap on the number of algebraic terms a single continued-fraction /
/// polynomial value may carry. Consumed by the work meter in the follow-up.
pub const DEFAULT_MAX_ALGEBRAIC_TERMS: usize = 100_000;

/// Operand width for the work meter on an exact (Tier 0 or Tier 1) value:
/// the wider of what it stores in coefficients and in radicands.
pub fn exact_work_bits(value: &ExactReal) -> u64 {
    match value {
        ExactReal::Algebraic(a) => a.max_coefficient_bits().max(a.max_radicand_bits()),
        other => other.max_coefficient_bits(),
    }
}

/// Operand width for the work meter, without paying to measure a small one.
///
/// `Fraction` keeps machine-word values in a `Small` representation, which is
/// the overwhelming majority of ordinary arithmetic; asking a `BigInt` for its
/// bit length on every add in a hot loop would cost more than the guard saves.
/// A small fraction is one limb by definition.
pub fn fraction_work_bits(f: &Fraction) -> u64 {
    if f.is_small() {
        return 1;
    }
    fraction_result_bits(f)
}

/// The width a fraction actually occupies: the wider of its two halves.
pub fn fraction_result_bits(f: &Fraction) -> u64 {
    if f.is_nil() {
        return 0;
    }
    f.numerator().bits().max(f.denominator().bits())
}

/// Unified internal-computation-cost ceilings (CS5).
///
/// This deliberately does **not** include the execution-step budget, which
/// remains the adjacent `Interpreter::max_execution_steps` field: the step
/// budget prices word count, whereas `RuntimeLimits` prices the per-word
/// internal work the step budget cannot see. The two together are the
/// interpreter's complete runtime-safety envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeLimits {
    /// Max elements a single generative built-in may materialize in one call.
    /// Folds the former `MAX_MATERIALIZED_ELEMENTS` guard into this structure.
    pub max_materialized_elements: usize,
    /// Max byte length of a source program handed to `execute` (checked before
    /// tokenization).
    pub max_source_bytes: usize,
    /// Max digit count of a single numeric literal (checked before the BigInt
    /// parse builds the value).
    pub max_numeric_literal_digits: usize,
    /// Max accumulated internal numeric work units. Consumed by the work meter
    /// in the CS5 follow-up.
    pub max_numeric_work: u64,
    /// Max bit length of a BigInt arithmetic result. Consumed by the work meter
    /// in the CS5 follow-up.
    pub max_bigint_bits: u64,
    /// Max algebraic-term count of a single continued-fraction / polynomial
    /// value. Consumed by the work meter in the CS5 follow-up.
    pub max_algebraic_terms: usize,
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            max_materialized_elements: DEFAULT_MAX_MATERIALIZED_ELEMENTS,
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_numeric_literal_digits: DEFAULT_MAX_NUMERIC_LITERAL_DIGITS,
            max_numeric_work: DEFAULT_MAX_NUMERIC_WORK,
            max_bigint_bits: DEFAULT_MAX_BIGINT_BITS,
            max_algebraic_terms: DEFAULT_MAX_ALGEBRAIC_TERMS,
        }
    }
}

impl RuntimeLimits {
    /// Reject a source program larger than `max_source_bytes`, before
    /// tokenization. Returns a diagnosable `AjisaiError` (never a panic/OOM).
    pub fn check_source_bytes(&self, byte_len: usize) -> Result<()> {
        if byte_len > self.max_source_bytes {
            return Err(AjisaiError::ResourceLimitExceeded {
                resource: ResourceLimit::SourceBytes,
                limit: self.max_source_bytes as u64,
                observed: Some(byte_len as u64),
            });
        }
        Ok(())
    }

    /// Reject a numeric literal with more than `max_numeric_literal_digits`
    /// digits, before the BigInt parse builds the value. `digit_len` counts
    /// digit characters only (sign, radix point, and separators excluded).
    pub fn check_numeric_literal_digits(&self, digit_len: usize) -> Result<()> {
        if digit_len > self.max_numeric_literal_digits {
            return Err(AjisaiError::ResourceLimitExceeded {
                resource: ResourceLimit::NumericLiteralDigits,
                limit: self.max_numeric_literal_digits as u64,
                observed: Some(digit_len as u64),
            });
        }
        Ok(())
    }

    /// Reject an exact (Tier 1 algebraic) arithmetic result whose size crosses
    /// the internal-computation ceilings: `term_count` past
    /// `max_algebraic_terms` (multiplicative term explosion, e.g. repeatedly
    /// multiplying distinct `√p`), or `coeff_bits` past `max_bigint_bits`
    /// (BigInt blow-up). Bounds *accumulation* so operands feeding the next
    /// operation stay sane; the per-operation work is bounded separately by the
    /// work meter's pre-charge. Each ceiling reports itself by name through
    /// `ResourceLimitExceeded`: "this value carries too many algebraic terms"
    /// and "this coefficient is too many bits wide" are separately declared
    /// limits, and folding both into the step budget's category left a host
    /// unable to say which of the two an agent had actually hit.
    pub fn check_algebraic_size(&self, term_count: usize, coeff_bits: u64) -> Result<()> {
        if term_count > self.max_algebraic_terms {
            return Err(AjisaiError::ResourceLimitExceeded {
                resource: ResourceLimit::AlgebraicTerms,
                limit: self.max_algebraic_terms as u64,
                observed: Some(term_count as u64),
            });
        }
        self.check_bigint_bits(coeff_bits)
    }

    /// Reject an arithmetic result wider than `max_bigint_bits`.
    ///
    /// Split out of `check_algebraic_size` because it is not an algebraic
    /// concern: a plain rational product blows a coefficient up exactly the
    /// same way, and used to reach this ceiling through no path at all. A
    /// repeated-multiplication chain was bounded only by the *step* budget,
    /// which prices word count — so four hundred multiplications, 0.4% of that
    /// budget, could spend tens of seconds building a multi-megabyte integer
    /// while every size ceiling stayed silent.
    pub fn check_bigint_bits(&self, bits: u64) -> Result<()> {
        if bits > self.max_bigint_bits {
            return Err(AjisaiError::ResourceLimitExceeded {
                resource: ResourceLimit::BigintBits,
                limit: self.max_bigint_bits,
                observed: Some(bits),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_documented_ceilings() {
        let limits = RuntimeLimits::default();
        assert_eq!(
            limits.max_materialized_elements,
            DEFAULT_MAX_MATERIALIZED_ELEMENTS
        );
        assert_eq!(limits.max_source_bytes, DEFAULT_MAX_SOURCE_BYTES);
        assert_eq!(
            limits.max_numeric_literal_digits,
            DEFAULT_MAX_NUMERIC_LITERAL_DIGITS
        );
    }

    #[test]
    fn source_byte_ceiling_fires_at_a_low_injected_limit() {
        let limits = RuntimeLimits {
            max_source_bytes: 8,
            ..RuntimeLimits::default()
        };
        assert!(
            limits.check_source_bytes(8).is_ok(),
            "at the limit is allowed"
        );
        let err = limits
            .check_source_bytes(9)
            .expect_err("over the limit must error");
        assert!(err.to_string().contains("exceeds the limit"));
    }

    #[test]
    fn numeric_literal_digit_ceiling_fires_at_a_low_injected_limit() {
        let limits = RuntimeLimits {
            max_numeric_literal_digits: 4,
            ..RuntimeLimits::default()
        };
        assert!(limits.check_numeric_literal_digits(4).is_ok());
        let err = limits
            .check_numeric_literal_digits(5)
            .expect_err("over the digit limit must error");
        assert!(err.to_string().contains("exceeds the limit"));
    }
}
