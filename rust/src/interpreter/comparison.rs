use crate::error::{AjisaiError, Result};
use crate::interpreter::lane_lift::lift_lanes;
use crate::interpreter::value_extraction_helpers::nil_passthrough_binary;
use crate::interpreter::Interpreter;
use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

fn push_boolean_result(interp: &mut Interpreter, result: bool) {
    interp.stack.push(Value::from_bool(result));
}

struct ScalarFastOperand {
    fraction: Fraction,
}

fn scalar_fast_operand(value: &Value) -> Option<ScalarFastOperand> {
    match &value.data {
        ValueData::Scalar(f) => Some(ScalarFastOperand {
            fraction: f.clone(),
        }),
        _ => None,
    }
}

/// The top two stack operands, if the fast path applies to both: enabled,
/// two-deep, and both plain Scalars. Shared by the ordering and equality
/// fast paths so their eligibility check has one definition.
fn scalar_fastpath_pair(interp: &Interpreter) -> Option<(ScalarFastOperand, ScalarFastOperand)> {
    if !interp.scalar_fastpath_enabled || interp.stack.len() < 2 {
        return None;
    }
    let stack_len = interp.stack.len();
    let a = scalar_fast_operand(&interp.stack[stack_len - 2])?;
    let b = scalar_fast_operand(&interp.stack[stack_len - 1])?;
    Some((a, b))
}

fn record_fastpath_hit(interp: &mut Interpreter) {
    let count = &mut interp.runtime_metrics.scalar_fastpath_count;
    *count = count.saturating_add(1);
}

fn push_ordering_scalar_fastpath(interp: &mut Interpreter, kind: OrderingKind) -> bool {
    let Some((a, b)) = scalar_fastpath_pair(interp) else {
        return false;
    };
    let decided = kind.apply_to_fraction(&a.fraction, &b.fraction);
    interp.stack.pop();
    interp.stack.pop();
    push_boolean_result(interp, decided);
    record_fastpath_hit(interp);
    true
}

fn push_equality_scalar_fastpath(interp: &mut Interpreter, invert: bool) -> bool {
    let Some((a, b)) = scalar_fastpath_pair(interp) else {
        return false;
    };
    let eq = a.fraction == b.fraction;
    interp.stack.pop();
    interp.stack.pop();
    push_boolean_result(interp, if invert { !eq } else { eq });
    record_fastpath_hit(interp);
    true
}

/// Apply an ordering Word across the shapes LANG.COLLECTIONS.LIFT allows.
///
/// The alignment is `lane_lift`'s, shared with `booleanLogic`, so a
/// comparison and the `AND` that combines two of its results agree on what
/// pairs: equal lengths pair lane by lane, and a one-lane operand is reused
/// across the other's length. That last case is what makes a mask writable
/// against a bare threshold — `[ 1 2 3 ] [ 2 ] GT` — and it was the one shape
/// this family refused while `[ 1 2 3 ] [ 2 ] MUL` had always accepted it.
///
/// The comparison family used to do the reverse of this: it projected a
/// singleton Vector to its element (`[ 3 ] 4 LT` was `TRUE`) and refused the
/// element-wise application the clause requires (`[ 3 4 ] 4 LT` was an ERROR).
///
/// A NIL operand lane answers NIL, which is the scalar law's own outcome for
/// `NIL 3 LT` — the clause says each lane preserves the scalar law's NIL
/// distinction.
fn lift_comparison(a_val: &Value, b_val: &Value, kind: OrderingKind) -> Result<Value> {
    lift_lanes([a_val, b_val], &|[a, b]| compare_lane(a, b, kind))
}

