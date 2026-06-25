use std::sync::Arc;

use crate::builtins::lookup_builtin_spec;
use crate::error::Result;
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Token, Value, WordDefinition};

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
    /// A fully-literal vector (`[ 1 2 3 ]`, nested literals, `TRUE`/`FALSE`/`NIL`)
    /// built once at compile time, with the same promoted `Value` and element
    /// hint `collect_vector` would produce. Replaces the per-call vector walk
    /// and keeps lines with literal vectors on the compiled path instead of
    /// forcing them onto the interpreter via `FallbackToken`.
    PushVectorLiteral(Value, Interpretation),
    PushCodeBlock(Vec<Token>),
    SetTargetModeStackTop,
    SetTargetModeStack,
    SetConsumptionConsume,
    SetConsumptionKeep,
    CallBuiltin(String),
    /// A `COND` whose guard/body clauses were split once at compile time. The
    /// preceding `PushCodeBlock` ops are kept (they still push the clause blocks
    /// so stack discipline and the dynamic fallback are preserved); this op
    /// dispatches on the precomputed table instead of re-collecting and
    /// re-splitting those blocks every call. Internal-GOTO "jump table".
    CondDispatch(Arc<[super::control_cond::CondClause]>),
    CallUserWord(String),
    CallQualifiedWord {
        namespace: String,
        word: String,
    },
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

/// Try to build a fully-literal vector starting at `tokens[start]` (a
/// `VectorStart`). Mirrors `Interpreter::collect_vector` for the literal subset
/// — same element values, nesting, promotion, and element hint — but returns
/// `None` the moment a non-literal element appears (a bare symbol that could be
/// a user word, a `|` separator, an unclosed/empty vector, excessive nesting),
/// so those keep the interpreter's `collect_vector` behavior via `FallbackToken`.
/// On success returns the element values, tokens consumed (including both
/// brackets), and the element hint to attach on the stack.
fn try_collect_literal_vector(
    tokens: &[Token],
    start: usize,
    depth: usize,
) -> Option<(Vec<Value>, usize, Interpretation)> {
    if !matches!(tokens.get(start), Some(Token::VectorStart)) {
        return None;
    }
    if depth > crate::interpreter::MAX_VECTOR_NESTING_DEPTH {
        return None;
    }

    let mut values: Vec<Value> = Vec::new();
    let mut i = start + 1;
    let mut has_bool = false;
    let mut has_number = false;
    let mut has_other = false;

    while i < tokens.len() {
        match &tokens[i] {
            Token::VectorStart => {
                // A nested empty vector returns `None` from the recursive call
                // above (the interpreter rejects it), so `nested` is non-empty.
                let (nested, consumed, nested_hint) =
                    try_collect_literal_vector(tokens, i, depth + 1)?;
                values.push(Value::from_vector_promoted_with_hint(nested, nested_hint));
                has_other = true;
                i += consumed;
            }
            Token::VectorEnd => {
                if values.is_empty() {
                    // The interpreter rejects `[ ]`; leave it as a fallback so
                    // that error is raised rather than silently building a NIL.
                    return None;
                }
                let element_hint = if has_other {
                    Interpretation::Unassigned
                } else if has_bool && !has_number {
                    Interpretation::TruthValue
                } else if has_number && !has_bool {
                    Interpretation::RawNumber
                } else {
                    Interpretation::Unassigned
                };
                return Some((values, i - start + 1, element_hint));
            }
            Token::Number(n) => {
                values.push(Value::from_number(Fraction::from_str(n).ok()?));
                has_number = true;
                i += 1;
            }
            Token::String(s) => {
                values.push(Value::from_string(s));
                has_other = true;
                i += 1;
            }
            Token::Symbol(s) => {
                match Interpreter::normalize_symbol(s).as_ref() {
                    "TRUE" => {
                        values.push(Value::from_bool(true));
                        has_bool = true;
                    }
                    "FALSE" => {
                        values.push(Value::from_bool(false));
                        has_bool = true;
                    }
                    "NIL" => {
                        values.push(Value::nil());
                        has_other = true;
                    }
                    // Any other symbol may resolve to a user word that
                    // `collect_vector` would execute; not a literal.
                    _ => return None,
                }
                i += 1;
            }
            Token::LineBreak => {
                i += 1;
            }
            _ => return None,
        }
    }
    None // unclosed
}

