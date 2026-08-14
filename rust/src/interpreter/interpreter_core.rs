use crate::error::Result;
use crate::types::{Interpretation, Stack, Token, Value, WordDefinition};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::epoch::EpochSnapshot;

/// Derived in `runtime_limits.rs`, alongside `DEFAULT_MAX_NUMERIC_WORK` and
/// `DEFAULT_MAX_COLLECTION_WORK` — see that constant's doc comment for the
/// full derivation. Mirrors the existing `MAX_MATERIALIZED_ELEMENTS`
/// re-export pattern below: the step budget's *default* is part of the same
/// host-time-budget derivation as the other accumulating ceilings even though
/// the field it initializes (`Interpreter::max_execution_steps`) is not part
/// of `RuntimeLimits`.
pub const DEFAULT_MAX_EXECUTION_STEPS: usize = super::runtime_limits::DEFAULT_MAX_EXECUTION_STEPS;

/// Cap on the user-word call stack depth. Hit before Rust's native call stack
/// runs out and panics the WASM module to an unrecoverable trap. The value is
/// set well above any reasonable hand-written nesting (the existing deep
/// non-recursive test uses 5 frames) but well below the Rust stack budget on
/// WASM and on the 2 MiB default native test-thread stack — every user-word
/// recursion expands to several Rust frames (execute_word_core_inner →
/// plan/structure runner → execution loop → resolve), so the empirical
/// safe ceiling is roughly 256 levels in debug builds. The execution-step
/// limit (`DEFAULT_MAX_EXECUTION_STEPS`) is still the primary backstop for
/// non-recursive runaway computation; this guard turns deep recursion
/// specifically into a recoverable AjisaiError instead of a trap, independent
/// of whatever `DEFAULT_MAX_EXECUTION_STEPS` happens to be — a stack overflow
/// is a depth problem, not a count problem, so raising the step budget must
/// not raise this ceiling with it.
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

/// Default cap on the number of elements a single generative built-in
/// (`RANGE`, `FILL`, ...) may materialize in one call. Such words loop
/// internally to build a vector/tensor, so they each count as a *single*
/// execution step and therefore bypass `DEFAULT_MAX_EXECUTION_STEPS`. Without
/// this guard an input like `[ 0 9999999999999 ] RANGE` or
/// `[ 1000000 1000000 7 ] FILL` drives an unbounded allocation that aborts the
/// process with an OOM instead of a diagnosable `AjisaiError`.
///
/// CS5: this is now the *default* for [`RuntimeLimits::max_materialized_elements`]
/// (the injectable per-interpreter limit the RANGE/FILL guards actually read);
/// the constant remains as the shared default and for the parallelization
/// space-waterline heuristic. Single source of truth lives in
/// [`super::runtime_limits::DEFAULT_MAX_MATERIALIZED_ELEMENTS`].
pub const MAX_MATERIALIZED_ELEMENTS: usize =
    super::runtime_limits::DEFAULT_MAX_MATERIALIZED_ELEMENTS;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsumptionMode {
    Consume,
    Keep,
}

#[derive(Debug, Clone)]
pub struct ResolveCacheEntry {
    pub resolved_name: String,
    pub dictionary_epoch: u64,
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
    pub max_validation_input_len: usize,
    pub warmup_runs: u64,
    /// Reaction to a compiled-vs-plain disagreement. Defaults to the safe
    /// `Fallback`; see `IntegrityMode`.
    pub integrity_mode: IntegrityMode,
}

impl Default for ValidationPolicy {
    fn default() -> Self {
        Self {
            max_validation_input_len: 16,
            warmup_runs: 3,
            integrity_mode: IntegrityMode::Fallback,
        }
    }
}

