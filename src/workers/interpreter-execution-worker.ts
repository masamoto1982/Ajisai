

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
        if (event.data.executionMode) {
            interpreter!.set_execution_mode(event.data.executionMode);
        }

        if (isAborted) throw new Error('aborted');

        const metricsBefore = collectMetrics(interpreter!);
        const result: ExecuteResult = await interpreter!.execute(event.data.code);
        result.runtimeMetricsDelta = diffMetrics(metricsBefore, collectMetrics(interpreter!));

        // Attach the lossless stack snapshot (SPEC §2.3) so the main thread
        // restores exact post-run values (CodeBlock, ExactScalar) instead of the
        // lossy observation `stack`. The interpreter still holds the post-execute
        // state here, so this captures the result stack exactly. A snapshot
        // failure degrades to the observation `stack`, never dropping the result.
        if (typeof interpreter!.snapshot_stack === 'function') {
            try {
                result.stackSnapshot = interpreter!.snapshot_stack();
            } catch (e) {
                console.warn('[Worker] snapshot_stack failed; using observation stack', e);
            }
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
