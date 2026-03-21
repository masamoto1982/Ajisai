// js/workers/ajisai-worker.ts

import type { AjisaiInterpreter, ExecuteResult } from '../wasm-types';

let interpreter: AjisaiInterpreter | null = null;
let isAborted = false;
let currentTaskId: string | null = null;

// JSグルーコードを先行ロード。
// import()はJSバインディングの読み込みのみ行い、
// WASMコンパイルはinitSync/default呼び出し時まで発生しない。
const bindingsPromise = import('../pkg/ajisai_core.js');

/**
 * メインスレッドから受け取ったコンパイル済みWebAssembly.Moduleで初期化する。
 * initSync()はコンパイル済みモジュールからのインスタンス化のみ行うため、
 * WASMの再コンパイルが発生せず、ほぼ瞬時に完了する。
 */
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

/**
 * フォールバック: コンパイル済みモジュールが提供されない場合、
 * default()で自力初期化する（WASMコンパイルが発生するため低速）。
 */
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
        // メインスレッドからコンパイル済みWebAssembly.Moduleを受信
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

    // インタプリタ未初期化の場合、フォールバックで初期化を試行
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
        // 実行前に状態を復元
        interpreter!.reset();
        if (event.data.state?.importedModules?.length) {
            interpreter!.restore_imported_modules(event.data.state.importedModules);
        }
        if (event.data.state?.stack) {
            interpreter!.restore_stack(event.data.state.stack);
        }
        if (event.data.state?.customWords) {
            interpreter!.restore_custom_words(event.data.state.customWords);
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
