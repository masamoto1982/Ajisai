import {
    applyInterpreterSnapshot,
    ExecutionAbortedError,
    ExecutionTimeoutError,
    type InterpreterSnapshot
} from '../workers/execution-contract';
import { getPlatform } from '../platform';
import type {
    AjisaiInterpreter,
    ExecuteResult,
    ProtocolDiagnosis,
    UserWord,
    Value
} from '../wasm-interpreter-types';
import type { ExecutionSurfaceChanges } from './gui-layout-state';

/** What was thrown, as an Error: a thrown non-Error is wrapped, never cast. */
export const toError = (thrown: unknown): Error =>
    thrown instanceof Error ? thrown : new Error(String(thrown));

// A Record crosses the protocol as two aligned arrays of nodes
// (LANG.RECORDS.STRUCTURE); either may be missing or malformed on an untrusted
// payload, and then it reads as empty.
export const readRecordParts = (value: unknown): { keys: Value[]; values: Value[] } => {
    const record = value as { keys?: Value[]; values?: Value[] } | null;
    return {
        keys: Array.isArray(record?.keys) ? record.keys : [],
        values: Array.isArray(record?.values) ? record.values : []
    };
};

// ── Diagnosis report ────────────────────────────────────────────────────────
// The one place that knows what a diagnosis looks like when it is read.
//
// The `[DIAGNOSIS]` heading, the three numbered questions and the `next:`
// lines are a presentation contract: a reader learns the shape once and then
// reads every refusal the same way. The shape is written once, here, so
// renaming `Q3 why:` or adding a fourth question moves every diagnosis at
// once — including the wall-clock timeout, the one refusal the interpreter
// never gets to explain (the playground terminates the worker where it stands,
// so no diagnosis arrives with the result) and the one a reader is least
// likely to have seen before.
//
// So the timeout builds a `ProtocolDiagnosis` like any other and renders
// through here. What is genuinely its own — a ceiling with no observed value,
// enforced by the host rather than the language — arrives as `extraLines`
// rather than as a second renderer.

const evidenceValue = (
    entries: readonly string[] | undefined,
    key: string
): string | null => {
    const hit = entries?.find((entry) => entry.startsWith(`${key}=`));
    return hit ? hit.slice(key.length + 1) : null;
};

export interface DiagnosisReportContext {
    /**
     * Stack depth at the failing step, from the error-flow event that carried
     * the diagnosis. Absent when the diagnosis did not come from a step (the
     * host-guard case).
     */
    readonly stackLenBefore?: number;
    /**
     * Lines to print where a protocol `resourceLimit` would go, for a ceiling
     * the protocol cannot describe.
     */
    readonly extraLines?: readonly string[];
}

export const renderDiagnosisReport = (
    diagnosis: ProtocolDiagnosis,
    context: DiagnosisReportContext = {}
): string => {
    const where = diagnosis.where.word
        ? `${diagnosis.where.word} (${diagnosis.where.kind})`
        : diagnosis.where.kind;
    const depth =
        typeof context.stackLenBefore === 'number'
            ? `, stack depth ${context.stackLenBefore}`
            : '';
    // Where in the source the run was when it failed. The host records it
    // as evidence — the same `key=value` channel `stackLenBefore` uses.
    const sourceLine = evidenceValue(diagnosis.evidence, 'sourceLine');
    const sourceColumn = evidenceValue(diagnosis.evidence, 'sourceColumn');
    const at = sourceLine
        ? ` at line ${sourceLine}${sourceColumn ? `, column ${sourceColumn}` : ''}`
        : '';
    // The Words the failure happened *inside*, innermost first. A block and
    // a Word body are each their own token stream with no source of their
    // own, so the position above is the top-level token that reached the
    // failure; this says which construct the failing word was written in.
    const insideWords = evidenceValue(diagnosis.evidence, 'insideWords');
    const inside = insideWords ? `, inside ${insideWords.split(',').join(', ')}` : '';
    // The known Words closest to a name that did not resolve. Telling a
    // reader to check the spelling without saying what it might have been
    // is the one hint nobody can act on.
    const candidates = diagnosis.candidates?.length
        ? [`did you mean: ${diagnosis.candidates.join(', ')}`]
        : [];
    // Which declared ceiling fired, so "too big" says what was too big.
    const limit = diagnosis.resourceLimit
        ? [
              `limit ${diagnosis.resourceLimit.resource}: ${
                  diagnosis.resourceLimit.observed ?? '?'
              } against ${diagnosis.resourceLimit.limit}`
          ]
        : [];
    return [
        `[DIAGNOSIS] ${diagnosis.summary}`,
        `Q1 when: ${diagnosis.when}`,
        `Q2 where: ${where}${inside}${at}${depth}`,
        `Q3 why: ${diagnosis.why}`,
        ...candidates,
        ...limit,
        ...(context.extraLines ?? []),
        // One locale per line. Each check carries both; the playground's own
        // text is English (`<html lang="en">`), so English is the side that
        // matches its surroundings, and the `ja` half stays in the protocol
        // for a host that renders in Japanese.
        ...diagnosis.nextChecks.map((check) => `next: ${check.title.en} - ${check.detail.en}`)
    ].join('\n');
};

