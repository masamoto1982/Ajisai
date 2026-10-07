//! Where arithmetic is charged and bounded — the work meter's boundary.
//!
//! This lives beside the arithmetic dispatch rather than inside it, because it
//! is a property *of* the dispatch and not of any route the dispatch chooses.
//! It used to be neither: two of the six routes out of
//! `arithmetic::apply_exact_arithmetic_schema` charged, both reachable only
//! when both operands were scalar-shaped, and the other four were free. So
//! `2 3 MUL` was priced and `[ 2 ] 3 MUL` was not, and `algebraicTerms` was a
//! ceiling a vector literal turned off. Which route runs is an optimization
//! decision, unobservable by LANG.AUTHORITY.FREEDOM; a safety control priced
//! per route made it observable, which is the one thing a limit must never do.
//!
//! The charge is taken at the entry, before a route is chosen and before either
//! operand is consumed, so a refusal leaves the stack as the program left it.
//! The size check is taken per route, on the result, before the operands go —
//! it bounds what an operation *leaves behind* so the operand feeding the next
//! one is still a sane size.

use std::cell::Cell;

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::arithmetic::{ExactArithmeticSchema, ScalarFastWrap};
use crate::interpreter::runtime_limits::{
    broadcast_numeric_work, exact_work_bits, fraction_result_bits, fraction_work_bits, OperandWork,
    ALGEBRAIC_PAIR_UNITS,
};
use crate::interpreter::tensor_lane_ops::apply_lane_wise_broadcast;
use crate::interpreter::tensor_ops::apply_binary_broadcast;
use crate::interpreter::value_extraction_helpers::extract_operands;
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::{DenseTensor, Value, ValueData};

/// The work meter's view of an operand, gathered without regard to how the
/// value stores its lanes.
pub(crate) fn measure_operand(value: &Value) -> OperandWork {
    match &value.data {
        ValueData::Scalar(f) => OperandWork::leaf(fraction_work_bits(f)),
        ValueData::ExactScalar(er) => OperandWork {
            lanes: 1,
            bits: exact_work_bits(er),
            terms: er.algebraic_term_count() as u64,
        },
        // A dense tensor's lanes are `i64` numerators and denominators by
        // construction, so each is a single limb and the width is known without
        // reading the data — which matters, because this runs on every
        // arithmetic Word and a tensor is the shape that carries a million of
        // them.
        ValueData::Tensor { data, .. } => OperandWork {
            lanes: data.len() as u64,
            bits: 1,
            terms: 0,
        },
        ValueData::Vector(children) => children
            .iter()
            .map(measure_operand)
            .reduce(OperandWork::join)
            .unwrap_or(OperandWork::leaf(1)),
        // A Record lifts arithmetic over its values (LANG.COLLECTIONS.LIFT),
        // so its work is theirs.
        ValueData::Record(record) => record
            .values()
            .iter()
            .map(measure_operand)
            .reduce(OperandWork::join)
            .unwrap_or(OperandWork::leaf(1)),
        // No arithmetic happens on these; the structure error they raise is
        // not work.
        ValueData::Boolean(_) | ValueData::Nil | ValueData::Text(_) | ValueData::Symbol(_) => {
            OperandWork::leaf(1)
        }
    }
}

/// Charge the whole operation before any of it runs, and before either operand
/// is consumed — so a refusal leaves the stack exactly as the program left it
/// and the interpreter stays usable.
pub(crate) fn charge_binary_schema(
    interp: &mut Interpreter,
    schema: ExactArithmeticSchema,
    left: OperandWork,
    right: OperandWork,
) -> Result<()> {
    let algebraic = left.terms > 0 || right.terms > 0;
    let pair_units = if algebraic {
        // A term pair is not one bignum multiply — it is a coefficient product,
        // a radicand product, a square-free decomposition against a growing
        // basis and an ordered-map insert — so it carries the measured
        // constant on top of the bignum work each pair performs.
        let left_terms = left.terms.max(1);
        let right_terms = right.terms.max(1);
        let pairs = match schema {
            ExactArithmeticSchema::Mul => left_terms.saturating_mul(right_terms),
            // Division inverts the right operand (conjugation recursion, ~term²
            // inner products) and multiplies; bound by both.
            ExactArithmeticSchema::Div => left_terms
                .saturating_mul(right_terms)
                .saturating_add(right_terms.saturating_mul(right_terms)),
            ExactArithmeticSchema::Add | ExactArithmeticSchema::Sub => {
                left_terms.saturating_add(right_terms)
            }
        };
        pairs.saturating_mul(ALGEBRAIC_PAIR_UNITS)
    } else {
        1
    };

    interp.charge_numeric_work(broadcast_numeric_work(left, right, pair_units))
}

