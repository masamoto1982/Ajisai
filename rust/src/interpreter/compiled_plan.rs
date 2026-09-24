use std::sync::Arc;

use crate::builtins::lookup_builtin_spec;
use crate::error::Result;
use crate::types::{Token, Value, WordDefinition};

use super::compiled_call::{execute_compiled_call, CompiledCall};
use super::{EpochSnapshot, Interpreter};

#[derive(Debug, Clone)]
pub struct CompiledPlan {
    pub line: CompiledLine,
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
    /// A literal whose source token was a *Word name* — `TRUE`, `FALSE`, `NIL`.
    ///
    /// Distinct from `PushLiteral` only in what it costs. The interpreted route
    /// reaches these three through the ordinary Symbol dispatch, because they are
    /// Core Words in the registry, so each costs one execution step; lowering
    /// them to a plain `PushLiteral` made them free, and a step the two routes
    /// disagree on is a budget and a ceiling the two routes disagree on.
    PushWordLiteral(Value, &'static str),
    /// A fully-literal vector (`[ 1 2 3 ]`, nested literals, `TRUE`/`FALSE`/`NIL`)
    /// built once at compile time, with the same promoted `Value`
    /// `collect_vector` would produce. Replaces the per-call vector walk
    /// and keeps lines with literal vectors on the compiled path instead of
    /// forcing them onto the interpreter via `FallbackToken`.
    PushVectorLiteral(Value),
    CallBuiltin(Arc<CompiledCall>),
    CallUserWord(String),
    CallQualifiedWord {
        namespace: String,
        word: String,
    },
    // FallbackToken keeps runtime-sensitive tokens in the interpreter path:
    // - directives / control markers (NilCoalesce)
    // - unresolved symbols at compile time
    // - structural tokens we cannot lower safely in current pass (e.g. vectors)
    FallbackToken(Token),
}

pub fn is_plan_valid(plan: &CompiledPlan, interp: &Interpreter) -> bool {
    plan.compiled_at.dictionary_epoch == interp.dictionary_epoch
}

fn compile_symbol(token: &Token, symbol: &str, interp: &Interpreter) -> CompiledOp {
    match symbol {
        "TRUE" => CompiledOp::PushWordLiteral(Value::from_bool(true), "TRUE"),
        "FALSE" => CompiledOp::PushWordLiteral(Value::from_bool(false), "FALSE"),
        "NIL" => CompiledOp::PushWordLiteral(Value::nil(), "NIL"),
        _ => {
            if lookup_builtin_spec(symbol).is_some() {
                CompiledOp::CallBuiltin(Arc::new(CompiledCall::resolve(symbol)))
            } else if let Some((resolved, _)) = interp.resolve_word_entry(symbol) {
                if let Some((namespace, word)) = resolved.split_once('@') {
                    CompiledOp::CallQualifiedWord {
                        namespace: namespace.to_string(),
                        word: word.to_string(),
                    }
                } else {
                    CompiledOp::CallUserWord(resolved.to_string())
                }
            } else {
                CompiledOp::FallbackToken(token.clone())
            }
        }
    }
}

/// Try to build a fully-literal vector starting at `tokens[start]` (a
/// `VectorStart`). Mirrors `Interpreter::collect_vector` for the literal subset
/// — same element values, nesting, and promotion — but returns
/// `None` the moment a non-literal element appears (a bare symbol that could be
/// a user word, a `|` separator, an unclosed/empty vector, excessive nesting),
/// so those keep the interpreter's `collect_vector` behavior via `FallbackToken`.
/// On success returns the element values and the tokens consumed (including
/// both brackets).
fn try_collect_literal_vector(
    tokens: &[Token],
    start: usize,
    depth: usize,
) -> Option<(Vec<Value>, usize)> {
    if !matches!(tokens.get(start), Some(Token::VectorStart)) {
        return None;
    }
    if depth > crate::interpreter::MAX_VECTOR_NESTING_DEPTH {
        return None;
    }

    let mut values: Vec<Value> = Vec::new();
    let mut i = start + 1;

    while i < tokens.len() {
        match &tokens[i] {
            Token::VectorStart => {
                // A nested empty vector returns `None` from the recursive call
                // above (the interpreter rejects it), so `nested` is non-empty.
                let (nested, consumed) = try_collect_literal_vector(tokens, i, depth + 1)?;
                values.push(Value::from_vector_promoted(nested));
                i += consumed;
            }
            Token::Value(value) => {
                values.push((**value).clone());
                i += 1;
            }
            Token::VectorEnd => {
                if values.is_empty() {
                    // The interpreter rejects `[ ]`; leave it as a fallback so
                    // that error is raised rather than silently building a NIL.
                    return None;
                }
                return Some((values, i - start + 1));
            }
            Token::Number(literal) => {
                values.push(Value::from_number(literal.value()?));
                i += 1;
            }
            Token::String(s) => {
                values.push(Value::from_string(s));
                i += 1;
            }
            Token::Symbol(s) => {
                match Interpreter::normalize_symbol(s).as_ref() {
                    "TRUE" => values.push(Value::from_bool(true)),
                    "FALSE" => values.push(Value::from_bool(false)),
                    "NIL" => values.push(Value::nil()),
                    // LANG.VALUES.VECTOR: a name inside a Vector literal
                    // denotes a Symbol — data until something executes it —
                    // never executed by appearing here. This mirrors
                    // `collect_bracketed_with_depth`, so a symbol-bearing
                    // vector is a literal and lowers here identically to the
                    // interpreter path.
                    _ => values.push(Value::from_symbol(s)),
                }
                i += 1;
            } // `[ IDLE | 1 ]`: `|` inside an unclosed `[` is data until COND
              // runs it, the same promotion an ordinary name gets — mirrors
              // `collect_bracketed_with_depth`'s handling exactly.
        }
    }
    None // unclosed
}

/// Compile one token sequence into a single `CompiledLine`.
fn compile_one_line(tokens: Vec<Token>, interp: &Interpreter) -> CompiledLine {
    let mut ops = Vec::with_capacity(tokens.len());
    let mut i = 0_usize;

    while i < tokens.len() {
        let token = &tokens[i];
        let op = match token {
            Token::Number(literal) => match literal.value() {
                Some(frac) => CompiledOp::PushLiteral(Value::from_number(frac)),
                None => CompiledOp::FallbackToken(token.clone()),
            },
            Token::String(s) => CompiledOp::PushLiteral(Value::from_string(s)),
            Token::VectorStart => match try_collect_literal_vector(&tokens, i, 1) {
                Some((values, consumed)) if interp.vector_literal_enabled => {
                    i += consumed - 1;
                    CompiledOp::PushVectorLiteral(Value::from_vector_promoted(values))
                }
                _ => CompiledOp::FallbackToken(token.clone()),
            },
            Token::VectorEnd => CompiledOp::FallbackToken(token.clone()),
            Token::Value(value) => CompiledOp::PushLiteral((**value).clone()),
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

/// Compile a block of tokens — a higher-order Word's code operand — into a
/// one-line plan.
///
/// `MAP`, `FILTER` and `FOLD` used to re-interpret their block's
/// tokens once per element, which means resolving every Symbol in it by name
/// every time: `[ SQRT ] MAP` over 20,000 lanes hashed the string `"SQRT"` and
/// probed the dictionary 20,000 times to reach the one Word it names. A block is
/// fixed for the length of the loop, so it is compiled before the loop instead,
/// and `CompiledOp::CallBuiltin` carries the `CompiledCall` that resolution
/// already produced.
///
/// Same lowering as a word body, including the `COND` dispatch pass, so the two
/// compiled routes cannot drift; and the same epoch snapshot, so
/// [`is_plan_valid`] refuses a plan whose dictionary has moved underneath it —
/// a block that runs `DEF` falls back to interpretation from that element on.
pub fn compile_token_block(tokens: Vec<Token>, interp: &Interpreter) -> CompiledPlan {
    CompiledPlan {
        line: compile_one_line(tokens, interp),
        compiled_at: interp.current_epoch_snapshot(),
    }
}

pub fn compile_word_definition(word_def: &WordDefinition, interp: &Interpreter) -> CompiledPlan {
    CompiledPlan {
        line: compile_one_line(word_def.body.to_vec(), interp),
        compiled_at: interp.current_epoch_snapshot(),
    }
}

/// `Interpreter::execute_nested_block` from a compiled plan instead of from
/// tokens.
///
/// The same transparent frame, for the same reason, around the same work:
/// `execute_compiled_line` falls back to `execute_section_core` on the
/// source tokens for any op it could not lower, which is exactly what the
/// interpreted route runs — so a block behaves the same whichever route it
/// took, which is what compiling one has to preserve
/// (LANG.AUTHORITY.FREEDOM).
pub(crate) fn execute_compiled_nested_block(
    interp: &mut Interpreter,
    plan: &CompiledPlan,
) -> Result<()> {
    interp.open_binding_scope(false);
    let result = execute_compiled_plan(interp, plan);
    interp.close_binding_scope();
    result
}

pub fn execute_compiled_plan(interp: &mut Interpreter, plan: &CompiledPlan) -> Result<()> {
    execute_compiled_line(interp, &plan.line)
}

fn execute_compiled_line(interp: &mut Interpreter, line: &CompiledLine) -> Result<()> {
    if line
        .ops
        .iter()
        .any(|op| matches!(op, CompiledOp::FallbackToken(_)))
    {
        // A line the compiler could not lower is re-interpreted from its
        // source tokens.
        return interp
            .execute_section_core(&line.source_tokens, 0)
            .map(|_| ());
    }

    for op in line.ops.iter() {
        match op {
            CompiledOp::PushLiteral(v) | CompiledOp::PushVectorLiteral(v) => {
                interp.stack.push(v.clone());
            }
            CompiledOp::PushWordLiteral(v, name) => {
                // A Word, so it costs a step, exactly as the Symbol dispatch the
                // interpreted route takes for it does — and a refusal by the
                // ceiling is that Word's failure, recorded like any other.
                let stack_len_before = interp.stack.len();
                if let Err(err) = interp.charge_execution_step() {
                    interp.record_word_dispatch_failure(name, &err, stack_len_before);
                    return Err(err);
                }
                interp.stack.push(v.clone());
            }
            CompiledOp::CallBuiltin(call) => {
                // The step and the call are one dispatch, so one failure record
                // covers both. Charging with `?` instead would let the ceiling's
                // own refusal escape unattributed — and the ceiling firing on a
                // Word *is* that Word failing, which is what the interpreted
                // route records when its charge, made inside the dispatch, fails.
                let stack_len_before = interp.stack.len();
                let mut outcome = interp.charge_execution_step();
                if outcome.is_ok() {
                    outcome = execute_compiled_call(interp, call);
                }
                if let Err(err) = outcome {
                    interp.record_word_dispatch_failure(&call.name, &err, stack_len_before);
                    return Err(err);
                }
            }
            CompiledOp::CallUserWord(name) => {
                interp.execute_word_core(name)?;
            }
            CompiledOp::CallQualifiedWord { namespace, word } => {
                let full_name = format!("{}@{}", namespace, word);
                interp.execute_word_core(&full_name)?;
            }
            CompiledOp::FallbackToken(_) => {}
        }
    }
    Ok(())
}

pub fn plan_is_all_fallback(plan: &CompiledPlan) -> bool {
    plan.line
        .ops
        .iter()
        .all(|op| matches!(op, CompiledOp::FallbackToken(_)))
}

pub fn arc_plan(plan: CompiledPlan) -> Arc<CompiledPlan> {
    Arc::new(plan)
}
