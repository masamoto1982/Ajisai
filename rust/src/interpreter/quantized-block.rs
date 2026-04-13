use std::sync::Arc;

use crate::types::Token;

use super::{compile_word_definition, CompiledPlan, EpochSnapshot, Interpreter, WordDefinition};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantizedArity {
    Fixed(usize),
    Variable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantizedPurity {
    Pure,
    SideEffecting,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct QuantizedBlock {
    pub compiled_plan: Arc<CompiledPlan>,
    pub captured_epoch: EpochSnapshot,
    pub input_arity: QuantizedArity,
    pub output_arity: QuantizedArity,
    pub purity: QuantizedPurity,
    pub can_fuse: bool,
    pub can_short_circuit: bool,
    pub dependency_words: Vec<String>,
}

pub fn is_quantizable_block(tokens: &[Token]) -> bool {
    !tokens.is_empty() && !tokens.iter().any(|t| matches!(t, Token::LineBreak | Token::SafeMode))
}

pub fn quantize_code_block(tokens: &[Token], interp: &Interpreter) -> Option<QuantizedBlock> {
    if !is_quantizable_block(tokens) {
        return None;
    }
    let lines = vec![crate::types::ExecutionLine {
        body_tokens: tokens.to_vec().into(),
    }];
    let def = WordDefinition {
        lines: lines.into(),
        is_builtin: false,
        description: None,
        dependencies: Default::default(),
        original_source: None,
        namespace: None,
        registration_order: 0,
        compiled_plan: None,
    };
    let plan = Arc::new(compile_word_definition(&def, interp));
    Some(QuantizedBlock {
        compiled_plan: plan,
        captured_epoch: interp.current_epoch_snapshot(),
        input_arity: QuantizedArity::Variable,
        output_arity: QuantizedArity::Variable,
        purity: QuantizedPurity::Unknown,
        can_fuse: false,
        can_short_circuit: false,
        dependency_words: Vec::new(),
    })
}
