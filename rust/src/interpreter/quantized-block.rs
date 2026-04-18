use std::collections::HashSet;
use std::sync::Arc;

use crate::elastic::purity_table::{purity_by_name, Purity};
use crate::types::Token;

use super::{
    compile_word_definition, compiled_plan::CompiledOp, CompiledPlan, EpochSnapshot, Interpreter,
    WordDefinition,
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelKind {
    MapUnaryPure,
    PredicateUnaryPure,
    FoldBinaryPure,
    /// Reserved for a future SCAN-specific kernel (accumulator-preserving).
    /// Currently SCAN uses `FoldBinaryPure` because the per-step binary op is
    /// identical; this variant is retained only for forward compatibility.
    #[allow(dead_code)]
    ScanBinaryPure,
    GenericCompiled,
    NonQuantizable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardSignature {
    pub dictionary_epoch: u64,
    pub module_epoch: u64,
    pub kernel_kind: KernelKind,
    pub purity: QuantizedPurity,
}

#[derive(Debug, Clone)]
pub struct QuantizedBlock {
    pub compiled_plan: Arc<CompiledPlan>,
    pub captured_epoch: EpochSnapshot,
    pub input_arity: QuantizedArity,
    pub output_arity: QuantizedArity,
    pub purity: QuantizedPurity,
    pub kernel_kind: KernelKind,
    pub fast_path_id: Option<String>,
    pub guard_signature: GuardSignature,
    pub lowered_kernel_ir: Vec<CompiledOp>,
    pub eligible_for_cache: bool,
    pub eligible_for_fusion: bool,
    pub can_fuse: bool,
    pub can_short_circuit: bool,
    pub dependency_words: Vec<String>,
}

/// Returns (inputs_consumed, outputs_produced) for well-known pure builtins.
/// Returns None for unknown or variable-arity builtins.
fn builtin_arity(name: &str) -> Option<(i32, i32)> {
    match name {
        // Binary arithmetic
        "+" | "-" | "*" | "/" | "%" | "^" => Some((2, 1)),
        // Binary comparison
        "<" | ">" | "<=" | ">=" | "=" | "!=" => Some((2, 1)),
        // Binary logical
        "AND" | "OR" | "XOR" => Some((2, 1)),
        // Unary arithmetic / math
        "NOT" | "ABS" | "NEG" | "SQRT" | "FLOOR" | "CEIL" | "ROUND" | "SIGN" | "SUCC" | "PRED"
        | "EXP" | "LOG" | "LOG2" | "LOG10" | "SIN" | "COS" | "TAN" | "ASIN" | "ACOS" | "ATAN" => {
            Some((1, 1))
        }
        // Type cast / unary
        "INT" | "FLOAT" | "STR" | "BOOL" | "CHAR" => Some((1, 1)),
        // Unknown arity (vector ops, HOF, stack words not in BUILTIN_SPECS, etc.) → None
        _ => None,
    }
}

/// Returns true if the given builtin name has observable side effects
/// (I/O, time, randomness, dictionary mutation, concurrency).
///
/// Authoritative source: `crate::elastic::purity_table`.
/// - `Purity::Pure`    → not side-effecting
/// - `Purity::Impure`  → side-effecting
/// - `Purity::Unknown` → conservatively treated as side-effecting
///                       (higher-order / control-flow words whose behavior
///                       depends on runtime arguments)
/// - Unrecognized name (user-defined or non-spec) → false
///   (handled separately via the `CallUserWord` / fallback paths in
///   `analyze_compiled_plan_with_context`)
fn is_side_effecting_builtin(name: &str) -> bool {
    match purity_by_name(name) {
        Some(info) => info.purity != Purity::Pure,
        None => false,
    }
}

const MAX_PURITY_ANALYSIS_DEPTH: usize = 4;

/// Context-aware variant used both at the top level and for recursive
/// user-word purity propagation.
fn analyze_compiled_plan_with_context(
    plan: &CompiledPlan,
    interp: Option<&Interpreter>,
    visited: &mut HashSet<String>,
    depth: usize,
) -> (QuantizedArity, QuantizedArity, QuantizedPurity, Vec<String>) {
    let mut cur_depth: i32 = 0;
    let mut min_depth: i32 = 0;
    let mut min_depth_at_first_unknown: Option<i32> = None;
    let mut all_known = true;
    let mut is_pure = true;
    let mut dep_words: Vec<String> = Vec::new();

    for line in &plan.lines {
        for op in &line.ops {
            match op {
                CompiledOp::PushLiteral(_) | CompiledOp::PushCodeBlock(_) => {
                    cur_depth += 1;
                }
                // Meta-ops with no stack effect
                CompiledOp::SetTargetModeStackTop
                | CompiledOp::SetTargetModeStack
                | CompiledOp::SetConsumptionConsume
                | CompiledOp::SetConsumptionKeep
                | CompiledOp::BeginGuardedBlock
                | CompiledOp::LineBreak => {}

                CompiledOp::CallBuiltin(name) => {
                    let normalized = Interpreter::normalize_symbol(name);
                    let key: &str = normalized.as_ref();
                    if is_side_effecting_builtin(key) {
                        is_pure = false;
                    }
                    if let Some((inputs, outputs)) = builtin_arity(key) {
                        cur_depth -= inputs;
                        if cur_depth < min_depth {
                            min_depth = cur_depth;
                        }
                        cur_depth += outputs;
                    } else {
                        if min_depth_at_first_unknown.is_none() {
                            min_depth_at_first_unknown = Some(min_depth);
                        }
                        all_known = false;
                    }
                }

                CompiledOp::CallUserWord(name) => {
                    dep_words.push(name.clone());
                    if min_depth_at_first_unknown.is_none() {
                        min_depth_at_first_unknown = Some(min_depth);
                    }
                    let propagated_pure =
                        try_user_word_is_pure(name, interp, visited, depth);
                    if !propagated_pure {
                        is_pure = false;
                    }
                    all_known = false;
                }

                CompiledOp::CallQualifiedWord { namespace, word } => {
                    let qualified = format!("{}@{}", namespace, word);
                    dep_words.push(qualified.clone());
                    if min_depth_at_first_unknown.is_none() {
                        min_depth_at_first_unknown = Some(min_depth);
                    }
                    let propagated_pure =
                        try_user_word_is_pure(&qualified, interp, visited, depth);
                    if !propagated_pure {
                        is_pure = false;
                    }
                    all_known = false;
                }

                CompiledOp::FallbackToken(_) => {
                    if min_depth_at_first_unknown.is_none() {
                        min_depth_at_first_unknown = Some(min_depth);
                    }
                    all_known = false;
                }
            }
        }
    }

    let (input_arity, output_arity) = if all_known {
        let input = if min_depth < 0 {
            QuantizedArity::Fixed((-min_depth) as usize)
        } else {
            QuantizedArity::Fixed(0)
        };
        // Values remaining on the mini-stack = final_depth - min_depth
        let output = QuantizedArity::Fixed((cur_depth - min_depth) as usize);
        (input, output)
    } else {
        // Partial info: keep input arity only when the stack went negative
        // before the first unknown op (i.e., we have proven the block consumes
        // external inputs).  A min_depth of 0 at the unknown point means we
        // haven't proven anything about inputs → Variable.
        let input = match min_depth_at_first_unknown {
            Some(m) if m < 0 => QuantizedArity::Fixed((-m) as usize),
            _ => QuantizedArity::Variable,
        };
        (input, QuantizedArity::Variable)
    };

    let purity = if is_pure {
        QuantizedPurity::Pure
    } else {
        QuantizedPurity::SideEffecting
    };

    (input_arity, output_arity, purity, dep_words)
}

/// Attempt to determine whether a user-defined word is pure by recursively
/// analysing its compiled plan. Conservative on failure.
///
/// Returns `true` only when we can prove the word is pure within the depth
/// budget; otherwise returns `false`.
fn try_user_word_is_pure(
    name: &str,
    interp: Option<&Interpreter>,
    visited: &mut HashSet<String>,
    depth: usize,
) -> bool {
    let Some(interp) = interp else {
        return false;
    };
    if depth >= MAX_PURITY_ANALYSIS_DEPTH {
        return false;
    }
    if visited.contains(name) {
        // Recursive cycle → conservative
        return false;
    }

    let Some((_, def)) = interp.resolve_word_entry_readonly(name) else {
        return false;
    };

    let plan = compile_word_definition(&def, interp);

    visited.insert(name.to_string());
    let (_ia, _oa, purity, _deps) =
        analyze_compiled_plan_with_context(&plan, Some(interp), visited, depth + 1);
    visited.remove(name);

    purity == QuantizedPurity::Pure
}

pub fn is_quantizable_block(tokens: &[Token]) -> bool {
    !tokens.is_empty()
        && !tokens
            .iter()
            .any(|t| matches!(t, Token::LineBreak | Token::SafeMode))
}

fn is_const_vector_token(token: &Token) -> bool {
    matches!(token, Token::Number(_) | Token::String(_))
        || matches!(token, Token::Symbol(sym) if sym.as_ref() == "TRUE" || sym.as_ref() == "FALSE")
}

fn is_const_vector_pattern(tokens: &[Token], op: &str) -> bool {
    if tokens.len() != 4 {
        return false;
    }
    matches!(
        (&tokens[0], &tokens[1], &tokens[2], &tokens[3]),
        (Token::VectorStart, constant, Token::VectorEnd, Token::Symbol(sym))
            if is_const_vector_token(constant) && sym.as_ref() == op
    )
}

fn detect_kernel_kind(
    tokens: &[Token],
    purity: QuantizedPurity,
    _input_arity: QuantizedArity,
) -> KernelKind {
    if purity != QuantizedPurity::Pure {
        return KernelKind::GenericCompiled;
    }

    // Initial pattern recognition (Phase B-2)
    if is_const_vector_pattern(tokens, "+")
        || is_const_vector_pattern(tokens, "-")
        || is_const_vector_pattern(tokens, "*")
        || is_const_vector_pattern(tokens, "/")
        || is_const_vector_pattern(tokens, "MOD")
        || is_const_vector_pattern(tokens, "=")
        || is_const_vector_pattern(tokens, "<")
    {
        return KernelKind::MapUnaryPure;
    }

    if tokens.len() == 1 {
        if let Token::Symbol(sym) = &tokens[0] {
            return match sym.as_ref() {
                "NOT" => KernelKind::PredicateUnaryPure,
                "ABS" | "NEG" => KernelKind::MapUnaryPure,
                "+" | "-" | "*" | "/" | "MOD" => KernelKind::FoldBinaryPure,
                _ => KernelKind::GenericCompiled,
            };
        }
    }

    KernelKind::GenericCompiled
}

pub fn quantize_code_block(tokens: &[Token], interp: &mut Interpreter) -> Option<QuantizedBlock> {
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
        execution_plans: None,
    };
    let plan = Arc::new(compile_word_definition(&def, interp));
    interp.bump_execution_epoch();
    interp.runtime_metrics.quantized_block_build_count += 1;

    let (input_arity, output_arity, purity, dependency_words) =
        analyze_compiled_plan_with_context(&plan, Some(interp), &mut HashSet::new(), 0);
    let kernel_kind = detect_kernel_kind(tokens, purity, input_arity);

    let can_fuse = purity == QuantizedPurity::Pure;
    let can_short_circuit = purity == QuantizedPurity::Pure;
    let captured_epoch = interp.current_epoch_snapshot();
    let fast_path_id = match kernel_kind {
        KernelKind::GenericCompiled | KernelKind::NonQuantizable => None,
        _ => Some(format!(
            "kernel::{kernel_kind:?}::in={input_arity:?}::out={output_arity:?}"
        )),
    };
    let guard_signature = GuardSignature {
        dictionary_epoch: captured_epoch.dictionary_epoch,
        module_epoch: captured_epoch.module_epoch,
        kernel_kind,
        purity,
    };
    let lowered_kernel_ir = plan
        .lines
        .iter()
        .flat_map(|line| line.ops.iter().cloned())
        .collect::<Vec<_>>();
    let eligible_for_cache = purity == QuantizedPurity::Pure;
    let eligible_for_fusion = matches!(
        kernel_kind,
        KernelKind::MapUnaryPure | KernelKind::PredicateUnaryPure
    );

    #[cfg(feature = "trace-quant")]
    eprintln!(
        "[trace-quant] quantized block generated: input={:?} output={:?} purity={:?} deps={:?}",
        input_arity, output_arity, purity, dependency_words
    );

    Some(QuantizedBlock {
        compiled_plan: plan,
        captured_epoch,
        input_arity,
        output_arity,
        purity,
        kernel_kind,
        fast_path_id,
        guard_signature,
        lowered_kernel_ir,
        eligible_for_cache,
        eligible_for_fusion,
        can_fuse,
        can_short_circuit,
        dependency_words,
    })
}
