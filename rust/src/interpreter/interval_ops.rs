use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::interval::{
    default_sqrt_eps, exact_rational_sqrt, sqrt_rational_interval, Interval,
};
use crate::types::{Interpretation, Value, ValueData};

pub(crate) fn value_to_interval(value: &Value) -> Option<Interval> {
    match (&value.data, value.hint) {
        (ValueData::Scalar(f), _) => Some(Interval::from_scalar(f.clone())),
        (ValueData::Vector(v), Interpretation::Interval) if v.len() == 2 => {
            let lo = v[0].as_scalar()?.clone();
            let hi = v[1].as_scalar()?.clone();
            Interval::new(lo, hi).ok()
        }
        _ => None,
    }
}

pub(crate) fn interval_to_value(interval: Interval) -> Value {
    if interval.is_exact() {
        Value::from_fraction(interval.lo)
    } else {
        Value::from_interval(interval)
    }
}

fn pop_with_keep(interp: &mut Interpreter) -> Result<(Value, Value)> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    if interp.consumption_mode == ConsumptionMode::Keep {
        let len = interp.stack.len();
        Ok((interp.stack[len - 2].clone(), interp.stack[len - 1].clone()))
    } else {
        let b = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let a = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        Ok((a, b))
    }
}

pub(crate) fn op_interval(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from("INTERVAL: Stack mode is not supported"));
    }
    let (lo_v, hi_v) = pop_with_keep(interp)?;
    let lo = lo_v
        .as_scalar()
        .ok_or_else(|| AjisaiError::from("INTERVAL: lower bound must be scalar"))?
        .clone();
    let hi = hi_v
        .as_scalar()
        .ok_or_else(|| AjisaiError::from("INTERVAL: upper bound must be scalar"))?
        .clone();
    let interval = Interval::new(lo, hi)?;
    interp.stack.push(Value::from_interval(interval));
    interp.semantic_registry.push_hint(Interpretation::Interval);
    Ok(())
}

/// `>CF` — tag the top-of-stack value with the ContinuedFraction
/// interpretation role (SPEC §12.2). Value-preserving: it only retags
/// the existing top, leaving the underlying data untouched.
pub(crate) fn op_to_cf(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from(">CF: Stack mode is not supported"));
    }
    let len = interp.stack.len();
    if len == 0 {
        return Err(AjisaiError::StackUnderflow);
    }
    interp
        .semantic_registry
        .update_hint_at(len - 1, Interpretation::ContinuedFraction);
    Ok(())
}

pub(crate) fn op_lower(interp: &mut Interpreter) -> Result<()> {
    unary_interval_accessor(interp, |i| i.lo)
}

pub(crate) fn op_upper(interp: &mut Interpreter) -> Result<()> {
    unary_interval_accessor(interp, |i| i.hi)
}

pub(crate) fn op_width(interp: &mut Interpreter) -> Result<()> {
    unary_interval_accessor(interp, |i| i.width())
}

pub(crate) fn op_is_exact(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from("IS_EXACT: Stack mode is not supported"));
    }
    let value = if interp.consumption_mode == ConsumptionMode::Keep {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };
    let interval = value_to_interval(&value)
        .ok_or_else(|| AjisaiError::from("IS_EXACT: expected Number or Interval"))?;
    interp.stack.push(Value::from_bool(interval.is_exact()));
    interp
        .semantic_registry
        .push_hint(Interpretation::TruthValue);
    Ok(())
}

fn unary_interval_accessor<F>(interp: &mut Interpreter, f: F) -> Result<()>
where
    F: Fn(Interval) -> Fraction,
{
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from(
            "interval accessor: Stack mode is not supported",
        ));
    }
    let value = if interp.consumption_mode == ConsumptionMode::Keep {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };
    let interval = value_to_interval(&value)
        .ok_or_else(|| AjisaiError::from("interval accessor: expected Number or Interval"))?;
    interp.stack.push(Value::from_fraction(f(interval)));
    interp
        .semantic_registry
        .push_hint(Interpretation::RawNumber);
    Ok(())
}

pub(crate) fn op_sqrt(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from("SQRT: Stack mode is not supported"));
    }
    let value = if interp.consumption_mode == ConsumptionMode::Keep {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    // Fast exact path for rational scalar inputs: produce an ExactReal.
    if let Some(f) = value.as_scalar() {
        match ExactReal::from_sqrt_rational(f.clone()) {
            Some(er) => {
                // from_exact_real already collapses Rational variants to Scalar(Fraction)
                interp.stack.push(Value::from_exact_real(er));
                interp
                    .semantic_registry
                    .push_hint(Interpretation::RawNumber);
            }
            None => {
                // Negative input → NIL
                interp
                    .stack
                    .push(Value::nil_with_reason(NilReason::DivisionByZero));
                interp.semantic_registry.push_hint(Interpretation::Nil);
            }
        }
        return Ok(());
    }

    // Fallback: interval path for Interval-type inputs (SQRT_EPS etc.)
    let interval = value_to_interval(&value)
        .ok_or_else(|| AjisaiError::from("SQRT: expected Number or Interval"))?;

    let result = sqrt_interval_with_eps(interval, default_sqrt_eps())?;
    let out_hint = if result.is_exact() {
        Interpretation::RawNumber
    } else {
        Interpretation::Interval
    };
    interp.stack.push(interval_to_value(result));
    interp.semantic_registry.push_hint(out_hint);
    Ok(())
}

pub(crate) fn op_sqrt_eps(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from("SQRT_EPS: Stack mode is not supported"));
    }
    let (value, eps_value) = pop_with_keep(interp)?;
    let interval = value_to_interval(&value)
        .ok_or_else(|| AjisaiError::from("SQRT_EPS: expected Number or Interval as first arg"))?;
    let eps = eps_value
        .as_scalar()
        .ok_or_else(|| AjisaiError::from("SQRT_EPS: eps must be scalar rational"))?
        .clone();
    let result = sqrt_interval_with_eps(interval, eps)?;
    let out_hint = if result.is_exact() {
        Interpretation::RawNumber
    } else {
        Interpretation::Interval
    };
    interp.stack.push(interval_to_value(result));
    interp.semantic_registry.push_hint(out_hint);
    Ok(())
}

fn sqrt_interval_with_eps(interval: Interval, eps: Fraction) -> Result<Interval> {
    if interval.hi.lt(&Fraction::from(0)) {
        Err(AjisaiError::from("sqrt of negative value"))
    } else if interval.lo.lt(&Fraction::from(0)) {
        let hi = sqrt_value_to_interval(&interval.hi, &eps)?;
        Ok(Interval::new(Fraction::from(0), hi.hi)?)
    } else {
        let lo = sqrt_value_to_interval(&interval.lo, &eps)?;
        let hi = sqrt_value_to_interval(&interval.hi, &eps)?;
        Ok(Interval::new(lo.lo, hi.hi)?)
    }
}

fn sqrt_value_to_interval(v: &Fraction, eps: &Fraction) -> Result<Interval> {
    if let Some(exact) = exact_rational_sqrt(v) {
        return Ok(Interval::from_scalar(exact));
    }
    sqrt_rational_interval(v, eps)
}
