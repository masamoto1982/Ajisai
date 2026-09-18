use super::higher_order::{execute_executable_code, extract_executable_code, ExecutableCode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::is_vector_value;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::Stack;
use crate::types::Value;

/// What an accumulator walk answers with.
///
/// `FOLD` and `SCAN` are one walk: seed an accumulator, then for each element
/// run the caller's block over (accumulator, element) and take what it leaves
/// as the next accumulator. They differ only in which accumulators come back
/// — so the walk is written once and this says which.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// `FOLD`: the accumulator the last element left, alone.
    Last,
    /// `SCAN`: every accumulator the walk passed through, one per element, in
    /// visit order. The seed is not among them, so the answer has one lane per
    /// input lane — which is what lets a scan pair with the Vector it came
    /// from, under `ZIP`, a comparison, or `SELECT`'s truth operand.
    Every,
}

impl Answer {
    /// The answer for a walk with no elements to visit.
    ///
    /// The shape of the answer follows the shape of the question. `FOLD`
    /// reduces to one value, so with nothing to reduce it answers the seed it
    /// was given. `SCAN` answers one lane per lane, so with no lanes it
    /// answers an empty Vector, and the seed — which is not a lane — is not
    /// it.
    fn empty_walk(self, init: Value) -> Value {
        match self {
            Answer::Last => init,
            Answer::Every => Value::from_vector(Vec::new()),
        }
    }

    /// The answer for an absent target, by the same rule.
    ///
    /// `FOLD` of an absent Vector is the seed, because reducing nothing is the
    /// seed. `SCAN` of an absent Vector is that same absence, reason intact,
    /// because there is no lane count to answer with — the collection-shaped
    /// Words (`MAP`) answer an absent target the same way.
    fn absent_target(self, init: Value, target: Value) -> Value {
        match self {
            Answer::Last => init,
            Answer::Every => target,
        }
    }
}

pub fn op_fold(interp: &mut Interpreter) -> Result<()> {
    run_accumulator_walk(interp, "FOLD", Answer::Last)
}

/// `SCAN` — `FOLD` that keeps its working.
///
/// `[ vec ] [ init ] [ combine ] SCAN` answers the accumulator after each
/// element rather than only the last one, so `[ 1 2 3 4 ] 0 [ ADD ] SCAN` is
/// `[ 1/1 3/1 6/1 10/1 ]`. The seed's own shape carries through the walk
/// exactly as it does in `FOLD`: seeding with `[ 0 ]` instead makes every
/// accumulator a one-lane Vector, because that is what `ADD` answers when one
/// operand is one.
///
/// It is derivable — re-fold every prefix, which is what
/// `standard_operational_laws.rs` witnesses — and it is retained natively
/// because that derivation re-reads each prefix from the start and so does
/// quadratic work for a linear answer. In a language with no recursion and no
/// unbounded loop (`spec/termination.json`), a walk that carries state from
/// one element to the next is the only shape a running computation has, so
/// paying n² for it is paying it everywhere.
pub fn op_scan(interp: &mut Interpreter) -> Result<()> {
    run_accumulator_walk(interp, "SCAN", Answer::Every)
}

fn run_accumulator_walk(
    interp: &mut Interpreter,
    word: &'static str,
    answer: Answer,
) -> Result<()> {
    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let executable: ExecutableCode = match extract_executable_code(interp, &code_val) {
        Ok(exec) => exec,
        Err(e) => {
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    let init_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let target_val: Value = if is_keep_mode {
        interp.stack.last().cloned().ok_or_else(|| {
            interp.stack.push(init_val.clone());
            interp.stack.push(code_val.clone());
            AjisaiError::StackUnderflow
        })?
    } else {
        interp.stack.pop().ok_or_else(|| {
            interp.stack.push(init_val.clone());
            interp.stack.push(code_val.clone());
            AjisaiError::StackUnderflow
        })?
    };

    if target_val.is_nil() {
        interp
            .stack
            .push(answer.absent_target(init_val, target_val));
        return Ok(());
    }

    if !is_vector_value(&target_val) {
        if !is_keep_mode {
            interp.stack.push(target_val);
        }
        interp.stack.push(init_val);
        interp.stack.push(code_val);
        return Err(AjisaiError::declared(
            "nonVector",
            format!("{word}: expected a Vector, got a non-vector value"),
        ));
    }

    let n_elements: usize = target_val.len();
    if n_elements == 0 {
        interp.stack.push(answer.empty_walk(init_val));
        return Ok(());
    }

    let mut accumulator: Value = init_val;
    let mut visited: Vec<Value> = Vec::new();
    if answer == Answer::Every {
        visited.reserve(n_elements);
    }
    let mut saved_stack: Stack = Stack::new();
    std::mem::swap(&mut interp.stack, &mut saved_stack);
    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let mut error: Option<AjisaiError> = None;
    for i in 0..n_elements {
        let elem: Value = target_val
            .child(i)
            .unwrap_or_else(|| panic!("{word}: child index in 0..len must be valid"));
        interp.stack.clear();
        interp.stack.push(accumulator.clone());
        interp.stack.push(elem);
        match execute_executable_code(interp, &executable) {
            Ok(_) => match interp.stack.pop() {
                Some(result) => {
                    if answer == Answer::Every {
                        visited.push(result.clone());
                    }
                    accumulator = result;
                }
                None => {
                    error = Some(AjisaiError::declared(
                        "blockContractViolation",
                        format!("{word}: expected return value, got empty stack"),
                    ));
                    break;
                }
            },
            Err(e) => {
                error = Some(e);
                break;
            }
        }
    }
    interp.disable_no_change_check = saved_no_change_check;
    interp.stack = saved_stack;

    if let Some(e) = error {
        if !is_keep_mode {
            interp.stack.push(target_val);
        }
        interp.stack.push(accumulator);
        interp.stack.push(code_val);
        return Err(e);
    }

    interp.stack.push(match answer {
        Answer::Last => accumulator,
        Answer::Every => Value::from_vector(visited),
    });
    Ok(())
}
