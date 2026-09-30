import type { ExecuteResult, UserWord, Value } from '../wasm-interpreter-types';
import type { ExecutionSurfaceChanges } from './gui-layout-state';
import { isFailure } from './interpreter-execution-utils';

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
const normalizeUserWords = (words: readonly UserWord[]): string =>
    toJson(
        [...words]
            .map(word => ({ name: word.name, definition: word.definition ?? null }))
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
    if (value.type === 'vector' && Array.isArray(value.value)) {
        return (value.value as Value[]).some(checkHoldsNil);
    }
    if (value.type === 'record') {
        const record = value.value as { keys?: Value[]; values?: Value[] } | null;
        return [...(record?.keys ?? []), ...(record?.values ?? [])].some(checkHoldsNil);
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
