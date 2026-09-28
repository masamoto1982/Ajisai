
export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

/**
 * One entry of `collect_user_words_info`: a User Word's name, and whether
 * another User Word calls it (DEL refuses it until that caller is gone).
 */
export type UserWordInfo = [name: string, hasDependents: boolean];

/**
 * One entry of `collect_core_words_info`. `hoverSummary` is the button title
 * ("WORD — short verb phrase"); `hoverSyntax` is the shortest useful
 * invocation, operands included. See docs/dev/three-layer-documentation-model.md §4.
 */
export type CoreWordInfo = [name: string, hoverSummary: string, hoverSyntax: string];

/** One term c·√r of an irrational's exact normal form (LANG.VALUES.EXACT). */
export interface ExactTerm {
    readonly numerator: string;
    readonly denominator: string;
    readonly radicand: string;
}

export interface UserWord {
    name: string;
    definition: string | null;
    // The `#:contract NAME ...` directive text `DEF` captured for this word
    // (a host affordance — the Dictionary panel's hover — never a checked
    // contract; see `execute_def::extract_pending_word_descriptions` on the
    // Rust side). `undefined`/absent for a word that was never given one.
    description?: string | null;
}

export interface AjisaiInterpreter {
    execute(code: string): Promise<ExecuteResult>;
    /**
     * The resource ceilings this interpreter is running under, as a JSON
     * `HostProfile`. Read from the live interpreter, so what a host displays
     * is what it enforces.
     */
    host_profile(): string;
    reset(): ExecuteResult;
    collect_stack(): Value[];
    collect_user_words_info(): UserWordInfo[];
    // Content identity per user word (LANG.AUTHORITY.FREEDOM).
    // Tuple shape: [fullyQualifiedName, contentId].
    collect_word_identities(): Array<[name: string, id: string]>;
    collect_core_words_info(): CoreWordInfo[];
    lookup_word_definition(name: string): string | null;
    // See `UserWord.description`.
    lookup_word_description(name: string): string | null;
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
    // The one stack format persistence accepts (LANG.OBSERVATION.FIREWALL). `snapshot_stack`
    // captures exact values (CodeBlock, ExactScalar, …) that the observation
    // format used by `collect_stack` cannot round-trip, and
    // `restore_stack_snapshot` reinstates them. The payload is an opaque string.
    snapshot_stack(): string;
    restore_stack_snapshot(snapshot_json: string): void;
    // Discard every value on the stack, leaving the dictionary and the rest of
    // the session untouched. Clearing values is a host action, not a language
    // one — no Word does it — so it lives here rather than in the vocabulary.
    clear_stack(): void;
    // Throws on a malformed word list: the Rust side returns
    // `Result<(), String>`, which wasm-bindgen compiles to a synchronous call
    // that throws the `Err` — a Promise comes only from an `async fn`. Not
    // `Promise<void>`, which would invite a `.catch` that dies on `undefined`
    // and an `await` reading as a suspension point where there is none:
    // `applyInterpreterSnapshot` calls this synchronously and needs the words
    // in the dictionary when it returns.
    restore_user_words(words: UserWord[]): void;
    remove_word(name: string): void;
    // Execution step budget override (water level, LANG.MACHINE.LIMITS).
    // Host-side runtime safety control, not a language semantic; the wasm
    // side ignores non-positive values and falls back to its own
    // `DEFAULT_MAX_EXECUTION_STEPS`.
    set_max_execution_steps(steps: number): void;

}

/**
 * The resource ceilings a host actually applies, published under the same
 * names every Ajisai host uses. LANG.MACHINE.LIMITS makes limits a host safety control
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
     * message. `null` for every other cause class.
     */
    resourceLimit?: {
        resource: string;
        limit: number;
        observed: number | null;
        /** How far a cumulative meter had got; present only for one. */
        progress?: { completed: number; total: number; unit: string };
    } | null;
}

