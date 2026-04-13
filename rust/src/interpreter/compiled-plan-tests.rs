use crate::interpreter::{compile_word_definition, is_plan_valid, Interpreter};
use crate::types::{ExecutionLine, Token, WordDefinition};
use std::collections::HashSet;
use std::sync::Arc;

#[test]
fn compiled_plan_invalidates_on_dictionary_epoch_change() {
    let mut interp = Interpreter::new();
    let wd = WordDefinition {
        lines: Arc::new([ExecutionLine { body_tokens: Arc::new([Token::Number("1".into())]) }]),
        is_builtin: false,
        description: None,
        dependencies: HashSet::new(),
        original_source: None,
        namespace: None,
        registration_order: 0,
        compiled_plan: None,
    };
    let plan = compile_word_definition(&wd, &interp);
    assert!(is_plan_valid(&plan, &interp));
    interp.bump_dictionary_epoch();
    assert!(!is_plan_valid(&plan, &interp));
}
