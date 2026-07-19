//! Tier 2 vocabulary (Phase 7): the first words that construct and observe a
//! general computable real from the ordinary surface.
//!
//! - `MATH@PI` pushes π, a Tier 2 value (no algebraic normal form).
//! - `MATH@ENCLOSE` observes any value's rational enclosure under an explicit
//!   water budget, exposing the observation process without leaking the tier.
//!
//! Comparing a Tier 2 value with `COMPARE-WITHIN` can now reach the logical
//! Unknown when the budget is exhausted (see `comparison::op_compare_within`).

use crate::error::{AjisaiError, Result};
use crate::interpreter::comparison::extract_exact_real_for_comparison;
use crate::interpreter::value_extraction_helpers::{
    extract_integer_from_value, nil_passthrough_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::exact::{pi, ExactReal};
use crate::types::interval::Interval;
use crate::types::{Interpretation, Value};

/// `MATH@PI` — push π as a Tier 2 computable real. Nullary (mass 0 → 1).
pub(crate) fn op_pi(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from("PI: Stack mode is not supported"));
    }
    interp
        .stack
        .push(Value::from_exact_real(ExactReal::Computable(pi::pi())));
    let len = interp.stack.len();
    interp.semantic_registry.normalize_to_stack_len(len);
    interp
        .semantic_registry
        .update_hint_at(len - 1, Interpretation::RawNumber);
    Ok(())
}

/// `MATH@ENCLOSE` — `[ x ] [ budget ] -> [ [lo hi] ]`. Observe `x`'s rational
/// enclosure after spending `budget` water, as an interval value. Tier ≤ 1
/// values yield a point `[x, x]`; a Tier 2 value yields a genuinely narrowing
/// enclosure the budget governs, with rational endpoints (no tier leaks). A
/// non-positive / non-integer budget is malformed use and errors; a NIL `x`
/// passes through (SPEC §7.12).
pub(crate) fn op_enclose(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from("ENCLOSE: Stack mode is not supported"));
    }
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    let len = interp.stack.len();
    let budget_val = interp.stack[len - 1].clone();
    let x_val = interp.stack[len - 2].clone();

    // Read the budget without mutating the stack so an error leaves operands
    // intact (mirrors COMPARE-WITHIN).
    let budget_i = extract_integer_from_value(&budget_val)?;
    if budget_i <= 0 {
        return Err(AjisaiError::create_structure_error(
            "positive integer budget",
            "non-positive budget",
        ));
    }

    if let Some(nil) = nil_passthrough_value(std::slice::from_ref(&x_val)) {
        if !is_keep {
            interp.stack.truncate(len - 2);
        }
        interp.stack.push(nil);
        let new_len = interp.stack.len();
        interp.semantic_registry.normalize_to_stack_len(new_len);
        return Ok(());
    }

    let er = extract_exact_real_for_comparison(&x_val)?;
    let iv = er.observe_enclosure(budget_i as u64).ok_or_else(|| {
        AjisaiError::create_structure_error("observable value", "empty observation")
    })?;
    let interval = Interval::new(iv.lo, iv.hi)?;

    if !is_keep {
        interp.stack.truncate(len - 2);
    }
    interp.stack.push(Value::from_interval(interval));
    let new_len = interp.stack.len();
    interp.semantic_registry.normalize_to_stack_len(new_len);
    interp
        .semantic_registry
        .update_hint_at(new_len - 1, Interpretation::Interval);
    Ok(())
}
