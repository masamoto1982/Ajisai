// Run, Step, the wall-clock guard, Abort and Lookup answer through one path
// (`applyExecutionResult` in execution-controller.ts): the report and its
// diagnosis go to Output, and the surfaces the answer changed are shown. These
// pin that each of them takes it, rather than a copy that drifts — a failing
// Step that printed no diagnosis and never left the editor, an Abort that
// could not stop a run, and a "Why NIL" hung on a DEF were each such a copy.
//
// The worker pool is replaced by a fake whose `execute` the test answers by
// hand; the interpreter is the minimal fake the snapshot round trip needs.

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
    AjisaiInterpreter,
    ErrorFlowTraceEvent,
    ExecuteResult,
    ProtocolDiagnosis,
    UserWord,
    Value
} from '../wasm-interpreter-types';
import type { ExecutionSurfaceChanges } from './gui-layout-state';
import { ExecutionTimeoutError } from '../workers/execution-timeout';
import { ExecutionAbortedError } from '../workers/execution-aborted';
import { createExecutionController } from './execution-controller';

type Answer = (code: string) => Promise<ExecuteResult>;

const pool = vi.hoisted(() => ({
    answer: null as null | ((code: string) => Promise<unknown>),
    abortAll: (): void => { /* replaced per test */ }
}));

vi.mock('../workers/execution-worker-manager', () => ({
    WORKER_MANAGER: {
        execute: (code: string) => pool.answer!(code),
        abortAll: () => pool.abortAll(),
        resetAllWorkers: async () => { /* not exercised */ }
    }
}));

const num = (n: number): Value =>
    ({ type: 'number', value: { numerator: String(n), denominator: '1' } } as unknown as Value);
const nil = (reason: string): Value =>
    ({ type: 'nil', value: null, semantics: { absence: { reason } } } as unknown as Value);

const DIV_DIAGNOSIS: ProtocolDiagnosis = {
    when: 'wordExecution',
    where: { kind: 'coreWord', word: 'DIV' },
    why: 'domain',
    summary: 'wordExecution / DIV (coreWord) / domain',
    evidence: [],
    candidates: [],
    nextChecks: []
};

const nilEvent = (word: string): ErrorFlowTraceEvent => ({
    kind: 'nilProduced',
    word,
    stackLenBefore: 2,
    stackLenAfter: 1,
    message: `NIL produced by ${word}`,
    diagnosis: DIV_DIAGNOSIS
});

// The main-thread interpreter: a stack and a User dictionary, restored from a
// result's snapshot the way the wasm one is.
const createFakeInterpreter = () => {
    let stack: Value[] = [];
    const words = new Map<string, string>();
    const fake: Partial<AjisaiInterpreter> = {
        collect_stack: () => stack,
        collect_user_words_info: () =>
            [...words.keys()].map(name => ['USER', name, false] as [string, string, boolean]),
        lookup_word_definition: (name: string) => words.get(name) ?? null,
        lookup_word_description: () => null,
        snapshot_stack: () => JSON.stringify(stack),
        restore_stack_snapshot: (snapshot: string) => { stack = JSON.parse(snapshot) as Value[]; },
        restore_user_words: (restored: UserWord[]) => {
            for (const word of restored) if (word.definition) words.set(word.name, word.definition);
        },
        reset: () => {
            stack = [];
            words.clear();
            return { status: 'OK' } as ExecuteResult;
        },
        set_max_execution_steps: () => { /* not modelled */ },
        resolve_host_lookup: (name: string) =>
            name === 'DIV' ? { kind: 'documentation', text: 'DIV — divide values' } : null
    };
    return {
        interpreter: fake as AjisaiInterpreter,
        setStack: (next: Value[]) => { stack = next; },
        words
    };
};

// An OK answer that leaves `stack` behind, as the worker's would.
const ok = (stack: Value[], overrides: Partial<ExecuteResult> = {}): ExecuteResult => ({
    status: 'OK',
    output: '',
    stack,
    stackSnapshot: JSON.stringify(stack),
    userWords: [],
    ...overrides
});

const setup = () => {
    const fake = createFakeInterpreter();
    const log: string[] = [];
    const surfaces: ExecutionSurfaceChanges[] = [];
    const views: string[] = [];
    const runStatus: (string | null)[] = [];
    let editor = '';
    const controller = createExecutionController(fake.interpreter, {
        extractEditorValue: () => editor,
        clearEditor: () => { log.push('clearEditor'); },
        showInfo: (text, append) => { log.push(`${append ? 'info+' : 'info'}: ${text}`); },
        showFoldedInfo: (label) => { log.push(`folded: ${label}`); },
        highlightSourceRange: () => { /* not observed */ },
        showDocumentation: (text) => { log.push(`doc: ${text}`); },
        showError: (error) => {
            log.push(`error: ${error instanceof Error ? error.message : error}`);
        },
        showExecutionResult: () => { log.push('result'); },
        updateDisplays: () => { /* not observed */ },
        saveState: async () => { /* not observed */ },
        fullReset: async () => { /* not observed */ },
        updateView: (mode) => { views.push(mode); },
        updateAfterExecution: (changes) => { surfaces.push(changes); },
        showRunStatus: (text) => { runStatus.push(text); }
    });
    return {
        ...fake,
        controller,
        log,
        surfaces,
        views,
        runStatus,
        setEditor: (text: string) => { editor = text; }
    };
};

