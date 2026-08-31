use std::sync::Arc;

use crate::builtins::lookup_builtin_spec;
use crate::error::Result;
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Token, Value, WordDefinition};

use super::compiled_call::{execute_compiled_call, CompiledCall};
use super::{ConsumptionMode, EpochSnapshot, Interpreter};

/// Schema version of the `CompiledPlan` lowering. Bump whenever the set of
/// `CompiledOp` variants or their semantics change in a way that makes an
/// older-lowered plan unsafe to reuse. Part of the cross-reset artifact key so
/// a plan compiled by a different schema is never reused (Phase 5).
pub const COMPILED_PLAN_SCHEMA_VERSION: u32 = 1;

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
    SetConsumptionKeep,
    CallBuiltin(Arc<CompiledCall>),
    /// A `COND` whose guard/body clauses were split once at compile time. The
    /// preceding `PushVectorLiteral` op is kept (it still pushes the clauses
    /// vector so stack discipline and the dynamic fallback are preserved); this
    /// op dispatches on the precomputed table instead of re-collecting and
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
    // - directives / control markers (NilCoalesce, CondClauseSep)
    // - unresolved symbols at compile time
    // - structural tokens we cannot lower safely in current pass (e.g. vectors)
    // - tokens that could alter semantic hint behavior in dynamic ways
    FallbackToken(Token),
}

pub fn is_plan_valid(plan: &CompiledPlan, interp: &Interpreter) -> bool {
    plan.compiled_at.dictionary_epoch == interp.dictionary_epoch
}

