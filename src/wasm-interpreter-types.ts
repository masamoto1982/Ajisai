
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
    /**
     * The resource ceilings this interpreter is running under, as a JSON
     * `HostProfile`. Read from the live interpreter, so what a host displays
     * is what it enforces.
     */
    host_profile(): string;
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
    /**
     * Answer the host's lookup of `name` against the current dictionary.
     *
     * A *query*, not a run: looking a Word up used to be the Word `LOOKUP`, so
     * asking what `ADD` does went through `execute` and came back on a field of
     * the result that no evaluation rule ever read. Asking here touches no
     * stack, no dictionary and no output.
     *
     * `documentation` is a Core Word's reference text, which is read, so it
     * belongs in the output area. `definition` is a User Word's reconstructed
     * `DEF`, which is edited, so it belongs in the editor — that is the point of
     * looking one up. `null` means the dictionary does not hold the name.
     */
    resolve_host_lookup(
        name: string
    ): { kind: 'documentation' | 'definition'; text: string } | null;
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

/**
 * The resource ceilings a host actually applies, published under the same
 * names every Ajisai host uses. SPEC §2.5 makes limits a host safety control
 * rather than value semantics, so hosts legitimately differ — which is only
 * safe to rely on when each one says what it applies.
 */
export interface HostProfile {
    profile: string;
    limits: Record<string, number>;
}

/** One display string in every locale the diagnosis vocabulary carries. */
export interface LocalizedText {
    en: string;
    ja: string;
}

/**
 * One repair step. `code` is the stable identifier to match on; `title` and
 * `detail` are display text and may be reworded or gain a locale without
 * anything downstream changing.
 */
export interface ProtocolDebugCheck {
    code: string;
    title: LocalizedText;
    detail: LocalizedText;
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
    nextChecks: ProtocolDebugCheck[];
    /**
     * Known Words closest to an unrecognized name, best match first. Empty
     * for every cause class other than `typoOrUnknownName`.
     */
    candidates?: string[];
    /**
     * Which declared ceiling a resource-limit failure crossed. `resource`
     * names an entry of the host's limit profile, so a reader can tell an
     * exhausted step budget from an oversized value without parsing a
     * message. Absent for every other cause class.
     */
    resourceLimit?: {
        resource: string;
        limit: number;
        observed?: number;
    };
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
    inputHelper?: string;

    // The observation-format stack, for display only.
    stack?: Value[];
    // The lossless snapshot (opaque string from `snapshot_stack`) attached by
    // the execution worker, and the only format used to sync the post-run stack
    // back into the main-thread interpreter, so exact values (CodeBlock,
    // ExactScalar) survive the round-trip instead of being flattened to nil or
    // a rational approximation. See SPEC §2.3.
    stackSnapshot?: string;
    /**
     * Why the lossless snapshot could not be taken, when the run itself
     * succeeded. Some values the interpreter computes cannot be persisted —
     * `PI` and anything built from it is a Tier-2 computable real, which the
     * snapshot codec refuses rather than encode lossily — and taking the
     * snapshot happens *after* the run, so its failure used to be reported as
     * if the program had failed: `PI` answered "cannot persist a Tier-2
     * computable exact real" and looked like a Word that does not work.
     * The run's own outcome is `status` above; this says only that its result
     * cannot be carried into the session.
     */
    stackSnapshotError?: string;
    userWords?: UserWord[];
    /**
     * On an ERROR result, the Words the failed run defined or deleted before it
     * failed. The result carries no `userWords`, so the host keeps its
     * pre-run dictionary and every one of these changes is thrown away — while
     * the `output` above still holds the `Defined word:` line each one printed.
     * The host reports these so those lines are corrected instead of left
     * standing; without it a failed run silently loses definitions and the log
     * claims Words the session does not contain.
     */
    discardedDictionaryChanges?: string[];
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
