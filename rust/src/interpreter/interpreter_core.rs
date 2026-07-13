use crate::error::Result;
use crate::types::{Interpretation, SemanticRegistry, Stack, Token, Value, WordDefinition};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::epoch::EpochSnapshot;

pub const DEFAULT_MAX_EXECUTION_STEPS: usize = 100_000;

/// Cap on the user-word call stack depth. Hit before Rust's native call stack
/// runs out and panics the WASM module to an unrecoverable trap. The value is
/// set well above any reasonable hand-written nesting (the existing deep
/// non-recursive test uses 5 frames) but well below the Rust stack budget on
/// WASM and on the 2 MiB default native test-thread stack — every user-word
/// recursion expands to several Rust frames (execute_word_core_inner →
/// plan/structure runner → execution loop → resolve), so the empirical
/// safe ceiling is roughly 256 levels in debug builds. The execution-step
/// limit (DEFAULT_MAX_EXECUTION_STEPS = 100_000) is still the primary
/// backstop for non-recursive runaway computation; this guard turns deep
/// recursion specifically into a recoverable AjisaiError instead of a trap.
pub const MAX_USER_WORD_DEPTH: usize = 256;

/// Cap on how deeply vector literals may nest (`[ [ [ ... ] ] ]`). The literal
/// builder `collect_vector_with_depth` recurses one frame per level, and so do
/// every downstream traversal of the resulting value — `Display`, the derived
/// recursive `Drop` of the nested `Arc<Vec<Value>>`, and the JSON
/// arena/stringify conversions. None of those had a depth guard, so a few
/// thousand levels of nesting from plain source overflowed the native stack and
/// aborted the process (an unrecoverable trap inside the WASM playground)
/// rather than producing a diagnosable `AjisaiError`. The ceiling matches
/// `MAX_USER_WORD_DEPTH`: a single self-recursive vector frame is lighter than
/// a user-word call (which expands to several Rust frames per level), so a
/// value capped at this depth stays safely within the same WASM stack envelope
/// that depth is already vetted against, while remaining ~20x the deepest
/// hand-written nesting in the corpus.
pub const MAX_VECTOR_NESTING_DEPTH: usize = 256;

