use crate::error::{AjisaiError, Result};
use crate::interpreter::tensor_ops::FlatTensor;
use crate::interpreter::value_extraction_helpers::{
    extract_count_from_value, extract_integer_from_value, nil_passthrough_binary,
    nil_passthrough_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::exact::{ExactCmp, ExactReal, Water};
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

    fn surface(self) -> &'static str {
        match self {
            OrderingKind::Lt => "<",
            OrderingKind::Le => "<=",
            OrderingKind::Gt => ">",
            OrderingKind::Ge => ">=",
        }
    }
}

/// Result of a three-valued scalar comparison (SPEC §7.4.1): a decided
/// boolean, or the logical `Unknown` (U) carrying the refinement-step
/// diagnosis surfaced as `diagnosis.agreedPrefix`. Tier ≤ 1 operands —
/// everything the current vocabulary can construct — always decide, so
/// the `Unknown` arm is reserved for Tier 2 observations (and, as a
/// defensive fallback, an absent operand that slipped past the NIL
/// passthrough).
enum ScalarCmp {
    Decided(bool),
}

fn push_boolean_result(interp: &mut Interpreter, result: bool) {
    interp.stack.push(Value::from_bool(result));
    let stack_len = interp.stack.len();
    interp
        .stack
        .set_role_at(stack_len - 1, Interpretation::TruthValue);
}