// ── Snapshot, sync and the run's own explanations ───────────────────────────

// Every User word with its definition and description, looked up by name.
// `restore_user_words` skips a definition-less word, so a lookup that missed
// here would run the worker without the user's words.
export const collectUserWords = (interpreter: AjisaiInterpreter): UserWord[] =>
    interpreter.collect_user_words_info().map(([name]) => ({
        name,
        definition: interpreter.lookup_word_definition(name),
        description: interpreter.lookup_word_description(name)
    }));

// The one reading of a result's success. The host sets `status` and `error`
// together; either says the run did not complete.
export const isFailure = (result: ExecuteResult): boolean =>
    result.status !== 'OK' || Boolean(result.error);

// What the worker needs to start a run from the main thread's interpreter.
// `userWords` may be passed in when the caller has just collected them for
// its own before/after view, so the dictionary is read once per run.
export const createExecutionSnapshot = (
    interpreter: AjisaiInterpreter,
    userWords: UserWord[] = collectUserWords(interpreter)
): InterpreterSnapshot => ({
    // Carry the lossless snapshot into the worker so exact values on the
    // stack (CodeBlock, ExactScalar) are not flattened by the observation
    // format before this run executes (LANG.OBSERVATION.FIREWALL).
    stackSnapshot: interpreter.snapshot_stack(),
    userWords,
    // Host-configured step budget (LANG.MACHINE.LIMITS water level); undefined
    // keeps the interpreter's own default (`DEFAULT_MAX_EXECUTION_STEPS`).
    stepLimit: getPlatform().executionConfig.stepLimit
});

// What a failed run printed, with its dictionary claims corrected.
//
// `syncInterpreterState` below ignores an ERROR result, so the session keeps
// its pre-run dictionary and every `DEF` the failed run reached is discarded —
// but each of those printed `Defined word: X` on its way through, and those
// lines are still in the output the error path shows. A reader who believes
// them finds out only when `LOOKUP` answers `Unknown word` for something the
// log says exists. The correction goes below them, where it cancels what they
// claimed.
export const describeFailedRunOutput = (result: ExecuteResult): string => {
    const output = result.output || '';
    const discarded = result.discardedDictionaryChanges ?? [];
    if (discarded.length === 0) return output;
    const plural = discarded.length === 1 ? '' : 's';
    const correction =
        `Rolled back ${discarded.length} dictionary change${plural}: ${discarded.join(', ')}. ` +
        'The run failed, so the dictionary is unchanged and the lines above it do not hold.';
    return output ? `${output.replace(/\n*$/, '')}\n${correction}` : correction;
};

