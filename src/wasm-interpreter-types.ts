
export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

export interface UserWord {
    dictionary?: string | null;
    name: string;
    definition: string | null;
}

export interface AjisaiInterpreter {
    execute(code: string): Promise<ExecuteResult>;
    execute_step(code: string): ExecuteResult;
    reset(): ExecuteResult;
    // Session reset: reinitializes session state but keeps the cross-reset
    // compiled-artifact cache alive so an unchanged user word's compiled plan
    // is reused instead of recompiled.
    reset_session(): ExecuteResult;
    collect_stack(): Value[];
    // Tuple shape: [dictionary, name, isProtected].
    collect_user_words_info(): Array<[string, string, boolean]>;
    // Content identity per user word (SPECIFICATION.html §8.6).
    // Tuple shape: [fullyQualifiedName, contentId].
    collect_word_identities(): Array<[string, string]>;
    // Tuple shape: [name, hover_summary, hover_syntax].
    // hover_summary is the native button title ("WORD — short verb phrase");
    // hover_syntax is the inline word-info preview (shortest useful invocation,
    // operands included). See docs/dev/three-layer-documentation-model.md §4.
    collect_core_words_info(): Array<[string, string, string]>;
    collect_core_listed_words_info(): Array<[string, string, string]>;
    collect_core_word_aliases_info(): Array<[string, string, string, string]>;
    collect_input_helper_words_info(): Array<[string, string]>;
    lookup_word_definition(name: string): string | null;
    // The one stack format persistence accepts (SPEC §2.3). `snapshot_stack`
    // captures exact values (CodeBlock, ExactScalar, …) that the observation
    // format used by `collect_stack` cannot round-trip, and
    // `restore_stack_snapshot` reinstates them. The payload is an opaque string.
    snapshot_stack(): string;
    restore_stack_snapshot(snapshot_json: string): void;
    // Discard every value on the stack, leaving the dictionary and the rest of
    // the session untouched. Clearing values is a host action, not a language
    // one — no Word does it — so it lives here rather than in the vocabulary.
    clear_stack(): void;
    restore_user_words(words: UserWord[]): void;
    remove_word(name: string): void;
    push_json_string(json: string): { status: string; message?: string };
    // Execution step budget override (water level, SPECIFICATION.html §5.3).
    // Host-side runtime safety control, not a language semantic; the wasm
    // side ignores non-positive values and defaults to 100,000.
    set_max_execution_steps(steps: number): void;
    // Cost-model counters (SPECIFICATION.html §4.8): observational only,
    // session-cumulative, reset with the interpreter.
    collect_runtime_metrics(): RuntimeMetricsSnapshot;
    // Serial RX inbox injection (SPECIFICATION.html §9.4). Filled before execution
    // from the platform serial adapter; drained by SERIAL@READ.
    update_serial_inbox(portId: string, bytes: Uint8Array): void;
    mark_serial_disconnected(portId: string): void;
    clear_serial_inboxes(): void;
}

/**
 * Cost-model counters as exposed by `collect_runtime_metrics()`
 * (SPECIFICATION.html §4.8). These are the machine-channel names; the GUI
 * renders them in the Reference cost-model vocabulary (fast lane, dense
 * vectors, comparison depth) and never shows these identifiers to users.
 * Counters are diagnostics: reading them changes no result.
 */
export interface RuntimeMetricsSnapshot {
    scalarFastpathCount: number;
    bulkKernelUseCount: number;
    simdKernelUseCount: number;
    tensorFlattenCount: number;
    tensorRebuildCount: number;
    sparseCandidateCount: number;
    compareWithinCount: number;
    compareWithinLazyCount: number;
    compareWithinUnknownCount: number;
    compareWithinBudgetTermsConsumed: number;
    // Cross-reset artifact cache: compiled plans reused across a GUI session
    // reset instead of being rebuilt.
    artifactCacheBuildCount: number;
    artifactCacheHitCount: number;
    artifactCacheMissCount: number;
    artifactCacheEvictionCount: number;
}

export interface ProtocolDiagnosis {
    when: string;
    where: {
        kind: string;
        word?: string;
        module?: string;
        dictionary?: string;
    };
    why: string;
    summary: string;
    evidence: string[];
    nextChecks: Array<{
        label: string;
        detail: string;
    }>;
    /**
     * CF-comparison agreed-prefix length (SPEC §4.5.0 / §7.4.1): the number
     * of leading partial quotients that matched before the partial-quotient
     * budget was exhausted on an `Unknown` (U) comparison result. Present
     * only on diagnoses produced by an undecidable continued-fraction
     * comparison (e.g. `COMPARE-WITHIN`). Machine-readable.
     */
    agreedPrefix?: number;
}

