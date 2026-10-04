use std::sync::Arc;

use crate::builtins::lookup_builtin_spec;
use crate::error::Result;
use crate::kernel::generated::{generated_word, GeneratedWord};
use crate::types::{Token, Value, WordDefinition};

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
    /// The runs of `ops` lowered to typed segments (`segment`).
    pub(crate) segments: Vec<super::segment::LineSegment>,
    /// Whether the line is re-interpreted from `source_tokens` instead.
    pub(crate) reinterpret: bool,
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
                CompiledOp::CallUserWord(resolved.to_string())
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
/// a user word, a `|` separator, an unclosed vector, excessive nesting),
/// so those keep the interpreter's `collect_vector` behavior via `FallbackToken`.
/// On success returns the element values and the tokens consumed (including
/// both brackets).
fn try_collect_literal_vector(
    tokens: &[Token],
    start: usize,
    depth: usize,
    max_depth: usize,
) -> Option<(Vec<Value>, usize)> {
    if !matches!(tokens.get(start), Some(Token::VectorStart)) {
        return None;
    }
    if depth > max_depth {
        return None;
    }

    let mut values: Vec<Value> = Vec::new();
    let mut i = start + 1;

    while i < tokens.len() {
        match &tokens[i] {
            Token::VectorStart => {
                let (nested, consumed) =
                    try_collect_literal_vector(tokens, i, depth + 1, max_depth)?;
                values.push(Value::from_vector_promoted(nested));
                i += consumed;
            }
            Token::Value(value) => {
                values.push((**value).clone());
                i += 1;
            }
            // `[ ]` included: the empty Vector is a value on the interpreter
            // path too, built by the same promotion, so both routes lower it
            // alike. It was left as a fallback from when the interpreter
            // rejected it, which only sent a body holding one off the
            // compiled route.
            Token::VectorEnd => return Some((values, i - start + 1)),
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

/// Lower one token sequence to compiled ops.
pub(crate) fn lower_tokens_to_ops(tokens: &[Token], interp: &Interpreter) -> Vec<CompiledOp> {
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
            Token::VectorStart => match try_collect_literal_vector(
                tokens,
                i,
                1,
                interp.runtime_limits.max_nesting_depth,
            ) {
                Some((values, consumed)) if interp.vector_literal_enabled => {
                    i += consumed - 1;
                    CompiledOp::PushVectorLiteral(Value::from_vector_promoted(values))
                }
                _ => CompiledOp::FallbackToken(token.clone()),
            },
            Token::VectorEnd => CompiledOp::FallbackToken(token.clone()),
            Token::Value(value) => CompiledOp::PushLiteral((**value).clone()),
            Token::Symbol(s) => {
                let upper = crate::word_name::canonical_word_name(s);
                compile_symbol(token, upper.as_ref(), interp)
            }
        };
        ops.push(op);
        i += 1;
    }
    ops
}

/// Whether a line must be re-interpreted from its source tokens.
///
/// A name the compiler could not resolve is a binding, or a Word the line
/// itself defines, and is dispatched by name where it stands, exactly as
/// the token walk dispatches it. Every other fallback — a malformed number,
/// a vector the compiler could not build — sends the whole line back to the
/// token walk, which reports it; and so does a line that runs `DEF` beside an
/// unresolved name, since the token walk hands `DEF` the body tokens a
/// literal before it was written as (`pending_def_body_tokens`), which a
/// compiled line, lacking the source of its prebuilt literals, does not.
///
/// Dispatching a name in place is part of the segment route: with segments
/// off (`AJISAI_NO_SEGMENTS`), every fallback re-interprets the line, as it
/// did before either existed.
fn must_reinterpret(ops: &[CompiledOp], interp: &Interpreter) -> bool {
    if !interp.segments_enabled {
        return ops
            .iter()
            .any(|op| matches!(op, CompiledOp::FallbackToken(_)));
    }
    let mut names = false;
    let mut defines = false;
    for op in ops {
        match op {
            CompiledOp::FallbackToken(Token::Symbol(_)) => names = true,
            CompiledOp::FallbackToken(_) => return true,
            CompiledOp::CallBuiltin(call) if call.name == "DEF" => defines = true,
            _ => {}
        }
    }
    names && defines
}

/// Compile one token sequence into a single `CompiledLine`.
fn compile_one_line(tokens: Vec<Token>, interp: &Interpreter) -> CompiledLine {
    let ops = lower_tokens_to_ops(&tokens, interp);
    let reinterpret = must_reinterpret(&ops, interp);
    let segments = if reinterpret {
        Vec::new()
    } else {
        super::segment_lower::segment_line(&ops, interp)
    };
    CompiledLine {
        ops,
        source_tokens: tokens,
        segments,
        reinterpret,
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
    if line.reinterpret {
        return interp
            .execute_section_core(&line.source_tokens, 0)
            .map(|_| ());
    }

    let mut segments = line.segments.iter().peekable();
    let mut i = 0;
    while i < line.ops.len() {
        if let Some(segment) = segments.next_if(|segment| segment.start == i) {
            if segment.code.try_run(interp) {
                i = segment.end;
                continue;
            }
        }
        let op = &line.ops[i];
        i += 1;
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
                if let Some(word) = call.word {
                    if super::quickened::try_scalar_call(interp, word.id) {
                        continue;
                    }
                }
                // The step and the call are one dispatch, so one failure record
                // covers both. Charging with `?` instead would let the ceiling's
                // own refusal escape unattributed — and the ceiling firing on a
                // Word *is* that Word failing, which is what the interpreted
                // route records when its charge, made inside the dispatch, fails.
                let witness = interp.begin_dispatch();
                let mut outcome = interp.charge_execution_step();
                if outcome.is_ok() {
                    outcome = execute_compiled_call(interp, call)
                        .and_then(|()| interp.check_fresh_nesting());
                }
                match outcome {
                    Ok(()) => interp.trace_nil_outcome(&call.name, &witness),
                    Err(err) => {
                        interp.record_word_dispatch_failure(
                            &call.name,
                            &err,
                            witness.stack_len_before,
                        );
                        return Err(err);
                    }
                }
            }
            // A User Word call is a dispatch like the Symbol dispatch the
            // interpreted route makes for it, and owes the same records: the
            // NIL it answered, or the frame it encloses when the failure or the
            // NIL is a Word's inside its body.
            CompiledOp::CallUserWord(name) => {
                let witness = interp.begin_dispatch();
                match interp.execute_word_core(name) {
                    Ok(()) => interp.trace_nil_outcome(name, &witness),
                    Err(err) => {
                        interp.record_word_dispatch_failure(name, &err, witness.stack_len_before);
                        return Err(err);
                    }
                }
            }
            // A name dispatched where it stands, as the token walk's Symbol
            // arm dispatches it.
            CompiledOp::FallbackToken(Token::Symbol(symbol)) => {
                let name = crate::word_name::canonical_word_name(symbol);
                let witness = interp.begin_dispatch();
                match interp.execute_word_core(name.as_ref()) {
                    Ok(()) => interp.trace_nil_outcome(name.as_ref(), &witness),
                    Err(err) => {
                        interp.record_word_dispatch_failure(
                            name.as_ref(),
                            &err,
                            witness.stack_len_before,
                        );
                        return Err(err);
                    }
                }
            }
            // Unreachable: a line holding any other fallback token is
            // re-interpreted whole, above.
            CompiledOp::FallbackToken(_) => {}
        }
    }
    Ok(())
}

