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

use crate::error::{AjisaiError, Result};

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
/// enclosure refinement). Consumed by the work meter in the CS5 follow-up.
pub const DEFAULT_MAX_NUMERIC_WORK: u64 = 1_000_000_000;

/// Default cap on the bit length of a BigInt arithmetic result. ~300k decimal
/// digits — generous for exact rationals, but bounded so a doubling cascade
/// cannot blow up to gigabytes. Consumed by the work meter in the follow-up.
pub const DEFAULT_MAX_BIGINT_BITS: u64 = 1_000_000;

/// Default cap on the number of algebraic terms a single continued-fraction /
/// polynomial value may carry. Consumed by the work meter in the follow-up.
pub const DEFAULT_MAX_ALGEBRAIC_TERMS: usize = 100_000;

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
            return Err(AjisaiError::from(format!(
                "source program of {} bytes exceeds the limit of {} bytes",
                byte_len, self.max_source_bytes
            )));
        }
        Ok(())
    }

    /// Reject a numeric literal with more than `max_numeric_literal_digits`
    /// digits, before the BigInt parse builds the value. `digit_len` counts
    /// digit characters only (sign, radix point, and separators excluded).
    pub fn check_numeric_literal_digits(&self, digit_len: usize) -> Result<()> {
        if digit_len > self.max_numeric_literal_digits {
            return Err(AjisaiError::from(format!(
                "numeric literal of {} digits exceeds the limit of {} digits",
                digit_len, self.max_numeric_literal_digits
            )));
        }
        Ok(())
    }

    /// Reject an exact (Tier 1 algebraic) arithmetic result whose size crosses
    /// the internal-computation ceilings: `term_count` past
    /// `max_algebraic_terms` (multiplicative term explosion, e.g. repeatedly
    /// multiplying distinct `√p`), or `coeff_bits` past `max_bigint_bits`
    /// (BigInt blow-up). Bounds *accumulation* so operands feeding the next
    /// operation stay sane; the per-operation work is bounded separately by the
    /// work meter's pre-charge. Maps to `ExecutionLimitExceeded` — an existing
    /// resource-limit category, per the CS5 plan — never a new category.
    pub fn check_algebraic_size(&self, term_count: usize, coeff_bits: u64) -> Result<()> {
        if term_count > self.max_algebraic_terms {
            return Err(AjisaiError::ExecutionLimitExceeded {
                limit: self.max_algebraic_terms,
            });
        }
        if coeff_bits > self.max_bigint_bits {
            return Err(AjisaiError::ExecutionLimitExceeded {
                limit: usize::try_from(self.max_bigint_bits).unwrap_or(usize::MAX),
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
