use crate::interpreter::{compile_word_definition, is_plan_valid, CompiledOp, Interpreter};
use crate::types::{ExecutionLine, Token, WordDefinition};
use std::collections::HashSet;
use std::sync::Arc;

fn test_word(tokens: Vec<Token>) -> WordDefinition {
    WordDefinition {
        lines: Arc::new([ExecutionLine {
            body_tokens: Arc::from(tokens),
        }]),
        is_builtin: false,
        description: None,
        dependencies: HashSet::new(),
        original_source: None,
        namespace: None,
        registration_order: 0,
        compiled_plan: None,
    }
}

#[test]
fn compiled_plan_invalidates_on_dictionary_epoch_change() {
    let mut interp = Interpreter::new();
    let wd = test_word(vec![Token::Number("1".into())]);
    let plan = compile_word_definition(&wd, &interp);
    assert!(is_plan_valid(&plan, &interp));
    interp.bump_dictionary_epoch();
    assert!(!is_plan_valid(&plan, &interp));
}

#[test]
fn compiled_plan_invalidates_on_module_epoch_change() {
    let mut interp = Interpreter::new();
    let wd = test_word(vec![Token::Number("1".into())]);
    let plan = compile_word_definition(&wd, &interp);
    assert!(is_plan_valid(&plan, &interp));
    interp.bump_module_epoch();
    assert!(!is_plan_valid(&plan, &interp));
}

#[test]
fn compile_collects_code_block_literal() {
    let interp = Interpreter::new();
    let wd = test_word(vec![
        Token::BlockStart,
        Token::Number("1".into()),
        Token::Symbol("+".into()),
        Token::BlockEnd,
    ]);
    let plan = compile_word_definition(&wd, &interp);
    assert!(matches!(plan.lines[0].ops[0], CompiledOp::PushCodeBlock(_)));
}
