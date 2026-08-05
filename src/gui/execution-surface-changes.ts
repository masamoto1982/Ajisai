import type { ExecuteResult, UserWord, Value } from '../wasm-interpreter-types';
import type { ExecutionSurfaceChanges } from './gui-layout-state';

const stableStringify = (value: unknown): string => JSON.stringify(value ?? null);

// Whether the stack changed, decided without building a string of it.
//
// This used to compare `JSON.stringify(before.stack)` with the same of
// `after.stack`. That is two full serializations of the stack on every single
// run, and a stack is not small by construction: `[ 1 200000 ] RANGE` is a
// legal program whose one value holds two hundred thousand elements, and
// stringifying it twice cost the better part of a second of frozen main thread
// for an answer that a length mismatch settles immediately. A structural walk
// with an early exit answers the same question, allocates nothing, and stops at
// the first difference — which for a run that produced anything is usually the
// first slot it looks at.
const checkValuesEqual = (left: unknown, right: unknown): boolean => {
    if (left === right) return true;
    // JSON.stringify wrote both null and undefined into the same text for a
    // top-level value, so treat the pair as equal here too rather than reporting
    // a change the previous comparison never saw.
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
// set is sorted by fully-qualified name before comparison — otherwise a pure
// stack op like `2 3 +` would look like a dictionary change whenever any user
// word exists, and wrongly pull the right column to the Words sheet.
const normalizeUserWords = (words: readonly UserWord[]): string =>
    stableStringify(
        [...words]
            .map(word => ({
                dictionary: word.dictionary ?? null,
                name: word.name,
                definition: word.definition ?? null
            }))
            .sort((a, b) =>
                `${a.dictionary ?? ''}@${a.name}`.localeCompare(`${b.dictionary ?? ''}@${b.name}`))
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
    const hasError = result.status !== 'OK' || Boolean(result.error);

    return {
        outputChanged: hasError || Boolean((result.output ?? '').trim()),
        stackChanged: !checkValuesEqual(before.stack, after.stack),
        dictionaryChanged: userWordsChanged,
        // Defining your own word lands on the 'user' sheet.
        dictionarySheetId: userWordsChanged ? 'user' : undefined
    };
};