/// Cap on the number of elements a single generative built-in (`RANGE`,
/// `FILL`, ...) is allowed to materialize in one call. Such words loop
/// internally to build a vector/tensor, so they each count as a *single*
/// execution step and therefore bypass `DEFAULT_MAX_EXECUTION_STEPS`. Without
/// this guard an input like `[ 0 9999999999999 ] RANGE` or
/// `[ 1000000 1000000 7 ] FILL` drives an unbounded allocation that aborts the
/// process with an OOM (an unrecoverable trap inside the WASM playground)
/// instead of a diagnosable `AjisaiError`. The ceiling sits three orders of
/// magnitude above any realistic generated size (benchmarks top out in the
/// hundreds) while keeping worst-case materialization recoverable: each
/// generated `Value` costs a few hundred bytes, so one million elements bounds
/// a single call to a few hundred MiB rather than the multi-gigabyte abort that
/// unbounded counts produce.
pub const MAX_MATERIALIZED_ELEMENTS: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperationTargetMode {
    StackTop,
    Stack,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsumptionMode {
    Consume,
    Keep,
}

#[derive(Debug, Clone)]
pub(crate) struct UserDictionary {
    pub order: u64,
    pub words: HashMap<String, Arc<WordDefinition>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ModuleDictionary {
    pub words: HashMap<String, Arc<WordDefinition>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportedModule {
    pub import_all_public: bool,
    pub imported_words: HashSet<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ImportTable {
    pub modules: HashMap<String, ImportedModule>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DictionaryDependencyInfo {
    pub depends_on: HashSet<String>,
    pub depended_by: HashSet<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum ChildState {
    Running,
    Completed,
    Failed,
    Killed,
    Timeout,
}

#[derive(Debug, Clone)]
pub(crate) enum ExitReason {
    Normal,
    Error,
    Killed,
    Timeout,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeDictionarySnapshot {
    pub user_words: HashMap<String, Arc<WordDefinition>>,
    pub user_dictionaries: HashMap<String, UserDictionary>,
    pub dependents: HashMap<String, HashSet<String>>,
    pub import_table: ImportTable,
    pub module_vocabulary: HashMap<String, ModuleDictionary>,
    pub dictionary_dependencies: HashMap<String, DictionaryDependencyInfo>,
    pub next_registration_order: u64,
    pub active_user_dictionary: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ChildRuntime {
    pub code_block: Vec<Token>,
    pub dictionary_snapshot: RuntimeDictionarySnapshot,
    pub state: ChildState,
    pub exit_reason: Option<ExitReason>,
    pub result_snapshot: Option<Vec<Value>>,
    pub monitored: bool,
}

#[derive(Debug, Clone)]
pub struct ResolveCacheEntry {
    pub resolved_name: String,
    pub dictionary_epoch: u64,
    pub module_epoch: u64,
    pub registration_order: u64,
}

/// How the runtime reacts when the compiled (optimized) path and the plain
/// (reference) path disagree during shadow validation.
///
/// This is an *internal* safety control, never a user-facing knob. The default
/// (`Fallback`) already guarantees that a divergent optimization result is
/// never committed: the reference path wins. Ajisai programs get this
/// protection transparently just by running. The remaining variants exist for
/// benchmarking the comparison cost (`Off`) and for tests that need to observe
/// (`Observe`) or hard-reject (`Strict`) a disagreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IntegrityMode {
    /// Skip the enriched comparison (host effects / absence metadata) and keep
    /// the historical behavior. Used only to measure the comparison's own cost.
    Off,
    /// Run the full comparison and count disagreements, but still adopt the
    /// compiled path. Non-disruptive characterization.
    Observe,
    /// Default. On any disagreement, prefer the plain reference path so a
    /// result the reference path does not agree with is never committed.
    #[default]
    Fallback,
    /// On disagreement, refuse the result and surface an integrity failure
    /// instead of silently substituting the reference path.
    Strict,
}

#[derive(Debug, Clone, Copy)]
pub struct ValidationPolicy {
    pub enable_shadow_validation: bool,
    pub max_validation_input_len: usize,
    pub warmup_runs: u64,
    /// Reaction to a compiled-vs-plain disagreement. Defaults to the safe
    /// `Fallback`; see `IntegrityMode`.
    pub integrity_mode: IntegrityMode,
}

impl Default for ValidationPolicy {
    fn default() -> Self {
        Self {
            enable_shadow_validation: true,
            max_validation_input_len: 16,
            warmup_runs: 3,
            integrity_mode: IntegrityMode::Fallback,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeMetrics {
    pub compiled_plan_build_count: u64,
    pub compiled_plan_cache_hit_count: u64,
    pub compiled_plan_cache_miss_count: u64,
    pub quantized_block_build_count: u64,
    pub quantized_block_use_count: u64,
    pub hedged_race_started_count: u64,
    pub hedged_race_winner_quantized_count: u64,
    pub hedged_race_winner_plain_count: u64,
    pub hedged_race_fallback_count: u64,
    pub hedged_race_cancel_count: u64,
    pub hedged_race_validation_reject_count: u64,
    pub cond_guard_prefetch_count: u64,
    /// COND invocations that dispatched on a compile-time-precomputed clause
    /// table (`CompiledOp::CondDispatch`) instead of re-collecting, cloning, and
    /// re-splitting the clause blocks off the stack. Observational only.
    pub cond_dispatch_fast_count: u64,
    /// COND guard/body executions that ran a compiled sub-plan instead of
    /// re-interpreting the clause's token stream. Observational only.
    pub cond_clause_compiled_count: u64,
    /// Scalar-scalar arithmetic/comparison operations completed by the D1
    /// value-model fast path, bypassing the tensor broadcast wrapper while
    /// preserving the same observable Value and semantic hint.
    pub scalar_fastpath_count: u64,
    /// Homogeneous element-wise binary ops over equal-length vectors of
    /// irrational `ExactScalar` lanes routed through the compute-bound parallel
    /// map (`parallel::compute_bound_map`). Counts how many such flat exact
    /// broadcasts were dispatched; whether each actually fanned out across
    /// worker threads is decided inside the kernel by the compute-bound floor.
    /// Observational only; never alters value results (parallel == sequential).
    pub exact_real_parallel_broadcast_count: u64,
    pub shadow_validation_started_count: u64,
    pub shadow_validation_success_count: u64,
    pub shadow_validation_fallback_count: u64,
    /// Number of shadow validations where the compiled and plain paths
    /// disagreed on stack value, semantic hint, absence metadata, or emitted
    /// host effects. Counts genuine divergences the enriched comparison caught,
    /// independent of how the active `IntegrityMode` then resolved them.
    pub shadow_validation_integrity_mismatch_count: u64,
    pub resolve_cache_hit_count: u64,
    pub resolve_cache_miss_count: u64,
    pub resolve_cache_invalidation_count: u64,

    // ── Virtual Tensor Unit / Energy-aware Execution ──────────────────────
    // These counters are observational proxies for energy / data-movement
    // cost. They never alter execution semantics; they only describe how
    // much shape-aware work the runtime did. See
    // `docs/dev/virtual-tensor-unit-design.md`.
    /// Number of times a Value was flattened into FlatTensor form.
    pub vtu_tensor_flatten_count: u64,
    /// Total scalar elements observed during flattening.
    pub vtu_tensor_flattened_elements: u64,
    /// Number of times a FlatTensor was rebuilt into nested Value form.
    pub vtu_tensor_rebuild_count: u64,
    /// Total scalar elements rebuilt into Value form.
    pub vtu_tensor_rebuilt_elements: u64,
    /// Number of binary broadcast operations that began executing.
    pub vtu_broadcast_count: u64,
    /// Number of unary flat tensor operations that began executing.
    pub vtu_unary_flat_count: u64,
    /// Output scalar elements allocated by tensor operations.
    pub vtu_allocated_elements: u64,
    /// Same-shape elementwise fast paths taken inside tensor ops.
    pub vtu_same_shape_elementwise_count: u64,
    /// Real broadcast index-projection paths taken inside tensor ops.
    pub vtu_projected_broadcast_count: u64,
    /// SIMD fast paths actually taken (target-arch dependent).
    pub vtu_simd_kernel_use_count: u64,
    /// Native multi-core data-parallel kernels actually dispatched (Phase 3,
    /// hierarchy A). Bumped only when an element-wise integer op cleared both
    /// the Phase-2 `parallel_kernel_eligible` gate and the runtime dispatch
    /// floor and fanned across worker threads. Observational only; never alters
    /// execution semantics, and stays 0 on wasm (sequential fallback).
    pub vtu_parallel_kernel_use_count: u64,
    /// Dense Tensor values classified as sparse optimization candidates.
    pub vtu_sparse_candidate_count: u64,
    /// Total dense lanes in sparse candidate Tensor values.
    pub vtu_sparse_candidate_elements: u64,
    /// Non-zero dense lanes in sparse candidate Tensor values.
    pub vtu_sparse_candidate_nonzero_elements: u64,
    /// Zero dense lanes that sparse handling may skip moving or scanning.
    pub vtu_sparse_skippable_zero_elements: u64,
    /// QuantizedBlocks classified as VTU candidates (Strong or Weak).
    pub vtu_candidate_block_count: u64,
    /// QuantizedBlocks rejected as not suitable for VTU.
    pub vtu_rejected_block_count: u64,
    /// QuantizedBlocks whose VtuHint marked them as fusion candidates.
    pub vtu_fusion_candidate_count: u64,
    /// HOF invocations that took a Tensor-bulk fast path, iterating
    /// `Tensor.data: &[Fraction]` directly without per-element Value
    /// materialization. Phase III metric.
    pub vtu_bulk_kernel_use_count: u64,

    /// Guarded tail self-calls eliminated into an internal backward jump
    /// (the "internal GOTO" trampoline). Each increment is one recursive
    /// call that ran as a loop iteration instead of growing `call_depth`
    /// and the native stack. Observational only; never alters value results.
    pub tail_call_jump_count: u64,

    // ── Pure HOF kernel memoization (direction B) ─────────────────────────
    /// Per-element MAP kernel applications served from the pure-result cache
    /// instead of re-running the kernel. Observational only.
    pub hof_memo_hit_count: u64,
    /// Per-element MAP kernel applications that missed the cache and ran the
    /// kernel. Observational only.
    pub hof_memo_miss_count: u64,
    /// Pure per-element MAP kernel results written into the cache.
    pub hof_memo_store_count: u64,
}

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) core_vocabulary: HashMap<String, Arc<WordDefinition>>,
    pub(crate) user_words: HashMap<String, Arc<WordDefinition>>,
    pub(crate) user_dictionaries: HashMap<String, UserDictionary>,
    pub(crate) dependents: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    /// Structured, ordered host effects produced during execution. This is the
    /// language-independent observation channel for the conformance suite
    /// (`tests/conformance/`): two implementations agree iff they emit the same
    /// effect列. The legacy `output_buffer` string protocol is still emitted in
    /// parallel so existing front-ends keep working.
    pub(crate) host_effects: Vec<super::HostEffect>,
    /// Host boundary for clocks, entropy, capability checks, and effect sinks.
    /// Core execution must not call platform APIs directly; Hosted words route
    /// boundary access through this trait object so conformance can inject a
    /// deterministic or restricted host.
    pub(crate) host_env: Arc<dyn super::HostEnv>,
    pub(crate) definition_to_load: Option<String>,
    pub(crate) operation_target_mode: OperationTargetMode,
    pub(crate) consumption_mode: ConsumptionMode,
    pub(crate) force_flag: bool,
    pub(crate) disable_no_change_check: bool,
    pub(crate) pending_tokens: Option<Vec<Token>>,
    pub(crate) pending_token_index: usize,
    pub(crate) module_state: HashMap<String, Box<dyn std::any::Any + Send>>,
    pub(crate) import_table: ImportTable,
    pub(crate) call_stack: SmallVec<[String; 5]>,
    /// User-word call depth. Incremented on entry to a user-word body in
    /// `execute_word_core_inner`, decremented on exit. Compared against
    /// `MAX_USER_WORD_DEPTH` to prevent a deep recursion from blowing the
    /// Rust call stack and trapping the WASM module.
    pub(crate) call_depth: usize,
    pub(crate) execution_step_count: usize,
    pub(crate) max_execution_steps: usize,
    pub(crate) input_buffer: String,
    pub(crate) io_output_buffer: String,

    /// Host-injected serial receive buffers, keyed by opaque port id. Filled
    /// before execution from the platform serial adapter (Section 9.4); drained
    /// by `SERIAL@READ`.
    pub(crate) serial_inbox: HashMap<String, Vec<u8>>,
    /// Port ids the host has reported disconnected. `SERIAL@READ` projects this
    /// to `NilReason::PortDisconnected` once the inbox for the port is empty.
    pub(crate) serial_disconnected: HashSet<String>,

    pub(crate) module_vocabulary: HashMap<String, ModuleDictionary>,
    pub(crate) dictionary_dependencies: HashMap<String, DictionaryDependencyInfo>,
    pub(crate) next_registration_order: u64,
    pub(crate) active_user_dictionary: String,

    pub(crate) global_epoch: u64,
    pub(crate) dictionary_epoch: u64,
    pub(crate) module_epoch: u64,
    pub(crate) execution_epoch: u64,

    pub(crate) semantic_registry: SemanticRegistry,

    pub(crate) child_runtimes: HashMap<u64, ChildRuntime>,
    pub(crate) next_child_id: u64,
    pub(crate) monitor_notifications: Vec<Vec<Value>>,
    pub(crate) next_supervisor_id: u64,

    pub(crate) runtime_metrics: RuntimeMetrics,
    pub(crate) hedged_trace_log: Vec<String>,
    pub(crate) error_flow_trace_log: Vec<super::error_flow_trace::ErrorFlowEvent>,
    pub(crate) force_no_quant: bool,

    // ── Elastic Engine (MVP) ──────────────────────────────────────────────
    pub(crate) elastic_mode: crate::elastic::ElasticMode,
    pub(crate) elastic_cache: crate::elastic::CacheManager,
    pub(crate) resolve_cache: HashMap<String, ResolveCacheEntry>,
    pub(crate) validation_policy: ValidationPolicy,

    /// Owning user dictionary of the word currently being defined,
    /// dependency-scanned, or executed. Bare names resolve through this
    /// dictionary's words first (Section 8.6), so an imported word group is
    /// self-referential regardless of which other dictionaries are loaded.
    /// `None` at top level, where resolution falls back to the global order.
    pub(crate) owning_dictionary_context: Option<String>,

    /// Content identity of each user word, keyed by fully-qualified name
    /// (Section 8.6). Derived state: recomputed whenever the user-word graph
    /// changes.
    pub(crate) word_identities: HashMap<String, String>,

    /// Content store for definition bodies (Section 8.6), keyed by content key.
    /// Textually identical bodies share a single `Arc<[ExecutionLine]>`, so
    /// re-importing or copying a word group does not duplicate its code in
    /// memory.
    pub(crate) body_store: HashMap<String, std::sync::Arc<[crate::types::ExecutionLine]>>,

    /// When set, `recompute_word_identities` is a no-op. Bulk operations (e.g.
    /// restoring or importing many words) set this for the duration of the
    /// batch and recompute once at the end, avoiding O(N^2) identity hashing.
    pub(crate) defer_identity_recompute: bool,

    // ── Internal tail-call elimination ("internal GOTO") ──────────────────
    // Guarded tail self-recursion (a self-call in the tail position of a
    // COND clause body) is run as an internal backward jump instead of a
    // native recursive call. This keeps such loops in O(1) native stack and
    // lifts them past `MAX_USER_WORD_DEPTH`, without exposing any jump or
    // label to the surface language. See `docs/dev/internal-goto-tail-call.md`.
    /// Master toggle. Defaults to true; set `AJISAI_NO_TAIL_CALL=1` to force
    /// the legacy native-recursion path (used by the A/B benchmark harness).
    pub(crate) tail_call_enabled: bool,
    /// Resolved name of the word whose body is currently executing and is
    /// eligible for self-tail-call elimination. `Some` only inside a
    /// trampolined user-word frame.
    pub(crate) tail_self_word: Option<String>,
    /// True while executing a token section that sits in the tail position of
    /// the current word (set by the COND tail op for the selected clause body).
    pub(crate) in_tail_context: bool,
    /// Raised by the deferral site when a guarded tail self-call is recognized
    /// and skipped; consumed by the trampoline loop in `execute_word_core_inner`.
    pub(crate) tail_jump_pending: bool,

    /// When true (default), `compile_word_definition` lowers `COND` ops with
    /// statically-known clause blocks into `CompiledOp::CondDispatch`, so the
    /// per-call clause collect/clone/split is replaced by a precomputed jump
    /// table. Disable via `AJISAI_NO_COND_DISPATCH` for an A/B comparison.
    pub(crate) cond_dispatch_enabled: bool,

    /// When true (default), `compile_word_definition` lowers fully-literal
    /// vectors into a prebuilt `CompiledOp::PushVectorLiteral` instead of
    /// leaving them on the interpreter via `FallbackToken`. Disable via
    /// `AJISAI_NO_VECTOR_LITERAL` for an A/B comparison.
    pub(crate) vector_literal_enabled: bool,

    /// When true (default), precompiled COND clauses (`CondDispatch`) carry
    /// compiled guard/body sub-plans, so the loop body runs compiled instead of
    /// re-interpreted each iteration. Disable via `AJISAI_NO_COMPILED_CLAUSE`.
    pub(crate) compiled_clause_enabled: bool,

    /// When true (default), StackTop scalar-scalar arithmetic and comparison can
    /// bypass the tensor broadcast wrapper for bare scalars and same-shape
    /// singleton tensor/vector wrappers in Consume and Keep modes. Disable via
    /// `AJISAI_NO_SCALAR_FASTPATH` for A/B measurement.
    pub(crate) scalar_fastpath_enabled: bool,

    /// When true (default), `MAP` memoizes pure quantized kernel applications
    /// per element, reusing the result across repeated elements. Engages only
    /// for pure kernels and rational-scalar elements outside hedged modes;
    /// every other case runs the kernel unchanged. Disable via
    /// `AJISAI_NO_HOF_MEMO` for an A/B comparison.
    pub(crate) hof_memo_enabled: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::with_host(super::default_host_env())
    }

    pub fn with_host(host_env: Arc<dyn super::HostEnv>) -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            core_vocabulary: HashMap::new(),
            user_words: HashMap::new(),
            user_dictionaries: HashMap::new(),
            dependents: HashMap::new(),
            output_buffer: String::new(),
            host_effects: Vec::new(),
            host_env,
            definition_to_load: None,
            operation_target_mode: OperationTargetMode::StackTop,
            consumption_mode: ConsumptionMode::Consume,
            force_flag: false,
            disable_no_change_check: true,
            pending_tokens: None,
            pending_token_index: 0,
            module_state: HashMap::new(),
            import_table: ImportTable::default(),
            call_stack: SmallVec::new(),
            call_depth: 0,
            execution_step_count: 0,
            max_execution_steps: DEFAULT_MAX_EXECUTION_STEPS,
            input_buffer: String::new(),
            io_output_buffer: String::new(),
            serial_inbox: HashMap::new(),
            serial_disconnected: HashSet::new(),
            module_vocabulary: HashMap::new(),
            dictionary_dependencies: HashMap::new(),
            next_registration_order: 1,
            active_user_dictionary: "EXAMPLE".to_string(),
            global_epoch: 0,
            dictionary_epoch: 0,
            module_epoch: 0,
            execution_epoch: 0,
            semantic_registry: SemanticRegistry::new(),
            child_runtimes: HashMap::new(),
            next_child_id: 1,
            monitor_notifications: Vec::new(),
            next_supervisor_id: 1,
            runtime_metrics: RuntimeMetrics::default(),
            hedged_trace_log: Vec::new(),
            error_flow_trace_log: Vec::new(),
            force_no_quant: cfg!(feature = "force-no-quant"),

            // Elastic Engine
            elastic_mode: crate::elastic::ElasticMode::Greedy,
            elastic_cache: crate::elastic::CacheManager::new(),
            resolve_cache: HashMap::new(),
            validation_policy: ValidationPolicy::default(),
            owning_dictionary_context: None,
            word_identities: HashMap::new(),
            body_store: HashMap::new(),
            defer_identity_recompute: false,
            tail_call_enabled: std::env::var("AJISAI_NO_TAIL_CALL").is_err(),
            tail_self_word: None,
            in_tail_context: false,
            tail_jump_pending: false,
            cond_dispatch_enabled: std::env::var("AJISAI_NO_COND_DISPATCH").is_err(),
            vector_literal_enabled: std::env::var("AJISAI_NO_VECTOR_LITERAL").is_err(),
            compiled_clause_enabled: std::env::var("AJISAI_NO_COMPILED_CLAUSE").is_err(),
            scalar_fastpath_enabled: std::env::var("AJISAI_NO_SCALAR_FASTPATH").is_err(),
            hof_memo_enabled: std::env::var("AJISAI_NO_HOF_MEMO").is_err(),
        };
        crate::elastic::tracer::init_from_env();
        crate::builtins::register_builtins(&mut interpreter.core_vocabulary);
        interpreter
    }

    // ── Elastic Engine public API ─────────────────────────────────────────

    /// Set the execution mode (greedy / elastic-safe / elastic-force).
    #[cfg(feature = "elastic-engine")]
    pub fn set_elastic_mode(&mut self, mode: crate::elastic::ElasticMode) {
        self.elastic_mode = mode;
    }

    /// Without the `elastic-engine` cargo feature only `Greedy` is supported:
    /// requests for any other mode are ignored with a warning so the execution
    /// path stays exactly the greedy one.
    #[cfg(not(feature = "elastic-engine"))]
    pub fn set_elastic_mode(&mut self, mode: crate::elastic::ElasticMode) {
        if mode != crate::elastic::ElasticMode::Greedy {
            eprintln!(
                "[warn] Execution mode '{}' requires the 'elastic-engine' cargo feature; staying greedy.",
                mode
            );
            return;
        }
        self.elastic_mode = mode;
    }

    /// Read the current execution mode.
    pub fn elastic_mode(&self) -> crate::elastic::ElasticMode {
        self.elastic_mode
    }

    /// Enable or disable word-level tracing (`AJISAI_TRACE=1` equivalent).
    pub fn set_trace_enabled(&mut self, enabled: bool) {
        crate::elastic::tracer::set_enabled(enabled);
    }

    /// Returns `(hit_count, miss_count, hit_rate)` for the pure-result cache.
    pub fn elastic_cache_stats(&self) -> (u64, u64, f64) {
        (
            self.elastic_cache.hit_count(),
            self.elastic_cache.miss_count(),
            self.elastic_cache.hit_rate(),
        )
    }

    pub(crate) fn next_epoch(&mut self) -> u64 {
        self.global_epoch += 1;
        self.global_epoch
    }

    pub(crate) fn clear_resolve_cache(&mut self) {
        self.resolve_cache.clear();
        self.runtime_metrics.resolve_cache_invalidation_count += 1;
    }

    pub(crate) fn invalidate_execution_artifacts(&mut self) {
        self.clear_resolve_cache();
        self.elastic_cache.clear();
    }

    pub(crate) fn bump_dictionary_epoch(&mut self) {
        self.dictionary_epoch = self.next_epoch();
        self.invalidate_execution_artifacts();
        #[cfg(feature = "trace-epoch")]
        eprintln!(
            "[trace-epoch] dictionary_epoch={} global_epoch={}",
            self.dictionary_epoch, self.global_epoch
        );
    }

    pub(crate) fn bump_module_epoch(&mut self) {
        self.module_epoch = self.next_epoch();
        self.invalidate_execution_artifacts();
        #[cfg(feature = "trace-epoch")]
        eprintln!(
            "[trace-epoch] module_epoch={} global_epoch={}",
            self.module_epoch, self.global_epoch
        );
    }

    pub(crate) fn bump_execution_epoch(&mut self) {
        self.execution_epoch = self.next_epoch();
        #[cfg(feature = "trace-epoch")]
        eprintln!(
            "[trace-epoch] execution_epoch={} global_epoch={}",
            self.execution_epoch, self.global_epoch
        );
    }

    pub fn runtime_metrics(&self) -> RuntimeMetrics {
        self.runtime_metrics
    }

    pub fn push_hedged_trace(&mut self, message: impl Into<String>) {
        self.hedged_trace_log.push(message.into());
    }

    pub fn drain_hedged_trace(&mut self) -> Vec<String> {
        std::mem::take(&mut self.hedged_trace_log)
    }

    pub fn push_error_flow_trace(&mut self, event: super::error_flow_trace::ErrorFlowEvent) {
        self.error_flow_trace_log.push(event);
    }

    pub fn drain_error_flow_trace(&mut self) -> Vec<super::error_flow_trace::ErrorFlowEvent> {
        std::mem::take(&mut self.error_flow_trace_log)
    }

    pub fn peek_error_flow_trace(&self) -> &[super::error_flow_trace::ErrorFlowEvent] {
        &self.error_flow_trace_log
    }

    pub fn clear_error_flow_trace(&mut self) {
        self.error_flow_trace_log.clear();
    }

    pub fn current_epoch_snapshot(&self) -> EpochSnapshot {
        EpochSnapshot {
            global_epoch: self.global_epoch,
            dictionary_epoch: self.dictionary_epoch,
            module_epoch: self.module_epoch,
            execution_epoch: self.execution_epoch,
        }
    }

    pub(crate) fn update_operation_target_mode(&mut self, mode: OperationTargetMode) {
        self.operation_target_mode = mode;
    }

    pub(crate) fn update_consumption_mode(&mut self, mode: ConsumptionMode) {
        self.consumption_mode = mode;
    }

    pub(crate) fn reset_execution_modes(&mut self) {
        self.operation_target_mode = OperationTargetMode::StackTop;
        self.consumption_mode = ConsumptionMode::Consume;
    }

    pub(crate) fn normalize_symbol<'a>(symbol: &'a str) -> std::borrow::Cow<'a, str> {
        match symbol {
            "%" => std::borrow::Cow::Borrowed("MOD"),
            "&" => std::borrow::Cow::Borrowed("AND"),
            _ => {
                if symbol.as_bytes().iter().any(|b| b.is_ascii_lowercase()) {
                    std::borrow::Cow::Owned(symbol.to_uppercase())
                } else {
                    std::borrow::Cow::Borrowed(symbol)
                }
            }
        }
    }

    pub(crate) fn next_registration_order(&mut self) -> u64 {
        let order = self.next_registration_order;
        self.next_registration_order += 1;
        order
    }

    pub fn execute_reset(&mut self) -> Result<()> {
        self.stack.clear();
        self.core_vocabulary.clear();
        self.user_words.clear();
        self.user_dictionaries.clear();
        self.dependents.clear();
        self.output_buffer.clear();
        self.host_effects.clear();
        self.definition_to_load = None;
        self.reset_execution_modes();
        self.force_flag = false;
        self.pending_tokens = None;
        self.pending_token_index = 0;
        self.module_state.clear();
        self.call_stack.clear();
        self.call_depth = 0;
        self.tail_self_word = None;
        self.in_tail_context = false;
        self.tail_jump_pending = false;
        // `cond_dispatch_enabled` is a configuration flag, not run state, so it
        // is intentionally not reset here.
        self.owning_dictionary_context = None;
        self.word_identities.clear();
        self.body_store.clear();
        self.defer_identity_recompute = false;
        self.import_table.modules.clear();
        self.module_vocabulary.clear();
        self.dictionary_dependencies.clear();
        self.next_registration_order = 1;
        self.active_user_dictionary = "EXAMPLE".to_string();
        self.semantic_registry.clear();
        self.child_runtimes.clear();
        self.next_child_id = 1;
        self.monitor_notifications.clear();
        self.next_supervisor_id = 1;
        self.runtime_metrics = RuntimeMetrics::default();
        self.hedged_trace_log.clear();
        self.error_flow_trace_log.clear();
        crate::builtins::register_builtins(&mut self.core_vocabulary);
        Ok(())
    }

    pub fn collect_output(&mut self) -> String {
        std::mem::take(&mut self.output_buffer)
    }

    /// The ordered sequence of structured host effects produced so far. This is
    /// the language-independent observation channel used by the conformance
    /// suite, distinct from the human-readable `output_buffer`.
    pub fn host_effects(&self) -> &[super::HostEffect] {
        &self.host_effects
    }

    pub(crate) fn emit_host_effect(&mut self, effect: super::HostEffect) {
        self.host_env.emit_effect(&effect);
        self.host_effects.push(effect);
    }

    /// HostedEffect schema: capability.check → request construction → Eff append.
    ///
    /// The capability gate runs before the request builder, so missing-host
    /// failures emit only the structured diagnostic and do not let the word
    /// consume stack values or touch a host boundary. The builder constructs the
    /// structured effect payload (and may update the legacy output channel kept
    /// for adapters); the resulting `HostEffect` is then appended to the
    /// language-independent effect log.
    pub(crate) fn run_hosted_effect_schema<F>(
        &mut self,
        word: &str,
        capability: super::HostCapability,
        build_effect: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<super::HostEffect>,
    {
        self.require_host_capability(word, capability)?;
        let effect = build_effect(self)?;
        self.emit_host_effect(effect);
        Ok(())
    }

    pub(crate) fn require_host_capability(
        &mut self,
        word: &str,
        capability: super::HostCapability,
    ) -> Result<()> {
        if self.host_env.has_capability(capability) {
            return Ok(());
        }
        let payload = super::host::missing_capability_payload(word, capability);
        self.emit_host_effect(super::HostEffect::Diagnostic(payload));
        Err(super::host::missing_capability_error(word, capability))
    }

    pub fn get_stack(&self) -> &Stack {
        &self.stack
    }

    pub fn set_force_no_quant(&mut self, force_no_quant: bool) {
        self.force_no_quant = force_no_quant;
    }

    /// Enable or disable internal tail-call elimination (the guarded-tail-`COND`
    /// backward-jump trampoline). Default is on; this is the in-process
    /// equivalent of the `AJISAI_NO_TAIL_CALL` environment switch and exists so
    /// benchmarks can A/B the same interpreter against the legacy recursion path.
    pub fn set_tail_call_enabled(&mut self, enabled: bool) {
        self.tail_call_enabled = enabled;
    }

    /// Enable or disable precompiled COND clause dispatch (the internal "jump
    /// table"). In-process equivalent of `AJISAI_NO_COND_DISPATCH`; lets a
    /// benchmark A/B the compiled dispatch against the dynamic stack-collection
    /// path. Takes effect for word plans compiled after the change.
    pub fn set_cond_dispatch_enabled(&mut self, enabled: bool) {
        self.cond_dispatch_enabled = enabled;
    }

    /// Enable or disable compile-time lowering of fully-literal vectors. In-process
    /// equivalent of `AJISAI_NO_VECTOR_LITERAL`; takes effect for word plans
    /// compiled after the change.
    pub fn set_vector_literal_enabled(&mut self, enabled: bool) {
        self.vector_literal_enabled = enabled;
    }

    /// Enable or disable compiled COND guard/body sub-plans. In-process
    /// equivalent of `AJISAI_NO_COMPILED_CLAUSE`; takes effect for word plans
    /// compiled after the change.
    pub fn set_compiled_clause_enabled(&mut self, enabled: bool) {
        self.compiled_clause_enabled = enabled;
    }

    /// Enable or disable the D1 scalar-scalar arithmetic/comparison fast path.
    /// In-process equivalent of `AJISAI_NO_SCALAR_FASTPATH`; unlike compiled
    /// plan toggles this affects subsequent primitive executions immediately.
    pub fn set_scalar_fastpath_enabled(&mut self, enabled: bool) {
        self.scalar_fastpath_enabled = enabled;
    }

    /// Enable or disable pure HOF kernel memoization (`MAP`). In-process
    /// equivalent of `AJISAI_NO_HOF_MEMO`; lets a benchmark or differential
    /// test A/B the memoized path against re-running the kernel. Takes effect
    /// immediately for subsequent `MAP` calls.
    pub fn set_hof_memo_enabled(&mut self, enabled: bool) {
        self.hof_memo_enabled = enabled;
    }

    /// Override the execution step budget (water level). Raising it lets a
    /// benchmark drive a tail-recursive loop far past the default
    /// `DEFAULT_MAX_EXECUTION_STEPS` to observe O(1)-native-stack iteration.
    pub fn set_max_execution_steps(&mut self, steps: usize) {
        self.max_execution_steps = steps;
    }

    pub fn update_stack(&mut self, stack: Stack) {
        self.stack = stack;
        self.semantic_registry
            .normalize_to_stack_len(self.stack.len());
    }

    pub fn update_stack_with_hints(&mut self, stack: Stack, hints: Vec<Interpretation>) {
        self.stack = stack;
        self.semantic_registry.stack_hints = hints;
        self.semantic_registry
            .normalize_to_stack_len(self.stack.len());
    }

    pub fn collect_stack_hints(&self) -> &[Interpretation] {
        &self.semantic_registry.stack_hints
    }
}
