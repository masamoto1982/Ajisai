use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::nil_passthrough_binary;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::exact::{ExactCmp, ExactReal};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};

/// One of the four ordering comparisons. Carries the dispatch
/// decision through the SCALAR-comparison helper so the helper can
/// keep the Fraction fast path for both-Rational operands while
/// routing any non-Rational ExactReal pair through the total Tier 1
/// comparison `ExactReal::cmp_exact` (SPEC §7.4.1).
#[derive(Debug, Clone, Copy)]
enum OrderingKind {
    Lt,
    Le,
    Gt,
    Ge,
}

impl OrderingKind {
    fn apply_to_fraction(self, a: &Fraction, b: &Fraction) -> bool {
        match self {
            OrderingKind::Lt => a.lt(b),
            OrderingKind::Le => a.le(b),
            OrderingKind::Gt => a.gt(b),
            OrderingKind::Ge => a.ge(b),
        }
    }

    /// Apply the relation to a decided `ExactReal` three-way ordering.
    fn apply_ordering(self, o: std::cmp::Ordering) -> bool {
        use std::cmp::Ordering;
        match self {
            OrderingKind::Lt => o == Ordering::Less,
            OrderingKind::Le => o != Ordering::Greater,
            OrderingKind::Gt => o == Ordering::Greater,
            OrderingKind::Ge => o != Ordering::Less,
        }
    }
}

/// Result of a three-valued scalar comparison (LANG.VALUES.TRUTH): a decided
/// boolean, or the logical `Unknown` (U). Tier ≤ 1 operands (rational,
/// algebraic) always decide; `Undecided` is reached only when a Tier 2
/// operand's (`PI`'s) comparison-refinement budget exhausts without
/// separating the pair — and, as a defensive fallback, an absent operand
/// that slipped past the NIL passthrough.
enum ScalarCmp {
    Decided(bool),
    Undecided,
}

fn push_boolean_result(interp: &mut Interpreter, result: bool) {
    interp.stack.push(Value::from_bool(result));
    let stack_len = interp.stack.len();
    interp
        .stack
        .set_role_at(stack_len - 1, Interpretation::TruthValue);
}

/// The logical Unknown (U): a NIL read in truth position (LANG.VALUES.TRUTH),
/// carrying the reason a comparison could not decide. Mirrors
/// `interpreter::logic::as_unknown` — U's `hint` is `TruthValue` directly
/// (not just the stack role) so `Value::truth_value()` reports `"unknown"`
/// from the value alone, and `NIL?`/`NIL-REASON`/`VENT` still see the
/// absence it is (SPEC: being read in truth position adds an observation, it
/// takes none away).
fn undecidable_truth_value() -> Value {
    let mut v = Value::bubble_with_reason(NilReason::Undecidable, Recoverability::Retryable);
    v.hint = Interpretation::TruthValue;
    v
}

fn push_undecidable_result(interp: &mut Interpreter) {
    interp.stack.push(undecidable_truth_value());
    let stack_len = interp.stack.len();
    interp
        .stack
        .set_role_at(stack_len - 1, Interpretation::TruthValue);
}

/// Compare two scalar values under an ordering kind. Returns `Err(_)` for
/// structurally-non-comparable operands. Both-rational operands take the
/// Fraction fast path; an algebraic pair decides through the total
/// `ExactReal::cmp_exact`. A pair involving a Tier 2 operand (`PI`) may
/// exhaust its refinement budget without separating — `Undecided`, not an
/// error: the operands are perfectly well-formed, the comparison just could
/// not decide within its water.
fn compare_scalar_pair(a_val: &Value, b_val: &Value, kind: OrderingKind) -> Result<ScalarCmp> {
    let a = extract_exact_real_for_comparison(a_val)?;
    let b = extract_exact_real_for_comparison(b_val)?;
    Ok(match (a.as_rational(), b.as_rational()) {
        (Some(af), Some(bf)) => ScalarCmp::Decided(kind.apply_to_fraction(af, bf)),
        _ => match a.cmp_exact(&b) {
            ExactCmp::Decided(o) => ScalarCmp::Decided(kind.apply_ordering(o)),
            ExactCmp::Starved { .. } | ExactCmp::Absent => ScalarCmp::Undecided,
        },
    })
}

