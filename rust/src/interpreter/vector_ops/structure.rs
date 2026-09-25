use super::extract_vector_elements;
use super::targeting::with_stacktop_vector_target_no_arg;
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_bigint_from_value;
use crate::interpreter::Interpreter;
use crate::types::Value;
use num_traits::ToPrimitive;

/// Join two vectors, lifting one level of nesting out of each.
///
/// Both operands are vectors by the time this runs — `op_concat` rejects
/// anything else — so there is no singleton-lifting branch here: an element is
/// carried across exactly as it sits, and `[ [ 1 ] ] [ [ 2 ] ] CONCAT` stays
/// `[ [ 1 ] [ 2 ] ]`.
fn concat_values(left: &Value, right: &Value) -> Value {
    let mut elements = Vec::new();
    elements.extend(extract_vector_elements(left));
    elements.extend(extract_vector_elements(right));
    Value::from_vector(elements)
}

/// One bound of `RANGE`: an integer the machine can count to. Anything else —
/// a fraction, a String, a Boolean — names no position in an integer sequence,
/// so it is the malformed use `invalidInteger` declares. A Vector never reaches
/// here: `leaf` operands are lifted by the dispatcher first.
fn parse_range_bound(bound: &Value, label: &str) -> Result<i64> {
    let bigint = extract_bigint_from_value(bound).map_err(|_| {
        AjisaiError::declared(
            "invalidInteger",
            format!(
                "the {} must be an integer, got {}",
                label,
                bound.domain_name()
            ),
        )
    })?;
    bigint.to_i64().ok_or_else(|| {
        AjisaiError::declared("invalidInteger", format!("the {} is too large", label))
    })
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
    if interp.stack.len() < 2 {
        return Err(AjisaiError::stack_underflow());
    }

    let base = interp.stack.len() - 2;
    let operands: Vec<Value> = interp.stack.split_off(base).into_values();

    if let Some(got) = operands
        .iter()
        .find(|operand| !operand.is_vector())
        .map(|operand| operand.domain_name())
    {
        // The operands were already taken off; put them back so the
        // stack a reader inspects after the error is the one they wrote.
        for operand in operands {
            interp.stack.push(operand);
        }
        return Err(AjisaiError::declared(
            "nonVector",
            format!("expected two Vectors, got {got}"),
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
        for operand in operands {
            interp.stack.push(operand);
        }
        return Err(e);
    }

    interp.stack.push(concat_values(&operands[0], &operands[1]));
    Ok(())
}

pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    crate::interpreter::collection_meter::charge_stacktop_copy(interp, |len| len)?;

    let reversed = with_stacktop_vector_target_no_arg(interp, |vector_val| {
        // A flat dense buffer reverses as columns. The nested route below
        // unpacked the tensor into one boxed `Value` per lane, reversed *those*,
        // and handed back an AoS `Vector` — so reversing 262,144 numbers cost
        // 68.5 ms and, worse, threw the dense representation away, leaving every
        // Word downstream to decline its own dense path. Rank is the caller's
        // check because reversing a rank-2 tensor reverses rows, and a row is a
        // stride rather than a lane; those keep the nested route.
        //
        // The charge is unaffected: `charge_stacktop_copy` above prices this by
        // length alone, so both routes cost the same, which is what keeps the
        // choice of route unobservable (LANG.AUTHORITY.FREEDOM).
        if let crate::types::ValueData::Tensor { data, shape } = &vector_val.data {
            if shape.len() == 1 {
                return Ok(Value::from_dense_tensor(
                    data.reversed_lanes(),
                    (**shape).clone(),
                ));
            }
        }
        let mut v = extract_vector_elements(vector_val).to_vec();
        v.reverse();
        Ok(Value::from_vector(v))
    })?;
    interp.stack.push(reversed);
    Ok(())
}

/// `start end RANGE` — every integer from `start` to `end`, both included,
/// counting down when `end` is below `start`: `0 3 RANGE` is `[ 0 1 2 3 ]`,
/// `3 0 RANGE` is `[ 3 2 1 0 ]`. There is no step operand: a stride is a
/// multiplication of this sequence (`0 3 RANGE 3 MUL`), so the bounds alone
/// decide the direction and no pair of bounds describes an infinite sequence.
pub fn op_range(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::stack_underflow());
    }
    let end_val = interp.stack.pop().expect("length checked");
    let start_val = interp.stack.pop().expect("length checked");
    let bounds = parse_range_bound(&start_val, "start")
        .and_then(|start| parse_range_bound(&end_val, "end").map(|end| (start, end)));
    let (start, end) = match bounds {
        Ok(bounds) => bounds,
        Err(error) => {
            interp.stack.push(start_val);
            interp.stack.push(end_val);
            return Err(error);
        }
    };
    let step: i64 = if start <= end { 1 } else { -1 };

    // Guard against unbounded materialization before allocating. RANGE loops
    // internally, so it counts as one execution step and bypasses the
    // step-count backstop; an input like `0 9999999999999 RANGE` would
    // otherwise drive the process into an OOM abort (a WASM trap in the
    // playground) instead of a recoverable error. Count the elements in i128
    // so the span arithmetic cannot overflow for extreme i64 bounds.
    let element_count = (end as i128 - start as i128).unsigned_abs() + 1;
    // CS5: the cap is the injectable per-interpreter ceiling (folded into
    // `RuntimeLimits`), so tests can fire this guard with a tiny limit and
    // child runtimes inherit it — same behavior and message as before.
    let max_materialized = interp.runtime_limits.max_materialized_elements;
    if element_count > max_materialized as u128 {
        // Phase 3 (structural-memory-safety roadmap): a well-formed, finite
        // range whose materialized length exceeds the space water level is a
        // well-formed operation that cannot produce a value within budget. The
        // NIL Projection Rule projects it onto a diagnosable NIL (reason
        // `spaceExhausted`) so a pipeline can recover it with a chosen fallback,
        // instead of a channel error that halts evaluation.
        interp
            .stack
            .push(crate::interpreter::space_projection::space_exhausted_nil(
                "RANGE",
                max_materialized,
                Some(element_count),
            ));
        return Ok(());
    }

    // Materializing an element costs what copying one costs: the elements are
    // freshly built rather than cloned, but they are the same boxed values, and
    // a program can ask for them as often as it likes. Charged before the
    // allocation, with the bounds put back on a refusal.
    if let Err(e) =
        crate::interpreter::collection_meter::charge_materialization(interp, element_count as usize)
    {
        interp.stack.push(start_val);
        interp.stack.push(end_val);
        return Err(e);
    }

    // Built as columns, not as boxed lanes. `parse_range_bound` answers in
    // `i64`, so *every* value RANGE can produce is an `i64` with denominator 1
    // and no lane absent — a 1-D pure-integer dense tensor is not a guess about
    // this result, it is what the result is. Building `Vec<Value>` instead
    // boxed each lane into a 96-byte `Value` wrapping a 64-byte `Fraction` to
    // carry 8 bytes of integer, and then every Word downstream had to decline
    // its dense fast path because the dense representation had been thrown away
    // at construction: `0 262143 RANGE` spent 9.9 ms laying out 25 MB to
    // describe 2 MB of numbers.
    //
    // `element_count` is exact (`|end - start| + 1` counts the lanes the
    // comparison loops below used to visit), so the count drives the loop and
    // the bound comparison is gone with it. `saturating_add` matters only on the
    // final, unused step past the last lane, where `current += step` could
    // overflow `i64` for an extreme bound; the lanes themselves are unchanged.
    let count = element_count as usize;
    let mut numerators = Vec::with_capacity(count);
    let mut current = start;
    for _ in 0..count {
        numerators.push(current);
        current = current.saturating_add(step);
    }

    interp.stack.push(Value::from_int_tensor(numerators));

    Ok(())
}

pub fn op_collect(interp: &mut Interpreter) -> Result<()> {
    let count_val = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;

    let count_bigint = match extract_bigint_from_value(&count_val) {
        Ok(bi) => bi,
        Err(_) => {
            let got = crate::types::display::describe_operand(&count_val);
            interp.stack.push(count_val);
            return Err(AjisaiError::declared(
                "invalidInteger",
                format!("expected an integer count, got {got}"),
            ));
        }
    };

    let count: usize = match count_bigint.to_usize() {
        Some(c) if c > 0 => c,
        _ => {
            interp.stack.push(count_val);
            return Err(AjisaiError::declared(
                "invalidInteger",
                "COLLECT count must be a positive integer",
            ));
        }
    };

    if interp.stack.len() < count {
        interp.stack.push(count_val);
        return Err(AjisaiError::stack_underflow());
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
