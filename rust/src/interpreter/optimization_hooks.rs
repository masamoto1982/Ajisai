use crate::types::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InPlaceJudgment {
    Safe,
    Aliased,
}

#[inline]
pub(crate) fn check_in_place_candidate(value: &Value) -> InPlaceJudgment {
    if !value.is_uniquely_owned() {
        return InPlaceJudgment::Aliased;
    }
    InPlaceJudgment::Safe
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DisplayHint, ValueData};
    use std::rc::Rc;

    fn scalar(n: i64) -> Value {
        Value::from_int(n)
    }

    #[test]
    fn test_safe_for_scalar() {
        let v = scalar(7);
        assert_eq!(check_in_place_candidate(&v), InPlaceJudgment::Safe);
    }

    #[test]
    fn test_safe_for_nil() {
        let v = Value::nil();
        assert_eq!(check_in_place_candidate(&v), InPlaceJudgment::Safe);
    }

    #[test]
    fn test_safe_for_uniquely_owned_vector() {
        let v = Value::from_children(vec![scalar(1), scalar(2), scalar(3)]);
        assert_eq!(check_in_place_candidate(&v), InPlaceJudgment::Safe);
    }

    #[test]
    fn test_aliased_when_rc_shared() {
        let children = Rc::new(vec![scalar(1), scalar(2)]);
        let v1 = Value {
            data: ValueData::Vector(Rc::clone(&children)),
            hint: DisplayHint::Auto,
            absence: None,
        };
        let _v2 = Value {
            data: ValueData::Vector(Rc::clone(&children)),
            hint: DisplayHint::Auto,
            absence: None,
        };

        assert_eq!(check_in_place_candidate(&v1), InPlaceJudgment::Aliased);
    }

    #[test]
    fn test_safe_after_alias_dropped() {
        let children = Rc::new(vec![scalar(1), scalar(2)]);
        let v1 = Value {
            data: ValueData::Vector(Rc::clone(&children)),
            hint: DisplayHint::Auto,
            absence: None,
        };

        drop(children);
        assert_eq!(check_in_place_candidate(&v1), InPlaceJudgment::Safe);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanValidationEvent {
    Validated,
    Invalidated,
    ShadowMismatch,
}

impl crate::interpreter::Interpreter {
    pub(crate) fn record_plan_validation_event(
        &mut self,
        word_name: &str,
        event: PlanValidationEvent,
    ) {
        match event {
            PlanValidationEvent::Validated => {
                self.push_hedged_trace(format!("plan:validated word={}", word_name));
            }
            PlanValidationEvent::Invalidated => {
                self.push_hedged_trace(format!("plan:invalidated word={}", word_name));
            }
            PlanValidationEvent::ShadowMismatch => {
                self.push_hedged_trace(format!("plan:mismatch word={}", word_name));
            }
        }
    }
}
