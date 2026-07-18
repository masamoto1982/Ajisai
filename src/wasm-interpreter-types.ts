
export type ExecutionMode =
    | 'greedy'
    | 'elastic-safe'
    | 'elastic-force'
    | 'hedged-safe'
    | 'hedged-trace';


export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

/** Detailed per-module import state: [module, importAllPublic, words, samples]. */
export type ImportStateEntry = [string, boolean, string[], string[]];

export interface UserWord {
    dictionary?: string | null;
    name: string;
    definition: string | null;
}

export interface AjisaiInterpreter {
    execute(code: string): Promise<ExecuteResult>;
    execute_step(code: string): ExecuteResult;
    reset(): ExecuteResult;
    // Session reset (Phase 5): reinitializes session state but keeps the
    // cross-reset compiled-artifact cache alive so an unchanged user word's
    // compiled plan is reused instead of recompiled. Optional so the GUI
    // degrades to a full `reset()` against a wasm bundle that predates the API.
    reset_session?(): ExecuteResult;
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
    restore_stack(stack_js: Value[]): void;
    restore_user_words(words: UserWord[]): void;
    remove_word(name: string): void;
    push_json_string(json: string): { status: string; message?: string };
    collect_imported_modules(): string[];
    collect_available_modules(): string[];
    collect_module_words_info(module_name: string): Array<[string, string | null]>;
    // Tuple shape: [shortName, description, imported]. Returns the
    // full module catalog (active + inactive words) regardless of import state.
    collect_module_catalog_words_info(module_name: string): Array<[string, string, boolean]>;
    collect_dictionary_dependencies(): Array<[string, string[], string[]]>;
    restore_imported_modules(modules: string[]): void;
    // Tuple shape: [module, importAllPublic, words, samples]. Captures partial
    // imports (IMPORT-ONLY / UNIMPORT-ONLY) that module-name lists cannot.
    collect_import_state(): ImportStateEntry[];
    restore_import_state(state: ImportStateEntry[]): void;
    set_execution_mode(mode: ExecutionMode): void;
    get_execution_mode(): ExecutionMode;
    // Execution step budget override (water level, SPECIFICATION.html §5.3).
    // Host-side runtime safety control, not a language semantic; the wasm
    // side ignores non-positive values and defaults to 100,000.
    set_max_execution_steps(steps: number): void;
    // Only exported by wasm bundles built with the opt-in `elastic-engine`
    // cargo feature; the default (trusted core) bundle omits it.
    collect_hedged_trace?(): string[];
    // Cost-model counters (SPECIFICATION.html §4.8): observational only,
    // session-cumulative, reset with the interpreter. Optional so the GUI
    // degrades gracefully against a wasm bundle that predates the API.
    collect_runtime_metrics?(): RuntimeMetricsSnapshot;
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
    // Cross-reset artifact cache (Phase 5): compiled plans reused across a GUI
    // session reset instead of being rebuilt. Optional so the GUI degrades
    // gracefully against a wasm bundle that predates the counters.
    artifactCacheBuildCount?: number;
    artifactCacheHitCount?: number;
    artifactCacheMissCount?: number;
    artifactCacheEvictionCount?: number;
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

export interface ProtocolAbsence {
    reason?: string;
    origin: string;
    recoverability: string;
    diagnosis?: ProtocolDiagnosis;
}

export interface ProtocolValueSemantics {
    semanticKind: string;
    shape: string;
    /**
     * Three-valued logic surface (SPEC §2.3, §7.5). Present only on
     * truth-valued values; `'true'` / `'false'` / `'unknown'`. This is the
     * only observable surface for the third value — do not infer it from
     * `semanticKind` or the internal NIL representation.
     */
    truthValue?: 'true' | 'false' | 'unknown';
    capabilities: string[];
    origin: string;
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
    definition_to_load?: string;
    inputHelper?: string;

    stack?: Value[];
    userWords?: UserWord[];
    importedModules?: string[];
    hedgedTrace?: string[];
    hedgedWinner?: string;
    hedgedFallbackReason?: string;
    hedgedCancelled?: string[];
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
