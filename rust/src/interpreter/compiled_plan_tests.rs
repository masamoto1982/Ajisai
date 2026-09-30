//! Test suite for `crate::interpreter::compiled_plan`.

use crate::interpreter::{compile_word_definition, is_plan_valid, CompiledOp, Interpreter};
use crate::types::{Token, WordDefinition};
use std::collections::HashSet;
use std::sync::Arc;

fn test_word(tokens: Vec<Token>) -> WordDefinition {
    WordDefinition {
        body: Arc::from(tokens),
        is_builtin: false,
        description: None,
        dependencies: HashSet::new(),
        text_references: HashSet::new(),
        registration_order: 0,
        compiled_plan: None,
        generated: None,
    }
}

#[test]
fn compiled_plan_invalidates_on_dictionary_epoch_change() {
    let mut interp = Interpreter::new();
    let wd = test_word(vec![Token::number("1")]);
    let plan = compile_word_definition(&wd, &interp);
    assert!(is_plan_valid(&plan, &interp));
    interp.bump_dictionary_epoch();
    assert!(!is_plan_valid(&plan, &interp));
}
#[test]
fn compile_collects_vector_literal() {
    let interp = Interpreter::new();
    let wd = test_word(vec![
        Token::VectorStart,
        Token::number("1"),
        Token::Symbol("+".into()),
        Token::VectorEnd,
    ]);
    let plan = compile_word_definition(&wd, &interp);
    assert!(matches!(plan.line.ops[0], CompiledOp::PushVectorLiteral(_)));
}

/// A body the compiler can lower none of still gets a plan — one that runs
/// its source through the interpreter, as a body with no plan would. It used
/// to be declined instead, and since nothing remembered the refusal the body
/// was recompiled, and its definition copied, on every call.
#[tokio::test]
async fn a_body_the_compiler_cannot_lower_is_compiled_once() {
    let mut interp = Interpreter::new();
    // `[ LATER ]` names nothing yet, so the quotation is not a literal the
    // compiler can build, and neither is anything else in the body.
    interp
        .execute("[ [ LATER ] ] 'QUOTE' DEF")
        .await
        .expect("must define");
    let before = interp.runtime_metrics();
    interp.execute("QUOTE QUOTE QUOTE").await.expect("must run");
    let after = interp.runtime_metrics();
    assert_eq!(
        after.compiled_plan_build_count - before.compiled_plan_build_count,
        1
    );
    assert_eq!(
        after.compiled_plan_cache_hit_count - before.compiled_plan_cache_hit_count,
        2
    );
}
