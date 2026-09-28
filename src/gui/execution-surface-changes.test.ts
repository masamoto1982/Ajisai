import { describe, expect, it } from 'vitest';
import {
    checkRunLeftOwnNil,
    detectExecutionSurfaceChanges,
    type ExecutionStateView
} from './execution-surface-changes';
import type { ExecuteResult, UserWord, Value } from '../wasm-interpreter-types';

const num = (n: number): Value => ({ type: 'number', value: { numerator: String(n), denominator: '1' } } as unknown as Value);

const word = (name: string, definition: string): UserWord => ({ name, definition });

const okResult = (overrides: Partial<ExecuteResult> = {}): ExecuteResult => ({
    status: 'OK',
    ...overrides
});

const view = (overrides: Partial<ExecutionStateView> = {}): ExecutionStateView => ({
    stack: [],
    userWords: [],
    ...overrides
});

describe('detectExecutionSurfaceChanges', () => {
    it('reports a stack-only change for a pure stack op', () => {
        const changes = detectExecutionSurfaceChanges(
            view({ stack: [] }),
            view({ stack: [num(5)] }),
            okResult()
        );
        expect(changes.stackChanged).toBe(true);
        expect(changes.dictionaryChanged).toBe(false);
        expect(changes.outputChanged).toBe(false);
    });

    it('does NOT flag a dictionary change for `2 3 ADD` when unchanged user words exist', () => {
        // Regression: pre/post are read from different sources that can enumerate
        // the same words in different orders; the comparison must be order-insensitive
        // so a pure stack op never pulls the right column to the Words sheet.
        const before = view({
            stack: [],
            userWords: [word('FOO', '1 2 ADD'), word('BAR', '3 4 ADD')]
        });
        const after = view({
            stack: [num(5)],
            // Same set, different enumeration order (a synced interpreter rebuilds
            // its dictionaries from scratch).
            userWords: [word('BAR', '3 4 ADD'), word('FOO', '1 2 ADD')]
        });

        const changes = detectExecutionSurfaceChanges(before, after, okResult());

        expect(changes.stackChanged).toBe(true);
        expect(changes.dictionaryChanged).toBe(false);
    });

    it('flags a dictionary change and the user sheet when a word is defined', () => {
        const changes = detectExecutionSurfaceChanges(
            view({ userWords: [] }),
            view({ userWords: [word('FOO', '1 2 ADD')] }),
            okResult()
        );
        expect(changes.dictionaryChanged).toBe(true);
    });

    it('treats a failed run as an Output change even with no program output', () => {
        const changes = detectExecutionSurfaceChanges(
            view(),
            view(),
            okResult({ status: 'ERROR', error: true })
        );
        expect(changes.outputChanged).toBe(true);
    });

    it('treats real program output as an Output change', () => {
        const changes = detectExecutionSurfaceChanges(
            view(),
            view(),
            okResult({ output: 'hello' })
        );
        expect(changes.outputChanged).toBe(true);
    });

    // The stack comparison walks the values structurally instead of stringifying
    // the whole stack twice per run (a stack can legally hold hundreds of
    // thousands of elements). These pin the equality it has to reproduce.
    it('reports no stack change when equal values are distinct objects', () => {
        const changes = detectExecutionSurfaceChanges(
            view({ stack: [num(5)] }),
            view({ stack: [num(5)] }),
            okResult()
        );
        expect(changes.stackChanged).toBe(false);
    });

    it('sees a change deep inside a nested vector', () => {
        const vector = (...elements: Value[]): Value =>
            ({ type: 'vector', value: elements } as unknown as Value);
        const changes = detectExecutionSurfaceChanges(
            view({ stack: [vector(vector(num(1), num(2)))] }),
            view({ stack: [vector(vector(num(1), num(3)))] }),
            okResult()
        );
        expect(changes.stackChanged).toBe(true);
    });

    it('sees a change of length alone', () => {
        const changes = detectExecutionSurfaceChanges(
            view({ stack: [num(1), num(2)] }),
            view({ stack: [num(1)] }),
            okResult()
        );
        expect(changes.stackChanged).toBe(true);
    });

    it('sees a value that gained a property', () => {
        const plain = { type: 'number', value: { numerator: '1', denominator: '1' } } as unknown as Value;
        const tagged = {
            type: 'number',
            value: { numerator: '1', denominator: '1' },
            semantics: { approximate: true }
        } as unknown as Value;
        const changes = detectExecutionSurfaceChanges(
            view({ stack: [plain] }),
            view({ stack: [tagged] }),
            okResult()
        );
        expect(changes.stackChanged).toBe(true);
    });

    it('reports no stack change for two empty stacks', () => {
        const changes = detectExecutionSurfaceChanges(view(), view(), okResult());
        expect(changes.stackChanged).toBe(false);
    });
});

// A "Why NIL" is about a NIL this run left on the stack. The trace also names
// the Word that merely left an older NIL on top — `[ 2 MUL ] 'G' DEF` on a
// stack already holding one — which is not this run's to explain.
describe('checkRunLeftOwnNil', () => {
    const nil = (reason: string): Value =>
        ({ type: 'nil', value: null, semantics: { absence: { reason } } } as unknown as Value);
    const vector = (...elements: Value[]): Value =>
        ({ type: 'vector', value: elements } as unknown as Value);

    it('sees a NIL the run pushed', () => {
        expect(checkRunLeftOwnNil(view(), view({ stack: [nil('divisionByZero')] }))).toBe(true);
    });

    it('sees a NIL the run left below another value', () => {
        expect(checkRunLeftOwnNil(view(), view({ stack: [nil('divisionByZero'), num(5)] }))).toBe(true);
    });

    it('sees a NIL lane in a Vector the run produced', () => {
        expect(checkRunLeftOwnNil(view(), view({ stack: [vector(num(1), nil('divisionByZero'))] }))).toBe(true);
    });

    it('ignores a NIL the run was handed and left where it was', () => {
        const before = view({ stack: [nil('divisionByZero')] });
        const after = view({ stack: [nil('divisionByZero')], userWords: [word('G', '2 MUL')] });
        expect(checkRunLeftOwnNil(before, after)).toBe(false);
    });

    it('sees a second NIL pushed on top of an earlier one', () => {
        const before = view({ stack: [nil('divisionByZero')] });
        const after = view({ stack: [nil('divisionByZero'), nil('divisionByZero')] });
        expect(checkRunLeftOwnNil(before, after)).toBe(true);
    });

    it('reports nothing for a run that left no NIL', () => {
        expect(checkRunLeftOwnNil(view(), view({ stack: [num(3)] }))).toBe(false);
    });
});