/// The widest lane of a dense tensor. Its lanes are `i64` by construction, so
/// this reads machine words rather than bignums.
fn dense_tensor_result_bits(data: &DenseTensor) -> u64 {
    fn width(value: i64) -> u64 {
        u64::from(64 - value.unsigned_abs().leading_zeros())
    }
    data.numerators
        .iter()
        .zip(data.denominators.iter())
        .map(|(numerator, denominator)| width(*numerator).max(width(*denominator)))
        .max()
        .unwrap_or(0)
}

/// The widest lane and largest algebraic term count a result carries.
fn measure_result(value: &Value) -> (u64, usize) {
    match &value.data {
        ValueData::Scalar(f) => (fraction_result_bits(f), 0),
        ValueData::ExactScalar(er) => (er.max_coefficient_bits(), er.algebraic_term_count()),
        ValueData::Tensor { data, .. } => (dense_tensor_result_bits(data), 0),
        ValueData::Vector(children) => children.iter().fold((0, 0), |(bits, terms), child| {
            let (child_bits, child_terms) = measure_result(child);
            (bits.max(child_bits), terms.max(child_terms))
        }),
        ValueData::Record(record) => record.values().iter().fold((0, 0), |(bits, terms), child| {
            let (child_bits, child_terms) = measure_result(child);
            (bits.max(child_bits), terms.max(child_terms))
        }),
        ValueData::Boolean(_) | ValueData::Nil | ValueData::Text(_) | ValueData::Symbol(_) => {
            (0, 0)
        }
    }
}

/// Reject a result whose widest lane crosses the accumulation ceilings, so the
/// operand feeding the next operation is still a sane size. The per-operation
/// cost is bounded separately by the pre-charge above; this bounds what the
/// operation leaves behind.
pub(crate) fn check_result_size(interp: &Interpreter, value: &Value) -> Result<()> {
    let (bits, terms) = measure_result(value);
    interp.runtime_limits.check_algebraic_size(terms, bits)
}

// The zero-divisor projection law: what a zero divisor does to the value
// around it, for `DIV`, the Word that meets one.
//
// Split out of `arithmetic.rs` because these are the exact-arithmetic laws
// that can *project* — answer NIL for a well-formed operand
// (`LANG.FAILURE.TRICHOTOMY`) — and lifting a projecting law over a
// collection is a different problem from lifting a total one. `ADD`, `SUB`
// and `MUL` either answer with a number in every lane or raise.
//
// A remainder written out as `a - b * floor(a/b)` goes through the same
// division, so a zero divisor answers the same way whichever phrase wraps it.
pub(crate) fn division_by_zero_projection() -> Value {
    Value::nil_with_reason(NilReason::DivisionByZero, Recoverability::Recoverable)
}

/// The scalar law of `DIV` as a whole `Value`, for the lane-wise lift.
///
/// A zero divisor is a projection, not a failure (`LANG.FAILURE.TRICHOTOMY`),
/// so it answers with the reasoned NIL the scalar `6 0 DIV` answers with.
///
/// An absent operand never reaches here: `apply_lane_wise_broadcast` lifts the
/// scalar passthrough law over each lane *before* consulting this one, while
/// the lane is still a `Value` and its reason is still readable. The guard
/// stays because it is not only about absence — `Fraction::nil` has
/// denominator *and* numerator 0, so an absent divisor answers `is_zero` too,
/// and dropping the test would read one as a zero divisor and invent a
/// `divisionByZero` the program never performed.
fn divide_lane(a: &Fraction, b: &Fraction) -> Result<Value> {
    if a.is_nil() || b.is_nil() {
        return Ok(Value::nil());
    }
    if b.is_zero() {
        return Ok(division_by_zero_projection());
    }
    Ok(Value::from_fraction(a.div(b)))
}

/// A zero divisor on the one-lane fast path projects *inside* the operand's
/// wrap, for the same reason it projects per lane in the broadcast: the shape
/// of `[ 6 ] [ 0 ] DIV` is the shape of `[ 6 ] [ 2 ] DIV`. Answering with a bare
/// NIL here made `DIV` the one Word whose result shape depended on whether it
/// projected — `[ 6 ] [ 2 ] DIV` gave `[ 3/1 ]` while `[ 6 ] [ 0 ] DIV` gave a
/// scalar `NIL`.
///
/// The projection is a reasoned NIL, so the wrap is rebuilt as a nested
/// `Vector`: a dense lane could hold the absence but not the reason for it.
pub(crate) fn build_scalar_fast_projection(wrap: &ScalarFastWrap) -> Value {
    match wrap {
        ScalarFastWrap::Scalar => division_by_zero_projection(),
        ScalarFastWrap::Tensor(shape) => {
            let mut value = division_by_zero_projection();
            for _ in shape {
                value = Value::from_children(vec![value]);
            }
            value
        }
    }
}