/**
 * The absence envelope the current protocol carries: the reason, plus the
 * diagnosis when the runtime produced one. An absence's origin and
 * recoverability are diagnostic state, not wire fields.
 */
export interface ProtocolAbsence {
    reason?: string;
    diagnosis?: ProtocolDiagnosis;
}

export interface ProtocolValueSemantics {
    /**
     * Three-valued logic surface (SPEC §2.3, §7.5). Present only on
     * truth-valued values; `'true'` / `'false'` / `'unknown'`. This is the
     * only observable surface for the third value — do not infer it from
     * the value's `type` or the internal NIL representation.
     */
    truthValue?: 'true' | 'false' | 'unknown';
    absence?: ProtocolAbsence;
    /**
     * Present and `true` only when this node's numeric `value` is a *best
     * rational approximation* of an exact irrational (`ExactScalar`) rendered
     * under a lossy role (e.g. `rawNumber`), rather than an exact rational
     * (SPEC §2.3). The exact source is available via the node's `semantics`.
     * Lossless `continuedFraction` rendering carries no `semantics` block and
     * never sets this. The GUI may use it to prefix an `≈`; consumers that
     * ignore it are unaffected (additive, optional).
     */
    approximate?: boolean;
    /**
     * The exact value of an algebraic irrational, as the multiquadratic normal
     * form Σ c·√r it is stored in (SPEC §4.2): one entry per term, ascending by
     * radicand, with radicand `'1'` keying the rational part. These pairs *are*
     * the number, so a host that draws them shows the exact value in a line —
     * `√3`, `1/2 + 1/3√5` — instead of choosing between a thirty-line continued
     * fraction and the approximation `approximate` marks. Absent on rationals
     * and on every non-scalar node. Additive and optional.
     */
    exactTerms?: ReadonlyArray<{
        readonly numerator: string;
        readonly denominator: string;
        readonly radicand: string;
    }>;
}

export interface ErrorFlowTraceEvent {
    kind: string;
    word?: string;
    absence?: ProtocolAbsence;
    stackLenBefore: number;
    stackLenAfter: number;
    message: string;
    diagnosis?: ProtocolDiagnosis;
}

export interface ExecuteResult {
    status: 'OK' | 'ERROR';
    output?: string;
    debugOutput?: string;
    message?: string;
    error?: boolean;
    hasMore?: boolean;
    /** Reconstructed `DEF` source of a User Word, for the editor to load. */
    definition_to_load?: string;
    /**
     * `LOOKUP` reference text for a Core Word, for the output area to display.
     * Separate from `definition_to_load` because the two want opposite
     * destinations — a definition is loaded to be edited, documentation is only
     * read — and routing both into the editor overwrote unsaved code.
     */
    documentation?: string;
    inputHelper?: string;

    // The observation-format stack, for display only.
    stack?: Value[];
    // The lossless snapshot (opaque string from `snapshot_stack`) attached by
    // the execution worker, and the only format used to sync the post-run stack
    // back into the main-thread interpreter, so exact values (CodeBlock,
    // ExactScalar) survive the round-trip instead of being flattened to nil or
    // a rational approximation. See SPEC §2.3.
    stackSnapshot?: string;
    userWords?: UserWord[];
    errorFlowTrace?: ErrorFlowTraceEvent[];

    // Per-run cost-model activity: the counter delta across this execution,
    // attached by the execution worker. Diagnostics only (SPEC §4.8); the
    // GUI renders it in cost-model vocabulary, collapsed by default.
    runtimeMetricsDelta?: RuntimeMetricsSnapshot;
}

export interface Fraction {
    numerator: string;
    denominator: string;
}

/**
 * Semantic interpretation role attached to a value. This is the meaning
 * the runtime assigned, not a formatting switch — rendering is derived
 * from (data, role). `unassigned` means no role was assigned and the
 * value is shown structurally with no heuristic guessing.
 */
export type Interpretation =
    | 'unassigned'
    | 'rawNumber'
    | 'interval'
    | 'text'
    | 'truthValue'
    | 'timestamp'
    | 'nil';

export interface Value {
    type: string;
    value: any | Fraction | Value[];
    displayHint?: Interpretation;
    semantics?: ProtocolValueSemantics;
}

export interface WasmModule {
    AjisaiInterpreter: AjisaiInterpreterClass;
    default?: () => Promise<any>;
    init?: () => Promise<any>;
    init_panic_hook?: () => void;
}
