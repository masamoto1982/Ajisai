use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::Value;

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
