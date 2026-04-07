use crate::types::{FlowToken, Value, ValueData};
use std::rc::Rc;

/// Judgment returned by the linear consumption optimization hook.
///
/// This API is informational only — it does not alter execution behavior.
/// Operators call [`check_in_place_candidate`] to identify values eligible
/// for future in-place mutation optimizations without violating the
/// fractional-dataflow mass-conservation invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InPlaceJudgment {
    /// Safe to update in place: `remaining == total`, no bifurcation links,
    /// and the backing allocation has exactly one owner (Rc strong count == 1
    /// for Vector/Record; inherently safe for Scalar/Nil).
    Safe,
    /// The backing allocation is shared (Rc strong count > 1).
    /// In-place mutation would silently corrupt other references.
    Aliased,
    /// The FlowToken has been partially consumed (`remaining < total`), is
    /// part of a bifurcation tree, or has a non-unit mass ratio.
    /// Mass-conservation semantics require the original value to be preserved.
    PartiallyConsumed,
    /// No FlowToken context is available for this value.
    /// Safety cannot be determined without flow tracking.
    NoFlowContext,
}

/// Hook: determine whether `value` is a safe in-place update candidate.
///
/// This function encodes the linear-type safety check for the fractional-
/// dataflow model. Operators SHOULD call this before choosing between a
/// copy-and-transform path and a potential in-place transform path.
///
/// Conditions for [`InPlaceJudgment::Safe`]:
/// - `flow.remaining == flow.total` (full mass present; nothing consumed yet)
/// - `flow.parent_flow_id.is_none()` and `flow.child_flow_ids.is_empty()`
///   (value is not part of a bifurcation tree)
/// - `flow.mass_ratio == (1, 1)` (unshared, full-unit mass)
/// - `value.is_uniquely_owned()` (Rc strong count == 1 for Vector/Record,
///   or inherently safe for Scalar/Nil)
///
/// The hook does NOT change what operations execute or how results are
/// stored. Its return value is advisory. Operators are expected to act on
/// `Safe` judgments once the corresponding optimization pass is activated.
#[inline]
pub(crate) fn check_in_place_candidate(
    value: &Value,
    flow: Option<&FlowToken>,
) -> InPlaceJudgment {
    let Some(flow) = flow else {
        return InPlaceJudgment::NoFlowContext;
    };
    if !flow.is_reusable_allocation() {
        return InPlaceJudgment::PartiallyConsumed;
    }
    if !value.is_uniquely_owned() {
        return InPlaceJudgment::Aliased;
    }
    InPlaceJudgment::Safe
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::fraction::Fraction;

    fn scalar(n: i64) -> Value {
        Value::from_int(n)
    }

    fn fresh_flow(value: &Value) -> FlowToken {
        FlowToken::from_value(value)
    }

    // ── NoFlowContext ────────────────────────────────────────────────────

    #[test]
    fn test_no_flow_context_when_token_absent() {
        let v = scalar(42);
        assert_eq!(check_in_place_candidate(&v, None), InPlaceJudgment::NoFlowContext);
    }

    #[test]
    fn test_no_flow_context_for_nil_value() {
        let v = Value::nil();
        assert_eq!(check_in_place_candidate(&v, None), InPlaceJudgment::NoFlowContext);
    }

    // ── Safe ─────────────────────────────────────────────────────────────

    #[test]
    fn test_safe_for_scalar_with_fresh_flow() {
        let v = scalar(7);
        let flow = fresh_flow(&v);
        assert_eq!(check_in_place_candidate(&v, Some(&flow)), InPlaceJudgment::Safe);
    }

    #[test]
    fn test_safe_for_nil_with_fresh_flow() {
        let v = Value::nil();
        let flow = fresh_flow(&v);
        assert_eq!(check_in_place_candidate(&v, Some(&flow)), InPlaceJudgment::Safe);
    }

    #[test]
    fn test_safe_for_uniquely_owned_vector() {
        let v = Value::from_children(vec![scalar(1), scalar(2), scalar(3)]);
        let flow = fresh_flow(&v);
        // Rc strong count == 1 → uniquely owned → Safe
        assert_eq!(check_in_place_candidate(&v, Some(&flow)), InPlaceJudgment::Safe);
    }

    // ── PartiallyConsumed ─────────────────────────────────────────────────

    #[test]
    fn test_partially_consumed_when_remaining_less_than_total() {
        let v = scalar(10);
        let mut flow = fresh_flow(&v);
        let half = Fraction::new(
            num_bigint::BigInt::from(5),
            num_bigint::BigInt::from(1),
        );
        let (_, updated) = flow.consume(&half).expect("consume should succeed");
        flow = updated;
        // remaining (5) < total (10) → PartiallyConsumed
        assert_eq!(check_in_place_candidate(&v, Some(&flow)), InPlaceJudgment::PartiallyConsumed);
    }

    #[test]
    fn test_partially_consumed_when_parent_flow_set() {
        let v = scalar(4);
        let mut flow = fresh_flow(&v);
        flow.parent_flow_id = Some(99);
        assert_eq!(check_in_place_candidate(&v, Some(&flow)), InPlaceJudgment::PartiallyConsumed);
    }

    #[test]
    fn test_partially_consumed_when_child_flows_present() {
        let v = scalar(4);
        let mut flow = fresh_flow(&v);
        flow.child_flow_ids.push(1);
        assert_eq!(check_in_place_candidate(&v, Some(&flow)), InPlaceJudgment::PartiallyConsumed);
    }

    #[test]
    fn test_partially_consumed_when_mass_ratio_not_unit() {
        let v = scalar(4);
        let mut flow = fresh_flow(&v);
        flow.mass_ratio = (1, 2);
        assert_eq!(check_in_place_candidate(&v, Some(&flow)), InPlaceJudgment::PartiallyConsumed);
    }

    // ── Aliased ───────────────────────────────────────────────────────────

    #[test]
    fn test_aliased_when_rc_shared() {
        // Build a shared Rc: children (count=1) → v1 (count=2) → v2 (count=3).
        let children = Rc::new(vec![scalar(1), scalar(2)]);
        let v1 = Value { data: ValueData::Vector(Rc::clone(&children)) };
        let _v2 = Value { data: ValueData::Vector(Rc::clone(&children)) };
        // Rc strong count == 3 (children + v1 + _v2); uniquely_owned == false → Aliased
        let flow = fresh_flow(&v1);
        assert_eq!(check_in_place_candidate(&v1, Some(&flow)), InPlaceJudgment::Aliased);
    }

    #[test]
    fn test_safe_after_alias_dropped() {
        let children = Rc::new(vec![scalar(1), scalar(2)]);
        let v1 = Value { data: ValueData::Vector(Rc::clone(&children)) };
        // Drop the extra owner so only v1 holds a reference (count = 1)
        drop(children);
        let flow = fresh_flow(&v1);
        assert_eq!(check_in_place_candidate(&v1, Some(&flow)), InPlaceJudgment::Safe);
    }

    // ── is_reusable_allocation and can_update_in_place consistency ────────

    #[test]
    fn test_hook_consistent_with_can_update_in_place() {
        let v = scalar(3);
        let flow = fresh_flow(&v);
        let can = flow.can_update_in_place(&v);
        let judgment = check_in_place_candidate(&v, Some(&flow));
        assert_eq!(can, judgment == InPlaceJudgment::Safe);
    }
}
