import { describe, expect, it } from 'vitest';
import { detectExecutionSurfaceChanges, type ExecutionStateView } from './execution-surface-changes';
import type { ExecuteResult, UserWord, Value } from '../wasm-interpreter-types';

const num = (n: number): Value => ({ type: 'number', value: { numerator: String(n), denominator: '1' } } as unknown as Value);

const word = (name: string, definition: string): UserWord => ({ dictionary: 'USER', name, definition });

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

    it('does NOT flag a dictionary change for `2 3 +` when unchanged user words exist', () => {
        // Regression: pre/post are read from different sources that can enumerate
        // the same words in different orders; the comparison must be order-insensitive
        // so a pure stack op never pulls the right column to the Words sheet.
        const before = view({
            stack: [],
            userWords: [word('FOO', '1 2 +'), word('BAR', '3 4 +')]
        });
        const after = view({
            stack: [num(5)],
            // Same set, different enumeration order (a synced interpreter rebuilds
            // its dictionaries from scratch).
            userWords: [word('BAR', '3 4 +'), word('FOO', '1 2 +')]
        });

        const changes = detectExecutionSurfaceChanges(before, after, okResult());

        expect(changes.stackChanged).toBe(true);
        expect(changes.dictionaryChanged).toBe(false);
        expect(changes.dictionarySheetId).toBeUndefined();
    });

    it('flags a dictionary change and the user sheet when a word is defined', () => {
        const changes = detectExecutionSurfaceChanges(
            view({ userWords: [] }),
            view({ userWords: [word('FOO', '1 2 +')] }),
            okResult()
        );
        expect(changes.dictionaryChanged).toBe(true);
        expect(changes.dictionarySheetId).toBe('user');
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
