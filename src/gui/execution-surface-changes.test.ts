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
    importedModules: [],
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

    it('selects the imported module sheet on a module import', () => {
        const changes = detectExecutionSurfaceChanges(
            view({ importedModules: [] }),
            view({ importedModules: ['MATH'] }),
            okResult()
        );
        expect(changes.dictionaryChanged).toBe(true);
        expect(changes.dictionarySheetId).toBe('module-MATH');
    });

    it('ignores module reordering with no membership change', () => {
        const changes = detectExecutionSurfaceChanges(
            view({ importedModules: ['MATH', 'ALGO'] }),
            view({ importedModules: ['ALGO', 'MATH'] }),
            okResult()
        );
        expect(changes.dictionaryChanged).toBe(false);
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
});