/// Counters describing how the runtime went about its work: which cache
/// answered, which fast path fired, how often a plan was rebuilt.
///
/// These are optimizer observations and nothing else. What a *ceiling* measures
/// does not live here — it lives on the interpreter, where the ceiling reads
/// it, and reaches an observer through `Interpreter::resource_usage`. That
/// separation is the point: this struct used to carry an `execution_steps`
/// field that no code ever wrote, sitting beside the real
/// `Interpreter::execution_step_count` that every limit check increments. Two
/// counters for one fact, and the one that got reported was the one that was
/// always zero — so a 21,000-step fold and an empty program described their
/// work identically.
#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeMetrics {
    pub compiled_plan_build_count: u64,
    pub compiled_plan_cache_hit_count: u64,
    pub compiled_plan_cache_miss_count: u64,
    pub cond_dispatch_fast_count: u64,
    pub cond_clause_compiled_count: u64,
    pub scalar_fastpath_count: u64,
    pub resolve_cache_hit_count: u64,
    pub resolve_cache_miss_count: u64,
    pub resolve_cache_invalidation_count: u64,
    pub tail_call_jump_count: u64,
}

/// What this run spent of the budgets that can refuse it.
///
/// Every field here is the *same* value the corresponding ceiling compares
/// against — read from the counter the check reads, not from a parallel copy —
/// and every field's name is a key of the host's declared limit profile. An
/// agent that plans against `mcp.limits` can therefore subtract.
///
/// Deliberately only the two that exist. `bigintBits` and `algebraicTerms` are
/// checked per result and never accumulated, so there is no peak to report;
/// inventing fields for them would mean reporting a number nothing measured,
/// which is the defect this type was introduced to fix.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceUsage {
    /// Words executed, against `executionSteps`.
    pub execution_steps: u64,
    /// Internal arithmetic work charged, against `numericWork`.
    pub numeric_work: u64,
    /// Element operations charged inside collection Words, against
    /// `collectionWork`.
    pub collection_work: u64,
}

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) core_vocabulary: HashMap<String, Arc<WordDefinition>>,
    pub(crate) user_words: HashMap<String, Arc<WordDefinition>>,
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
    /// Local name bindings, innermost frame last (`super::bindings`). Not a
    /// second dictionary: each scope belongs to one frame of LANG.SOURCE.FRAME
    /// and dies with it, so a name here is never reachable from another Word.
    pub(crate) binding_scopes: Vec<super::bindings::BindingScope>,
    /// The dictionary changes this top-level `execute` has made so far, in
    /// order — `Defined word:` / `Deleted word:` without the prose.
    ///
    /// A host that treats a failed run as committing nothing needs to say so:
    /// the run still printed a success line for every `DEF` it reached, and
    /// leaving those standing while discarding the Words is a log that claims
    /// something the session does not contain. The tester who found this lost
    /// seven definitions and only noticed when `LOOKUP` answered
    /// `Unknown word`. Cleared at the start of each top-level run.
    pub(crate) dictionary_changes_this_run: Vec<String>,
    pub(crate) consumption_mode: ConsumptionMode,
    pub(crate) disable_no_change_check: bool,
    pub(crate) pending_tokens: Option<Vec<Token>>,
    pub(crate) pending_token_index: usize,
    /// Type-erased caches owned by runtime subsystems.
    pub(crate) runtime_scratch: HashMap<String, Box<dyn std::any::Any + Send>>,
    pub(crate) call_stack: SmallVec<[String; 5]>,
    /// User-word call depth. Incremented on entry to a user-word body in
    /// `execute_word_core_inner`, decremented on exit. Compared against
    /// `MAX_USER_WORD_DEPTH` to prevent a deep recursion from blowing the
    /// Rust call stack and trapping the WASM module.
    pub(crate) call_depth: usize,
    pub(crate) execution_step_count: usize,
    pub(crate) max_execution_steps: usize,
    /// Unified internal-computation-cost ceilings (CS5). The step budget above
    /// prices word count; these price the per-word work it cannot see
    /// (materialization, source/literal size, and — via the work meter —
    /// algebraic/BigInt blow-up). Child runtimes inherit a copy.
    pub(crate) runtime_limits: super::runtime_limits::RuntimeLimits,
    /// Cumulative internal numeric work charged this `execute` (CS5 work meter).
    /// Charged before each expensive exact-arithmetic operation so a runaway
    /// algebraic computation fails at `runtime_limits.max_numeric_work` rather
    /// than running for minutes. Reset per top-level `execute`.
    pub(crate) numeric_work_used: u64,
    /// Cumulative collection work charged this `execute`. A collection Word
    /// loops inside Rust and costs one execution step however many elements it
    /// touches, so this is the only counter that can see it. Reset per
    /// top-level `execute`, like the numeric meter beside it.
    pub(crate) collection_work_used: u64,

    pub(crate) next_registration_order: u64,

    pub(crate) global_epoch: u64,
    pub(crate) dictionary_epoch: u64,
    pub(crate) execution_epoch: u64,

    pub(crate) monitor_notifications: Vec<Vec<Value>>,
    pub(crate) next_supervisor_id: u64,

    pub(crate) runtime_metrics: RuntimeMetrics,
    pub(crate) error_flow_trace_log: Vec<super::error_flow_trace::ErrorFlowEvent>,

    // ── Elastic Engine (MVP) ──────────────────────────────────────────────
    pub(crate) resolve_cache: HashMap<String, ResolveCacheEntry>,

    /// Owning user dictionary of the word currently being defined,
    /// dependency-scanned, or executed. Bare names resolve through this
    /// dictionary's words first (Section 8.6), so an imported word group is
    /// self-referential regardless of which other dictionaries are loaded.
    /// `None` at top level, where resolution falls back to the global order.

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

    // ── Source positions ──────────────────────────────────────────────────
    /// Where each token of the program currently running was written,
    /// index-aligned with its token stream. Empty for any entry point that did
    /// not come from source text.
    pub(crate) source_spans: Vec<crate::tokenizer::SourceSpan>,
    /// Nesting depth of `execute_section_core`. Depth 1 is the program's own
    /// token stream — the only stream `source_spans` describes — so the cursor
    /// below is maintained there and nowhere else.
    pub(crate) section_depth: usize,
    /// The top-level token being executed. An error raised anywhere beneath it
    /// is attributed here, which is what turns "Stack underflow" into
    /// "Stack underflow at line 4, column 12, in COND".
    pub(crate) current_source_span: Option<crate::tokenizer::SourceSpan>,

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

    /// When true (default), compiled builtin call sites keep a monomorphic
    /// shape cache that routes scalar-fastpath-shaped operands straight to
    /// the D1 fast path (hidden-class-style call-site specialization; see
    /// `shape_ic.rs`). Routing only — observable values are unchanged.
    /// Disable via `AJISAI_NO_SHAPE_IC` for an A/B comparison.

    /// When true (default), `MAP`/`FILTER`/`FOLD` and the predicate family may
    /// route eligible quantized blocks through the specialized kernels in
    /// `higher_order/fast_kernels.rs` (per-element and bulk). Routing only —
    /// the kernels decline any input whose outcome the generic route defines
    /// differently (e.g. division by zero), so observable values, errors, and
    /// NIL reasons are unchanged. Disable via `AJISAI_NO_FAST_KERNEL` for an
    /// A/B comparison.
    pub(crate) fast_kernel_enabled: bool,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self::with_host(super::default_host_env())
    }

    pub fn with_host(host_env: Arc<dyn super::HostEnv>) -> Self {
        let mut interpreter = Interpreter {
            stack: Stack::new(),
            core_vocabulary: HashMap::new(),
            user_words: HashMap::new(),
            dependents: HashMap::new(),
            output_buffer: String::new(),
            host_effects: Vec::new(),
            host_env,
            binding_scopes: vec![super::bindings::BindingScope::root()],
            dictionary_changes_this_run: Vec::new(),
            consumption_mode: ConsumptionMode::Consume,
            disable_no_change_check: true,
            pending_tokens: None,
            pending_token_index: 0,
            runtime_scratch: HashMap::new(),
            call_stack: SmallVec::new(),
            call_depth: 0,
            execution_step_count: 0,
            max_execution_steps: DEFAULT_MAX_EXECUTION_STEPS,
            runtime_limits: super::runtime_limits::RuntimeLimits::default(),
            numeric_work_used: 0,
            collection_work_used: 0,
            next_registration_order: 1,
            global_epoch: 0,
            dictionary_epoch: 0,
            execution_epoch: 0,
            monitor_notifications: Vec::new(),
            next_supervisor_id: 1,
            runtime_metrics: RuntimeMetrics::default(),
            error_flow_trace_log: Vec::new(),

            // Elastic Engine
            resolve_cache: HashMap::new(),
            word_identities: HashMap::new(),
            body_store: HashMap::new(),
            defer_identity_recompute: false,
            tail_call_enabled: std::env::var("AJISAI_NO_TAIL_CALL").is_err(),
            tail_self_word: None,
            in_tail_context: false,
            tail_jump_pending: false,
            source_spans: Vec::new(),
            section_depth: 0,
            current_source_span: None,
            cond_dispatch_enabled: std::env::var("AJISAI_NO_COND_DISPATCH").is_err(),
            vector_literal_enabled: std::env::var("AJISAI_NO_VECTOR_LITERAL").is_err(),
            compiled_clause_enabled: std::env::var("AJISAI_NO_COMPILED_CLAUSE").is_err(),
            scalar_fastpath_enabled: std::env::var("AJISAI_NO_SCALAR_FASTPATH").is_err(),
            fast_kernel_enabled: std::env::var("AJISAI_NO_FAST_KERNEL").is_err(),
        };
        crate::builtins::register_builtins(&mut interpreter.core_vocabulary);
        interpreter
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
        self.clear_word_contract_cache();
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

    pub fn push_error_flow_trace(&mut self, event: super::error_flow_trace::ErrorFlowEvent) {
        self.error_flow_trace_log.push(event);
    }

    pub fn drain_error_flow_trace(&mut self) -> Vec<super::error_flow_trace::ErrorFlowEvent> {
        std::mem::take(&mut self.error_flow_trace_log)
    }

    pub fn peek_error_flow_trace(&self) -> &[super::error_flow_trace::ErrorFlowEvent] {
        &self.error_flow_trace_log
    }

    /// Record `word` as the Word a failure happened *inside*, when the frame
    /// below already recorded that same failure under the name of the Word
    /// that raised it. Answers whether it did.
    ///
    /// An error unwinds through every frame it passed through, and each of
    /// those frames used to record it again under its own name — so the last
    /// event, the one every reader takes, named the outermost Word. For
    /// `[ 1 2 ] { 'x' 1 ADD } MAP` that was `MAP`, and the reader was told to
    /// check `MAP`'s expected shape for a failure `ADD` raised about its own
    /// operand. Keeping the innermost attribution and folding the outer frames
    /// into `insideWords` says both things: what failed, and where it was
    /// written.
    pub(crate) fn attribute_enclosing_word(&mut self, word: &str, error_text: &str) -> bool {
        let Some(last) = self.error_flow_trace_log.last_mut() else {
            return false;
        };
        if last.kind != super::error_flow_trace::ErrorFlowEventKind::WordError
            || last.error_text != error_text
        {
            return false;
        }
        // The frame that raised it and the frame recording it are the same
        // Word: a User Word records its own failure at the call boundary, and
        // the loop that ran the call then reaches here. Nothing to add — it is
        // not written inside itself.
        if last.word.as_deref() == Some(word) {
            return true;
        }
        if let Some(diagnosis) = last.diagnosis.as_mut() {
            diagnosis.with_enclosing_word(word);
        }
        true
    }

    /// Record a Word's failure in the error-flow trace, attributed to that
    /// Word and positioned at the top-level token that reached it.
    pub(crate) fn record_word_failure(
        &mut self,
        word: &str,
        err: &crate::error::AjisaiError,
        stack_len_before: usize,
    ) {
        use super::debug_diagnosis::DebugDiagnosis;
        use super::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
        let stack_len_after = self.stack.len();
        let mut diagnosis =
            DebugDiagnosis::from_error(err, Some(word), stack_len_before, stack_len_after)
                .with_source_position(self.current_source_span);
        // The compiled-in registry cannot know a user Word, and a misspelled
        // user Word is exactly the case a fresh vocabulary lookup misses. This
        // is the one place that holds the live dictionary.
        diagnosis.with_user_vocabulary(self.user_words.keys().map(String::as_str));
        self.push_error_flow_trace(ErrorFlowEvent {
            kind: ErrorFlowEventKind::WordError,
            word: Some(word.to_string()),
            error_category: Some(crate::error::ErrorCategory::from_error(err)),
            absence: None,
            stack_len_before,
            stack_len_after,
            message: format!("word error word={} error={}", word, err),
            diagnosis: Some(diagnosis),
            error_text: err.to_string(),
        });
    }

    pub fn clear_error_flow_trace(&mut self) {
        self.error_flow_trace_log.clear();
    }

    pub fn current_epoch_snapshot(&self) -> EpochSnapshot {
        EpochSnapshot {
            global_epoch: self.global_epoch,
            dictionary_epoch: self.dictionary_epoch,
            execution_epoch: self.execution_epoch,
        }
    }
    pub(crate) fn update_consumption_mode(&mut self, mode: ConsumptionMode) {
        self.consumption_mode = mode;
    }

    pub(crate) fn reset_execution_modes(&mut self) {
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

    /// Effect schema: request construction → effect append.
    ///
    /// Output is the only effect that reaches a host (LANG.EFFECTS.OUTPUT), so
    /// there is no capability to gate on: the builder constructs the structured payload
    /// (and may update the legacy output channel kept for adapters), and the
    /// resulting `HostEffect` is appended to the effect log in request order.
    pub(crate) fn run_effect_schema<F>(&mut self, build_effect: F) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<super::HostEffect>,
    {
        let effect = build_effect(self)?;
        self.emit_host_effect(effect);
        Ok(())
    }

    pub fn get_stack(&self) -> &Stack {
        &self.stack
    }

    /// The Words the current top-level run has defined or deleted, in order.
    /// A host that discards a failed run's state reports these so its own
    /// success lines are corrected rather than left standing.
    pub fn dictionary_changes_this_run(&self) -> &[String] {
        &self.dictionary_changes_this_run
    }
    /// Enable or disable internal tail-call elimination (the guarded-tail-`COND`
    /// backward-jump trampoline). Default is on; this is the in-process
    /// equivalent of the `AJISAI_NO_TAIL_CALL` environment switch and exists so
    /// benchmarks can A/B the same interpreter against the legacy recursion path.
    /// Where in the source the interpreter is, as of the last top-level token
    /// it began. `None` for an entry point that did not come from source text.
    pub fn current_source_position(&self) -> Option<crate::tokenizer::SourceSpan> {
        self.current_source_span
    }

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

    /// Enable or disable the call-site shape inline cache for compiled
    /// builtin calls. In-process equivalent of `AJISAI_NO_SHAPE_IC`; takes
    /// effect immediately for subsequent compiled call sites. Routing only —
    /// disabling it never changes observable values, just the route taken.
    pub fn set_shape_ic_enabled(&mut self, _enabled: bool) {}

    /// Enable or disable pure HOF kernel memoization (`MAP`). In-process
    /// equivalent of `AJISAI_NO_HOF_MEMO`; lets a benchmark or differential
    /// test A/B the memoized path against re-running the kernel. Takes effect
    /// immediately for subsequent `MAP` calls.
    /// Enable or disable the specialized HOF kernels (per-element and bulk)
    /// in `higher_order/fast_kernels.rs`. In-process equivalent of
    /// `AJISAI_NO_FAST_KERNEL`; lets a differential test or benchmark A/B the
    /// kernel route against the generic quantized-block route. Routing only —
    /// disabling it never changes observable values, errors, or NIL reasons.
    pub fn set_fast_kernel_enabled(&mut self, enabled: bool) {
        self.fast_kernel_enabled = enabled;
    }

    /// Override the execution step budget (water level). Raising it lets a
    /// benchmark drive a tail-recursive loop far past the default
    /// `DEFAULT_MAX_EXECUTION_STEPS` to observe O(1)-native-stack iteration.
    /// The step budget in force. A host that publishes its limit profile has
    /// to be able to read the value it is actually running under, not restate
    /// the default and hope no one changed it.
    pub fn max_execution_steps(&self) -> usize {
        self.max_execution_steps
    }

    pub fn set_max_execution_steps(&mut self, steps: usize) {
        self.max_execution_steps = steps;
    }

    /// The unified internal-computation-cost ceilings (CS5) in force.
    pub fn runtime_limits(&self) -> &super::runtime_limits::RuntimeLimits {
        &self.runtime_limits
    }

    /// Internal numeric work charged by this `execute`, in limb-multiply units.
    ///
    /// The meter's price is only meaningful if the rate it charges at can be
    /// *measured* rather than asserted, and a rate needs both halves: the units
    /// and the wall clock. `examples/work_meter_calibration.rs` reads this one.
    pub fn numeric_work_used(&self) -> u64 {
        self.numeric_work_used
    }

    /// Collection work charged by this `execute`, in element-operation units.
    ///
    /// Read by `examples/collection_word_calibration.rs` for the same reason
    /// the numeric counter is read by its own calibration: a price is only
    /// meaningful against a measured rate.
    pub fn collection_work_used(&self) -> u64 {
        self.collection_work_used
    }

    /// What this run spent of the budgets that can refuse it.
    ///
    /// Read from the counters the ceilings themselves read. There is no second
    /// copy to fall out of step, because reporting a resource and enforcing it
    /// are two readings of one number — which is the whole content of this
    /// method, and the reason it exists.
    pub fn resource_usage(&self) -> ResourceUsage {
        ResourceUsage {
            execution_steps: self.execution_step_count as u64,
            numeric_work: self.numeric_work_used,
            collection_work: self.collection_work_used,
        }
    }

    /// Override the internal-computation-cost ceilings. Used by tests to inject
    /// small limits that fire a guard without allocating anything huge, and by
    /// hosts that need a tighter or looser envelope. Child runtimes spawned
    /// afterwards inherit the new limits.
    pub fn set_runtime_limits(&mut self, limits: super::runtime_limits::RuntimeLimits) {
        self.runtime_limits = limits;
    }

    /// Charge `units` of internal numeric work to the CS5 work meter and fail
    /// (diagnosably, before the expensive computation runs) if the cumulative
    /// total crosses `runtime_limits.max_numeric_work`. Saturating so the
    /// counter itself can never overflow. Reports itself by name through
    /// `ResourceLimitExceeded`, so an agent can tell an exhausted work meter
    /// from an exhausted step budget without reading the message.
    pub(crate) fn charge_numeric_work(&mut self, units: u64) -> Result<()> {
        self.numeric_work_used = self.numeric_work_used.saturating_add(units);
        if self.numeric_work_used > self.runtime_limits.max_numeric_work {
            return Err(crate::error::AjisaiError::ResourceLimitExceeded {
                resource: crate::error::ResourceLimit::NumericWork,
                limit: self.runtime_limits.max_numeric_work,
                observed: Some(self.numeric_work_used),
                progress: None,
            });
        }
        Ok(())
    }

    pub fn update_stack(&mut self, stack: impl Into<Stack>) {
        self.stack = stack.into();
    }

    pub fn update_stack_with_hints(&mut self, values: Vec<Value>, hints: Vec<Interpretation>) {
        self.stack = Stack::from_values_and_roles(values, hints);
    }

    pub fn collect_stack_hints(&self) -> &[Interpretation] {
        self.stack.roles()
    }
}
