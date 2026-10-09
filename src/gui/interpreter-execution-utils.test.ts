// A pure stack program such as the Reference's `3 4 ADD` must not read as a
// dictionary change and pull the right column to the Dictionary instead of
// the Stack.
//
// Name addressing is what that rests on. The dictionary has two tiers and User
// is one of them (LANG.DICTIONARY.RESOLUTION), so a word is addressed by its
// bare name, the one `collect_user_words_info` reports; `restore_user_words`
// skips a definition-less word, so a host that looked a definition up under
// any other spelling would run the worker without the user's words and the
// post-execution sync would wipe them from the main interpreter.
//
// The fake below reproduces exactly those three contracts of the wasm boundary
// (bare-name lookup, definition-less words skipped on restore, session reset
// clears the User tier), so the round trip is exercised without the wasm.

import { describe, expect, it } from 'vitest';
import type {
    AjisaiInterpreter,
    ExecuteResult,
    UserWord,
    Value,
    ProtocolDiagnosis
} from '../wasm-interpreter-types';
import {
    collectUserWords,
    createExecutionSnapshot,
    describeFailedRunOutput,
    describeTimeoutDiagnosis,
    resolveExecutionException,
    syncInterpreterState,
    detectExecutionSurfaceChanges,
    renderDiagnosisReport,
    checkRunLeftOwnNil,
    type ExecutionStateView
} from './interpreter-execution-utils';
import { ExecutionTimeoutError } from '../workers/execution-contract';
import { num } from '../test-support';

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
        // Tuple shape: [name, hasDependents].
        collect_user_words_info: () =>
            [...words.keys()].sort().map(name => [name, false] as [string, boolean]),
        lookup_word_definition: (name: string) => words.get(name.toUpperCase()) ?? null,
        lookup_word_description: () => null,
        snapshot_stack: () => JSON.stringify(stack),
        restore_stack_snapshot: (snapshot: string) => { stack = JSON.parse(snapshot) as Value[]; },
        restore_user_words: (restored: UserWord[]) => {
            for (const word of restored) {
                // A word with no definition cannot be defined, so it is skipped.
                if (!word.definition) continue;
                words.set(word.name.toUpperCase(), word.definition);
            }
            return [] as Array<[string, string]>;
        },
        reset: () => {
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

    worker.reset();
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
    it('reads a definition by the bare name the dictionary reports', () => {
        const interpreter = createFakeInterpreter();
        interpreter.words.set('ADD10', '10 ADD');

        expect(collectUserWords(interpreter)).toEqual([
            { name: 'ADD10', definition: '10 ADD', description: null }
        ]);
    });
});

