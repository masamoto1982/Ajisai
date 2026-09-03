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
import {
    collectUserWords,
    createExecutionSnapshot,
    describeFailedRunOutput,
    describeSnapshotRefusal,
    describeTimeoutDiagnosis,
    resolveExecutionException,
    syncInterpreterState
} from './interpreter-execution-utils';
import { ExecutionTimeoutError } from '../workers/execution-timeout';
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

// A failed run's `Defined word:` lines outlive the definitions they announce:
// `syncInterpreterState` ignores an ERROR result, so the session keeps its
// pre-run dictionary. The tester who hit this lost seven definitions and only
// noticed when `LOOKUP` answered `Unknown word` for a name the log said had
// been defined.
describe('describeFailedRunOutput', () => {
    it('cancels the success lines of a run whose definitions were discarded', () => {
        const reported = describeFailedRunOutput({
            status: 'ERROR',
            error: true,
            output: 'Defined word: GY\nDefined word: LR\n',
            discardedDictionaryChanges: ['GY', 'LR']
        } as ExecuteResult);

        expect(reported).toContain('Defined word: GY');
        expect(reported).toContain('Rolled back 2 dictionary changes: GY, LR.');
        // The correction reads after the claims it corrects, not before them.
        expect(reported.indexOf('Rolled back')).toBeGreaterThan(reported.indexOf('Defined word: LR'));
    });

    it('counts a single change in the singular', () => {
        const reported = describeFailedRunOutput({
            status: 'ERROR',
            error: true,
            output: 'Defined word: GY\n',
            discardedDictionaryChanges: ['GY']
        } as ExecuteResult);

        expect(reported).toContain('Rolled back 1 dictionary change: GY.');
    });

    it('leaves a failed run that changed no dictionary untouched', () => {
        const reported = describeFailedRunOutput({
            status: 'ERROR',
            error: true,
            output: 'partial trace\n'
        } as ExecuteResult);

        expect(reported).toBe('partial trace\n');
    });

    it('reports the correction alone when the run printed nothing else', () => {
        const reported = describeFailedRunOutput({
            status: 'ERROR',
            error: true,
            discardedDictionaryChanges: ['GY']
        } as ExecuteResult);

        expect(reported.startsWith('Rolled back 1 dictionary change: GY.')).toBe(true);
    });
});

// A value the snapshot codec refuses (`PI`, and anything built from it, is a
// Tier-2 computable real) fails *after* the program has already produced it.
// Reported as the program's own failure, `PI` read as a Word that does not
// work: the answer the run had computed never reached the reader, and the
// message named a persistence concept the language never mentions.
describe('a run whose result cannot be snapshotted', () => {
    it('keeps the pre-run stack instead of restoring an empty one', () => {
        const main = createFakeInterpreter();
        main.setStack([num(1), num(2)]);

        syncInterpreterState(main, {
            status: 'OK',
            stack: [num(3)],
            stackSnapshotError: 'cannot persist a Tier-2 computable exact real'
        } as ExecuteResult);

        // Not the run's stack, and not an empty one either: the session is
        // exactly as it was, which is the answer a failed run also gets.
        expect(main.collect_stack()).toEqual([num(1), num(2)]);
    });

    it('says the run succeeded and only its result stops here', () => {
        const explained = describeSnapshotRefusal({
            status: 'OK',
            stackSnapshotError: 'cannot persist a Tier-2 computable exact real'
        } as ExecuteResult);

        expect(explained).toContain('The program ran and produced its result');
        expect(explained).toContain('cannot persist a Tier-2 computable exact real');
        expect(explained).toContain('The stack is unchanged from before the run.');
    });

    it('explains nothing for an ordinary successful run', () => {
        expect(describeSnapshotRefusal({ status: 'OK' } as ExecuteResult)).toBeNull();
    });
});

// The wall-clock guard is the one refusal the interpreter never diagnoses: the
// worker is terminated where it stands, so a diagnosis has to be written on
// this side or the reader gets a bare sentence where every other failure
// answers with when / where / why and what to do next.
describe('describeTimeoutDiagnosis', () => {
    it('answers in the same shape as an interpreter diagnosis', () => {
        const diagnosis = describeTimeoutDiagnosis(5_000);

        expect(diagnosis).toContain('[DIAGNOSIS]');
        expect(diagnosis).toContain('Q1 when:');
        expect(diagnosis).toContain('Q2 where:');
        expect(diagnosis).toContain('Q3 why: resourceLimit');
        expect(diagnosis).toContain('executionTimeoutMs: 5000');
        expect(diagnosis.split('\n').filter((line) => line.startsWith('next: ')).length)
            .toBeGreaterThanOrEqual(3);
    });

    it('says the guard is the host’s and not the language’s', () => {
        const diagnosis = describeTimeoutDiagnosis(5_000);

        expect(diagnosis).toContain('not part of the language');
        expect(diagnosis).toContain('did not refuse this program');
    });
});

describe('resolveExecutionException', () => {
    const collect = () => {
        const info: string[] = [];
        const errors: string[] = [];
        return {
            info,
            errors,
            showInfo: (text: string) => { info.push(text); },
            showError: (error: Error | string) => {
                errors.push(error instanceof Error ? error.message : error);
            }
        };
    };

    it('writes a diagnosis under a wall-clock stop', () => {
        const sink = collect();

        resolveExecutionException(
            'test',
            new ExecutionTimeoutError(5_000),
            sink.showInfo,
            sink.showError
        );

        expect(sink.errors[0]).toBe('Execution timed out after 5000 ms');
        expect(sink.info.join('\n')).toContain('[DIAGNOSIS]');
    });

    it('leaves an ordinary failure to the diagnosis the interpreter already built', () => {
        const sink = collect();

        resolveExecutionException('test', new Error('Stack underflow'), sink.showInfo, sink.showError);

        expect(sink.errors[0]).toBe('Stack underflow');
        expect(sink.info).toEqual([]);
    });
});