/**
 * The absence envelope: the same one the CLI emits, since both hosts render
 * it with one serializer (spec/host-protocol.schema.json). `reason` is the
 * NIL's observable content (LANG.VALUES.NIL); `detail` is the text a
 * `userDeclared` reason carries; `origin` and `recoverability` are diagnostic
 * state beyond the reason.
 */
export interface ProtocolAbsence {
    reason?: string;
    detail?: string;
    origin?: string;
    recoverability?: string;
    diagnosis?: ProtocolDiagnosis;
}

export interface ProtocolValueSemantics {
    /**
     * Truth axis (LANG.VALUES.TRUTH): present only on a Boolean. UNKNOWN is a
     * NIL read in truth position and is observed as a NIL (`type: 'nil'`,
     * with its `absence`), never on this axis.
     */
    truthValue?: 'true' | 'false';
    absence?: ProtocolAbsence;
    /**
     * Present and `true` only when this node's numeric `value` is a *best
     * rational approximation* of an exact irrational (`ExactScalar`) rather
     * than an exact rational (LANG.OBSERVATION.FIREWALL). The GUI may use it
     * to prefix an `≈`.
     */
    approximate?: boolean;
    /**
     * The exact value of an algebraic irrational, as the multiquadratic normal
     * form Σ c·√r it is stored in (LANG.VALUES.EXACT): one entry per term, ascending by
     * radicand, with radicand `'1'` keying the rational part. These pairs *are*
     * the number, so a host that writes them shows the exact value in a line —
     * `sqrt(3)`, `1/2+1/3*sqrt(5)` — instead of choosing between a thirty-line continued
     * fraction and the approximation `approximate` marks. Absent on rationals
     * and on every non-scalar node. Additive and optional.
     */
    exactTerms?: ReadonlyArray<ExactTerm>;
}

export interface ErrorFlowTraceEvent {
    kind: string;
    word?: string;
    absence?: ProtocolAbsence;
    stackLenBefore: number;
    stackLenAfter: number;
    message: string;
    // Only on a `nilProduced` event: an ERROR's diagnosis is the result's
    // top-level `diagnosis`, which the error event does not repeat.
    diagnosis?: ProtocolDiagnosis;
}

export interface ExecuteResult {
    status: 'OK' | 'ERROR';
    output?: string;
    debugOutput?: string;
    message?: string;
    error?: boolean;
    /**
     * On an ERROR result, the same `aiDiagnostic` the CLI reports: `category`
     * is the failure's spec/outcomes.json error category, the machine-readable
     * name a host branches on instead of the display text in `message`;
     * `repair` is `'program'` exactly when the registry says so.
     */
    aiDiagnostic?: {
        category: string | null;
        repair?: 'program';
        word: string | null;
        family: string | null;
    } | null;
    /** On an ERROR result, its diagnosis — the one copy the report carries. */
    diagnosis?: ProtocolDiagnosis;

    // The observation-format stack, for display only.
    stack?: Value[];
    // The lossless snapshot (opaque string from `snapshot_stack`) attached by
    // the execution worker, and the only format used to sync the post-run stack
    // back into the main-thread interpreter, so exact values (CodeBlock,
    // ExactScalar) survive the round-trip instead of being flattened to nil or
    // a rational approximation. See LANG.OBSERVATION.FIREWALL.
    stackSnapshot?: string;
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

}

export interface Fraction {
    numerator: string;
    denominator: string;
}

/**
 * One observed stack value (LANG.OBSERVATION.PROTOCOL). Every field is derived
 * from the value itself: `type` is its domain.
 */
export interface Value {
    type: string;
    value: any | Fraction | Value[];
    semantics?: ProtocolValueSemantics;
}

export interface WasmModule {
    AjisaiInterpreter: AjisaiInterpreterClass;
    default?: () => Promise<any>;
    init?: () => Promise<any>;
    init_panic_hook?: () => void;
}