/// Compare two scalar values under an ordering kind. Returns `Err(_)` for
/// structurally-non-comparable operands. Comparison over the exact domain is
/// total (LANG.VALUES.EXACT): both-rational operands take the Fraction fast
/// path, and any pair involving an algebraic decides through the total
/// `ExactReal::cmp_exact`. There is no undecided outcome.
fn compare_scalar_pair(a_val: &Value, b_val: &Value, kind: OrderingKind) -> Result<ScalarCmp> {
    let a = extract_exact_real_for_comparison(a_val)?;
    let b = extract_exact_real_for_comparison(b_val)?;
    Ok(match (a.as_rational(), b.as_rational()) {
        (Some(af), Some(bf)) => ScalarCmp::Decided(kind.apply_to_fraction(af, bf)),
        _ => match a.cmp_exact(&b) {
            ExactCmp::Decided(o) => ScalarCmp::Decided(kind.apply_ordering(o)),
            ExactCmp::Starved { .. } | ExactCmp::Absent => {
                return Err(AjisaiError::from(
                    "comparison: operand is outside the exact domain",
                ))
            }
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
        ValueData::Vector(_)  => {
            let tensor = FlatTensor::from_value(val)?;
            if tensor.data.len() != 1 {
                return Err(AjisaiError::create_structure_error(
                    "scalar value",
                    "non-scalar value",
                ));
            }
            Ok(tensor.data[0].clone())
        }
        ValueData::Tensor { data, .. } => {
            if data.len() != 1 {
                return Err(AjisaiError::create_structure_error(
                    "scalar value",
                    "non-scalar value",
                ));
            }
            data.get_small_fraction(0).ok_or_else(|| {
                AjisaiError::create_structure_error("scalar value", "non-scalar value")
            })
        }
        ValueData::Nil  => Err(AjisaiError::create_structure_error(
            "scalar value",
            "non-scalar value",
        )),
        ValueData::Boolean(_)
        | ValueData::CodeBlock(_)
         => Err(AjisaiError::create_structure_error(
            "scalar value",
            "non-scalar value",
        )),
    }
}

enum ScalarFastWrap {
    Scalar,
    Tensor(Vec<usize>),
}

struct ScalarFastOperand {
    fraction: Fraction,
    wrap: ScalarFastWrap,
}

fn scalar_fast_operand(value: &Value) -> Option<ScalarFastOperand> {
    match &value.data {
        ValueData::Scalar(f) => Some(ScalarFastOperand {
            fraction: f.clone(),
            wrap: ScalarFastWrap::Scalar,
        }),
        ValueData::Tensor { data, shape } if data.len() == 1 => Some(ScalarFastOperand {
            fraction: data.get_small_fraction(0)?,
            wrap: ScalarFastWrap::Tensor((**shape).clone()),
        }),
        ValueData::Vector(children)
            if value.hint != Interpretation::Text && children.len() == 1 =>
        {
            let child = scalar_fast_operand(&children[0])?;
            let mut shape = Vec::with_capacity(2);
            shape.push(1);
            match child.wrap {
                ScalarFastWrap::Scalar => {}
                ScalarFastWrap::Tensor(child_shape) => shape.extend(child_shape),
            }
            Some(ScalarFastOperand {
                fraction: child.fraction,
                wrap: ScalarFastWrap::Tensor(shape),
            })
        }
        _ => None,
    }
}

fn same_scalar_fast_wrap(a: &ScalarFastWrap, b: &ScalarFastWrap) -> bool {
    match (a, b) {
        (ScalarFastWrap::Scalar, ScalarFastWrap::Scalar) => true,
        (ScalarFastWrap::Tensor(a_shape), ScalarFastWrap::Tensor(b_shape)) => a_shape == b_shape,
        _ => false,
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
    if !same_scalar_fast_wrap(&a.wrap, &b.wrap) {
        return false;
    }

    let decided = kind.apply_to_fraction(&a.fraction, &b.fraction);
    if interp.consumption_mode == ConsumptionMode::Consume {
        interp.stack.pop();
        interp.stack.pop();
    }
    push_boolean_result(interp, decided);
    interp.runtime_metrics.scalar_fastpath_count =
        interp.runtime_metrics.scalar_fastpath_count.saturating_add(1);
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
    if !same_scalar_fast_wrap(&a.wrap, &b.wrap) {
        return false;
    }

    let eq = a.fraction == b.fraction;
    if interp.consumption_mode == ConsumptionMode::Consume {
        interp.stack.pop();
        interp.stack.pop();
    }
    push_boolean_result(interp, if invert { !eq } else { eq });
    interp.runtime_metrics.scalar_fastpath_count =
        interp.runtime_metrics.scalar_fastpath_count.saturating_add(1);
    true
}

/// Check whether every adjacent pair in `items` satisfies `kind`.
/// Returns `Ok(Some(bool))` when the property is decidable for every
/// pair, `Ok(None)` when some pair triggers SPEC §7.4.1's comparison
/// budget short-circuit. SPEC §7.4 requires the entire STAK-mode
/// result to be the logical `Unknown` (U) on the first U-producing
/// pair regardless of later pairs.
fn check_all_adjacent_pairs(items: &[Value], kind: OrderingKind) -> Result<ScalarCmp> {
    for pair in items.windows(2) {
        match compare_scalar_pair(&pair[0], &pair[1], kind)? {
            ScalarCmp::Decided(true) => continue,
            ScalarCmp::Decided(false) => return Ok(ScalarCmp::Decided(false)),
        }
    }
    Ok(ScalarCmp::Decided(true))
}

/// Same three-valued discipline as `check_all_adjacent_pairs` for
/// the EQ relation: `Some(true)` iff every adjacent pair decides
/// equal, `Some(false)` on the first decidedly-unequal pair, `None`
/// on the first §7.4.1 budget-exhausted pair (short-circuit per
/// SPEC §7.4 STAK-mode short-circuit rule). `invert` flips the
/// per-pair predicate to drive `NEQ`'s "all adjacent pairs unequal"
/// semantics.
fn check_all_adjacent_eq(items: &[Value], invert: bool) -> ScalarCmp {
    for pair in items.windows(2) {
        match pairwise_eq(&pair[0], &pair[1]) {
            ScalarCmp::Decided(eq) => {
                let pair_ok = if invert { !eq } else { eq };
                if !pair_ok {
                    return ScalarCmp::Decided(false);
                }
            }
        }
    }
    ScalarCmp::Decided(true)
}

fn apply_binary_comparison(
    interp: &mut Interpreter,
    kind: OrderingKind,
    _op_name: &str,
) -> Result<()> {
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

    match compare_scalar_pair(&a_val, &b_val, kind) {
        Ok(ScalarCmp::Decided(b)) => push_boolean_result(interp, b),
        Err(e) => {
            if !is_keep_mode {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
            }
            return Err(e);
        }
    }
    Ok(())
}

/// Shape-IC entry points (see `shape_ic.rs`): attempt exactly the D1 scalar
/// fast path for one comparison word. Same equivalence argument as the
/// arithmetic wrappers — the fast path only accepts operands the preceding
/// NIL-passthrough check would have ignored.
pub(crate) fn scalar_fastpath_lt(interp: &mut Interpreter) -> bool {
    push_ordering_scalar_fastpath(interp, OrderingKind::Lt)
}

pub(crate) fn scalar_fastpath_le(interp: &mut Interpreter) -> bool {
    push_ordering_scalar_fastpath(interp, OrderingKind::Le)
}

pub(crate) fn scalar_fastpath_gt(interp: &mut Interpreter) -> bool {
    push_ordering_scalar_fastpath(interp, OrderingKind::Gt)
}

pub(crate) fn scalar_fastpath_ge(interp: &mut Interpreter) -> bool {
    push_ordering_scalar_fastpath(interp, OrderingKind::Ge)
}

pub(crate) fn scalar_fastpath_eq(interp: &mut Interpreter) -> bool {
    push_equality_scalar_fastpath(interp, false)
}

pub(crate) fn scalar_fastpath_neq(interp: &mut Interpreter) -> bool {
    push_equality_scalar_fastpath(interp, true)
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
    apply_binary_comparison(interp, kind, kind.surface())
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
/// and the singleton-projection paths are total, as is scalar comparison.
fn pairwise_eq(a_val: &Value, b_val: &Value) -> ScalarCmp {
    if a_val.data == b_val.data {
        return ScalarCmp::Decided(true);
    }
    match (&a_val.data, &b_val.data) {
        (ValueData::Scalar(_), ValueData::Scalar(_))
        | (ValueData::ExactScalar(_), ValueData::ExactScalar(_))
        | (ValueData::ExactScalar(_), ValueData::Scalar(_))
        | (ValueData::Scalar(_), ValueData::ExactScalar(_)) => scalar_pair_eq(a_val, b_val),
        (ValueData::Scalar(_), ValueData::Vector(children)) if children.len() == 1 => {
            ScalarCmp::Decided(a_val.data == children[0].data)
        }
        (ValueData::Vector(children), ValueData::Scalar(_)) if children.len() == 1 => {
            ScalarCmp::Decided(children[0].data == b_val.data)
        }
        (ValueData::Scalar(_), ValueData::Tensor { .. }) if b_val.len() == 1 => ScalarCmp::Decided(
            b_val
                .child(0)
                .map(|c| a_val.data == c.data)
                .unwrap_or(false),
        ),
        (ValueData::Tensor { .. }, ValueData::Scalar(_)) if a_val.len() == 1 => ScalarCmp::Decided(
            a_val
                .child(0)
                .map(|c| c.data == b_val.data)
                .unwrap_or(false),
        ),
        _ => ScalarCmp::Decided(false),
    }
}

/// Scalar–scalar equality (SPEC §7.4.1). Both-Rational operands decide
/// via `Fraction` `PartialEq` — value equality on canonical reduced
/// rationals. Anything mixing in a Tier 1 algebraic decides through the
/// total `ExactReal::cmp_exact` — equal values built through different
/// histories (√8 vs √2+√2) decide `Equal` exactly.
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
            ExactCmp::Starved { .. } | ExactCmp::Absent => ScalarCmp::Decided(false),
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
    }
    Ok(())
}

