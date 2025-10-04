// js/workers/ajisai-worker.ts

import type { WasmModule, AjisaiInterpreter, ExecuteResult, Value, CustomWord } from '../wasm-types';

let interpreter: AjisaiInterpreter | null = null;
let isAborted = false;
let currentTaskId: string | null = null;

async function init() {
    if (interpreter) return;
    const wasmModule = await import('../pkg/ajisai_core.js') as unknown as WasmModule;
    if (wasmModule.default) {
        await wasmModule.default();
    }
    interpreter = new wasmModule.AjisaiInterpreter();
    console.log('[Worker] WASM Interpreter Initialized');
}

self.onmessage = async (event: MessageEvent) => {
    const { type, id, code, state } = event.data;
    
    if (type === 'abort') {
        if (id === currentTaskId || id === '*') {
            isAborted = true;
        }
        return;
    }

    if (type !== 'execute') return;

    await init();
    if (!interpreter) {
        self.postMessage({ type: 'error', id, data: 'Interpreter not initialized' });
        return;
    }

    isAborted = false;
    currentTaskId = id;

    try {
        // 実行前に状態を復元
        interpreter.reset();
        if (state.stack) {
            interpreter.restore_stack(state.stack);
        }
        if (state.customWords) {
            interpreter.restore_custom_words(state.customWords);
        }

        if (isAborted) throw new Error('aborted');

        const result: ExecuteResult = await interpreter.execute(code);

        if (isAborted) throw new Error('aborted');

        self.postMessage({ type: 'result', id, data: result });

    } catch (error: any) {
        if (error.message === 'aborted') {
            self.postMessage({ type: 'aborted', id });
        } else {
            self.postMessage({ type: 'error', id, data: error.toString() });
        }
    } finally {
        currentTaskId = null;
    }
};

init();
