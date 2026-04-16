use std::sync::Arc;

use crate::builtins::lookup_builtin_spec;
use crate::error::Result;
use crate::types::{Token, Value, WordDefinition};

use super::{modules, ConsumptionMode, EpochSnapshot, Interpreter, OperationTargetMode};

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
    // FallbackToken is used for tokens that must preserve legacy runtime behavior:
    // - runtime-sensitive directives / control markers (Pipeline, NilCoalesce, CondClauseSep, SafeMode)
    // - unresolved symbols at compile time
    // - structural tokens we cannot lower safely in current pass (e.g. vectors)
    // - tokens that could alter semantic hint behavior in dynamic ways
    FallbackToken(Token),
}

pub fn is_plan_valid(plan: &CompiledPlan, interp: &Interpreter) -> bool {
    plan.compiled_at.dictionary_epoch == interp.dictionary_epoch
        && plan.compiled_at.module_epoch == interp.module_epoch
}

fn compile_symbol(token: &Token, symbol: &str, _interp: &Interpreter) -> CompiledOp {
    match symbol {
        "." => CompiledOp::SetTargetModeStackTop,
        ".." => CompiledOp::SetTargetModeStack,
        "," => CompiledOp::SetConsumptionConsume,
        ",," => CompiledOp::SetConsumptionKeep,
        "TRUE" => CompiledOp::PushLiteral(Value::from_bool(true)),
        "FALSE" => CompiledOp::PushLiteral(Value::from_bool(false)),
        "NIL" => CompiledOp::PushLiteral(Value::nil()),
        _ => {
            if lookup_builtin_spec(symbol).is_some() {
                CompiledOp::CallBuiltin(symbol.to_string())
            } else {
                CompiledOp::FallbackToken(token.clone())
            }
        }
    }
}

fn collect_code_block(tokens: &[Token], start: usize) -> Option<(Vec<Token>, usize)> {
    let mut depth = 1_i32;
    let mut i = start + 1;
    let mut block_tokens = Vec::new();

    while i < tokens.len() {
        match &tokens[i] {
            Token::BlockStart => {
                depth += 1;
                block_tokens.push(tokens[i].clone());
            }
            Token::BlockEnd => {
                depth -= 1;
                if depth == 0 {
                    return Some((block_tokens, i + 1));
                }
                block_tokens.push(tokens[i].clone());
            }
            t => block_tokens.push(t.clone()),
        }
        i += 1;
    }

    None
}

pub fn compile_word_definition(word_def: &WordDefinition, interp: &Interpreter) -> CompiledPlan {
    let mut lines = Vec::with_capacity(word_def.lines.len());

    for line in word_def.lines.iter() {
        let tokens = line.body_tokens.to_vec();
        let mut ops = Vec::with_capacity(tokens.len());
        let mut i = 0_usize;

        while i < tokens.len() {
            let token = &tokens[i];
            let op = match token {
                Token::Number(n) => match crate::types::fraction::Fraction::from_str(n) {
                    Ok(frac) => CompiledOp::PushLiteral(Value::from_number(frac)),
                    Err(_) => CompiledOp::FallbackToken(token.clone()),
                },
                Token::String(s) => CompiledOp::PushLiteral(Value::from_string(s)),
                Token::BlockStart => {
                    if let Some((block, next_i)) = collect_code_block(&tokens, i) {
                        i = next_i - 1;
                        CompiledOp::PushCodeBlock(block)
                    } else {
                        CompiledOp::FallbackToken(token.clone())
                    }
                }
                Token::BlockEnd => CompiledOp::FallbackToken(token.clone()),
                Token::VectorStart | Token::VectorEnd => CompiledOp::FallbackToken(token.clone()),
                Token::Pipeline | Token::NilCoalesce | Token::CondClauseSep | Token::SafeMode => {
                    CompiledOp::FallbackToken(token.clone())
                }
                Token::LineBreak => CompiledOp::LineBreak,
                Token::Symbol(s) => {
                    let upper = Interpreter::normalize_symbol(s);
                    compile_symbol(token, upper.as_ref(), interp)
                }
            };
            ops.push(op);
            i += 1;
        }

        lines.push(CompiledLine {
            ops,
            source_tokens: tokens,
        });
    }

    CompiledPlan {
        lines,
        compiled_at: interp.current_epoch_snapshot(),
    }
}

fn post_call_cleanup(interp: &mut Interpreter, name: &str) {
    interp
        .semantic_registry
        .normalize_to_stack_len(interp.stack.len());
    if !modules::is_mode_preserving_word(name) {
        interp.reset_execution_modes();
    }
}

pub fn execute_compiled_plan(interp: &mut Interpreter, plan: &CompiledPlan) -> Result<()> {
    for line in &plan.lines {
        execute_compiled_line(interp, line)?;
    }
    Ok(())
}

fn execute_compiled_line(interp: &mut Interpreter, line: &CompiledLine) -> Result<()> {
    if line
        .ops
        .iter()
        .any(|op| matches!(op, CompiledOp::FallbackToken(_)))
    {
        interp.execute_section_core(&line.source_tokens, 0)?;
        return Ok(());
    }

    for op in &line.ops {
        match op {
            CompiledOp::PushLiteral(v) => {
                interp.stack.push(v.clone());
                interp
                    .semantic_registry
                    .normalize_to_stack_len(interp.stack.len());
            }
            CompiledOp::PushCodeBlock(tokens) => {
                interp.stack.push(Value::from_code_block(tokens.clone()));
                interp
                    .semantic_registry
                    .normalize_to_stack_len(interp.stack.len());
            }
            CompiledOp::SetTargetModeStackTop => {
                interp.update_operation_target_mode(OperationTargetMode::StackTop)
            }
            CompiledOp::SetTargetModeStack => {
                interp.update_operation_target_mode(OperationTargetMode::Stack)
            }
            CompiledOp::SetConsumptionConsume => {
                interp.update_consumption_mode(ConsumptionMode::Consume)
            }
            CompiledOp::SetConsumptionKeep => interp.update_consumption_mode(ConsumptionMode::Keep),
            CompiledOp::CallBuiltin(name) => {
                interp.execute_builtin(name)?;
                post_call_cleanup(interp, name);
            }
            CompiledOp::CallUserWord(name) => {
                interp.execute_word_core(name)?;
                post_call_cleanup(interp, name);
            }
            CompiledOp::CallQualifiedWord { namespace, word } => {
                let full_name = format!("{}@{}", namespace, word);
                interp.execute_word_core(&full_name)?;
                post_call_cleanup(interp, &full_name);
            }
            CompiledOp::BeginGuardedBlock
            | CompiledOp::LineBreak
            | CompiledOp::FallbackToken(_) => {}
        }
    }
    Ok(())
}

pub fn plan_is_all_fallback(plan: &CompiledPlan) -> bool {
    plan.lines.iter().all(|l| {
        l.ops
            .iter()
            .all(|op| matches!(op, CompiledOp::FallbackToken(_) | CompiledOp::LineBreak))
    })
}

pub fn arc_plan(plan: CompiledPlan) -> Arc<CompiledPlan> {
    Arc::new(plan)
}
