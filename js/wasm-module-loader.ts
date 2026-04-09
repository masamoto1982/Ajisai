import type { WasmModule } from './wasm-interpreter-types';

let wasmModule: WasmModule | null = null;
let compiledModule: WebAssembly.Module | null = null;

export async function initWasm(): Promise<WasmModule | null> {
    if (wasmModule) return wasmModule;

    try {



        if (!compiledModule) {
            const wasmUrl = new URL('./pkg/ajisai_core_bg.wasm', import.meta.url);
            try {
                compiledModule = await WebAssembly.compileStreaming(fetch(wasmUrl));
            } catch {

                const response = await fetch(wasmUrl);
                const bytes = await response.arrayBuffer();
                compiledModule = await WebAssembly.compile(bytes);
            }
        }

        const module = await import('./pkg/ajisai_core.js') as unknown as WasmModule;



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






export function extractCompiledWasmModule(): WebAssembly.Module | null {
    return compiledModule;
}