pub fn arc_plan(plan: CompiledPlan) -> Arc<CompiledPlan> {
    Arc::new(plan)
}

// Pre-resolved builtin call sites for compiled plans.
//
// A builtin call site, specialized once at compile time so the per-call
// dispatch work (name canonicalization, linear registry scan) is never
// repeated at runtime. Everything precomputed here depends only on static
// tables, never on dictionary state, so no epoch guard is needed.
#[derive(Debug)]
pub struct CompiledCall {
    /// Canonical builtin name. Kept for the unresolved fallback
    /// path, diagnostics, and plan introspection.
    pub name: String,
    /// Pre-resolved contract, replacing the runtime registry scan.
    /// `None` for a name the registry does not know — the fallback path reports
    /// it as an unknown word.
    pub word: Option<&'static GeneratedWord>,
}

impl CompiledCall {
    pub fn resolve(name: &str) -> Self {
        let canonical = crate::word_name::canonical_word_name(name).into_owned();
        let word = generated_word(&canonical);
        Self {
            word,
            name: canonical,
        }
    }
}

/// Run a pre-resolved builtin call site. Mirrors `execute_builtin` exactly —
/// declared NIL contract, then executor dispatch — but
/// consumes the decisions `CompiledCall::resolve` already made instead of
/// re-scanning the registry table.
///
/// "Mirrors `execute_builtin` exactly" is the whole contract of this function,
/// and the NIL guard is part of what it has to mirror: a compiled body that
/// skipped the guard would let a Word behave one way when called directly and
/// another when called from inside a user word, which is precisely the
/// unobservability that compiling a body is required to preserve
/// (LANG.AUTHORITY.FREEDOM).
pub(crate) fn execute_compiled_call(interp: &mut Interpreter, call: &CompiledCall) -> Result<()> {
    let Some(word) = call.word else {
        return interp.execute_builtin_direct(&call.name);
    };
    let result = match interp.apply_declared_nil_contract(word) {
        Some(decided) => decided,
        None => match interp.apply_declared_lift(word) {
            Some(lifted) => lifted,
            None => interp.execute_builtin_by_id(word.id),
        },
    };
    result.map_err(|err| err.attributed_to(word.name))
}
