

import type {
    AjisaiInterpreter,
    ExecuteResult,
    RuntimeMetricsSnapshot,
} from '../wasm-interpreter-types';
import { applyInterpreterSnapshot } from './interpreter-snapshot';

// Cost-model counters (SPEC §4.8) are session-cumulative on the interpreter,
// and this worker's interpreter is reused across runs, so the per-run
// activity is the before/after delta around one execute call. Undefined when
// the wasm bundle predates collect_runtime_metrics.
const collectMetrics = (interp: AjisaiInterpreter): RuntimeMetricsSnapshot | undefined =>
    interp.collect_runtime_metrics?.();

const diffMetrics = (
    before: RuntimeMetricsSnapshot | undefined,
    after: RuntimeMetricsSnapshot | undefined
): RuntimeMetricsSnapshot | undefined => {
    if (!before || !after) return undefined;
    const delta = {} as Record<keyof RuntimeMetricsSnapshot, number>;
    for (const key of Object.keys(after) as Array<keyof RuntimeMetricsSnapshot>) {
        delta[key] = Math.max(0, (after[key] ?? 0) - (before[key] ?? 0));
    }
    return delta;
};

let interpreter: AjisaiInterpreter | null = null;
let isAborted = false;
let currentTaskId: string | null = null;


const bindingsPromise = import('../wasm/generated/ajisai_core.js');


async function initFromCompiledModule(wasmModule: WebAssembly.Module): Promise<boolean> {
    try {
        const bindings = await bindingsPromise;
        bindings.initSync({ module: wasmModule });
        interpreter = new bindings.AjisaiInterpreter() as unknown as AjisaiInterpreter;
        console.log('[Worker] Initialized from pre-compiled module');
        return true;
    } catch (e) {
        console.error('[Worker] Failed to init from pre-compiled module:', e);
        return false;
    }
}


async function initFallback(): Promise<boolean> {
    if (interpreter) return true;
    try {
        const bindings = await bindingsPromise;
        await bindings.default({});
        interpreter = new bindings.AjisaiInterpreter() as unknown as AjisaiInterpreter;
        console.log('[Worker] Initialized via fallback (default init)');
        return true;
    } catch (e) {
        console.error('[Worker] Fallback initialization failed:', e);
        return false;
    }
}

self.onmessage = async (event: MessageEvent) => {
    const { type, id } = event.data;

    if (type === 'init') {

        if (event.data.wasmModule instanceof WebAssembly.Module) {
            await initFromCompiledModule(event.data.wasmModule);
        }
        return;
    }

    if (type === 'abort') {
        if (id === currentTaskId || id === '*') {
            isAborted = true;
        }
        return;
    }

    if (type !== 'execute') return;


    if (!interpreter) {
        const success = await initFallback();
        if (!success) {
            self.postMessage({ type: 'error', id, data: 'Interpreter not initialized' });
            return;
        }
    }

    isAborted = false;
    currentTaskId = id;

    try {

        applyInterpreterSnapshot(interpreter!, event.data.state);

        if (isAborted) throw new Error('aborted');

        const metricsBefore = collectMetrics(interpreter!);
        const result: ExecuteResult = await interpreter!.execute(event.data.code);
        result.runtimeMetricsDelta = diffMetrics(metricsBefore, collectMetrics(interpreter!));

        // Attach the lossless stack snapshot (SPEC §2.3): it is the format the
        // main thread restores from, so exact post-run values (CodeBlock,
        // ExactScalar) survive instead of the lossy observation `stack`. The
        // interpreter still holds the post-execute state here, so this captures
        // the result stack exactly.
        //
        // Its own failure is reported as its own: the snapshot codec refuses a
        // value it cannot encode without loss (`PI`, and anything built from
        // it, is a Tier-2 computable real), and this runs *after* the program
        // has already succeeded. Letting that throw out of the shared `try`
        // blamed the program for it — `PI` alone answered "cannot persist a
        // Tier-2 computable exact real" and read as a Word that does not work,
        // while the run had in fact produced π.
        try {
            result.stackSnapshot = interpreter!.snapshot_stack();
        } catch (snapshotError: any) {
            result.stackSnapshotError = String(snapshotError?.message ?? snapshotError);
        }

        if (isAborted) throw new Error('aborted');

        self.postMessage({ type: 'result', id, data: result });

    } catch (error: any) {
        if (isAborted || error.message === 'aborted') {
            self.postMessage({ type: 'aborted', id });
        } else {
            self.postMessage({ type: 'error', id, data: error.toString() });
        }
    } finally {
        currentTaskId = null;
    }
};