// The diagnosis a wall-clock stop can answer with.
//
// Every other refusal is built by the interpreter, which knows the Word, the
// position and the ceiling. This one is not: the playground terminates the
// worker where it stands, so nothing of the run survives to be diagnosed. What
// is knowable here is knowable without the run — which guard fired, that it is
// the host's and not the language's, and what makes a program fit inside it.
//
// It is assembled as a `ProtocolDiagnosis` and rendered by
// `renderDiagnosisReport`, the same way a diagnosis that arrives from the
// interpreter is, so the reading format stays one thing. Only the ceiling line
// is its own, because a wall-clock stop has no observed value to report
// against the limit and no Word to attribute it to.
const TIMEOUT_DIAGNOSIS: ProtocolDiagnosis = {
    when: 'hostGuard',
    // `playground` is the host, not a Word: the guard belongs to the page that
    // owns the worker, and naming it here is what tells a reader the language
    // did not refuse their program.
    where: { kind: 'hostEnvironment', word: 'playground' },
    why: 'resourceLimit',
    summary: 'hostGuard / playground (hostEnvironment) / resourceLimit (executionTimeout)',
    evidence: [],
    nextChecks: [
        {
            code: 'checkWhichGuardStopped',
            title: { en: 'Check which guard stopped it', ja: 'どのガードが止めたか確認する' },
            detail: {
                en: 'The playground stops a run on wall-clock time. '
                    + "The interpreter's own budgets (execution steps, materialized elements, "
                    + 'numeric work) did not refuse this program; it was still running when the '
                    + 'time ran out.',
                ja: 'プレイグラウンドは実時間で実行を停止する。インタプリタ自身の予算（実行ステップ、'
                    + '実体化要素数、数値処理量）はこのプログラムを拒否していない。時間切れの時点で'
                    + 'まだ実行中だった。'
            }
        },
        {
            code: 'rewriteLoopAsBulkOperation',
            title: { en: 'Rewrite the loop as a bulk operation', ja: 'ループを一括操作に書き換える' },
            detail: {
                en: 'A whole-vector Word does in one step what a per-element loop does in as '
                    + 'many, and only the loop is charged per step.',
                ja: 'ベクタ全体を扱うWordは、要素ごとのループが要素数だけ費やす処理を1ステップで行う。'
                    + 'ステップ課金を受けるのはループの側だけである。'
            }
        },
        {
            code: 'trimWhatTheRunCarries',
            title: { en: 'Trim what the run carries', ja: '実行が抱える値を削る' },
            detail: {
                en: 'Exact values grow as they are combined; rounding to a grid (x d MUL FLOOR d DIV) '
                    + 'bounds a denominator that is otherwise free to grow every iteration.',
                ja: '厳密値は組み合わせるほど大きくなる。格子への丸め(x d MUL FLOOR d DIV)は、'
                    + 'そのままでは反復ごとに増え続ける分母に上限を与える。'
            }
        },
        {
            code: 'checkHostProfile',
            title: { en: 'Check the host profile', ja: 'ホストプロファイルを確認する' },
            detail: {
                en: 'This guard is not part of the language. Another host (the MCP server) '
                    + 'applies different limits; the profile badge beside the build version lists '
                    + 'the ones in force here.',
                ja: 'このガードは言語の一部ではない。別のホスト（MCPサーバ）は異なる上限を適用する。'
                    + 'ここで有効な上限は、ビルド版数の隣にあるプロファイル表示に並んでいる。'
            }
        }
    ]
};

export const describeTimeoutDiagnosis = (limitMs: number): string =>
    renderDiagnosisReport(TIMEOUT_DIAGNOSIS, {
        extraLines: [`limit executionTimeoutMs: ${limitMs} (wall clock, this host only)`]
    });

export const syncInterpreterState = (
    interpreter: AjisaiInterpreter,
    result: ExecuteResult
): void => {
    if (isFailure(result)) return;
    applyInterpreterSnapshot(interpreter, {
        // The worker's lossless snapshot is what restores the post-run stack
        // into the main-thread interpreter, so it keeps its exact values
        // (LANG.OBSERVATION.FIREWALL).
        stackSnapshot: result.stackSnapshot,
        userWords: result.userWords
    });
};

export const resolveExecutionException = (
    context: string,
    error: unknown,
    showInfo: (text: string, append: boolean) => void,
    showError: (error: Error | string) => void
): void => {
    console.error(`[${context}] Execution failed:`, error);
    if (error instanceof ExecutionAbortedError) {
        showInfo('Execution aborted', true);
        return;
    }
    showError(toError(error));
    // The one refusal the interpreter never gets to explain: it is stopped from
    // outside, so the diagnosis is written here instead of arriving with the
    // result.
    if (error instanceof ExecutionTimeoutError) {
        showInfo(describeTimeoutDiagnosis(error.limitMs), true);
    }
};

// ── What a run changed ──────────────────────────────────────────────────────

const toJson = (value: unknown): string => JSON.stringify(value ?? null);

