use crate::types::{FlowToken, Value, ValueData};
use std::rc::Rc;







#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InPlaceJudgment {



    Safe,


    Aliased,



    PartiallyConsumed,


    NoFlowContext,
}


















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

        assert_eq!(check_in_place_candidate(&v, Some(&flow)), InPlaceJudgment::Safe);
    }



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



    #[test]
    fn test_aliased_when_rc_shared() {

        let children = Rc::new(vec![scalar(1), scalar(2)]);
        let v1 = Value { data: ValueData::Vector(Rc::clone(&children)) };
        let _v2 = Value { data: ValueData::Vector(Rc::clone(&children)) };

        let flow = fresh_flow(&v1);
        assert_eq!(check_in_place_candidate(&v1, Some(&flow)), InPlaceJudgment::Aliased);
    }

    #[test]
    fn test_safe_after_alias_dropped() {
        let children = Rc::new(vec![scalar(1), scalar(2)]);
        let v1 = Value { data: ValueData::Vector(Rc::clone(&children)) };

        drop(children);
        let flow = fresh_flow(&v1);
        assert_eq!(check_in_place_candidate(&v1, Some(&flow)), InPlaceJudgment::Safe);
    }



    #[test]
    fn test_hook_consistent_with_can_update_in_place() {
        let v = scalar(3);
        let flow = fresh_flow(&v);
        let can = flow.can_update_in_place(&v);
        let judgment = check_in_place_candidate(&v, Some(&flow));
        assert_eq!(can, judgment == InPlaceJudgment::Safe);
    }
}