/// One lane of an ordering comparison: both operands are past the alignment,
/// so neither is a Vector here.
fn compare_lane(a_val: &Value, b_val: &Value, kind: OrderingKind) -> Result<Value> {
    // The scalar passthrough law, per lane (LANG.FAILURE.PASSTHROUGH): the
    // leftmost absent operand *is* the result, carried whole. Rebuilding a
    // NIL from the reason alone dropped the rest of the absence — the text a
    // `userDeclared` reason carries, which is part of that reason — and
    // minted a fresh absence for one the lane only received.
    if a_val.is_nil() {
        return Ok(Value::nil_inheriting_absence_from(a_val));
    }
    if b_val.is_nil() {
        return Ok(Value::nil_inheriting_absence_from(b_val));
    }
    // `nonNumeric`: LT/GT, the only callers of
    // `compare_lane`, declare it uniformly. EQ never reaches here —
    // `pairwise_eq` is total and raises nothing.
    compare_scalar_pair(a_val, b_val, kind)
        .map_err(|e| {
            AjisaiError::declared("nonNumeric", format!("expected two Scalars, got {}", e.got))
        })
        .map(Value::from_bool)
}

fn apply_binary_comparison(interp: &mut Interpreter, kind: OrderingKind) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::stack_underflow());
    }

    let b_val = interp.stack.pop().unwrap();
    let a_val = interp.stack.pop().unwrap();

    match lift_comparison(&a_val, &b_val, kind) {
        Ok(result) => {
            interp.stack.push(result);
            Ok(())
        }
        Err(e) => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            Err(e)
        }
    }
}

fn apply_ordering_schema(interp: &mut Interpreter, kind: OrderingKind) -> Result<()> {
    if nil_passthrough_binary(interp) {
        return Ok(());
    }
    if push_ordering_scalar_fastpath(interp, kind) {
        return Ok(());
    }
    apply_binary_comparison(interp, kind)
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    apply_ordering_schema(interp, OrderingKind::Lt)
}

pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    apply_ordering_schema(interp, OrderingKind::Gt)
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    apply_equality(interp, false)
}

/// Pairwise equality. Every pair decides: the structural Vector / Tensor paths
/// are total, as is scalar comparison over the field.
///
/// Equality is *structural over disjoint domains* (LANG.VALUES.DISJOINT): two
/// values are "never equal merely because their encodings resemble one
/// another". Two consequences are load-bearing here.
///
/// A singleton Vector is not its element. `[ 3 ] 3 EQ` used to decide TRUE via
/// a projection path, which made the Vector and Scalar domains overlap for
/// this one Word while `EQ`'s own contract promises a decision over tagged
/// values. It now decides FALSE, like every other cross-domain pair.
///
/// Two NILs are the same value exactly when their reasons agree
/// (LANG.VALUES.NIL: "the reason is the entire observable content of a NIL").
/// The `ValueData` comparison below cannot see a reason — `ValueData::Nil`
/// carries none — so NIL pairs are decided before it, on the reason itself.
fn pairwise_eq(a_val: &Value, b_val: &Value) -> bool {
    if a_val.is_nil() || b_val.is_nil() {
        return a_val.is_nil() && b_val.is_nil() && a_val.nil_reason() == b_val.nil_reason();
    }
    if a_val.data == b_val.data {
        return true;
    }
    match (&a_val.data, &b_val.data) {
        (ValueData::Scalar(_), ValueData::Scalar(_))
        | (ValueData::ExactScalar(_), ValueData::ExactScalar(_))
        | (ValueData::ExactScalar(_), ValueData::Scalar(_))
        | (ValueData::Scalar(_), ValueData::ExactScalar(_)) => scalar_pair_eq(a_val, b_val),
        // Disjoint domains are unequal whatever they carry
        // (LANG.VALUES.DISJOINT).
        _ => false,
    }
}