/// Compile one token sequence into a single `CompiledLine`. `collect_vector`'s
/// flat treatment of a section is preserved: internal `LineBreak`s become no-op
/// `LineBreak` ops rather than line splits.
fn compile_one_line(tokens: Vec<Token>, interp: &Interpreter) -> CompiledLine {
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
            Token::VectorStart => match try_collect_literal_vector(&tokens, i, 1) {
                Some((values, consumed, hint)) if interp.vector_literal_enabled => {
                    i += consumed - 1;
                    CompiledOp::PushVectorLiteral(Value::from_vector_promoted(values), hint)
                }
                _ => CompiledOp::FallbackToken(token.clone()),
            },
            Token::VectorEnd => CompiledOp::FallbackToken(token.clone()),
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

    CompiledLine {
        ops,
        source_tokens: tokens,
    }
}

pub fn compile_word_definition(word_def: &WordDefinition, interp: &Interpreter) -> CompiledPlan {
    let mut lines = Vec::with_capacity(word_def.lines.len());
    for line in word_def.lines.iter() {
        lines.push(compile_one_line(line.body_tokens.to_vec(), interp));
    }

    if interp.cond_dispatch_enabled {
        lower_cond_dispatch(&mut lines, interp);
    }

    CompiledPlan {
        lines,
        compiled_at: interp.current_epoch_snapshot(),
    }
}

/// Compile a COND guard or body token slice into a sub-plan. A section is run
/// flat (a single line, matching `execute_section_core`), then lowered so nested
/// `COND`s and literal vectors inside it are compiled too. Returns `None` when
/// the section did not compile to anything beyond fallbacks — there the
/// interpreter path is kept, with no behavior change and no wasted dispatch.
fn compile_clause_plan(tokens: &[Token], interp: &Interpreter) -> Option<Arc<CompiledPlan>> {
    let mut lines = vec![compile_one_line(tokens.to_vec(), interp)];
    if interp.cond_dispatch_enabled {
        lower_cond_dispatch(&mut lines, interp);
    }
    let plan = CompiledPlan {
        lines,
        compiled_at: interp.current_epoch_snapshot(),
    };
    if plan_is_all_fallback(&plan) {
        None
    } else {
        Some(Arc::new(plan))
    }
}

fn is_cond_tail_op(op: &CompiledOp) -> bool {
    matches!(op, CompiledOp::CondDispatch(_))
        || matches!(op, CompiledOp::CallBuiltin(n) if n == "COND")
}

/// Whether `op` is a call to the word currently being trampolined
/// (`tail_self_word`). Mirrors the deferral check in `execute_section_core`'s
/// interpreter path so a compiled clause body trampolines identically.
fn is_self_tail_call(interp: &Interpreter, op: &CompiledOp) -> bool {
    let Some(target) = interp.tail_self_word.as_deref() else {
        return false;
    };
    match op {
        CompiledOp::CallUserWord(name) => name == target,
        CompiledOp::CallQualifiedWord { namespace, word } => {
            // `target` is the resolved `namespace@word` form.
            target.len() == namespace.len() + 1 + word.len()
                && target.as_bytes().get(namespace.len()) == Some(&b'@')
                && target.starts_with(namespace.as_str())
                && target.ends_with(word.as_str())
        }
        _ => false,
    }
}

