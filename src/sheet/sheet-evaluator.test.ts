// Sheet view cell evaluation tests (plan §2.4). The executor is stubbed, so
// these pin the host-side evaluation contract without WASM: serial order,
// state classification, host-error short-circuit, and edit serialization.

import { describe, expect, test } from 'vitest';
import type { ExecuteResult } from '../wasm-interpreter-types';
import { SheetEvaluator } from './sheet-evaluator';

const ok = (stackLabel: string): ExecuteResult =>
    ({ status: 'OK', stack: [{ type: 'string', value: stackLabel }] }) as ExecuteResult;

describe('SheetEvaluator.applyPlan', () => {
    test('evaluates plan order serially, upstream first', async () => {
        const calls: string[] = [];
        const evaluator = new SheetEvaluator(async (fqName) => {
            calls.push(fqName);
            return ok(fqName);
        });

        const changed = await evaluator.applyPlan({
            order: ['TABLE1@A1', 'TABLE1@B1'],
            cyclic: [],
            blocked: [],
        });

        expect(calls).toEqual(['TABLE1@A1', 'TABLE1@B1']);
        expect(changed.get('TABLE1@B1')).toEqual({
            kind: 'value',
            stack: [{ type: 'string', value: 'TABLE1@B1' }],
        });
        expect(evaluator.getState('TABLE1@A1')?.kind).toBe('value');
    });

    test('an ERROR result becomes an error state, not a value', async () => {
        const evaluator = new SheetEvaluator(async () => ({
            status: 'ERROR',
            message: 'Unknown word: NOPE',
        }));

        await evaluator.applyPlan({ order: ['TABLE1@A1'], cyclic: [], blocked: [] });
        expect(evaluator.getState('TABLE1@A1')).toEqual({
            kind: 'error',
            message: 'Unknown word: NOPE',
        });
    });

    test('an executor rejection becomes an error state', async () => {
        const evaluator = new SheetEvaluator(async () => {
            throw new Error('worker died');
        });

        await evaluator.applyPlan({ order: ['TABLE1@A1'], cyclic: [], blocked: [] });
        expect(evaluator.getState('TABLE1@A1')).toEqual({
            kind: 'error',
            message: 'worker died',
        });
    });

    test('cyclic and blocked cells are marked without executing', async () => {
        const calls: string[] = [];
        const evaluator = new SheetEvaluator(async (fqName) => {
            calls.push(fqName);
            return ok(fqName);
        });

        const changed = await evaluator.applyPlan({
            order: [],
            cyclic: ['TABLE1@A1', 'TABLE1@B1'],
            blocked: ['TABLE1@C1'],
        });

        expect(calls).toEqual([]);
        expect(changed.get('TABLE1@A1')).toEqual({ kind: 'cyclic' });
        expect(changed.get('TABLE1@C1')).toEqual({ kind: 'blocked' });
    });

    test('a host error short-circuits execution for that cell only', async () => {
        const calls: string[] = [];
        const evaluator = new SheetEvaluator(async (fqName) => {
            calls.push(fqName);
            return ok(fqName);
        });

        await evaluator.applyPlan(
            { order: ['TABLE1@A1', 'TABLE1@B1'], cyclic: [], blocked: [] },
            {
                resolveHostError: (fqName) =>
                    fqName === 'TABLE1@A1' ? 'range too large' : null,
            },
        );

        expect(calls).toEqual(['TABLE1@B1']);
        expect(evaluator.getState('TABLE1@A1')).toEqual({
            kind: 'error',
            message: 'range too large',
        });
    });

    test('overlapping applyPlan calls run one after another', async () => {
        const events: string[] = [];
        let release: (() => void) | null = null;
        const evaluator = new SheetEvaluator(async (fqName) => {
            events.push(`start ${fqName}`);
            if (fqName === 'TABLE1@A1') {
                await new Promise<void>((resolve) => {
                    release = resolve;
                });
            }
            events.push(`end ${fqName}`);
            return ok(fqName);
        });

        const first = evaluator.applyPlan({ order: ['TABLE1@A1'], cyclic: [], blocked: [] });
        const second = evaluator.applyPlan({ order: ['TABLE1@B1'], cyclic: [], blocked: [] });

        // Give the first run a chance to start, then release it.
        await new Promise((resolve) => setTimeout(resolve, 0));
        expect(events).toEqual(['start TABLE1@A1']);
        (release as unknown as () => void)();
        await Promise.all([first, second]);

        expect(events).toEqual([
            'start TABLE1@A1',
            'end TABLE1@A1',
            'start TABLE1@B1',
            'end TABLE1@B1',
        ]);
    });

    test('invalidate forgets a cell state', async () => {
        const evaluator = new SheetEvaluator(async (fqName) => ok(fqName));
        await evaluator.applyPlan({ order: ['TABLE1@A1'], cyclic: [], blocked: [] });
        evaluator.invalidate('TABLE1@A1');
        expect(evaluator.getState('TABLE1@A1')).toBeNull();
    });
});
