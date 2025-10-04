// js/workers/ajisai-worker.ts

import type { WasmModule, AjisaiInterpreter, ExecuteResult } from '../wasm-types';

let interpreter: AjisaiInterpreter | null = null;
let isAborted = false;
let currentTaskId: string | null = null;

async function init() {
    if (interpreter) return;
    try {
        const wasmModule = await import('../pkg/ajisai_core.js') as unknown as WasmModule;
        if (wasmModule.default) {
            await wasmModule.default();
        } else if (wasmModule.init) {
            await wasmModule.init();
        }
        interpreter = new wasmModule.AjisaiInterpreter();
        console.log('[Worker] WASM Interpreter Initialized');
    } catch (e) {
        console.error('[Worker] Failed to initialize WASM', e);
        // 初期化失敗をメインスレッドに通知
        self.postMessage({ type: 'error', id: 'init', data: 'Worker WASM initialization failed' });
    }
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
        if (isAborted || error.message === 'aborted') {
            self.postMessage({ type: 'aborted', id });
        } else {
            self.postMessage({ type: 'error', id, data: error.toString() });
        }
    } finally {
        currentTaskId = null;
    }
};

// Workerがロードされたらすぐに初期化を開始
init();
