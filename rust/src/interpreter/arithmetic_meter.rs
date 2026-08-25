//! Where arithmetic is charged and bounded — the work meter's boundary.
//!
//! This lives beside the arithmetic dispatch rather than inside it, because it
//! is a property *of* the dispatch and not of any route the dispatch chooses.
//! It used to be neither: two of the six routes out of
//! `arithmetic::apply_exact_arithmetic_schema` charged, both reachable only
//! when both operands were scalar-shaped, and the other four were free. So
//! `2 3 *` was priced and `[ 2 ] 3 *` was not, and `algebraicTerms` was a
//! ceiling a vector literal turned off. Which route runs is an optimization
//! decision, unobservable by LANG.AUTHORITY.FREEDOM; a safety control priced
//! per route made it observable, which is the one thing a limit must never do.
//!
//! The charge is taken at the entry, before a route is chosen and before either
//! operand is consumed, so a refusal leaves the stack as the program left it.
//! The size check is taken per route, on the result, before the operands go —
//! it bounds what an operation *leaves behind* so the operand feeding the next
//! one is still a sane size.

use crate::error::Result;
use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::runtime_limits::{
    broadcast_numeric_work, exact_work_bits, fraction_result_bits, fraction_work_bits, OperandWork,
    ALGEBRAIC_PAIR_UNITS,
};
use crate::interpreter::Interpreter;
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