const answerWith = (answer: Answer): void => { pool.answer = answer; };

beforeEach(() => {
    pool.answer = null;
    pool.abortAll = () => { /* nothing running */ };
});

describe('Run', () => {
    it('says it is running on the Input surface until the run answers', async () => {
        const page = setup();
        answerWith(async () => ok([num(1)]));

        await page.controller.executeCode('1');

        expect(page.runStatus).toHaveLength(2);
        expect(page.runStatus[0]).toContain('Escape');
        expect(page.runStatus[1]).toBeNull();
    });

    it('offers "Why NIL" for a NIL the run produced', async () => {
        const page = setup();
        answerWith(async () => ok([nil('divisionByZero')], { errorFlowTrace: [nilEvent('DIV')] }));

        await page.controller.executeCode('1 0 DIV');

        expect(page.log).toContain('folded: Why NIL');
    });

    it('offers no "Why NIL" for a DEF that left an earlier NIL on top', async () => {
        const page = setup();
        page.setStack([nil('divisionByZero')]);
        answerWith(async () => ok([nil('divisionByZero')], {
            output: 'Defined word: G\n',
            userWords: [{ name: 'G', definition: '2 MUL' }],
            errorFlowTrace: [nilEvent('DEF')]
        }));

        await page.controller.executeCode("[ 2 MUL ] 'G' DEF");

        expect(page.log).not.toContain('folded: Why NIL');
    });
});

describe('Step', () => {
    it('reports a failing step with its diagnosis, keeps the step line, and shows Output', async () => {
        const page = setup();
        page.setEditor('1 FOO');
        let calls = 0;
        answerWith(async () => {
            calls += 1;
            if (calls === 1) return ok([num(1)]);
            return {
                status: 'ERROR',
                error: true,
                message: 'Unknown word: FOO',
                errorFlowTrace: [{ ...nilEvent('FOO'), kind: 'wordError' }]
            };
        });

        await page.controller.executeStep();
        await page.controller.executeStep();

        const failed = page.log.slice(page.log.indexOf('error: Unknown word: FOO'));
        expect(failed[1]).toMatch(/^info\+: \[DIAGNOSIS\]/);
        expect(failed[2]).toMatch(/^info\+: \[>\] Step 2\/2/);
        expect(page.surfaces.at(-1)?.outputChanged).toBe(true);
        expect(page.controller.checkIsStepModeActive()).toBe(false);
    });

    it('writes the step line after the answer rather than before it', async () => {
        const page = setup();
        page.setEditor('1 2');
        answerWith(async () => ok([num(1)]));

        await page.controller.executeStep();

        expect(page.log.indexOf('result')).toBeLessThan(
            page.log.findIndex(line => line.startsWith('info+: [>] Step 1/2'))
        );
    });
});

describe('a run that never answered', () => {
    it('reports the wall-clock stop with its diagnosis and shows Output', async () => {
        const page = setup();
        answerWith(async () => { throw new ExecutionTimeoutError(5_000); });

        await page.controller.executeCode('loop');

        expect(page.log).toContain('error: Execution timed out after 5000 ms');
        expect(page.log.some(line => line.includes('[DIAGNOSIS]'))).toBe(true);
        expect(page.surfaces.at(-1)).toEqual({
            outputChanged: true,
            stackChanged: false,
            dictionaryChanged: false
        });
        expect(page.runStatus.at(-1)).toBeNull();
    });

    it('stops on Abort through the pool and reports it the same way', async () => {
        const page = setup();
        answerWith(() => new Promise<ExecuteResult>((_, reject) => {
            pool.abortAll = () => reject(new ExecutionAbortedError());
        }));

        const run = page.controller.executeCode('loop');
        page.controller.abortExecution();
        await run;

        expect(page.log).toContain('info+: Execution aborted');
        expect(page.surfaces.at(-1)?.outputChanged).toBe(true);
    });
});

describe('Lookup', () => {
    it('answers a known word in Output', () => {
        const page = setup();
        page.controller.lookupWord('DIV');
        expect(page.log).toEqual(['doc: DIV — divide values']);
        expect(page.views).toEqual(['output']);
    });

    it('answers an unknown word in Output too', () => {
        const page = setup();
        page.controller.lookupWord('NOPE');
        expect(page.log).toEqual(['error: Unknown word: NOPE']);
        expect(page.views).toEqual(['output']);
    });
});
