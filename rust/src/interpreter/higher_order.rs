//! The higher-order Words `MAP` and `FILTER`, and the block machinery they
//! share with `higher_order_fold` (`FOLD`/`SCAN`).
//!
//! Each Word takes a `[ ... ]` code operand on top of the collection it walks,
//! compiles the block once (`extract_executable_code`) and runs it per element
//! on a scratch stack (`execute_executable_code`).

use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::{Stack, Value};

/// A block, with the compiled plan for it built once.
///
/// The tokens are kept because the plan can go stale: a block that runs `DEF`
/// moves the dictionary epoch, and from that element on the tokens are what
/// must be interpreted. They are also what `execute_compiled_line` itself falls
/// back to for an op it could not lower.
///
/// A Word name written as a String used to be the other way to spell a code
/// operand (`[ 1 2 3 ] 'DBL' MAP`). It is gone, and this is a struct rather
/// than an enum because of it — see `extract_executable_code`.
pub(crate) struct ExecutableCode {
    tokens: Vec<crate::types::Token>,
    plan: crate::interpreter::CompiledPlan,
}

impl ExecutableCode {
    /// The block lowered for a fused walk that starts it on `inputs` values,
    /// when its plan is current and inside the fused subset.
    pub(crate) fn fused(
        &self,
        interp: &Interpreter,
        inputs: usize,
    ) -> Option<crate::interpreter::fused_block::FusedBlock> {
        if !crate::interpreter::is_plan_valid(&self.plan, interp) {
            return None;
        }
        crate::interpreter::fused_block::FusedBlock::compile(&self.plan, inputs)
    }
}

pub(crate) fn extract_executable_code(
    interp: &mut Interpreter,
    val: &Value,
) -> Result<ExecutableCode> {
    // Every Vector is a code operand candidate now (CodeBlock/Vector
    // unification) — bridged back to tokens (`value_as_code.rs`).
    // `as_vector_view` (Tensor-aware) — see control.rs's EXEC for why.
    // Which of the higher-order word's two operands *is* the code one is a
    // separate question this function does not answer: callers decide by
    // stack position before reaching it (the top-of-stack operand), same as
    // before this unification.
    if let Some(elements) = val.as_vector_view() {
        let tokens = crate::interpreter::value_as_code::value_elements_to_tokens(&elements)?;
        // Compiled here, once, rather than in the caller's loop: this is the one
        // place a block becomes executable, and every higher-order Word reaches
        // it before its first element.
        let plan = crate::interpreter::compile_token_block(tokens.clone(), interp);
        return Ok(ExecutableCode { tokens, plan });
    }

    // A String is not code. LANG.SOURCE.CODE says code is a Vector, and
    // LANG.VALUES.DISJOINT makes String a different domain; `EXEC` has always
    // refused one here with this same category, and these Words used to accept
    // it as a Word name to call (`[ 1 2 3 ] 'DBL' MAP`).
    //
    // That spelling was the language's only dynamic call path, and it defeated
    // the DEF-time acyclicity check that LANG.DICTIONARY.ACYCLIC's termination
    // argument rests on: the name could be *computed*, so it appeared in no
    // token of the body for the check to see. `[ [ 1 ] [ 'J' 'W' ] JOIN MAP ]
    // 'JW' DEF` was accepted and then recursed until the native depth guard
    // fired — termination decided by a ceiling, which that clause says it is
    // not. A Symbol cannot be built this way (no Word turns text into one), so
    // refusing the String closes the path rather than narrowing it.
    Err(AjisaiError::declared(
        "notExecutable",
        format!(
            "expected a Vector ([ ... ]) as the code operand, got {}",
            val.domain_name()
        ),
    ))
}

/// Whether a higher-order block's predicate result keeps its element, read in
/// truth position (LANG.VALUES.TRUTH): TRUE keeps it, and FALSE and UNKNOWN — a
/// NIL read here, whatever its reason — do not, since only a predicate that
/// holds selects.
///
/// The domains are disjoint (LANG.VALUES.DISJOINT), so nothing else is a truth
/// value: a scalar is not a Boolean even when it is non-zero, and a singleton
/// Vector is not its element. Each of those used to be accepted here, which
/// gave `FILTER` a truthiness rule no other Word shared — `[ 1 2 3 ] [ 1 ]
/// FILTER` silently kept every element instead of raising `nonTruthValue`, the
/// same declared condition `AND`/`NOT` raise for the identical fault. A NIL, by
/// contrast, is UNKNOWN in truth position for every Word that reads one, so
/// here too.
pub(crate) fn extract_predicate_boolean(condition_result: Value) -> Result<bool> {
    if let Some(b) = condition_result.as_truth() {
        return Ok(b);
    }
    if condition_result.is_nil() {
        return Ok(false);
    }

    Err(AjisaiError::declared(
        "nonTruthValue",
        format!(
            "expected a truth value from the predicate block, got {}",
            condition_result.domain_name()
        ),
    ))
}