fn apply_equality(interp: &mut Interpreter, invert: bool) -> Result<()> {
    if nil_passthrough_binary(interp) {
        return Ok(());
    }

    if push_equality_scalar_fastpath(interp, invert) {
        return Ok(());
    }

    if interp.stack.len() < 2 {
        return Err(AjisaiError::stack_underflow());
    }

    let b_val = interp.stack.pop().unwrap();
    let a_val = interp.stack.pop().unwrap();

    let eq = pairwise_eq(&a_val, &b_val);
    push_boolean_result(interp, if invert { !eq } else { eq });
    Ok(())
}

// The scalar comparison law of LANG.VALUES.EXACT — how two numeric operands
// are ordered and tested for equality.
//
// Split out of `comparison.rs`, which keeps the comparison *Words*: the
// element-wise lifting, the stack fast paths, and the `LT`/`GT`/`EQ` entry
// points. What is here is the law those Words apply, one scalar pair at a
// time: a rational pair decides by `Fraction`, and any pair reaching the
// algebraic field decides exactly through `ExactReal::cmp_exact`. Every pair
// decides.
/// An operand the exact order is not defined on, named by its domain. Not an
/// `AjisaiError`: each comparing Word declares its own condition for this
/// (`nonNumeric` today, for every one of them), so the
/// caller names it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NotComparable {
    pub got: &'static str,
}

