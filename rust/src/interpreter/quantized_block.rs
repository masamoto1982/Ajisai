use std::collections::HashSet;
use std::sync::Arc;

use crate::elastic::purity_table::{purity_by_name, Purity};
use crate::types::{Capabilities, Stability, Tier, Token};

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

// ── Virtual Tensor Unit (VTU) classification ─────────────────────────────
//
// VTU is *not* a physical accelerator. It is an internal classification of
// pure, shape-aware kernels that lets the runtime explain (and, in the
// future, schedule) work onto the most appropriate execution surface.
//
// Important invariants:
//   - VtuHint never affects execution semantics. Including it in
//     `GuardSignature` would cause spurious guard invalidations on what is
//     fundamentally an explanation field, so it is intentionally kept out.
//   - All variants are forward-looking; only `CpuScalar`, `WasmSimd`, and
//     `DenseTensorLoop` and `SparseTensorLoop` map to surfaces Ajisai can
//     exercise internally today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VtuBackendCandidate {
    CpuScalar,
    WasmSimd,
    DenseTensorLoop,
    SparseTensorLoop,
    #[allow(dead_code)]
    NpuCandidate,
    #[allow(dead_code)]
    GpuCandidate,
    #[allow(dead_code)]
    TpuCandidate,
    FallbackInterpreter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VtuSuitability {
    StrongCandidate,
    WeakCandidate,
    NotSuitable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataMovementClass {
    None,
    Low,
    Medium,
    #[allow(dead_code)]
    High,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VtuHint {
    pub suitability: VtuSuitability,
    pub backend_candidates: Vec<VtuBackendCandidate>,
    pub data_movement: DataMovementClass,
    pub reason: &'static str,
}

impl Default for VtuHint {
    fn default() -> Self {
        Self::not_suitable("default")
    }
}

impl VtuHint {
    pub fn not_suitable(reason: &'static str) -> Self {
        Self {
            suitability: VtuSuitability::NotSuitable,
            backend_candidates: vec![VtuBackendCandidate::FallbackInterpreter],
            data_movement: DataMovementClass::Unknown,
            reason,
        }
    }

    pub fn is_candidate(&self) -> bool {
        matches!(
            self.suitability,
            VtuSuitability::StrongCandidate | VtuSuitability::WeakCandidate
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelKind {
    MapUnaryPure,
    PredicateUnaryPure,
    FoldBinaryPure,
    /// Reserved for a future SCAN-specific kernel (accumulator-preserving).
    /// Currently SCAN uses `FoldBinaryPure` because the per-step binary op is
    /// identical.
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
    /// Observational VTU classification. Never participates in
    /// `GuardSignature`; never affects execution semantics.
    pub vtu_hint: VtuHint,
}

/// Returns (inputs_consumed, outputs_produced) for well-known pure builtins.
/// Returns None for unknown or variable-arity builtins.
fn builtin_arity(name: &str) -> Option<(i32, i32)> {
    // Single source of truth: the §7.14 Coreword mass contract. Keeping this a
    // thin adapter prevents the compiled-plan analyzer and the contract registry
    // from drifting on arity (SPEC §13.1).
    crate::coreword_registry::mass_contract(name)
        .fixed()
        .map(|(consumes, produces)| (consumes as i32, produces as i32))
}

/// Returns true if the given builtin name has observable side effects
/// (I/O, time, randomness, dictionary mutation, concurrency).
///
/// Authoritative source: `crate::elastic::purity_table`.
/// - `Purity::Pure`    → not side-effecting
/// - `Purity::Impure`  → side-effecting
/// - `Purity::Unknown` → conservatively treated as side-effecting
///   (higher-order / control-flow words whose behavior depends on runtime
///   arguments)
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

/// Recurse into the body tokens of a `PushCodeBlock` to determine whether the
/// block (e.g. a HOF callback, an `EXEC` body, a `COND` clause body) is pure.
///
/// When an interpreter is available, the inner tokens are compiled to a
/// `CompiledPlan` and passed back through `analyze_compiled_plan_with_context`,
/// which gives full coverage of nested user words and nested code blocks. When
/// no interpreter is available, we fall back to a name-only scan that flags
/// any explicit impure builtin token; pure builtins, literals, and unknown
/// names (treated as pure for this gate, since user words are handled at the
/// outer level when an interpreter is present) leave the block pure.
///
/// Conservative on depth exhaustion: returns `false` so the outer block is
/// marked impure, preventing fast-path quantization of unanalysable callbacks.
fn inner_block_tokens_are_pure(
    tokens: &[Token],
    interp: Option<&Interpreter>,
    visited: &mut HashSet<String>,
    depth: usize,
) -> bool {
    if depth + 1 >= MAX_PURITY_ANALYSIS_DEPTH {
        return false;
    }

    if let Some(interp) = interp {
        let lines = vec![crate::types::ExecutionLine {
            body_tokens: tokens.to_vec().into(),
        }];
        let def = WordDefinition {
            lines: lines.into(),
            is_builtin: false,
            tier: Tier::Contrib,
            stability: Stability::Stable,
            capabilities: Capabilities::PURE,
            description: None,
            dependencies: Default::default(),
            original_source: None,
            namespace: None,
            registration_order: 0,
            execution_plans: None,
        };
        let inner_plan = compile_word_definition(&def, interp);
        let (_, _, inner_purity, _) =
            analyze_compiled_plan_with_context(&inner_plan, Some(interp), visited, depth + 1);
        return inner_purity == QuantizedPurity::Pure;
    }

    !tokens.iter().any(|t| {
        if let Token::Symbol(sym) = t {
            let canonical = crate::core_word_aliases::canonicalize_core_word_name(sym);
            if let Some(info) = purity_by_name(canonical.as_ref()) {
                return info.purity == Purity::Impure;
            }
        }
        false
    })
}

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
                CompiledOp::PushLiteral(_) | CompiledOp::PushVectorLiteral(_, _) => {
                    cur_depth += 1;
                }
                CompiledOp::PushCodeBlock(inner_tokens) => {
                    cur_depth += 1;
                    if !inner_block_tokens_are_pure(inner_tokens, interp, visited, depth) {
                        is_pure = false;
                    }
                }
                // Meta-ops with no stack effect
                CompiledOp::SetTargetModeStackTop
                | CompiledOp::SetTargetModeStack
                | CompiledOp::SetConsumptionConsume
                | CompiledOp::SetConsumptionKeep
                | CompiledOp::BeginGuardedBlock
                | CompiledOp::LineBreak => {}

                CompiledOp::CallBuiltin(call) => {
                    // `CompiledCall.name` is already canonical.
                    let key: &str = &call.name;
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
                    let propagated_pure = try_user_word_is_pure(name, interp, visited, depth);
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
                    let propagated_pure = try_user_word_is_pure(&qualified, interp, visited, depth);
                    if !propagated_pure {
                        is_pure = false;
                    }
                    all_known = false;
                }

                CompiledOp::FallbackToken(token) => {
                    if let Token::Symbol(sym) = token {
                        let normalized = crate::core_word_aliases::canonicalize_core_word_name(sym);
                        if is_side_effecting_builtin(normalized.as_ref()) {
                            is_pure = false;
                        }
                    }
                    if min_depth_at_first_unknown.is_none() {
                        min_depth_at_first_unknown = Some(min_depth);
                    }
                    all_known = false;
                }

                // Precompiled COND: data-dependent arity, like the dynamic
                // `COND` builtin. Clause purity is still captured by the kept
                // `PushCodeBlock` ops above, so freezing arity here suffices.
                CompiledOp::CondDispatch(_) => {
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

/// Gate: a block is eligible for quantization iff
///   1. it is non-empty,
///   2. it contains no `LineBreak` token, and
///   3. no symbol token resolves, via the purity table, to an impure builtin.
///
/// Clauses 1 and 2 are token-shape filters. Clause 3 is the
/// Phase 1-C "classification-direct reference" gate: it pulls the static
/// purity classification straight from `purity_by_name` so that explicit
/// impure builtins (PRINT, EVAL, DEF, …) are rejected before quantization
/// rather than caught later in the analyzer.
pub fn is_quantizable_block(tokens: &[Token]) -> bool {
    !tokens.is_empty()
        && !tokens.iter().any(|t| matches!(t, Token::LineBreak))
        && !tokens.iter().any(token_is_impure_builtin)
}

fn token_is_impure_builtin(t: &Token) -> bool {
    if let Token::Symbol(sym) = t {
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(sym);
        if let Some(info) = purity_by_name(canonical.as_ref()) {
            return info.purity == Purity::Impure;
        }
    }
    false
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

/// Derive a `VtuHint` from kernel classification + purity. This is purely
/// observational; the runtime ignores the hint when picking an execution
/// path. The conservative defaults below ensure that side-effecting,
/// unknown-purity, or non-quantizable blocks never surface as candidates.
fn infer_vtu_hint(kernel_kind: KernelKind, purity: QuantizedPurity) -> VtuHint {
    use DataMovementClass::*;
    use VtuBackendCandidate::*;
    use VtuSuitability::*;

    if matches!(purity, QuantizedPurity::SideEffecting) {
        return VtuHint::not_suitable("side-effecting block");
    }

    match kernel_kind {
        KernelKind::MapUnaryPure => VtuHint {
            suitability: StrongCandidate,
            backend_candidates: vec![
                CpuScalar,
                WasmSimd,
                DenseTensorLoop,
                SparseTensorLoop,
                NpuCandidate,
                GpuCandidate,
            ],
            data_movement: Low,
            reason: "elementwise pure map; embarrassingly parallel",
        },
        KernelKind::PredicateUnaryPure => VtuHint {
            suitability: StrongCandidate,
            backend_candidates: vec![CpuScalar, WasmSimd, DenseTensorLoop, SparseTensorLoop],
            data_movement: Low,
            reason: "elementwise pure predicate",
        },
        KernelKind::FoldBinaryPure => VtuHint {
            suitability: WeakCandidate,
            backend_candidates: vec![CpuScalar, DenseTensorLoop, SparseTensorLoop],
            data_movement: Medium,
            reason: "reduction may depend on order/associativity; \
                     parallelization requires Approx boundary",
        },
        KernelKind::ScanBinaryPure => VtuHint {
            suitability: WeakCandidate,
            backend_candidates: vec![CpuScalar, DenseTensorLoop, SparseTensorLoop],
            data_movement: Medium,
            reason: "scan carries an accumulator; not embarrassingly parallel",
        },
        KernelKind::GenericCompiled => match purity {
            QuantizedPurity::Pure => VtuHint {
                suitability: WeakCandidate,
                backend_candidates: vec![CpuScalar, FallbackInterpreter],
                data_movement: Unknown,
                reason: "pure but unspecialized",
            },
            QuantizedPurity::Unknown => VtuHint::not_suitable("unknown purity"),
            QuantizedPurity::SideEffecting => VtuHint::not_suitable("side-effecting block"),
        },
        KernelKind::NonQuantizable => VtuHint::not_suitable("non-quantizable block"),
    }
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
        tier: Tier::Contrib,
        stability: Stability::Stable,
        capabilities: Capabilities::PURE,
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

    let vtu_hint = infer_vtu_hint(kernel_kind, purity);

    // Count VTU classifications at build time. Cache hits do not bump these
    // counters, so the totals reflect distinct block builds, not uses.
    if vtu_hint.is_candidate() {
        interp.runtime_metrics.vtu_candidate_block_count = interp
            .runtime_metrics
            .vtu_candidate_block_count
            .saturating_add(1);
    } else {
        interp.runtime_metrics.vtu_rejected_block_count = interp
            .runtime_metrics
            .vtu_rejected_block_count
            .saturating_add(1);
    }
    if eligible_for_fusion || can_fuse {
        interp.runtime_metrics.vtu_fusion_candidate_count = interp
            .runtime_metrics
            .vtu_fusion_candidate_count
            .saturating_add(1);
    }

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
        vtu_hint,
    })
}
