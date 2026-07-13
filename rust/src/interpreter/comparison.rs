use crate::error::{AjisaiError, Result};
use crate::interpreter::interval_ops::value_to_interval;
use crate::interpreter::tensor_ops::FlatTensor;
use crate::interpreter::value_extraction_helpers::{
    extract_count_from_value, extract_integer_from_value, nil_passthrough_binary,
    nil_passthrough_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::continued_fraction::{CmpOutcome, ExactReal, DEFAULT_COMPARISON_BUDGET};
use crate::types::fraction::Fraction;
use crate::types::interval::Interval;
use crate::types::{Interpretation, Value, ValueData};

/// One of the four ordering comparisons. Carries the dispatch
/// decision through the SCALAR-comparison helper so the helper can
/// keep the Fraction fast path for both-Rational operands while
/// routing any non-Rational ExactReal pair through
/// `ExactReal::cmp_with_budget` (SPEC §7.4.1).
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

    fn interval_decision(self, a: &Interval, b: &Interval) -> Option<bool> {
        let (definitely_true, definitely_false) = match self {
            OrderingKind::Lt => (a.hi.lt(&b.lo), a.lo.ge(&b.hi)),
            OrderingKind::Le => (a.hi.le(&b.lo), a.lo.gt(&b.hi)),
            OrderingKind::Gt => (a.lo.gt(&b.hi), a.hi.le(&b.lo)),
            OrderingKind::Ge => (a.lo.ge(&b.hi), a.hi.lt(&b.lo)),
        };
        if definitely_true {
            Some(true)
        } else if definitely_false {
            Some(false)
        } else {
            None
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
/// boolean, or the logical `Unknown` (U) carrying the CF agreed-prefix
/// length — the number of leading partial quotients that matched before
/// the budget was exhausted, surfaced as `diagnosis.agreedPrefix`.
enum ScalarCmp {
    Decided(bool),
    Unknown(usize),
}

fn push_boolean_result(interp: &mut Interpreter, result: bool) {
    interp.stack.push(Value::from_bool(result));
    let stack_len = interp.stack.len();
    interp.semantic_registry.normalize_to_stack_len(stack_len);
    interp
        .semantic_registry
        .update_hint_at(stack_len - 1, Interpretation::TruthValue);
}

/// Push the SPEC §7.4.1 logical `Unknown` (U): a `TruthValue`-role value
/// produced when the partial-quotient budget is exhausted before two
/// continued fractions decide. U is a logical truth value, not an
/// operational NIL — it flows into the three-valued logic of SPEC §7.5
/// directly. The `TruthValue` interpretation role makes it observable as
/// `truthValue = unknown`.
fn push_unknown(interp: &mut Interpreter, agreed_prefix: Option<usize>) {
    let value = match agreed_prefix {
        Some(prefix) => Value::unknown_with_agreed_prefix(None, prefix),
        None => Value::unknown(),
    };
    interp.stack.push(value);
    let stack_len = interp.stack.len();
    interp.semantic_registry.normalize_to_stack_len(stack_len);
    interp
        .semantic_registry
        .update_hint_at(stack_len - 1, Interpretation::TruthValue);
}

/// Compare two scalar values under an ordering kind. Returns `Ok(Some(bool))`
/// when the comparison decides, `Ok(None)` when the comparison budget
/// exhausts (SPEC §7.4.1) — the caller projects `None` to the logical
/// `Unknown` (U). Returns `Err(_)` for structurally-non-comparable operands.
///
/// Both-Rational operands take the Fraction fast path (always
/// decidable per SPEC §7.4.1: "the budget value itself is not part
/// of observable semantics; it must be high enough that distinct
/// rationals always decide"). Any non-Rational ExactReal operand
/// routes through `ExactReal::cmp_with_budget` under the
/// `DEFAULT_COMPARISON_BUDGET`; budget exhaustion surfaces as
/// `Ok(None)` here.
fn compare_scalar_pair(a_val: &Value, b_val: &Value, kind: OrderingKind) -> Result<ScalarCmp> {
    let a = extract_exact_real_for_comparison(a_val)?;
    let b = extract_exact_real_for_comparison(b_val)?;
    Ok(match (a.as_rational(), b.as_rational()) {
        (Some(af), Some(bf)) => ScalarCmp::Decided(kind.apply_to_fraction(af, bf)),
        _ => match cmp_default_budget(&a, &b) {
            CmpOutcome::Decided(o) => ScalarCmp::Decided(kind.apply_ordering(o)),
            CmpOutcome::Undecided { agreed_prefix } => ScalarCmp::Unknown(agreed_prefix),
        },
    })
}

/// Outcome of a budgeted three-way order comparison shared by the
/// comparison-dependent words of SPEC §7.4.3 (`MIN`, `MAX`, `SORT`).
/// `Decided` carries the exact `a` vs `b` ordering. `Undecided` carries the
/// agreed-prefix length of the budget-exhausted continued-fraction
/// comparison; the caller projects it to the logical `Unknown` (U) with
/// `diagnosis.agreedPrefix`.
pub(crate) enum OrderOutcome {
    Decided(std::cmp::Ordering),
    Undecided(usize),
}

/// Three-way order of two scalar values under the SPEC §7.4.1 comparison
/// budget. Returns `Err(_)` for structurally non-comparable operands (the
/// malformed-use path). Both-`Rational` operands take the exact `Fraction`
/// fast path; any non-`Rational` `ExactReal` routes through the budgeted CF
/// comparison and may yield `Undecided`.
pub(crate) fn three_way_compare(a_val: &Value, b_val: &Value) -> Result<OrderOutcome> {
    let a = extract_exact_real_for_comparison(a_val)?;
    let b = extract_exact_real_for_comparison(b_val)?;
    Ok(match (a.as_rational(), b.as_rational()) {
        (Some(af), Some(bf)) => OrderOutcome::Decided(af.cmp(bf)),
        _ => match cmp_default_budget(&a, &b) {
            CmpOutcome::Decided(o) => OrderOutcome::Decided(o),
            CmpOutcome::Undecided { agreed_prefix } => OrderOutcome::Undecided(agreed_prefix),
        },
    })
}

/// Default-budget order of two scalar `ExactReal`s. A cheap interval
/// pre-filter decides well-separated values in O(1) without streaming
/// (SPEC §7.4.1 — a proven separation is the true order); when the
/// enclosures overlap it falls back to the budgeted NICF comparison under
/// `DEFAULT_COMPARISON_BUDGET`. Used only by the default-budget relations and
/// comparison-dependent words — never by `COMPARE-WITHIN`, whose `U` is
/// measured in NICF terms and must not be pre-empted by the filter.
fn cmp_default_budget(a: &ExactReal, b: &ExactReal) -> CmpOutcome {
    if let Some(order) = a.cmp_via_interval_filter(b) {
        return CmpOutcome::Decided(order);
    }
    a.cmp_with_budget_tracked(b, DEFAULT_COMPARISON_BUDGET)
}

/// Push the logical `Unknown` (U) carrying an agreed-prefix diagnosis, for
/// the comparison-dependent words of SPEC §7.4.3. Mirrors the relations'
/// own U production: a `TruthValue`-role value observed as
/// `truthValue = unknown` with `diagnosis.agreedPrefix`.
pub(crate) fn push_comparison_unknown(interp: &mut Interpreter, agreed_prefix: usize) {
    push_unknown(interp, Some(agreed_prefix));
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
fn extract_exact_real_for_comparison(val: &Value) -> Result<ExactReal> {
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
        ValueData::Vector(_) | ValueData::Record { .. } => {
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
        ValueData::Nil => Err(AjisaiError::create_structure_error(
            "scalar value",
            "non-scalar value",
        )),
        ValueData::Boolean(_)
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => Err(AjisaiError::create_structure_error(
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
    if !interp.scalar_fastpath_enabled
        || interp.operation_target_mode != OperationTargetMode::StackTop
        || interp.stack.len() < 2
    {
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
    interp.runtime_metrics.scalar_fastpath_count = interp
        .runtime_metrics
        .scalar_fastpath_count
        .saturating_add(1);
    true
}

fn push_equality_scalar_fastpath(interp: &mut Interpreter, invert: bool) -> bool {
    if !interp.scalar_fastpath_enabled
        || interp.operation_target_mode != OperationTargetMode::StackTop
        || interp.stack.len() < 2
    {
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
    interp.runtime_metrics.scalar_fastpath_count = interp
        .runtime_metrics
        .scalar_fastpath_count
        .saturating_add(1);
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
            ScalarCmp::Unknown(p) => return Ok(ScalarCmp::Unknown(p)),
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
            ScalarCmp::Unknown(p) => return ScalarCmp::Unknown(p),
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

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
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
                Ok(ScalarCmp::Unknown(p)) => push_unknown(interp, Some(p)),
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

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_count_from_value(&count_val)?;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Ok(());
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - count..].to_vec()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

            if let Some(nil) = nil_passthrough_value(&items) {
                interp.stack.push(nil);
                return Ok(());
            }

            match check_all_adjacent_pairs(&items, kind) {
                Ok(ScalarCmp::Decided(decided)) => push_boolean_result(interp, decided),
                Ok(ScalarCmp::Unknown(p)) => push_unknown(interp, Some(p)),
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.extend(items);
                    }
                    interp.stack.push(count_val);
                    return Err(e);
                }
            }
            Ok(())
        }
    }
}

/// Three-valued interval comparison for the same ordering schema used by the
/// budgeted exact-real path: `Some(true)` and `Some(false)` are decidable;
/// `None` means the intervals overlap in a way that depends on unresolved
/// precision and therefore projects to logical `Unknown`.
fn interval_relation_for_kind(interp: &mut Interpreter, kind: OrderingKind) -> Option<Result<()>> {
    if interp.stack.len() < 2 {
        return None;
    }
    let len = interp.stack.len();
    let a = interp.stack[len - 2].clone();
    let b = interp.stack[len - 1].clone();
    let (ai, bi) = match (value_to_interval(&a), value_to_interval(&b)) {
        (Some(ai), Some(bi)) => (ai, bi),
        _ => return None,
    };
    Some(match kind.interval_decision(&ai, &bi) {
        Some(v) => {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            push_boolean_result(interp, v);
            Ok(())
        }
        None => {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            // Interval-overlap undecidability carries no CF prefix.
            push_unknown(interp, None);
            Ok(())
        }
    })
}

fn apply_ordering_schema(interp: &mut Interpreter, kind: OrderingKind) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop {
        if push_ordering_scalar_fastpath(interp, kind) {
            return Ok(());
        }
        if let Some(res) = interval_relation_for_kind(interp, kind) {
            return res;
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

/// Three-valued pairwise equality matching the SPEC §7.4.1
/// discipline: `Some(true)` / `Some(false)` for decidable pairs,
/// `None` when budget exhaustion makes the comparison undecidable
/// (the caller projects `None` to the logical `Unknown` U).
/// `None` is only reachable for scalar pairs where at least one
/// operand is a non-Rational `ExactReal`; the structural Vector /
/// Tensor / Record paths and the singleton-projection paths always
/// decide.
fn pairwise_eq(a_val: &Value, b_val: &Value) -> ScalarCmp {
    if a_val.data == b_val.data {
        return ScalarCmp::Decided(true);
    }
    if let (Some(ai), Some(bi)) = (value_to_interval(a_val), value_to_interval(b_val)) {
        if ai.is_exact() && bi.is_exact() {
            return ScalarCmp::Decided(ai.lo == bi.lo);
        }
        return ScalarCmp::Decided(false);
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

/// Scalar–scalar equality routed through `ExactReal::eq_with_budget`
/// (SPEC §7.4.1). Both-Rational operands decide via `Fraction`
/// `PartialEq` — value equality on canonical reduced rationals.
/// Anything mixing in a non-Rational `ExactReal` runs the budgeted
/// CF expansion; budget exhaustion returns `None` and the caller
/// projects it to the Undecidable NIL.
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
        _ => match cmp_default_budget(&a, &b) {
            CmpOutcome::Decided(o) => ScalarCmp::Decided(o == std::cmp::Ordering::Equal),
            CmpOutcome::Undecided { agreed_prefix } => ScalarCmp::Unknown(agreed_prefix),
        },
    }
}

fn apply_equality(interp: &mut Interpreter, invert: bool) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }

    if push_equality_scalar_fastpath(interp, invert) {
        return Ok(());
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
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
                ScalarCmp::Decided(eq) => {
                    push_boolean_result(interp, if invert { !eq } else { eq })
                }
                ScalarCmp::Unknown(p) => push_unknown(interp, Some(p)),
            }
            Ok(())
        }

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_count_from_value(&count_val)?;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Ok(());
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - count..].to_vec()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

            if let Some(nil) = nil_passthrough_value(&items) {
                interp.stack.push(nil);
                return Ok(());
            }

            match check_all_adjacent_eq(&items, invert) {
                ScalarCmp::Decided(decided) => push_boolean_result(interp, decided),
                ScalarCmp::Unknown(p) => push_unknown(interp, Some(p)),
            }
            Ok(())
        }
    }
}