/// Replace each `CallBuiltin("COND")` whose clause blocks are statically known
/// (a contiguous run of preceding `PushCodeBlock` ops, possibly spanning line
/// breaks) with a `CondDispatch` carrying the split-once clause table. The
/// `PushCodeBlock` ops are left in place: they still push the blocks at runtime,
/// so `op_cond_dispatch` can count them and fall back to the dynamic split if an
/// unexpected block reached the stack. A clause set that fails to split is left
/// as the dynamic `COND` so its error still surfaces at runtime.
///
/// When `compiled_clause_enabled`, each clause's guard and body are also
/// compiled into sub-plans so they run compiled rather than re-interpreted.
fn lower_cond_dispatch(lines: &mut [CompiledLine], interp: &Interpreter) {
    let positions: Vec<(usize, usize)> = lines
        .iter()
        .enumerate()
        .flat_map(|(li, l)| (0..l.ops.len()).map(move |oi| (li, oi)))
        .collect();

    type Replacement = ((usize, usize), Arc<[super::control_cond::CondClause]>);
    let mut replacements: Vec<Replacement> = Vec::new();
    for (flat_idx, &(li, oi)) in positions.iter().enumerate() {
        if !matches!(&lines[li].ops[oi], CompiledOp::CallBuiltin(n) if n == "COND") {
            continue;
        }
        let mut blocks: Vec<Vec<Token>> = Vec::new();
        let mut k = flat_idx;
        while k > 0 {
            k -= 1;
            let (pli, poi) = positions[k];
            match &lines[pli].ops[poi] {
                CompiledOp::PushCodeBlock(b) => blocks.push(b.clone()),
                _ => break,
            }
        }
        if blocks.is_empty() {
            continue;
        }
        blocks.reverse();
        if let Ok(mut clauses) = super::control_cond::split_clause_blocks(blocks) {
            if interp.compiled_clause_enabled {
                for clause in &mut clauses {
                    clause.guard_plan = compile_clause_plan(&clause.guard, interp);
                    clause.body_plan = compile_clause_plan(&clause.body, interp);
                }
            }
            replacements.push(((li, oi), Arc::from(clauses)));
        }
    }

    for ((li, oi), clauses) in replacements {
        lines[li].ops[oi] = CompiledOp::CondDispatch(clauses);
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
    // a `COND` (dynamic or precompiled), propagate tail context into the
    // selected clause body so a guarded tail self-call there is eliminated (the
    // "internal GOTO"). The COND op consumes `in_tail_context` on entry, so
    // setting it here needs no explicit restore.
    let tail_op = if is_tail_line && interp.tail_call_enabled && interp.tail_self_word.is_some() {
        last_effective_op(&line.ops)
    } else {
        None
    };

    for (op_idx, op) in line.ops.iter().enumerate() {
        if tail_op == Some(op_idx) {
            if is_cond_tail_op(op) {
                interp.in_tail_context = true;
            } else if interp.in_tail_context && is_self_tail_call(interp, op) {
                // A guarded tail self-call reached as a compiled op (e.g. a COND
                // clause body run compiled). Defer to the trampoline instead of
                // recursing: leave the computed arguments on the stack and raise
                // `tail_jump_pending`. `in_tail_context` is true only inside a
                // tail COND clause body, so the word's own body plan (run with
                // it false) keeps native recursion and its depth-limit error.
                interp.tail_jump_pending = true;
                interp.in_tail_context = false;
                continue;
            }
        }
        match op {
            CompiledOp::PushLiteral(v) => {
                interp.stack.push(v.clone());
                interp
                    .semantic_registry
                    .normalize_to_stack_len(interp.stack.len());
            }
            CompiledOp::PushVectorLiteral(v, hint) => {
                // Match `execute_section_core`'s VectorStart handling exactly:
                // push the prebuilt vector and its element hint.
                interp.stack.push(v.clone());
                interp.semantic_registry.push_hint(*hint);
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
            CompiledOp::CondDispatch(clauses) => {
                super::control_cond::op_cond_dispatch(interp, clauses)?;
                post_call_cleanup(interp, "COND");
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
