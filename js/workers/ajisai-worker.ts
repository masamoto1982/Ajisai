// js/workers/ajisai-worker.ts

import type { WasmModule, AjisaiInterpreter, ExecuteResult } from '../wasm-types';

let interpreter: AjisaiInterpreter | null = null;
let isAborted = false;
let currentTaskId: string | null = null;
let initAttempts = 0;
let isInitializing = false;
const MAX_INIT_ATTEMPTS = 3;
const INIT_RETRY_DELAY_MS = 500;

const sleep = (ms: number): Promise<void> =>
    new Promise(resolve => setTimeout(resolve, ms));

async function init(): Promise<boolean> {
    if (interpreter) return true;
    if (isInitializing) {
        // 初期化中の場合は完了を待つ
        while (isInitializing) {
            await sleep(50);
        }
        return interpreter !== null;
    }

    isInitializing = true;

    while (initAttempts < MAX_INIT_ATTEMPTS) {
        initAttempts++;
        try {
            const wasmModule = await import('../pkg/ajisai_core.js') as unknown as WasmModule;
            if (wasmModule.default) {
                await wasmModule.default();
            } else if (wasmModule.init) {
                await wasmModule.init();
            }
            interpreter = new wasmModule.AjisaiInterpreter();
            console.log('[Worker] WASM Interpreter Initialized');
            isInitializing = false;
            return true;
        } catch (e) {
            console.error(`[Worker] Failed to initialize WASM (attempt ${initAttempts}/${MAX_INIT_ATTEMPTS})`, e);
            if (initAttempts < MAX_INIT_ATTEMPTS) {
                await sleep(INIT_RETRY_DELAY_MS * initAttempts);
            }
        }
    }

    // すべてのリトライが失敗
    isInitializing = false;
    console.error('[Worker] WASM initialization failed after all retries');
    self.postMessage({ type: 'error', id: 'init', data: 'Worker WASM initialization failed after all retries' });
    return false;
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

    const initSuccess = await init();
    if (!initSuccess || !interpreter) {
        self.postMessage({ type: 'error', id, data: 'Interpreter not initialized after all retries' });
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