/// Push the three-way sign scalar (`-1` / `0` / `1`) produced by
/// `COMPARE-WITHIN`, carrying the `RawNumber` interpretation role.
fn push_sign_result(interp: &mut Interpreter, sign: i64) {
    interp.stack.push(Value::from_int(sign));
    let stack_len = interp.stack.len();
    interp.semantic_registry.normalize_to_stack_len(stack_len);
    interp
        .semantic_registry
        .update_hint_at(stack_len - 1, Interpretation::RawNumber);
}

/// `COMPARE-WITHIN` (SPEC §7.4.2): three-way compare two values within an
/// explicit partial-quotient budget.
///
/// Stack effect: `[ a ] [ b ] [ budget ] -> [ -1 | 0 | 1 | UNKNOWN ]`.
///
/// Emits the partial quotients of `a` and `b` in parallel for at most
/// `budget` steps (SPEC §7.4.1) and pushes the exact sign of `a − b`
/// (`-1` if `a < b`, `0` if equal, `1` if `a > b`) when the order is
/// decided, or the logical `Unknown` (U) carrying `diagnosis.agreedPrefix`
/// when the budget is exhausted first. Two finite (rational) operands
/// always decide regardless of `budget`. A non-positive / non-integer
/// `budget` or non-numeric `a`/`b` is malformed use and raises an error
/// (not U); a NIL `a`/`b` operand passes through per SPEC §7.12.
pub fn op_compare_within(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    if interp.stack.len() < 3 {
        return Err(AjisaiError::StackUnderflow);
    }
    let stack_len = interp.stack.len();
    let budget_val = interp.stack[stack_len - 1].clone();
    let b_val = interp.stack[stack_len - 2].clone();
    let a_val = interp.stack[stack_len - 3].clone();

    // The budget must be a positive integer (SPEC §7.4.2): a non-integer
    // or non-positive budget is malformed use and raises an error rather
    // than producing U. Read it without mutating the stack so the error
    // path leaves operands intact.
    let budget_i = extract_integer_from_value(&budget_val)?;
    if budget_i <= 0 {
        return Err(AjisaiError::create_structure_error(
            "positive integer budget",
            "non-positive budget",
        ));
    }
    let budget = budget_i as usize;

    // NIL passthrough for the a/b operands (SPEC §7.12 / §7.4.2).
    if let Some(nil) = nil_passthrough_value(&[a_val.clone(), b_val.clone()]) {
        if !is_keep_mode {
            interp.stack.truncate(stack_len - 3);
        }
        interp.stack.push(nil);
        return Ok(());
    }

    // Non-numeric a/b is malformed use and raises an error. Extraction
    // happens before any pop, so the error path leaves the stack intact.
    let a = extract_exact_real_for_comparison(&a_val)?;
    let b = extract_exact_real_for_comparison(&b_val)?;

    let outcome = match (a.as_rational(), b.as_rational()) {
        // Both finite: decide exactly via Fraction order regardless of
        // budget (SPEC §7.4.2 — finite CFs differ at a bounded index).
        (Some(af), Some(bf)) => CmpOutcome::Decided(af.cmp(bf)),
        // Deliberately the *streamed* comparison, not the §4.2.7 total
        // decision procedure: COMPARE-WITHIN is defined as emitting at
        // most `budget` partial quotients and is the only observation
        // window on comparison depth (SPEC §7.4.2 / §16 #11), so its
        // Unknown outcome stays reachable even for admitted-domain
        // operands whose CF streams do not diverge within the budget.
        _ => a.cmp_streamed_with_budget_tracked(&b, budget),
    };

    if !is_keep_mode {
        interp.stack.truncate(stack_len - 3);
    }

    match outcome {
        CmpOutcome::Decided(o) => {
            use std::cmp::Ordering;
            let sign = match o {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            };
            push_sign_result(interp, sign);
        }
        CmpOutcome::Undecided { agreed_prefix } => {
            interp.stack.push(Value::unknown_with_agreed_prefix(
                Some("COMPARE-WITHIN"),
                agreed_prefix,
            ));
            let len = interp.stack.len();
            interp.semantic_registry.normalize_to_stack_len(len);
            interp
                .semantic_registry
                .update_hint_at(len - 1, Interpretation::TruthValue);
        }
    }
    Ok(())
}