pub(crate) fn execute_executable_code(
    interp: &mut Interpreter,
    exec: &ExecutableCode,
) -> Result<()> {
    let ExecutableCode { tokens, plan } = exec;
    interp.bump_execution_epoch();
    // `bump_execution_epoch` moves the execution epoch, not the dictionary one,
    // so a plan stays valid across the loop's elements; only a dictionary change
    // inside the block invalidates it.
    if crate::interpreter::is_plan_valid(plan, interp) {
        crate::interpreter::compiled_plan::execute_compiled_nested_block(interp, plan)
    } else {
        interp.execute_nested_block(tokens)
    }
}

pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;

    let executable: ExecutableCode = match extract_executable_code(interp, &code_val) {
        Ok(exec) => exec,
        Err(e) => {
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    let target_val: Value = interp.stack.pop().ok_or_else(|| {
        interp.stack.push(code_val.clone());
        AjisaiError::stack_underflow()
    })?;

    if target_val.is_nil() {
        interp
            .stack
            .push(Value::nil_inheriting_absence_from(&target_val));
        return Ok(());
    }

    if !target_val.is_vector() {
        let got = target_val.domain_name();
        interp.stack.push(target_val);
        interp.stack.push(code_val);
        return Err(AjisaiError::declared(
            "nonVector",
            format!("expected a Vector, got {got}"),
        ));
    }

    let n_elements: usize = target_val.len();
    if n_elements == 0 {
        interp.stack.push(Value::from_vector(Vec::new()));
        return Ok(());
    }

    let mut results: Vec<Value> = Vec::with_capacity(n_elements);
    let mut saved_stack: Stack = Stack::new();
    std::mem::swap(&mut interp.stack, &mut saved_stack);
    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let mut error: Option<AjisaiError> = None;
    for i in 0..n_elements {
        let elem: Value = target_val
            .child(i)
            .expect("FILTER: child index in 0..len must be valid");
        interp.stack.clear();
        interp.stack.push(elem.clone());
        match execute_executable_code(interp, &executable) {
            Ok(_) => {
                let condition_result: Value = match interp.stack.pop() {
                    Some(r) => r,
                    None => {
                        error = Some(AjisaiError::declared(
                            "blockContractViolation",
                            "expected the predicate block to leave one truth value, and it left none",
                        ));
                        break;
                    }
                };

                let is_true: bool = match extract_predicate_boolean(condition_result) {
                    Ok(v) => v,
                    Err(e) => {
                        error = Some(e);
                        break;
                    }
                };

                if is_true {
                    results.push(elem);
                }
            }
            Err(e) => {
                error = Some(e);
                break;
            }
        }
    }
    interp.disable_no_change_check = saved_no_change_check;
    interp.stack = saved_stack;

    if let Some(e) = error {
        interp.stack.push(target_val);
        interp.stack.push(code_val);
        return Err(e);
    }

    interp.stack.push(Value::from_vector_promoted(results));

    Ok(())
}

pub fn op_map(interp: &mut Interpreter) -> Result<()> {
    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;

    let executable: ExecutableCode = match extract_executable_code(interp, &code_val) {
        Ok(exec) => exec,
        Err(e) => {
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    let target_val: Value = interp.stack.pop().ok_or_else(|| {
        interp.stack.push(code_val.clone());
        AjisaiError::stack_underflow()
    })?;

    if target_val.is_nil() {
        interp
            .stack
            .push(Value::nil_inheriting_absence_from(&target_val));
        return Ok(());
    }

    if !target_val.is_vector() {
        let got = target_val.domain_name();
        interp.stack.push(target_val);
        interp.stack.push(code_val);
        return Err(AjisaiError::declared(
            "nonVector",
            format!("expected a Vector, got {got}"),
        ));
    }

    let n_elements: usize = target_val.len();
    if n_elements == 0 {
        interp.stack.push(Value::from_vector(Vec::new()));
        return Ok(());
    }

    if let Some(result) = executable.fused(interp, 1).and_then(|block| {
        block.run(
            interp,
            crate::interpreter::fused_block::FusedWalk::Map,
            &target_val,
            None,
        )
    }) {
        interp.stack.push(result);
        return Ok(());
    }

    let mut results: Vec<Value> = Vec::with_capacity(n_elements);
    let mut saved_stack: Stack = Stack::new();
    std::mem::swap(&mut interp.stack, &mut saved_stack);
    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let mut error: Option<AjisaiError> = None;
    for i in 0..n_elements {
        let elem: Value = target_val
            .child(i)
            .expect("MAP: child index in 0..len must be valid");
        interp.stack.clear();
        interp.stack.push(elem);
        match execute_executable_code(interp, &executable) {
            Ok(_) => match interp.stack.pop() {
                // The block's one result *is* the mapped element, whatever its
                // shape. A one-element Vector used to be unwrapped here, back
                // when a scalar was itself a one-element Vector and the two
                // were indistinguishable. They are separate domains now
                // (LANG.VALUES.DISJOINT), and the unwrapping outlived its
                // reason: `[ 1 2 ] { 1 COLLECT } MAP` answered `[ 1/1 2/1 ]`,
                // so a block asking in as many words for a Vector of one got a
                // scalar, and there was no way at all to map to singletons.
                // Worse, it was silent and unequal — `[ [ 1 ] ] { REVERSE } MAP
                // 0 GET 5 ADD` answered `6/1` where `[ 6/1 ]` is the
                // answer, which is exactly the quiet wrong result
                // LANG.FAILURE.TRICHOTOMY exists to rule out.
                Some(result_val) => {
                    results.push(result_val);
                }
                None => {
                    error = Some(AjisaiError::declared(
                        "blockContractViolation",
                        "expected the block to leave one value, and it left none",
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
        interp.stack.push(target_val);
        interp.stack.push(code_val);
        return Err(e);
    }

    interp.stack.push(Value::from_vector_promoted(results));

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Test suite for `MAP`'s result contract: the block's one result *is* the
    //! mapped element, whatever shape it has.
    //!
    //! `MAP` used to unwrap a one-element Vector result into its element, a
    //! leftover from the time a scalar and a one-element Vector were the same
    //! thing. With the domains disjoint (LANG.VALUES.DISJOINT) that unwrapping
    //! silently changed the answer and left no way to map onto singletons at all,
    //! so these cases pin the shape rather than only the numbers.

    use crate::interpreter::Interpreter;
    use crate::types::display::render_stack;

    async fn run(source: &str) -> String {
        let mut interp = Interpreter::new();
        interp
            .execute(source)
            .await
            .unwrap_or_else(|e| panic!("{} should run: {:?}", source, e));
        render_stack(interp.get_stack()).join(" ")
    }

    #[tokio::test]
    async fn map_keeps_a_one_element_vector_result_whole() {
        assert_eq!(
            run("[ 1 2 ] [ 1 COLLECT ] MAP").await,
            "[ [ 1/1 ] [ 2/1 ] ]"
        );
    }

    #[tokio::test]
    async fn map_keeps_a_nested_one_element_vector_whole() {
        assert_eq!(
            run("[ [ 1 ] [ 2 3 ] ] [ REVERSE ] MAP").await,
            "[ [ 1/1 ] [ 3/1 2/1 ] ]"
        );
    }

    #[tokio::test]
    async fn a_mapped_singleton_stays_a_singleton_downstream() {
        // The failure this rules out was silent rather than loud: the element
        // read back as a scalar, so `5 ADD` answered `6/1` where `[ 6/1 ]` is
        // the answer and `[ 6 ] 6 EQ` is FALSE.
        assert_eq!(
            run("[ [ 1 ] [ 2 3 ] ] [ REVERSE ] MAP 0 GET 5 ADD").await,
            "[ 6/1 ]"
        );
    }

    #[tokio::test]
    async fn map_still_collects_scalar_results_as_scalars() {
        assert_eq!(run("[ 1 2 3 ] [ 2 MUL ] MAP").await, "[ 2/1 4/1 6/1 ]");
    }

    #[tokio::test]
    async fn map_by_word_name_follows_the_same_rule() {
        assert_eq!(
            run("[ 1 COLLECT ] 'WRAP' DEF [ 1 2 ] [ WRAP ] MAP").await,
            "[ [ 1/1 ] [ 2/1 ] ]"
        );
    }
}
