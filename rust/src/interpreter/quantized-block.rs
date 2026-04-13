use std::sync::Arc;

use crate::types::Token;

use super::{compile_word_definition, compiled_plan::CompiledOp, CompiledPlan, EpochSnapshot, Interpreter, WordDefinition};

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
        "NOT" | "ABS" | "NEG" | "SQRT" | "FLOOR" | "CEIL" | "ROUND"
        | "SIGN" | "SUCC" | "PRED" | "EXP" | "LOG" | "LOG2" | "LOG10"
        | "SIN" | "COS" | "TAN" | "ASIN" | "ACOS" | "ATAN" => Some((1, 1)),
        // Type cast / unary
        "INT" | "FLOAT" | "STR" | "BOOL" | "CHAR" => Some((1, 1)),
        // Unknown arity (vector ops, HOF, stack words not in BUILTIN_SPECS, etc.) → None
        _ => None,
    }
}

/// Words that have observable side effects (I/O, state mutation, concurrency).
fn is_side_effecting_builtin(name: &str) -> bool {
    matches!(
        name,
        "PRINT" | "EMIT" | "READ" | "WRITE" | "READLINE"
            | "SPAWN" | "AWAIT" | "SEND" | "RECV"
            | "DEF" | "DEL" | "IMPORT"
            | "RAND" | "SEED"
    )
}

/// Statically analyse a compiled plan and return
/// `(input_arity, output_arity, purity, dependency_words)`.
///
/// Arity analysis uses a symbolic stack-depth simulation:
/// - Start at depth 0.
/// - Each push op adds 1; each builtin with known arity adjusts depth.
/// - If depth would go below the running minimum, update `min_depth`.
/// - `input_arity  = -min_depth`  (values consumed from the external stack)
/// - `output_arity = final_depth - min_depth` (values left on stack after execution)
///
/// If any op has unknown arity (user words, qualified words, fallback tokens),
/// both arities are `Variable`.
fn analyze_compiled_plan(plan: &CompiledPlan) -> (QuantizedArity, QuantizedArity, QuantizedPurity, Vec<String>) {
    let mut depth: i32 = 0;
    let mut min_depth: i32 = 0;
    let mut all_known = true;
    let mut is_pure = true;
    let mut dep_words: Vec<String> = Vec::new();

    'outer: for line in &plan.lines {
        for op in &line.ops {
            match op {
                CompiledOp::PushLiteral(_) | CompiledOp::PushCodeBlock(_) => {
                    depth += 1;
                }
                // Meta-ops with no stack effect
                CompiledOp::SetTargetModeStackTop
                | CompiledOp::SetTargetModeStack
                | CompiledOp::SetConsumptionConsume
                | CompiledOp::SetConsumptionKeep
                | CompiledOp::BeginGuardedBlock
                | CompiledOp::LineBreak => {}

                CompiledOp::CallBuiltin(name) => {
                    if is_side_effecting_builtin(name) {
                        is_pure = false;
                    }
                    if let Some((inputs, outputs)) = builtin_arity(name) {
                        depth -= inputs;
                        if depth < min_depth {
                            min_depth = depth;
                        }
                        depth += outputs;
                    } else {
                        // Unknown arity; cannot determine statically
                        all_known = false;
                        break 'outer;
                    }
                }

                CompiledOp::CallUserWord(name) => {
                    dep_words.push(name.clone());
                    all_known = false;
                    break 'outer;
                }

                CompiledOp::CallQualifiedWord { namespace, word } => {
                    dep_words.push(format!("{}@{}", namespace, word));
                    all_known = false;
                    break 'outer;
                }

                CompiledOp::FallbackToken(_) => {
                    all_known = false;
                    break 'outer;
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
        let output = QuantizedArity::Fixed((depth - min_depth) as usize);
        (input, output)
    } else {
        (QuantizedArity::Variable, QuantizedArity::Variable)
    };

    let purity = if is_pure {
        QuantizedPurity::Pure
    } else {
        QuantizedPurity::SideEffecting
    };

    (input_arity, output_arity, purity, dep_words)
}

pub fn is_quantizable_block(tokens: &[Token]) -> bool {
    !tokens.is_empty() && !tokens.iter().any(|t| matches!(t, Token::LineBreak | Token::SafeMode))
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
        compiled_plan: None,
    };
    let plan = Arc::new(compile_word_definition(&def, interp));
    interp.bump_execution_epoch();
    interp.runtime_metrics.quantized_block_build_count += 1;

    let (input_arity, output_arity, purity, dependency_words) = analyze_compiled_plan(&plan);

    let can_fuse = purity == QuantizedPurity::Pure;
    let can_short_circuit = purity == QuantizedPurity::Pure;

    #[cfg(feature = "trace-quant")]
    eprintln!(
        "[trace-quant] quantized block generated: input={:?} output={:?} purity={:?} deps={:?}",
        input_arity, output_arity, purity, dependency_words
    );

    Some(QuantizedBlock {
        compiled_plan: plan,
        captured_epoch: interp.current_epoch_snapshot(),
        input_arity,
        output_arity,
        purity,
        can_fuse,
        can_short_circuit,
        dependency_words,
    })
}
