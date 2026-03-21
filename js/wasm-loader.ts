import type { WasmModule } from './wasm-types';

let wasmModule: WasmModule | null = null;
let compiledModule: WebAssembly.Module | null = null;

export async function initWasm(): Promise<WasmModule | null> {
    if (wasmModule) return wasmModule;

    try {
        // WASMモジュールを一度だけコンパイルする。
        // コンパイル済みモジュールはワーカーにpostMessageで転送し、
        // ワーカー側での再コンパイルを完全に回避する。
        if (!compiledModule) {
            const wasmUrl = new URL('./pkg/ajisai_core_bg.wasm', import.meta.url);
            try {
                compiledModule = await WebAssembly.compileStreaming(fetch(wasmUrl));
            } catch {
                // compileStreamingが利用不可の場合のフォールバック
                const response = await fetch(wasmUrl);
                const bytes = await response.arrayBuffer();
                compiledModule = await WebAssembly.compile(bytes);
            }
        }

        const module = await import('./pkg/ajisai_core.js') as unknown as WasmModule;

        // コンパイル済みモジュールを渡して高速にインスタンス化
        // (wasm-bindgen の新APIはオブジェクト形式を要求する)
        if (module.default) {
            await (module.default as (input?: unknown) => Promise<unknown>)({ module_or_path: compiledModule });
        } else if (module.init) {
            await (module.init as (input?: unknown) => Promise<unknown>)({ module_or_path: compiledModule });
        }

        wasmModule = module;
        return module;
    } catch (error) {
        console.error('Failed to load WASM:', error);
        return null;
    }
}

/**
 * コンパイル済みWebAssembly.Moduleを返す。
 * WorkerManagerがワーカーに転送するために使用する。
 * initWasm()の後に呼び出すこと。
 */
export function extractCompiledWasmModule(): WebAssembly.Module | null {
    return compiledModule;
}