describe('execution round trip with user words present', () => {
    it('keeps the user words and reports a stack-only change for `3 4 ADD`', () => {
        const main = createFakeInterpreter();
        const worker = createFakeInterpreter();
        main.words.set('ADD10', '10 ADD');

        const changes = runOneExecution(main, worker, (w) => {
            w.setStack([num(3), num(4), num(7)]);
            return { status: 'OK', output: '' } as ExecuteResult;
        });

        expect(changes.stackChanged).toBe(true);
        expect(changes.dictionaryChanged).toBe(false);
        // The words survived the worker round trip rather than being wiped.
        expect(collectUserWords(main)).toEqual([
            { name: 'ADD10', definition: '10 ADD', description: null }
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

// ── merged from diagnosis-report.test.ts ──

const check = (code: string, title: string, detail: string) => ({
    code,
    title: { en: title, ja: title },
    detail: { en: detail, ja: detail }
});

// The reading format is asserted here and nowhere else. Both the diagnoses the
// interpreter sends and the one the host writes for a wall-clock stop render
// through this module, so a change to the frame — a renamed question, a fourth
// one, a moved ceiling line — fails here once rather than passing in one
// surface and drifting in the other.
describe('renderDiagnosisReport', () => {
    it('renders the frame a reader learns once', () => {
        const diagnosis: ProtocolDiagnosis = {
            when: 'executeWord',
            where: { kind: 'coreWord', word: 'SQRT' },
            why: 'domain',
            summary: 'executeWord / SQRT (coreWord) / domain (nil:domainMiss)',
            evidence: ['sourceLine=3', 'sourceColumn=7', 'insideWords=SAFE-ROOT,REPORT'],
            candidates: [],
            nextChecks: [check('checkOperandDomain', 'Check the operand domain', 'A negative radicand projects NIL.')]
        };

        expect(renderDiagnosisReport(diagnosis, { stackLenBefore: 2 })).toBe(
            [
                '[DIAGNOSIS] executeWord / SQRT (coreWord) / domain (nil:domainMiss)',
                'Q1 when: executeWord',
                'Q2 where: SQRT (coreWord), inside SAFE-ROOT, REPORT at line 3, column 7, stack depth 2',
                'Q3 why: domain',
                'next: Check the operand domain - A negative radicand projects NIL.'
            ].join('\n')
        );
    });

    it('omits the position, the depth and the hints it was given nothing for', () => {
        const diagnosis: ProtocolDiagnosis = {
            when: 'resolveWord',
            where: { kind: 'unknown' },
            why: 'typoOrUnknownName',
            summary: 'resolveWord / unknown / typoOrUnknownName (error:unknownWord)',
            evidence: [],
            candidates: ['DUP', 'DROP'],
            nextChecks: []
        };

        expect(renderDiagnosisReport(diagnosis)).toBe(
            [
                '[DIAGNOSIS] resolveWord / unknown / typoOrUnknownName (error:unknownWord)',
                'Q1 when: resolveWord',
                'Q2 where: unknown',
                'Q3 why: typoOrUnknownName',
                'did you mean: DUP, DROP'
            ].join('\n')
        );
    });

    it('reports a declared ceiling with what was observed against it', () => {
        const diagnosis: ProtocolDiagnosis = {
            when: 'executeWord',
            where: { kind: 'coreWord', word: 'RANGE' },
            why: 'resourceLimit',
            summary: 'executeWord / RANGE (coreWord) / resourceLimit (nil:spaceExhausted)',
            evidence: [],
            resourceLimit: { resource: 'materializedElements', limit: 1_000_000, observed: 4_000_000 },
            nextChecks: []
        };

        expect(renderDiagnosisReport(diagnosis)).toContain(
            'limit materializedElements: 4000000 against 1000000'
        );
    });
});

// The wall-clock guard's whole output, locked. It was a hand-written string
// literal in the shape of the frame above; this is the same text, now produced
// by the frame itself.
describe('describeTimeoutDiagnosis', () => {
    it('renders the host guard through the shared frame', () => {
        expect(describeTimeoutDiagnosis(5_000)).toBe(
            [
                '[DIAGNOSIS] hostGuard / playground (hostEnvironment) / resourceLimit (executionTimeout)',
                'Q1 when: hostGuard',
                'Q2 where: playground (hostEnvironment)',
                'Q3 why: resourceLimit',
                'limit executionTimeoutMs: 5000 (wall clock, this host only)',
                'next: Check which guard stopped it - The playground stops a run on wall-clock time. '
                    + "The interpreter's own budgets (execution steps, materialized elements, numeric work) "
                    + 'did not refuse this program; it was still running when the time ran out.',
                'next: Rewrite the loop as a bulk operation - A whole-vector Word does in one step what a '
                    + 'per-element loop does in as many, and only the loop is charged per step.',
                'next: Trim what the run carries - Exact values grow as they are combined; rounding to a grid '
                    + '(x d MUL FLOOR d DIV) bounds a denominator that is otherwise free to grow every iteration.',
                'next: Check the host profile - This guard is not part of the language. Another host '
                    + '(the MCP server) applies different limits; the profile badge beside the build version '
                    + 'lists the ones in force here.'
            ].join('\n')
        );
    });

    it('offers four next-steps, each a protocol check rather than a printed line', () => {
        // The renderer prints the `en` side of a check; assembling these as
        // `ProtocolDebugCheck`s is what gives them a `ja` side at all — a
        // hand-written English line could not carry one, which is the second
        // thing the duplicated format cost.
        const rendered = describeTimeoutDiagnosis(5_000);
        const steps = rendered.split('\n').filter((line) => line.startsWith('next: '));

        expect(steps).toHaveLength(4);
    });
});

// ── merged from execution-surface-changes.test.ts ──

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

    it('flags a dictionary change when only a description changed', () => {
        // `[ 2 MUL ] 'G' DEF` under a new `#:contract G ...` line leaves the
        // body as it was; the Dictionary's text for G still changed, and the
        // redraw and the save follow this flag.
        const changes = detectExecutionSurfaceChanges(
            view({ userWords: [{ name: 'G', definition: '2 MUL', description: null }] }),
            view({ userWords: [{ name: 'G', definition: '2 MUL', description: 'doubles' }] }),
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

    // `'' PRINT` is one emission — the host writes it as one empty line — and
    // an emission is a change to Output whatever its characters are.
    it('treats an empty or whitespace-only emission as an Output change', () => {
        expect(detectExecutionSurfaceChanges(view(), view(), okResult({ output: '\n' })).outputChanged).toBe(true);
        expect(detectExecutionSurfaceChanges(view(), view(), okResult({ output: '  \n' })).outputChanged).toBe(true);
    });

    it('does not flag Output for a run that emitted nothing', () => {
        expect(detectExecutionSurfaceChanges(view(), view(), okResult({ output: '' })).outputChanged).toBe(false);
        expect(detectExecutionSurfaceChanges(view(), view(), okResult()).outputChanged).toBe(false);
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
        expect(checkRunLeftOwnNil(view(), view({ stack: [nil('domainMiss')] }))).toBe(true);
    });

    it('sees a NIL the run left below another value', () => {
        expect(checkRunLeftOwnNil(view(), view({ stack: [nil('domainMiss'), num(5)] }))).toBe(true);
    });

    it('sees a NIL lane in a Vector the run produced', () => {
        expect(checkRunLeftOwnNil(view(), view({ stack: [vector(num(1), nil('domainMiss'))] }))).toBe(true);
    });

    it('ignores a NIL the run was handed and left where it was', () => {
        const before = view({ stack: [nil('domainMiss')] });
        const after = view({ stack: [nil('domainMiss')], userWords: [word('G', '2 MUL')] });
        expect(checkRunLeftOwnNil(before, after)).toBe(false);
    });

    it('sees a second NIL pushed on top of an earlier one', () => {
        const before = view({ stack: [nil('domainMiss')] });
        const after = view({ stack: [nil('domainMiss'), nil('domainMiss')] });
        expect(checkRunLeftOwnNil(before, after)).toBe(true);
    });

    it('reports nothing for a run that left no NIL', () => {
        expect(checkRunLeftOwnNil(view(), view({ stack: [num(3)] }))).toBe(false);
    });
});
