// Regression for the auto-transition fault: running a pure stack program such
// as the Reference's `3 4 KEEP ADD` pulled the right column to the Dictionary
// instead of the Stack.
//
// The cause was name addressing, not the layout rule. The dictionary has two
// tiers and User is one of them (LANG.DICTIONARY.RESOLUTION), so a word is
// addressed by its bare name and the interpreter no longer resolves a
// `DICT@NAME` composite. The host still composed one, so every definition read
// back as null; `restore_user_words` skips a definition-less word, so the
// worker ran without the user's words and reported none back, and the
// post-execution sync wiped them from the main interpreter. Every run then
// looked like a dictionary change.
//
// The fake below reproduces exactly those three contracts of the wasm boundary
// (bare-name lookup, definition-less words skipped on restore, session reset
// clears the User tier), so the round trip is exercised without the wasm.

import { describe, expect, it } from 'vitest';
import type { AjisaiInterpreter, ExecuteResult, UserWord, Value } from '../wasm-interpreter-types';
import { collectUserWords, createExecutionSnapshot, syncInterpreterState } from './interpreter-execution-utils';
import { detectExecutionSurfaceChanges } from './execution-surface-changes';

const num = (n: number): Value =>
    ({ type: 'number', value: { numerator: String(n), denominator: '1' } } as unknown as Value);

interface FakeInterpreter extends AjisaiInterpreter {
    readonly words: Map<string, string>;
    setStack(stack: Value[]): void;
}

const createFakeInterpreter = (): FakeInterpreter => {
    const words = new Map<string, string>();
    let stack: Value[] = [];

    const fake: Partial<FakeInterpreter> = {
        words,
        setStack: (next: Value[]) => { stack = next; },
        collect_stack: () => stack,
        // Tuple shape: [dictionary, name, isProtected]. There is one User tier,
        // so the dictionary slot is a constant label, not an address.
        collect_user_words_info: () =>
            [...words.keys()].sort().map(name => ['USER', name, false] as [string, string, boolean]),
        // Resolves a bare name only: `USER@FOO` is not a name the dictionary has.
        lookup_word_definition: (name: string) => words.get(name.toUpperCase()) ?? null,
        snapshot_stack: () => JSON.stringify(stack),
        restore_stack_snapshot: (snapshot: string) => { stack = JSON.parse(snapshot) as Value[]; },
        restore_user_words: (restored: UserWord[]) => {
            for (const word of restored) {
                // A word with no definition cannot be defined, so it is skipped.
                if (!word.definition) continue;
                words.set(word.name.toUpperCase(), word.definition);
            }
        },
        reset_session: () => {
            words.clear();
            stack = [];
            return { status: 'OK' } as ExecuteResult;
        },
        set_max_execution_steps: () => { /* budget is not modelled here */ },
        update_serial_inbox: () => { /* no serial in this model */ },
        mark_serial_disconnected: () => { /* no serial in this model */ },
    };

    return fake as FakeInterpreter;
};

// One `executeCode` round: snapshot the main interpreter, run in a second
// interpreter (the worker), sync the result back, and report what changed.
const runOneExecution = (
    main: FakeInterpreter,
    worker: FakeInterpreter,
    execute: (worker: FakeInterpreter) => ExecuteResult
) => {
    const before = { stack: main.collect_stack(), userWords: collectUserWords(main) };
    const snapshot = createExecutionSnapshot(main);

    worker.reset_session();
    worker.restore_stack_snapshot(snapshot.stackSnapshot!);
    worker.restore_user_words(snapshot.userWords);

    const result = execute(worker);
    result.stackSnapshot = worker.snapshot_stack();
    result.userWords = collectUserWords(worker);

    syncInterpreterState(main, result);

    const after = { stack: main.collect_stack(), userWords: collectUserWords(main) };
    return detectExecutionSurfaceChanges(before, after, result);
};

describe('collectUserWords', () => {
    it('reads a definition by bare name, not by a DICT@NAME composite', () => {
        const interpreter = createFakeInterpreter();
        interpreter.words.set('ADD10', '10 ADD');

        expect(collectUserWords(interpreter)).toEqual([
            { dictionary: 'USER', name: 'ADD10', definition: '10 ADD' }
        ]);
    });
});

describe('execution round trip with user words present', () => {
    it('keeps the user words and reports a stack-only change for `3 4 KEEP ADD`', () => {
        const main = createFakeInterpreter();
        const worker = createFakeInterpreter();
        main.words.set('ADD10', '10 ADD');

        const changes = runOneExecution(main, worker, (w) => {
            w.setStack([num(3), num(4), num(7)]);
            return { status: 'OK', output: '' } as ExecuteResult;
        });

        expect(changes.stackChanged).toBe(true);
        expect(changes.dictionaryChanged).toBe(false);
        expect(changes.dictionarySheetId).toBeUndefined();
        // The words survived the worker round trip rather than being wiped.
        expect(collectUserWords(main)).toEqual([
            { dictionary: 'USER', name: 'ADD10', definition: '10 ADD' }
        ]);
    });

    it('carries the user words into the worker so a user word stays callable', () => {
        const main = createFakeInterpreter();
        const worker = createFakeInterpreter();
        main.words.set('ADD10', '10 ADD');

        runOneExecution(main, worker, (w) => {
            expect(w.lookup_word_definition('ADD10')).toBe('10 ADD');
            return { status: 'OK' } as ExecuteResult;
        });
    });

    it('still reports a dictionary change when a word is actually defined', () => {
        const main = createFakeInterpreter();
        const worker = createFakeInterpreter();

        const changes = runOneExecution(main, worker, (w) => {
            w.words.set('ADD10', '10 ADD');
            return { status: 'OK', output: 'Defined word: ADD10\n' } as ExecuteResult;
        });

        expect(changes.dictionaryChanged).toBe(true);
        expect(changes.dictionarySheetId).toBe('user');
    });

    it('still reports a dictionary change when a word is deleted', () => {
        const main = createFakeInterpreter();
        const worker = createFakeInterpreter();
        main.words.set('ADD10', '10 ADD');

        const changes = runOneExecution(main, worker, (w) => {
            w.words.delete('ADD10');
            return { status: 'OK', output: 'Deleted word: ADD10\n' } as ExecuteResult;
        });

        expect(changes.dictionaryChanged).toBe(true);
        expect(changes.dictionarySheetId).toBe('user');
    });
});
