

import type { AjisaiInterpreter, ExecuteResult } from '../wasm-interpreter-types';
import { applyInterpreterSnapshot } from './interpreter-snapshot';

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

        const result: ExecuteResult = await interpreter!.execute(event.data.code);

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