type ScalarResult<T> = std::result::Result<T, NotComparable>;

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
) -> ScalarResult<bool> {
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
pub(crate) fn three_way_compare(a_val: &Value, b_val: &Value) -> ScalarResult<std::cmp::Ordering> {
    if let Some((a, b)) = rational_pair(a_val, b_val) {
        return Ok(a.cmp(b));
    }
    let a = extract_exact_real_for_comparison(a_val)?;
    let b = extract_exact_real_for_comparison(b_val)?;
    a.cmp_exact(&b).ok_or(NotComparable { got: "NIL" })
}

/// Extract an `ExactReal` view of a value's scalar content for
/// comparison. Scalar (`Fraction`-backed) values lift to
/// `ExactReal::Rational`; singleton Vector / Tensor values also
/// project to their sole scalar. Non-scalar shapes and non-numeric
/// kinds error.
pub(crate) fn extract_exact_real_for_comparison(val: &Value) -> ScalarResult<ExactReal> {
    if let ValueData::ExactScalar(er) = &val.data {
        return Ok(er.clone());
    }
    let f = extract_scalar_for_comparison(val)?;
    Ok(ExactReal::from_fraction(f))
}

pub(crate) fn extract_scalar_for_comparison(val: &Value) -> ScalarResult<Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Ok(f.clone()),
        ValueData::ExactScalar(er) => {
            // Provide best rational approximation for contexts requiring a Fraction
            use num_bigint::BigInt;
            er.best_rational_approximation(&BigInt::from(1_000_000_000u64))
                .ok_or(NotComparable { got: "Scalar" })
        }
        // A Vector never reaches the scalar law: `lift_comparison` peels it
        // element-wise first. A one-element Vector used to project to its sole
        // element here, which made `[ 3 ] 4 LT` answer `TRUE` — a collapse
        // LANG.COLLECTIONS.LIFT forbids ("a scalar combines with every element
        // of a vector"), and one that contradicts a singleton Vector not being
        // its element (LANG.VALUES.DISJOINT).
        _ => Err(NotComparable {
            got: val.domain_name(),
        }),
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

#[cfg(test)]
mod tests {
    //! Two rational operands compare as the rationals they are.
    //!
    //! `three_way_compare`, `compare_scalar_pair` and `scalar_pair_eq` each ended in
    //! a `Fraction` comparison whenever both operands were rational — their
    //! `(Some, Some)` arm — but reached it by building an `ExactReal` from each
    //! operand first, and `extract_exact_real_for_comparison` clones the `Fraction`
    //! out of the `Value` to do that. Two clones and two constructions per
    //! comparison, to arrive at the two `Fraction`s the operands already held.
    //!
    //! `rational_pair` borrows them instead. That is only sound if the round trip it
    //! skips is the identity — `ExactReal::from_fraction(f).as_rational() == Some(f)`
    //! — and only *complete* if nothing but a plain rational takes the screen. Both
    //! are pinned here, the first directly and the second by leaving every other
    //! shape to answer through the route that can: an algebraic pair decides exactly
    //! (√8 vs √2+√2, LANG.VALUES.EXACT).

    use crate::test_support::answer;
    use crate::types::exact::ExactReal;
    use crate::types::fraction::Fraction;

    /// The round trip the screen skips is the identity, over a dense range of
    /// rationals including both signs, zero, integers, and the i64 extremes.
    #[test]
    fn a_rational_survives_the_exact_real_round_trip_it_no_longer_takes() {
        let mut fractions: Vec<Fraction> = Vec::new();
        for numerator in -40..=40 {
            for denominator in 1..=12 {
                fractions.push(Fraction::new(numerator.into(), denominator.into()));
            }
        }
        for n in [i64::MAX, i64::MIN + 1, i64::MIN, 0, 1, -1] {
            fractions.push(Fraction::from(n));
        }

        for fraction in &fractions {
            let exact = ExactReal::from_fraction(fraction.clone());
            let round_tripped = exact.as_rational();
            assert_eq!(
                round_tripped,
                Some(fraction),
                "{fraction} must survive from_fraction/as_rational unchanged"
            );
        }

        // And the order the screen reads is the order the general arm read.
        for a in fractions.iter().take(60) {
            for b in fractions.iter().take(60) {
                let screened = a.cmp(b);
                let (ea, eb) = (
                    ExactReal::from_fraction(a.clone()),
                    ExactReal::from_fraction(b.clone()),
                );
                let general = ea
                    .as_rational()
                    .expect("rational")
                    .cmp(eb.as_rational().expect("rational"));
                assert_eq!(screened, general, "comparing {a} with {b}");
            }
        }
    }

    /// Every ordering word, over the sign and magnitude cases a rational screen
    /// could get wrong: equal values, both signs, a negative denominator's worth
    /// of sign placement, and unequal denominators.
    #[tokio::test]
    async fn every_ordering_word_answers_the_same_for_rationals() {
        for (program, expected) in [
            ("2 3 LT", "TRUE"),
            ("3 2 LT", "FALSE"),
            ("2 2 LT", "FALSE"),
            ("2 2 GT NOT", "TRUE"),
            ("3 2 GT", "TRUE"),
            ("2 3 GT", "FALSE"),
            ("2 2 LT NOT", "TRUE"),
            ("2 2 EQ", "TRUE"),
            ("2 3 EQ", "FALSE"),
            ("-2 3 LT", "TRUE"),
            ("3 -2 LT", "FALSE"),
            ("-3 -2 LT", "TRUE"),
            ("-2 -3 LT", "FALSE"),
            ("0 0 EQ", "TRUE"),
            ("0 -0 EQ", "TRUE"),
            ("1/2 1/3 GT", "TRUE"),
            ("1/3 1/2 GT", "FALSE"),
            ("2/4 1/2 EQ", "TRUE"),
            ("-1/2 1/2 LT", "TRUE"),
        ] {
            assert_eq!(answer(program).await, expected, "`{program}`");
        }
    }

    /// An exact operand must not take the screen. √8 and √2+√2 are the same
    /// value by different histories, which only the exact route decides, and π
    /// is the case that may honestly not decide at all.
    #[tokio::test]
    async fn an_exact_operand_still_decides_through_the_exact_route() {
        assert_eq!(answer("8 SQRT 2 SQRT 2 SQRT ADD EQ").await, "TRUE");
        assert_eq!(answer("2 SQRT 1 GT").await, "TRUE");
        assert_eq!(answer("2 SQRT 2 LT").await, "TRUE");
        // A rational compared against an algebraic is still a mixed pair.
        assert_eq!(answer("1 2 SQRT LT").await, "TRUE");
    }
}