/// Outcome of a three-way order comparison shared by the
/// comparison-dependent words of SPEC §7.4.3 (`MIN`, `MAX`, `SORT`).
/// `Decided` carries the exact `a` vs `b` ordering. `Undecided` is
/// reserved for Tier 2 observations that cannot separate within their
/// water (and the defensive absent-operand fallback); it carries the
/// refinement-step diagnosis the caller projects to the logical
/// `Unknown` (U) with `diagnosis.agreedPrefix`. Tier ≤ 1 operands never
/// produce it.
pub(crate) enum OrderOutcome {
    Decided(std::cmp::Ordering),
    Undecided(usize),
}

/// Three-way order of two scalar values (SPEC §7.4.1). Returns `Err(_)`
/// for structurally non-comparable operands (the malformed-use path).
/// Both-`Rational` operands take the exact `Fraction` fast path; any pair
/// involving a Tier 1 algebraic decides through the total, budget-free
/// `ExactReal::cmp_exact`.
pub(crate) fn three_way_compare(a_val: &Value, b_val: &Value) -> Result<OrderOutcome> {
    let a = extract_exact_real_for_comparison(a_val)?;
    let b = extract_exact_real_for_comparison(b_val)?;
    Ok(match (a.as_rational(), b.as_rational()) {
        (Some(af), Some(bf)) => OrderOutcome::Decided(af.cmp(bf)),
        _ => match a.cmp_exact(&b) {
            ExactCmp::Decided(o) => OrderOutcome::Decided(o),
            ExactCmp::Starved { steps } => OrderOutcome::Undecided(steps),
            ExactCmp::Absent => OrderOutcome::Undecided(0),
        },
    })
}

/// Extract an `ExactReal` view of a value's scalar content for
/// comparison. Scalar (`Fraction`-backed) values lift to
/// `ExactReal::Rational`; singleton Vector / Tensor values also
/// project to their sole scalar. Non-scalar shapes and non-numeric
/// kinds error. When a future migration replaces
/// `ValueData::Scalar(Fraction)` with an `ExactReal`-backed
/// representation, this helper is the single point that needs to
/// surface the new variant, and `compare_scalar_pair` / `pairwise_eq`
/// will route it through the budgeted CF path automatically.
pub(crate) fn extract_exact_real_for_comparison(val: &Value) -> Result<ExactReal> {
    if let ValueData::ExactScalar(er) = &val.data {
        return Ok(er.clone());
    }
    let f = extract_scalar_for_comparison(val)?;
    Ok(ExactReal::from_fraction(f))
}

