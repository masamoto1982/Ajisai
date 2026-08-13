use super::extract_vector_elements;
use super::targeting::with_stacktop_vector_target_no_arg;
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::extract_bigint_from_value;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::fraction::Fraction;
use crate::types::Value;
use num_traits::ToPrimitive;

/// Join two vectors, lifting one level of nesting out of each, under the role
/// the join carries.
///
/// Both operands are vectors by the time this runs — `op_concat` rejects
/// anything else — so there is no singleton-lifting branch here: an element is
/// carried across exactly as it sits, and `[ [ 1 ] ] [ [ 2 ] ] CONCAT` stays
/// `[ [ 1 ] [ 2 ] ]`.
///
/// The role goes on the value, not only on the stack slot, so a Text survives
/// being put inside a vector or returned from a user Word — the same place
/// `Value::from_string` puts it.
fn concat_values(left: &Value, right: &Value) -> Value {
    let mut elements = Vec::new();
    elements.extend(extract_vector_elements(left));
    elements.extend(extract_vector_elements(right));
    Value::from_vector(elements)
}

fn parse_range_bound(args_val: &Value, index: usize, label: &str) -> Result<i64> {
    let child = args_val
        .child(index)
        .ok_or_else(|| AjisaiError::from(format!("RANGE missing {}", label)))?;
    let bigint = extract_bigint_from_value(&child)
        .map_err(|_| AjisaiError::from(format!("RANGE {} must be an integer", label)))?;
    bigint
        .to_i64()
        .ok_or_else(|| AjisaiError::from(format!("RANGE {} is too large", label)))
}

fn parse_range_args(args_val: &Value) -> Result<(i64, i64, i64)> {
    if !args_val.is_vector() {
        return Err(AjisaiError::from(
            "RANGE requires [start end] or [start end step]",
        ));
    }

    let n = args_val.len();
    if !(2..=3).contains(&n) {
        return Err(AjisaiError::from(
            "RANGE requires [start end] or [start end step]",
        ));
    }

    let start = parse_range_bound(args_val, 0, "start")?;
    let end = parse_range_bound(args_val, 1, "end")?;
    let step = if n == 3 {
        parse_range_bound(args_val, 2, "step")?
    } else if start <= end {
        1
    } else {
        -1
    };

    Ok((start, end, step))
}

/// `CONCAT` — join the top two vectors (SPEC: `2 -> 1`, `errorWhen:
/// [nonVector]`).
///
/// The arity is exactly the declared one. `CONCAT` used to accept an undeclared
/// count-prefixed form (`a b c 3 CONCAT`, negative for reversed order) that it
/// recognized by *sniffing* the stack top for a bare integer, and to treat a
/// non-vector operand as a singleton instead of refusing it. Both are gone: a
/// calling convention the specification does not declare is a second contract
/// written nowhere, and the sniff decided `CONCAT`'s arity from the *value* of
/// an operand — which is what made `[ 1 2 ] [ 3 ] CONCAT` an underflow, and
/// what left the declared `nonVector` error unreachable (`2 3 CONCAT` reported
/// a short stack rather than a non-vector operand). It also disagreed with the
/// dispatch NIL guard, which clamps its window to the declared arity of 2 and
/// so could not see the operands a longer count would reach.
pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let base = interp.stack.len() - 2;
    let operands: Vec<Value> = if is_keep_mode {
        interp.stack.as_slice()[base..].to_vec()
    } else {
        interp.stack.split_off(base).into_values()
    };

    if operands.iter().any(|operand| !operand.is_vector()) {
        // Consuming mode already took the operands off; put them back so the
        // stack a reader inspects after the error is the one they wrote.
        if !is_keep_mode {
            for operand in operands {
                interp.stack.push(operand);
            }
        }
        return Err(AjisaiError::create_structure_error(
            "vector",
            "other format",
        ));
    }

    // Both halves are copied into the join, so both are priced. Charged before
    // the copy runs, with the operands put back first so a refusal leaves the
    // stack the way the program wrote it.
    let units = crate::interpreter::collection_meter::element_cost(&operands[0])
        .copies(operands[0].len())
        .saturating_add(
            crate::interpreter::collection_meter::element_cost(&operands[1])
                .copies(operands[1].len()),
        );
    if let Err(e) = crate::interpreter::collection_meter::charge(interp, units) {
        if !is_keep_mode {
            for operand in operands {
                interp.stack.push(operand);
            }
        }
        return Err(e);
    }

    interp.stack.push(concat_values(&operands[0], &operands[1]));
    Ok(())
}

pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    crate::interpreter::collection_meter::charge_stacktop_copy(interp, |len| len)?;

    let reversed = with_stacktop_vector_target_no_arg(interp, is_keep_mode, |vector_val| {
        let mut v = extract_vector_elements(vector_val).to_vec();
        v.reverse();
        Ok(Value::from_vector(v))
    })?;
    interp.stack.push(reversed);
    Ok(())
}

pub fn op_range(interp: &mut Interpreter) -> Result<()> {
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let (start, end, step) = match parse_range_args(&args_val) {
        Ok(values) => values,
        Err(error) => {
            interp.stack.push(args_val);
            return Err(error);
        }
    };

    if step == 0 {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("RANGE step cannot be 0"));
    }

    if (start < end && step < 0) || (start > end && step > 0) {
        interp.stack.push(args_val);
        return Err(AjisaiError::from(
            "RANGE would create an infinite sequence (check start, end, and step values)",
        ));
    }

    // Guard against unbounded materialization before allocating. RANGE loops
    // internally, so it counts as one execution step and bypasses the
    // step-count backstop; an input like `[ 0 9999999999999 ] RANGE` would
    // otherwise drive the process into an OOM abort (a WASM trap in the
    // playground) instead of a recoverable error. Count the elements in i128
    // so the span arithmetic cannot overflow for extreme i64 bounds.
    let span = (end as i128 - start as i128).unsigned_abs();
    let stride = (step as i128).unsigned_abs();
    let element_count = span / stride + 1;
    // CS5: the cap is the injectable per-interpreter ceiling (folded into
    // `RuntimeLimits`), so tests can fire this guard with a tiny limit and
    // child runtimes inherit it — same behavior and message as before.
    let max_materialized = interp.runtime_limits.max_materialized_elements;
    if element_count > max_materialized as u128 {
        // Phase 3 (structural-memory-safety roadmap): a well-formed, finite
        // range whose materialized length exceeds the space water level is a
        // well-formed operation that cannot produce a value within budget. The
        // Bubble Rule projects it onto a diagnosable NIL (reason
        // `spaceExhausted`) so a pipeline can recover it with `^` (VENT),
        // instead of a channel error that halts evaluation. The malformed cases
        // above (zero step, infinite direction) remain ordinary errors.
        interp
            .stack
            .push(Value::nil_with_reason(NilReason::SpaceExhausted));
        return Ok(());
    }

    // Materializing an element costs what copying one costs: the elements are
    // freshly built rather than cloned, but they are the same boxed values, and
    // a program can ask for them as often as it likes. Charged before the
    // allocation, with the argument put back on a refusal.
    if let Err(e) =
        crate::interpreter::collection_meter::charge_materialization(interp, element_count as usize)
    {
        interp.stack.push(args_val);
        return Err(e);
    }

    let mut range_vec = Vec::with_capacity(element_count as usize);
    let mut current = start;

    if step > 0 {
        while current <= end {
            range_vec.push(Value::from_fraction(Fraction::from(current)));
            current += step;
        }
    } else {
        while current >= end {
            range_vec.push(Value::from_fraction(Fraction::from(current)));
            current += step;
        }
    }

    interp.stack.push(Value::from_vector(range_vec));

    Ok(())
}

pub fn op_collect(interp: &mut Interpreter) -> Result<()> {
    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let count_bigint = match extract_bigint_from_value(&count_val) {
        Ok(bi) => bi,
        Err(_) => {
            interp.stack.push(count_val);
            return Err(AjisaiError::create_structure_error(
                "integer",
                "other format",
            ));
        }
    };

    let count: usize = match count_bigint.to_usize() {
        Some(c) if c > 0 => c,
        _ => {
            interp.stack.push(count_val);
            return Err(AjisaiError::from(
                "COLLECT count must be a positive integer",
            ));
        }
    };

    if interp.stack.len() < count {
        interp.stack.push(count_val);
        return Err(AjisaiError::StackUnderflow);
    }

    if let Err(e) = crate::interpreter::collection_meter::charge_materialization(interp, count) {
        interp.stack.push(count_val);
        return Err(e);
    }

    let collected: Vec<Value> = interp
        .stack
        .split_off(interp.stack.len() - count)
        .into_values();

    interp.stack.push(Value::from_vector(collected));
    Ok(())
}