/// The `DIV` arm of [`apply_exact_arithmetic_schema`], after the fast paths
/// declined it.
///
/// [`apply_exact_arithmetic_schema`]: crate::interpreter::arithmetic
pub(crate) fn apply_division_schema(
    interp: &mut Interpreter,
    schema: ExactArithmeticSchema,
) -> Result<()> {
    let stack_len = interp.stack.len();
    if stack_len >= 2 {
        let slots = interp.stack.as_slice();
        let left_is_text = slots[stack_len - 2].is_text();
        let right_is_text = slots[stack_len - 1].is_text();
        if left_is_text || right_is_text {
            return Err(AjisaiError::declared(
                "nonNumeric",
                "expected a Scalar, got String",
            ));
        }
    }
    let operands = extract_operands(interp, 2)?;
    let a_val = &operands[0];
    let b_val = &operands[1];

    let computed = apply_binary_broadcast(a_val, b_val, |a, b| schema.fraction(a, b))
        // Bound accumulation before the result is pushed; on a refusal the
        // operands go back, exactly as for any other failure of this arm.
        .and_then(|result| {
            check_result_size(interp, &result)?;
            Ok(result)
        });

    // `LANG.COLLECTIONS.LIFT`: "Each lane preserves the exactness, truth, NIL,
    // and ERROR distinctions of the scalar law." A zero divisor empties its own
    // lane; it does not empty the vector.
    //
    // The flat rational broadcast above cannot say that. Its leaf law answers
    // with a `Fraction`, so a projection can only surface as one error for the
    // whole operation, and the lanes that had already divided were discarded
    // with it: `[ 6 6 6 ] [ 1 2 0 ] DIV` answered `NIL` where the same division
    // through `MAP` answered `[ 6/1 3/1 NIL ]`, so one `DIV` meant two
    // different things depending on the route it took.
    //
    // Re-run it lane-wise, where the leaf law answers with a value and each
    // projection carries its own reason — the shape `SQRT` already produces for
    // a negative lane. The re-run costs a second pass only when a zero divisor
    // was actually met; a division that projects nothing keeps the flat path.
    let computed = match computed {
        Err(AjisaiError::DivisionByZero) => apply_lane_wise_broadcast(a_val, b_val, divide_lane)
            .and_then(|result| {
                check_result_size(interp, &result)?;
                Ok(result)
            }),
        other => other,
    };

    match computed {
        Ok(result) => {
            interp.stack.push(result);
            Ok(())
        }
        Err(error) => {
            for val in operands {
                interp.stack.push(val);
            }
            Err(error)
        }
    }
}

// The share of the run's `numericWork` a square root may spend factoring its
// radicand (`types::exact::squarefree`).
//
// A radicand is reduced to its square-free part so that one number has one
// normal form (LANG.VALUES.DENOTATION). That needs a factorization, whose cost
// is not bounded by anything the operand's size alone predicts, so it is
// metered on the same budget as the rest of the arithmetic: the root is taken
// against what the run has left, the work actually spent is charged
// afterwards, and a radicand the budget cannot factor is the same
// `resourceLimitExceeded` any other exhausted work meter raises.
pub(crate) struct RadicandBudget {
    limit: u64,
    start: u64,
    remaining: Cell<u64>,
    exhausted: Cell<bool>,
}

impl RadicandBudget {
    pub(crate) fn of(interp: &Interpreter) -> Self {
        let start = interp
            .runtime_limits
            .max_numeric_work
            .saturating_sub(interp.numeric_work_used);
        RadicandBudget {
            limit: interp.runtime_limits.max_numeric_work,
            start,
            remaining: Cell::new(start),
            exhausted: Cell::new(false),
        }
    }

    /// The budget left for one root, taken back by [`Self::spent`].
    pub(crate) fn take(&self) -> u64 {
        self.remaining.get()
    }

    /// Record what one root left of the budget it was given, and whether it
    /// ran out.
    pub(crate) fn spent(&self, left: u64, exhausted: bool) {
        self.remaining.set(left);
        if exhausted {
            self.exhausted.set(true);
        }
    }

