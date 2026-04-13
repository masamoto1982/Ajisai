use std::sync::Arc;

use crate::builtins::lookup_builtin_spec;
use crate::error::Result;
use crate::types::{Token, Value, WordDefinition};

use super::{ConsumptionMode, EpochSnapshot, Interpreter, OperationTargetMode};

#[derive(Debug, Clone)]
pub struct CompiledPlan {
    pub lines: Vec<CompiledLine>,
    pub compiled_at: EpochSnapshot,
}

#[derive(Debug, Clone)]
pub struct CompiledLine {
    pub ops: Vec<CompiledOp>,
    pub source_tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub enum CompiledOp {
    PushLiteral(Value),
    PushCodeBlock(Vec<Token>),
    SetTargetModeStackTop,
    SetTargetModeStack,
    SetConsumptionConsume,
    SetConsumptionKeep,
    CallBuiltin(String),
    CallUserWord(String),
    CallQualifiedWord { namespace: String, word: String },
    BeginGuardedBlock,
    LineBreak,
    // Fallback token for unresolved/dynamic tokens that must preserve legacy execution path.
    FallbackToken(Token),
}

pub fn is_plan_valid(plan: &CompiledPlan, interp: &Interpreter) -> bool {
    plan.compiled_at.dictionary_epoch == interp.dictionary_epoch
        && plan.compiled_at.module_epoch == interp.module_epoch
}

pub fn compile_word_definition(word_def: &WordDefinition, interp: &Interpreter) -> CompiledPlan {
    let mut lines = Vec::with_capacity(word_def.lines.len());
    for line in word_def.lines.iter() {
        let mut ops = Vec::with_capacity(line.body_tokens.len());
        for token in line.body_tokens.iter() {
            let op = match token {
                Token::Number(n) => match crate::types::fraction::Fraction::from_str(n) {
                    Ok(frac) => CompiledOp::PushLiteral(Value::from_number(frac)),
                    Err(_) => CompiledOp::FallbackToken(token.clone()),
                },
                Token::String(s) => CompiledOp::PushLiteral(Value::from_string(s)),
                Token::BlockStart | Token::BlockEnd | Token::VectorStart | Token::VectorEnd => {
                    // structural tokens require parser context
                    CompiledOp::FallbackToken(token.clone())
                }
                Token::Pipeline | Token::NilCoalesce | Token::CondClauseSep | Token::SafeMode => {
                    CompiledOp::FallbackToken(token.clone())
                }
                Token::LineBreak => CompiledOp::LineBreak,
                Token::Symbol(s) => match s.as_ref() {
                    "." => CompiledOp::SetTargetModeStackTop,
                    ".." => CompiledOp::SetTargetModeStack,
                    "," => CompiledOp::SetConsumptionConsume,
                    ",," => CompiledOp::SetConsumptionKeep,
                    _ => {
                        let upper = Interpreter::normalize_symbol(s);
                        if lookup_builtin_spec(upper.as_ref()).is_some() {
                            CompiledOp::CallBuiltin(upper.into_owned())
                        } else if let Some((resolved, _)) = interp.resolve_word_entry(upper.as_ref()) {
                            if let Some((ns, word)) = resolved.split_once('@') {
                                CompiledOp::CallQualifiedWord {
                                    namespace: ns.to_string(),
                                    word: word.to_string(),
                                }
                            } else {
                                CompiledOp::CallUserWord(resolved)
                            }
                        } else {
                            CompiledOp::FallbackToken(token.clone())
                        }
                    }
                },
            };
            ops.push(op);
        }
        lines.push(CompiledLine { ops, source_tokens: line.body_tokens.to_vec() });
    }

    CompiledPlan {
        lines,
        compiled_at: interp.current_epoch_snapshot(),
    }
}

pub fn execute_compiled_plan(interp: &mut Interpreter, plan: &CompiledPlan) -> Result<()> {
    for line in &plan.lines {
        execute_compiled_line(interp, line)?;
    }
    Ok(())
}

fn execute_compiled_line(interp: &mut Interpreter, line: &CompiledLine) -> Result<()> {
    if line.ops.iter().any(|op| matches!(op, CompiledOp::FallbackToken(_))) {
        interp.execute_section_core(&line.source_tokens, 0)?;
        return Ok(());
    }
    for op in &line.ops {
        match op {
            CompiledOp::PushLiteral(v) => {
                interp.stack.push(v.clone());
                interp.semantic_registry.normalize_to_stack_len(interp.stack.len());
            }
            CompiledOp::PushCodeBlock(tokens) => interp.stack.push(Value::from_code_block(tokens.clone())),
            CompiledOp::SetTargetModeStackTop => {
                interp.update_operation_target_mode(OperationTargetMode::StackTop)
            }
            CompiledOp::SetTargetModeStack => {
                interp.update_operation_target_mode(OperationTargetMode::Stack)
            }
            CompiledOp::SetConsumptionConsume => interp.update_consumption_mode(ConsumptionMode::Consume),
            CompiledOp::SetConsumptionKeep => interp.update_consumption_mode(ConsumptionMode::Keep),
            CompiledOp::CallBuiltin(name) => interp.execute_builtin(name)?,
            CompiledOp::CallUserWord(name) => interp.execute_word_core(name)?,
            CompiledOp::CallQualifiedWord { namespace, word } => {
                interp.execute_word_core(&format!("{}@{}", namespace, word))?
            }
            CompiledOp::BeginGuardedBlock | CompiledOp::LineBreak => {}
            CompiledOp::FallbackToken(_) => {}
        }
    }
    Ok(())
}

pub fn plan_is_all_fallback(plan: &CompiledPlan) -> bool {
    plan.lines
        .iter()
        .all(|l| l.ops.iter().all(|op| matches!(op, CompiledOp::FallbackToken(_) | CompiledOp::LineBreak)))
}

pub fn arc_plan(plan: CompiledPlan) -> Arc<CompiledPlan> {
    Arc::new(plan)
}
