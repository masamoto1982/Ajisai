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
    // FallbackToken keeps runtime-sensitive tokens in the interpreter path:
    // - directives / control markers (Pipeline, NilCoalesce, CondClauseSep)
    // - unresolved symbols at compile time
    // - structural tokens we cannot lower safely in current pass (e.g. vectors)
    // - tokens that could alter semantic hint behavior in dynamic ways
    FallbackToken(Token),
}

pub fn is_plan_valid(plan: &CompiledPlan, interp: &Interpreter) -> bool {
    plan.compiled_at.dictionary_epoch == interp.dictionary_epoch
        && plan.compiled_at.module_epoch == interp.module_epoch
}

fn compile_symbol(token: &Token, symbol: &str, interp: &Interpreter) -> CompiledOp {
    match symbol {
        "TOP" => CompiledOp::SetTargetModeStackTop,
        "STAK" => CompiledOp::SetTargetModeStack,
        "EAT" => CompiledOp::SetConsumptionConsume,
        "KEEP" => CompiledOp::SetConsumptionKeep,
        "TRUE" => CompiledOp::PushLiteral(Value::from_bool(true)),
        "FALSE" => CompiledOp::PushLiteral(Value::from_bool(false)),
        "NIL" => CompiledOp::PushLiteral(Value::nil()),
        _ => {
            if lookup_builtin_spec(symbol).is_some() {
                CompiledOp::CallBuiltin(symbol.to_string())
            } else if let Some((resolved, _)) = interp.resolve_word_entry_readonly(symbol) {
                if let Some((namespace, word)) = resolved.split_once('@') {
                    CompiledOp::CallQualifiedWord {
                        namespace: namespace.to_string(),
                        word: word.to_string(),
                    }
                } else {
                    CompiledOp::CallUserWord(resolved)
                }
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
                Token::Pipeline | Token::NilCoalesce | Token::CondClauseSep => {
                    CompiledOp::FallbackToken(token.clone())
                }
                Token::LineBreak => CompiledOp::LineBreak,
                Token::Symbol(s) => {
                    let upper = crate::core_word_aliases::canonicalize_core_word_name(s);
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
    let last_line = plan.lines.len().saturating_sub(1);
    for (idx, line) in plan.lines.iter().enumerate() {
        // The last line of a word body holds its tail position. Only there can
        // a tail self-call be eliminated into an internal backward jump.
        let is_tail_line = idx == last_line;
        execute_compiled_line(interp, line, is_tail_line)?;
    }
    Ok(())
}

/// Index of the last op in `ops` that actually executes (skipping no-op
/// markers like `LineBreak`), or `None` when the line is effectively empty.
fn last_effective_op(ops: &[CompiledOp]) -> Option<usize> {
    ops.iter()
        .rposition(|op| !matches!(op, CompiledOp::LineBreak | CompiledOp::BeginGuardedBlock))
}

fn execute_compiled_line(
    interp: &mut Interpreter,
    line: &CompiledLine,
    is_tail_line: bool,
) -> Result<()> {
    if line
        .ops
        .iter()
        .any(|op| matches!(op, CompiledOp::FallbackToken(_)))
    {
        interp.execute_section_core(&line.source_tokens, 0)?;
        return Ok(());
    }

    // The tail op of the tail line carries the word's tail position. When it is
    // a `COND`, propagate tail context into the selected clause body so a
    // guarded tail self-call there is eliminated (the "internal GOTO").
    let tail_op = if is_tail_line && interp.tail_call_enabled && interp.tail_self_word.is_some() {
        last_effective_op(&line.ops)
    } else {
        None
    };

    for (op_idx, op) in line.ops.iter().enumerate() {
        if tail_op == Some(op_idx) {
            if let CompiledOp::CallBuiltin(name) = op {
                if name == "COND" {
                    let prev = interp.in_tail_context;
                    interp.in_tail_context = true;
                    let r = interp.execute_builtin(name);
                    interp.in_tail_context = prev;
                    r?;
                    post_call_cleanup(interp, name);
                    continue;
                }
            }
        }
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