fn extract_scalar_for_comparison(val: &Value) -> Result<Fraction> {
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
        ValueData::Vector(_) | ValueData::Tensor { .. } => Err(
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

fn push_ordering_scalar_fastpath(interp: &mut Interpreter, kind: OrderingKind) -> bool {
    if !interp.scalar_fastpath_enabled || interp.stack.len() < 2 {
        return false;
    }

    let stack_len = interp.stack.len();
    let Some(a) = scalar_fast_operand(&interp.stack[stack_len - 2]) else {
        return false;
    };
    let Some(b) = scalar_fast_operand(&interp.stack[stack_len - 1]) else {
        return false;
    };
    let decided = kind.apply_to_fraction(&a.fraction, &b.fraction);
    if interp.consumption_mode == ConsumptionMode::Consume {
        interp.stack.pop();
        interp.stack.pop();
    }
    push_boolean_result(interp, decided);
    interp.runtime_metrics.scalar_fastpath_count = interp
        .runtime_metrics
        .scalar_fastpath_count
        .saturating_add(1);
    true
}

fn push_equality_scalar_fastpath(interp: &mut Interpreter, invert: bool) -> bool {
    if !interp.scalar_fastpath_enabled || interp.stack.len() < 2 {
        return false;
    }

    let stack_len = interp.stack.len();
    let Some(a) = scalar_fast_operand(&interp.stack[stack_len - 2]) else {
        return false;
    };
    let Some(b) = scalar_fast_operand(&interp.stack[stack_len - 1]) else {
        return false;
    };
    let eq = a.fraction == b.fraction;
    if interp.consumption_mode == ConsumptionMode::Consume {
        interp.stack.pop();
        interp.stack.pop();
    }
    push_boolean_result(interp, if invert { !eq } else { eq });
    interp.runtime_metrics.scalar_fastpath_count = interp
        .runtime_metrics
        .scalar_fastpath_count
        .saturating_add(1);
    true
}

/// Apply an ordering Word across the shapes LANG.COLLECTIONS.LIFT allows.
///
/// "An arithmetic or comparison Word applies element-wise when given vectors.
/// Two vectors combine element-wise when their lengths are equal; a scalar
/// combines with every element of a vector. Any other pairing is ERROR."
///
/// The comparison family used to do the reverse of this: it projected a
/// singleton Vector to its element (`[ 3 ] 4 LT` was `TRUE`) and refused the
/// element-wise application the clause requires (`[ 3 4 ] 4 LT` was an ERROR).
///
/// A NIL operand lane answers NIL, which is the scalar law's own outcome for
/// `NIL 3 LT` — the clause says each lane preserves the scalar law's NIL
/// distinction.
fn lift_comparison(a_val: &Value, b_val: &Value, kind: OrderingKind) -> Result<Value> {
    let a_items = a_val.as_vector_view();
    let b_items = b_val.as_vector_view();

    match (a_items, b_items) {
        (None, None) => {
            if a_val.is_nil() || b_val.is_nil() {
                return Ok(Value::nil_with_reason(
                    a_val
                        .nil_reason()
                        .or_else(|| b_val.nil_reason())
                        .copied()
                        .unwrap_or(NilReason::Literal),
                ));
            }
            match compare_scalar_pair(a_val, b_val, kind)? {
                ScalarCmp::Decided(b) => Ok(Value::from_bool(b)),
                ScalarCmp::Undecided => Ok(undecidable_truth_value()),
            }
        }
        (Some(items), None) => {
            let lanes = items
                .iter()
                .map(|item| lift_comparison(item, b_val, kind))
                .collect::<Result<Vec<_>>>()?;
            Ok(Value::from_vector(lanes))
        }
        (None, Some(items)) => {
            let lanes = items
                .iter()
                .map(|item| lift_comparison(a_val, item, kind))
                .collect::<Result<Vec<_>>>()?;
            Ok(Value::from_vector(lanes))
        }
        (Some(a_items), Some(b_items)) => {
            if a_items.len() != b_items.len() {
                return Err(AjisaiError::create_structure_error(
                    "vectors of equal length",
                    "vectors of differing length",
                ));
            }
            let lanes = a_items
                .iter()
                .zip(b_items.iter())
                .map(|(x, y)| lift_comparison(x, y, kind))
                .collect::<Result<Vec<_>>>()?;
            Ok(Value::from_vector(lanes))
        }
    }
}

fn apply_binary_comparison(interp: &mut Interpreter, kind: OrderingKind) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let (a_val, b_val) = if is_keep_mode {
        let stack_len = interp.stack.len();
        let a_val = interp.stack[stack_len - 2].clone();
        let b_val = interp.stack[stack_len - 1].clone();
        (a_val, b_val)
    } else {
        let b_val = interp.stack.pop().unwrap();
        let a_val = interp.stack.pop().unwrap();
        (a_val, b_val)
    };

    match lift_comparison(&a_val, &b_val, kind) {
        Ok(result) => {
            interp.stack.push(result);
            Ok(())
        }
        Err(e) => {
            if !is_keep_mode {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
            }
            Err(e)
        }
    }
}

fn apply_ordering_schema(interp: &mut Interpreter, kind: OrderingKind) -> Result<()> {
    if nil_passthrough_binary(interp) {
        return Ok(());
    }
    {
        if push_ordering_scalar_fastpath(interp, kind) {
            return Ok(());
        }
    }
    apply_binary_comparison(interp, kind)
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    apply_ordering_schema(interp, OrderingKind::Lt)
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    apply_ordering_schema(interp, OrderingKind::Le)
}

pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    apply_ordering_schema(interp, OrderingKind::Gt)
}

pub fn op_gte(interp: &mut Interpreter) -> Result<()> {
    apply_ordering_schema(interp, OrderingKind::Ge)
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    apply_equality(interp, false)
}

pub fn op_neq(interp: &mut Interpreter) -> Result<()> {
    apply_equality(interp, true)
}