// Whether the stack changed, decided without building a string of it.
//
// A stack is not small by construction: `1 200000 RANGE` is a legal program
// whose one value holds two hundred thousand elements, and serializing it
// twice on every run would cost the better part of a second of frozen main
// thread for an answer that a length mismatch settles immediately. A
// structural walk with an early exit allocates nothing and stops at the first
// difference — which for a run that produced anything is usually the first
// slot it looks at.
const checkValuesEqual = (left: unknown, right: unknown): boolean => {
    if (left === right) return true;
    // null and undefined are the same absence here (as they are in JSON), so
    // the pair is equal rather than a change.
    if (left === null || left === undefined) return right === null || right === undefined;
    if (right === null || right === undefined) return false;
    if (typeof left !== 'object' || typeof right !== 'object') return false;

    if (Array.isArray(left) || Array.isArray(right)) {
        if (!Array.isArray(left) || !Array.isArray(right)) return false;
        if (left.length !== right.length) return false;
        return left.every((element, index) => checkValuesEqual(element, right[index]));
    }

    const leftKeys = Object.keys(left as object);
    const rightKeys = Object.keys(right as object);
    if (leftKeys.length !== rightKeys.length) return false;
    return leftKeys.every(key =>
        Object.prototype.hasOwnProperty.call(right, key)
        && checkValuesEqual(
            (left as Record<string, unknown>)[key],
            (right as Record<string, unknown>)[key]
        ));
};

// Order-insensitive identity of the user dictionary. The pre-execution
// snapshot and the post-execution read-back can enumerate words in different
// orders (a synced interpreter rebuilds its dictionaries from scratch), so the
// set is sorted by name before comparison — otherwise a pure stack op like
// `2 3 ADD` would look like a dictionary change whenever any user word exists,
// and wrongly pull the right column to the Words sheet.
//
// The description is part of the comparison: a `DEF` under a `#:contract`
// line can leave the body as it was and change only the text the Dictionary
// shows for it, and that run still changed the dictionary — the surface is
// redrawn and the session saved only when this says so.
const normalizeUserWords = (words: readonly UserWord[]): string =>
    toJson(
        [...words]
            .map(word => ({
                name: word.name,
                definition: word.definition ?? null,
                description: word.description ?? null
            }))
            .sort((a, b) => a.name.localeCompare(b.name))
    );

// A view of the surfaces an execution can touch, read from one interpreter
// instance so before/after are directly comparable.
export interface ExecutionStateView {
    readonly stack: Value[];
    readonly userWords: UserWord[];
}

export const detectExecutionSurfaceChanges = (
    before: ExecutionStateView,
    after: ExecutionStateView,
    result: ExecuteResult
): ExecutionSurfaceChanges => {
    const userWordsChanged = normalizeUserWords(before.userWords) !== normalizeUserWords(after.userWords);

    // Errors and diagnostics render into the Output surface, so a failed run
    // changes Output even when the program emitted no text of its own.
    const hasError = isFailure(result);

    // Any emission changes Output, an empty or whitespace-only one included:
    // `'' PRINT` writes a line, so the surface it wrote to is shown
    // (spec/gui-semantics.md, "a Run shows each surface it changed").
    return {
        outputChanged: hasError || Boolean(result.output),
        stackChanged: !checkValuesEqual(before.stack, after.stack),
        dictionaryChanged: userWordsChanged
    };
};

// Whether a value is a NIL or holds one in some lane.
const checkHoldsNil = (value: Value | undefined): boolean => {
    if (!value) return false;
    if (value.type === 'nil') return true;
    if (value.type === 'vector') {
        // A vector cut to its leading elements states whether the rest holds
        // one (`truncated.holdsNil`).
        if (value.truncated?.holdsNil) return true;
        return Array.isArray(value.value) && (value.value as Value[]).some(checkHoldsNil);
    }
    if (value.type === 'record') {
        const { keys, values } = readRecordParts(value.value);
        return [...keys, ...values].some(checkHoldsNil);
    }
    return false;
};

// Whether the run left a NIL of its own on the stack — the only NIL a "Why
// NIL" can be about.
//
// The trace names a Word for every NIL left on top of the stack, including one
// that was already there: on a stack holding an earlier run's NIL,
// `[ 2 MUL ] 'G' DEF` reports that NIL against DEF. A slot the run did not
// change still holds what the run was handed, so only a new or changed slot
// holding a NIL counts.
export const checkRunLeftOwnNil = (
    before: ExecutionStateView,
    after: ExecutionStateView
): boolean =>
    after.stack.some((value, index) =>
        checkHoldsNil(value)
        && (index >= before.stack.length || !checkValuesEqual(before.stack[index], value)));