fn compile_symbol(token: &Token, symbol: &str, interp: &Interpreter) -> CompiledOp {
    match symbol {
        "KEEP" => CompiledOp::SetConsumptionKeep,
        "TRUE" => CompiledOp::PushLiteral(Value::from_bool(true)),
        "FALSE" => CompiledOp::PushLiteral(Value::from_bool(false)),
        "NIL" => CompiledOp::PushLiteral(Value::nil()),
        _ => {
            if lookup_builtin_spec(symbol).is_some() {
                CompiledOp::CallBuiltin(Arc::new(CompiledCall::resolve(symbol)))
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
                    // LANG.VALUES.VECTOR: a name inside a Vector literal
                    // denotes a Symbol — data until something executes it —
                    // never executed by appearing here. This mirrors
                    // `collect_bracketed_with_depth`, so a symbol-bearing
                    // vector is a literal and lowers here identically to the
                    // interpreter path.
                    _ => {
                        values.push(Value::from_symbol(s));
                        has_other = true;
                    }
                }
                i += 1;
            }
            // `[ IDLE | 1 ]`: `|` inside an unclosed `[` is data until COND
            // runs it, the same promotion an ordinary name gets — mirrors
            // `collect_bracketed_with_depth`'s handling exactly, and is what
            // lets a `COND` clauses wrapper (now always `[ ]`-spelled) still
            // lower to a compile-time `PushVectorLiteral` so `lower_cond_
            // dispatch` can find it.
            Token::CondClauseSep => {
                values.push(Value::from_symbol("|"));
                has_other = true;
                i += 1;
            }
            Token::LineBreak | Token::NilCoalesce => {
                i += 1;
            }
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
            Token::VectorStart => match try_collect_literal_vector(&tokens, i, 1) {
                Some((values, consumed, hint)) if interp.vector_literal_enabled => {
                    i += consumed - 1;
                    CompiledOp::PushVectorLiteral(Value::from_vector_promoted(values), hint)
                }
                _ => CompiledOp::FallbackToken(token.clone()),
            },
            Token::VectorEnd => CompiledOp::FallbackToken(token.clone()),
            Token::NilCoalesce | Token::CondClauseSep => CompiledOp::FallbackToken(token.clone()),
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

/// Replace each `CallBuiltin("COND")` whose clauses operand is statically
/// known (the single preceding op is a literal `PushVectorLiteral` — `COND`'s
/// clauses are one fixed-position operand, the same convention
/// `MAP`/`FILTER`/`FOLD` use for their code operand) with a
/// `CondDispatch` carrying the split-once clause table. That preceding op is
/// left in place: it still pushes the wrapper value at runtime, so a compiled
/// and an interpreted run of the same source see the same stack traffic;
/// since the split is already known from compile time, `op_cond_dispatch`
/// pops it unread. A clauses operand that fails to split, or isn't a literal
/// at all, is left as the dynamic `COND` so its error still surfaces (or its
/// value is derived normally) at runtime.
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
        if !matches!(&lines[li].ops[oi], CompiledOp::CallBuiltin(c) if c.name == "COND") {
            continue;
        }
        if flat_idx == 0 {
            continue;
        }
        let (pli, poi) = positions[flat_idx - 1];
        let blocks: Option<Vec<Vec<Token>>> = match &lines[pli].ops[poi] {
            CompiledOp::PushVectorLiteral(value, _) => value
                .as_vector_view()
                .and_then(|elements| clause_blocks_from_values(&elements)),
            _ => None,
        };
        let Some(blocks) = blocks else { continue };
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

/// Bridge a literal clauses-vector's already-built elements back to tokens
/// (`value_as_code.rs`), the same conversion the dynamic path
/// (`control_cond::extract_clause_blocks`) applies at runtime.
fn clause_blocks_from_values(elements: &[Value]) -> Option<Vec<Vec<Token>>> {
    elements
        .iter()
        .map(|clause| {
            let inner = clause.as_vector_view()?;
            super::value_as_code::value_elements_to_tokens(&inner).ok()
        })
        .collect()
}

fn post_call_cleanup(interp: &mut Interpreter, _name: &str) {
    if true {
        interp.reset_execution_modes();
    }
}

pub fn execute_compiled_plan(interp: &mut Interpreter, plan: &CompiledPlan) -> Result<()> {
    for line in plan.lines.iter() {
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
        // A line the compiler could not lower is re-interpreted from its
        // source tokens.
        return interp.execute_section_core(&line.source_tokens, 0).map(|_| ());
    }

    for op in line.ops.iter() {
        match op {
            CompiledOp::PushLiteral(v) => {
                // The legacy path normalized the new slot's role to `Unassigned`
                // (it grew the value vector, then padded roles), so a compiled
                // literal is role-neutral regardless of the value's own hint.
                // Preserve that exactly.
                interp
                    .stack
                    .push_with_role(v.clone(), Interpretation::Unassigned);
            }
            CompiledOp::PushVectorLiteral(v, hint) => {
                // Match `execute_section_core`'s VectorStart handling exactly:
                // push the prebuilt vector and its element hint.
                interp.stack.push_with_role(v.clone(), *hint);
            }
            CompiledOp::SetConsumptionKeep => interp.update_consumption_mode(ConsumptionMode::Keep),
            CompiledOp::CallBuiltin(call) => {
                execute_compiled_call(interp, call)?;
                // Mirror the interpreted loop: retag the top role from the
                // word-hint table so the compiled route leaves the same
                // `(value, role)` observation (SPEC §12).
                super::execution_loop::apply_word_hint_override(interp, &call.name);
                // `post_call_cleanup` with the mode-preservation answer
                // precomputed at compile time (no per-call uppercase scan).
                if !call.mode_preserving {
                    interp.reset_execution_modes();
                }
            }
            CompiledOp::CondDispatch(clauses) => {
                super::control_cond::op_cond_dispatch(interp, clauses)?;
                super::execution_loop::apply_word_hint_override(interp, "COND");
                post_call_cleanup(interp, "COND");
            }
            CompiledOp::CallUserWord(name) => {
                interp.execute_word_core(name)?;
                super::execution_loop::apply_word_hint_override(interp, name);
                post_call_cleanup(interp, name);
            }
            CompiledOp::CallQualifiedWord { namespace, word } => {
                let full_name = format!("{}@{}", namespace, word);
                interp.execute_word_core(&full_name)?;
                super::execution_loop::apply_word_hint_override(interp, &full_name);
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
