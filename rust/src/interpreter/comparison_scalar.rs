//! The scalar comparison law of LANG.VALUES.EXACT — how two numeric operands
//! are ordered and tested for equality.
//!
//! Split out of `comparison.rs`, which keeps the comparison *Words*: the
//! element-wise lifting, the stack fast paths, and the `LT`/`GT`/`EQ` entry
//! points. What is here is the law those Words apply, one scalar pair at a
//! time: a rational pair decides by `Fraction`, and any pair reaching the
//! algebraic field decides exactly through `ExactReal::cmp_exact`. Every pair
//! decides.

use crate::error::{AjisaiError, Result};
use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

/// One of the four ordering comparisons. Carries the dispatch decision
/// through the scalar-comparison helper, which keeps the Fraction fast path
/// for both-Rational operands and routes any other pair through the total
/// Tier 1 `ExactReal::cmp_exact` (LANG.VALUES.EXACT).
#[derive(Debug, Clone, Copy)]
pub(crate) enum OrderingKind {
    Lt,
    Gt,
}

impl OrderingKind {
    pub(crate) fn apply_to_fraction(self, a: &Fraction, b: &Fraction) -> bool {
        match self {
            OrderingKind::Lt => a.lt(b),
            OrderingKind::Gt => a.gt(b),
        }
    }

    /// Apply the relation to a decided `ExactReal` three-way ordering.
    pub(crate) fn apply_ordering(self, o: std::cmp::Ordering) -> bool {
        use std::cmp::Ordering;
        match self {
            OrderingKind::Lt => o == Ordering::Less,
            OrderingKind::Gt => o == Ordering::Greater,
        }
    }
}

/// The two rationals a pair of operands compares as, borrowed rather than built.
///
/// All three routes below already end in a `Fraction` comparison when both
/// operands are rational — that is their `(Some, Some)` arm. They reached it by
/// constructing an `ExactReal` from each operand, and
/// `extract_exact_real_for_comparison` clones the `Fraction` out of the `Value`
/// to do it: two clones and two constructions per comparison, to arrive at the
/// two `Fraction`s the operands already held. A `ValueData::Scalar` *is* a
/// rational, so this borrows them.
///
/// Only that one shape is screened. `ExactScalar` (Tier 1 algebraic), `Text`,
/// `Vector` and the rest fall through to the general route, which is the only
/// one that can answer for them — an algebraic pair through the total
/// `ExactReal::cmp_exact`. A nil
/// `Fraction` (the 0-denominator sentinel) falls through too: it is not a
/// rational, and this is not the place to decide what comparing one means.
pub(crate) fn rational_pair<'a>(
    a_val: &'a Value,
    b_val: &'a Value,
) -> Option<(&'a Fraction, &'a Fraction)> {
    match (&a_val.data, &b_val.data) {
        (ValueData::Scalar(a), ValueData::Scalar(b)) if !a.is_nil() && !b.is_nil() => Some((a, b)),
        _ => None,
    }
}

/// Compare two scalar values under an ordering kind. Returns `Err(_)` for
/// structurally-non-comparable operands. Both-rational operands take the
/// Fraction fast path; an algebraic pair decides through the total
/// `ExactReal::cmp_exact`.
pub(crate) fn compare_scalar_pair(
    a_val: &Value,
    b_val: &Value,
    kind: OrderingKind,
) -> Result<bool> {
    if let Some((a, b)) = rational_pair(a_val, b_val) {
        return Ok(kind.apply_to_fraction(a, b));
    }
    Ok(kind.apply_ordering(three_way_compare(a_val, b_val)?))
}

/// Three-way order of two scalar values (LANG.VALUES.EXACT), shared by the
/// comparison-dependent words (`MIN`, `MAX`, `SORT`, `ORDER`, `BSEARCH`).
/// Returns `Err(_)` for structurally non-comparable operands (the
/// malformed-use path). Both-`Rational` operands take the exact `Fraction`
/// fast path; any pair involving an algebraic decides through the total
/// `ExactReal::cmp_exact`.
pub(crate) fn three_way_compare(a_val: &Value, b_val: &Value) -> Result<std::cmp::Ordering> {
    if let Some((a, b)) = rational_pair(a_val, b_val) {
        return Ok(a.cmp(b));
    }
    let a = extract_exact_real_for_comparison(a_val)?;
    let b = extract_exact_real_for_comparison(b_val)?;
    a.cmp_exact(&b)
        .ok_or_else(|| AjisaiError::create_structure_error("scalar value", "NIL"))
}

/// Extract an `ExactReal` view of a value's scalar content for
/// comparison. Scalar (`Fraction`-backed) values lift to
/// `ExactReal::Rational`; singleton Vector / Tensor values also
/// project to their sole scalar. Non-scalar shapes and non-numeric
/// kinds error.
pub(crate) fn extract_exact_real_for_comparison(val: &Value) -> Result<ExactReal> {
    if let ValueData::ExactScalar(er) = &val.data {
        return Ok(er.clone());
    }
    let f = extract_scalar_for_comparison(val)?;
    Ok(ExactReal::from_fraction(f))
}

pub(crate) fn extract_scalar_for_comparison(val: &Value) -> Result<Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Ok(f.clone()),
        ValueData::ExactScalar(er) => {
            // Provide best rational approximation for contexts requiring a Fraction
            use num_bigint::BigInt;
            er.best_rational_approximation(&BigInt::from(1_000_000_000u64))
                .ok_or_else(|| {
                    AjisaiError::create_structure_error("scalar value", "non-rational ExactReal")
                })
        }
        ValueData::Text(_) => Err(AjisaiError::create_structure_error(
            "scalar value",
            "string",
        )),
        // A Vector never reaches the scalar law: `lift_comparison` peels it
        // element-wise first. A one-element Vector used to project to its sole
        // element here, which made `[ 3 ] 4 LT` answer `TRUE` — a collapse
        // LANG.COLLECTIONS.LIFT forbids ("a scalar combines with every element
        // of a vector"), and one that contradicts a singleton Vector not being
        // its element (LANG.VALUES.DISJOINT).
        ValueData::Vector(_) | ValueData::Tensor { .. } | ValueData::Record(_) => Err(
            AjisaiError::create_structure_error("scalar value", "non-scalar value"),
        ),
        ValueData::Nil => Err(AjisaiError::create_structure_error(
            "scalar value",
            "non-scalar value",
        )),
        ValueData::Boolean(_) | ValueData::Symbol(_) => Err(AjisaiError::create_structure_error(
            "scalar value",
            "non-scalar value",
        )),
    }
}

/// Scalar–scalar equality (LANG.VALUES.EXACT). Both-Rational operands decide
/// via `Fraction` `PartialEq` — value equality on canonical reduced
/// rationals. Anything mixing in a Tier 1 algebraic decides through the
/// total `ExactReal::cmp_exact` — equal values built through different
/// histories (√8 vs √2+√2) decide `Equal` exactly.
pub(crate) fn scalar_pair_eq(a_val: &Value, b_val: &Value) -> bool {
    if let Some((a, b)) = rational_pair(a_val, b_val) {
        return a == b;
    }
    match (
        extract_exact_real_for_comparison(a_val),
        extract_exact_real_for_comparison(b_val),
    ) {
        (Ok(a), Ok(b)) => a.cmp_exact(&b) == Some(std::cmp::Ordering::Equal),
        // Only Scalar/ExactScalar operands route here, so extraction
        // does not fail in practice; treat any failure as unequal.
        _ => false,
    }
}
