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
        original_source: None,
        namespace: None,
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
    assert!(matches!(
        plan.line.ops[0],
        CompiledOp::PushVectorLiteral(_, _)
    ));
}