    /// √`radicand` within the budget, or the exhausted meter's error.
    pub(crate) fn sqrt(&self, radicand: Fraction) -> Result<Option<ExactReal>> {
        let mut left = self.take();
        let root = ExactReal::try_sqrt_rational(radicand, &mut left);
        self.spent(left, root.is_err());
        root.map_err(|_| self.exhausted_error())
    }

    /// The error the work meter raises when this root's factorization used
    /// up what the run had left.
    pub(crate) fn exhausted_error(&self) -> AjisaiError {
        AjisaiError::ResourceLimitExceeded {
            resource: crate::error::ResourceLimit::NumericWork,
            limit: self.limit,
            observed: Some(self.limit.saturating_add(1)),
            progress: None,
        }
    }

    /// Charge the work spent to the run's meter; an exhausted budget charges
    /// everything that was left and one unit more.
    pub(crate) fn settle(&self, interp: &mut Interpreter) -> Result<()> {
        if self.exhausted.get() {
            return interp.charge_numeric_work(self.start.saturating_add(1));
        }
        interp.charge_numeric_work(self.start - self.remaining.get())
    }
}

#[cfg(test)]
mod tests {
    //! Behavioral coverage for the ExactScalar path of `op_div` (LANG.VALUES.EXACT).
    //!
    //! Regression guard for the ordering bug where the generic broadcast block
    //! (`apply_binary_broadcast`) ran *before* the ExactScalar
    //! block and unconditionally `return`ed on the `FlatTensor::from_value`
    //! error for exact irrationals, making the ExactScalar `DIV` path dead code.
    //! `op_add`/`op_sub`/`op_mul` place the ExactScalar block before broadcast;
    //! these tests pin `op_div` to the same, correct ordering.

    use crate::error::NilReason;

    use crate::test_support::run_ok;
    use crate::types::ValueData;

    /// `√2 2 /` is evaluated as an exact value rather than hard-erroring on the
    /// (impossible) ExactScalar -> tensor conversion. This is the core P0-a
    /// guarantee: the ExactScalar block is now reachable before broadcast.
    #[tokio::test]
    async fn sqrt2_div_rational_is_exact_not_error() {
        let stack = run_ok("2 SQRT 2 DIV").await;
        assert_eq!(stack.len(), 1);
        let top = &stack[0];
        assert!(
            !top.is_nil(),
            "√2 2 / must not collapse to NIL/error, got {top:?}"
        );
        assert!(
            matches!(top.data, ValueData::ExactScalar(_) | ValueData::Scalar(_)),
            "√2 2 / must stay an exact real, got {top:?}"
        );
    }

    /// `√2 √2 /` is evaluated exactly (a recoverable exact real), not a hard
    /// error. NOTE: the current ExactReal engine represents this as a *lazy*
    /// `Gosper` (x/y) and does not structurally reduce it to the rational `1/1`;
    /// structural simplification is a CF-engine concern outside the P0-a
    /// ordering fix. The reachable-path guarantee (exact, not error) is what
    /// this test pins.
    #[tokio::test]
    async fn sqrt2_div_sqrt2_is_exact() {
        let stack = run_ok("2 SQRT 2 SQRT DIV").await;
        assert_eq!(stack.len(), 1);
        let top = &stack[0];
        assert!(
            !top.is_nil(),
            "√2 √2 / must not collapse to NIL/error, got {top:?}"
        );
        assert!(
            matches!(top.data, ValueData::ExactScalar(_) | ValueData::Scalar(_)),
            "√2 √2 / must stay an exact real, got {top:?}"
        );
    }

    /// `√2 0 /` is a recoverable DivisionByZero projection, not a hard error.
    #[tokio::test]
    async fn sqrt2_div_zero_is_division_by_zero_projection() {
        let stack = run_ok("2 SQRT 0 DIV").await;
        assert_eq!(stack.len(), 1);
        let top = &stack[0];
        assert!(top.is_nil(), "√2 0 / must be a reasoned NIL, got {top:?}");
        assert_eq!(
            top.nil_reason().cloned(),
            Some(NilReason::DivisionByZero),
            "√2 0 / must carry NilReason::DivisionByZero, got {top:?}"
        );
    }

    /// Ordinary rational division is unchanged (no regression): `6 3 DIV` -> `2`.
    #[tokio::test]
    async fn rational_division_unchanged() {
        let stack = run_ok("6 3 DIV").await;
        assert_eq!(stack.len(), 1);
        let frac = stack[0]
            .as_scalar()
            .cloned()
            .unwrap_or_else(|| panic!("6 3 / must be a rational, got {:?}", stack[0]));
        assert_eq!(frac.to_i64(), Some(2), "6 3 / must equal 2");
    }
}
