// Sheet view cell evaluation (redesign plan §2). UI-free: the actual
// execution is injected as a `CellExecutor`, so the module is unit-testable
// without WASM or workers. Phase 2 evaluates serially in plan order; the
// worker-parallel upgrade (plan Phase 7) only changes the executor side.
//
// Evaluation semantics (plan §2.1): a cell word runs on an empty stack
// against the shared vocabulary, and the final stack is the cell's value.
// A multi-value stack is kept as-is (rendered stacked; plan §8.1). Channel
// errors do not become values — the cell enters an error state instead
// (plan §2.2). Cell values are a host display cache only; the word remains
// the source of truth (plan §2.4).

import type { ExecuteResult, Value } from '../wasm-interpreter-types';
import type { RecalcPlan } from './sheet-engine';

/** Display state of one evaluated cell, keyed by fully-qualified word. */
export type CellEvaluationState =
    | { kind: 'value'; stack: Value[] }
    | { kind: 'error'; message: string }
    | { kind: 'cyclic' }
    | { kind: 'blocked' };

/**
 * Runs one cell word (fully qualified, e.g. `TABLE1@A1`) on an empty stack
 * and reports the result. The GUI wires this to the execution-worker path;
 * tests wire a stub.
 */
export type CellExecutor = (fqName: string) => Promise<ExecuteResult>;

export interface ApplyPlanOptions {
    /**
     * Host-side error for a cell (e.g. a preprocessing failure recorded on
     * its record). When non-null the cell is marked as an error without
     * being executed.
     */
    resolveHostError?: (fqName: string) => string | null;
}

export class SheetEvaluator {
    private readonly states = new Map<string, CellEvaluationState>();
    /** Serializes applyPlan calls so rapid edits cannot interleave. */
    private chain: Promise<unknown> = Promise.resolve();

    constructor(private readonly executeCell: CellExecutor) {}

    getState(fqName: string): CellEvaluationState | null {
        return this.states.get(fqName) ?? null;
    }

    /** Forget a cell (cleared cell / removed word). */
    invalidate(fqName: string): void {
        this.states.delete(fqName);
    }

    clear(): void {
        this.states.clear();
    }

    /**
     * Evaluate a recalculation plan serially (upstream first) and return the
     * cells whose state was (re)computed. Cyclic and blocked cells are
     * marked without evaluation (plan §2.4: the host refuses cell-reference
     * cycles spreadsheet-style).
     */
    applyPlan(
        plan: RecalcPlan,
        options: ApplyPlanOptions = {},
    ): Promise<Map<string, CellEvaluationState>> {
        const run = this.chain.then(() => this.runPlan(plan, options));
        // Keep the chain alive even when a run rejects.
        this.chain = run.catch(() => undefined);
        return run;
    }

    private async runPlan(
        plan: RecalcPlan,
        options: ApplyPlanOptions,
    ): Promise<Map<string, CellEvaluationState>> {
        const changed = new Map<string, CellEvaluationState>();
        const record = (fqName: string, state: CellEvaluationState): void => {
            this.states.set(fqName, state);
            changed.set(fqName, state);
        };

        for (const fqName of plan.cyclic) {
            record(fqName, { kind: 'cyclic' });
        }
        for (const fqName of plan.blocked) {
            record(fqName, { kind: 'blocked' });
        }

        for (const fqName of plan.order) {
            const hostError = options.resolveHostError?.(fqName) ?? null;
            if (hostError !== null) {
                record(fqName, { kind: 'error', message: hostError });
                continue;
            }
            try {
                const result = await this.executeCell(fqName);
                if (result.status === 'OK' && !result.error) {
                    record(fqName, { kind: 'value', stack: result.stack ?? [] });
                } else {
                    record(fqName, {
                        kind: 'error',
                        message: result.message ?? 'Unknown error',
                    });
                }
            } catch (error) {
                record(fqName, {
                    kind: 'error',
                    message: error instanceof Error ? error.message : String(error),
                });
            }
        }

        return changed;
    }
}