/// Pairwise equality. Every pair decides: the structural Vector / Tensor paths
/// are total, as is scalar comparison.
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
fn pairwise_eq(a_val: &Value, b_val: &Value) -> ScalarCmp {
    if a_val.is_nil() || b_val.is_nil() {
        return ScalarCmp::Decided(
            a_val.is_nil() && b_val.is_nil() && a_val.nil_reason() == b_val.nil_reason(),
        );
    }
    // A Tier 2 operand makes `ValueData` equality answer from allocation
    // identity, so it may not settle anything (`Value::carries_computable`).
    let tier2 = a_val.carries_computable() || b_val.carries_computable();
    if !tier2 && a_val.data == b_val.data {
        return ScalarCmp::Decided(true);
    }
    match (&a_val.data, &b_val.data) {
        (ValueData::Scalar(_), ValueData::Scalar(_))
        | (ValueData::ExactScalar(_), ValueData::ExactScalar(_))
        | (ValueData::ExactScalar(_), ValueData::Scalar(_))
        | (ValueData::Scalar(_), ValueData::ExactScalar(_)) => scalar_pair_eq(a_val, b_val),
        (ValueData::Vector(x), ValueData::Vector(y)) if tier2 => vector_pair_eq(x, y),
        // Disjoint domains are unequal whatever they carry
        // (LANG.VALUES.DISJOINT), so Tier 2 does not make them undecidable.
        _ => ScalarCmp::Decided(false),
    }
}

/// Element-wise equality of two Tier 2-carrying Vectors, combined as the
/// Kleene conjunction the truth domain already uses: one unequal element (or
/// a length difference) settles FALSE, and only an otherwise-equal pair with
/// an undecided element is UNKNOWN.
fn vector_pair_eq(x: &[Value], y: &[Value]) -> ScalarCmp {
    if x.len() != y.len() {
        return ScalarCmp::Decided(false);
    }
    let mut answer = ScalarCmp::Decided(true);
    for (p, q) in x.iter().zip(y.iter()) {
        match pairwise_eq(p, q) {
            ScalarCmp::Decided(false) => return ScalarCmp::Decided(false),
            ScalarCmp::Decided(true) => {}
            ScalarCmp::Undecided => answer = ScalarCmp::Undecided,
        }
    }
    answer
}

/// Scalar–scalar equality (LANG.VALUES.EXACT). Both-Rational operands decide
/// via `Fraction` `PartialEq` — value equality on canonical reduced
/// rationals. Anything mixing in a Tier 1 algebraic decides through the
/// total `ExactReal::cmp_exact` — equal values built through different
/// histories (√8 vs √2+√2) decide `Equal` exactly. A pair involving a Tier 2
/// operand may starve: equality of two computable reals is fundamentally
/// undecidable (`types::exact::computable`), so `Undecided` here is never a
/// false `FALSE` the way it briefly was — it is the honest answer.
fn scalar_pair_eq(a_val: &Value, b_val: &Value) -> ScalarCmp {
    let (a, b) = match (
        extract_exact_real_for_comparison(a_val),
        extract_exact_real_for_comparison(b_val),
    ) {
        (Ok(a), Ok(b)) => (a, b),
        // Only Scalar/ExactScalar operands route here, so extraction
        // does not fail in practice; treat any failure as unequal.
        _ => return ScalarCmp::Decided(false),
    };
    match (a.as_rational(), b.as_rational()) {
        (Some(af), Some(bf)) => ScalarCmp::Decided(af == bf),
        _ => match a.cmp_exact(&b) {
            ExactCmp::Decided(o) => ScalarCmp::Decided(o == std::cmp::Ordering::Equal),
            ExactCmp::Starved { .. } | ExactCmp::Absent => ScalarCmp::Undecided,
        },
    }
}

fn apply_equality(interp: &mut Interpreter, invert: bool) -> Result<()> {
    if nil_passthrough_binary(interp) {
        return Ok(());
    }

    if push_equality_scalar_fastpath(interp, invert) {
        return Ok(());
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let (a_val, b_val) = if is_keep_mode {
        let stack_len = interp.stack.len();
        let a_val = interp.stack[stack_len - 2].clone();
        let b_val = interp.stack[stack_len - 1].clone();
        (a_val, b_val)
    } else {
        let b_val = interp.stack.pop().unwrap();
        let a_val = interp.stack.pop().unwrap();
        (a_val, b_val)
    };

    match pairwise_eq(&a_val, &b_val) {
        ScalarCmp::Decided(eq) => push_boolean_result(interp, if invert { !eq } else { eq }),
        ScalarCmp::Undecided => push_undecidable_result(interp),
    }
    Ok(())
}
