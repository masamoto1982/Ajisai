import type { WasmModule } from './wasm-types';

let wasmModule: WasmModule | null = null;

export async function initWasm(): Promise<WasmModule | null> {
    if (wasmModule) return wasmModule;
    
    try {
        const module = await import('./pkg/ajisai_core.js') as WasmModule;
        
        if (module.default) {
            await module.default();
        } else if (module.init) {
            await module.init();
        }
        
        wasmModule = module;
        return module;
    } catch (error) {
        console.error('Failed to load WASM:', error);
        
        try {
            const wasmPath = new URL('./pkg/ajisai_core_bg.wasm', import.meta.url);
            await fetch(wasmPath);
        } catch (fallbackError) {
            console.error('Fallback also failed:', fallbackError);
        }
        
        return null;
    }
}
