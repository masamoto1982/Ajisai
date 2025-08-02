// js/wasm-loader.ts

import type { WasmModule } from './wasm-types';

let wasmModule: WasmModule | null = null;

export async function initWasm(): Promise<WasmModule | null> {
    if (wasmModule) return wasmModule;
    
    try {
        console.log('Attempting to load WASM module...');
        
        const baseUrl = import.meta.url.substring(0, import.meta.url.lastIndexOf('/'));
        console.log('Base URL:', baseUrl);
        console.log('Expected module path:', baseUrl + '/pkg/ajisai_core.js');
        
        // @ts-ignore - Dynamic import of generated WASM module
        const module = await import('./pkg/ajisai_core.js');
        console.log('Module loaded:', module);
        
        // init関数を呼び出す（wasm-bindgen 0.2.92以降の場合）
        if (module.default) {
            await module.default();
            console.log('WASM initialized via default export');
        } else if (module.init) {
            await module.init();
            console.log('WASM initialized via init function');
        }
        
        wasmModule = module as WasmModule;
        return module;
    } catch (error) {
        console.error('Failed to load WASM:', error);
        console.error('Error details:', {
            message: (error as Error).message,
            stack: (error as Error).stack
        });
        
        // フォールバックとして直接wasmファイルをロードしてみる
        try {
            console.log('Trying fallback method...');
            const wasmPath = new URL('./pkg/ajisai_core_bg.wasm', import.meta.url);
            console.log('WASM path:', wasmPath.href);
            
            const response = await fetch(wasmPath);
            console.log('WASM fetch response:', response.status, response.statusText);
        } catch (fallbackError) {
            console.error('Fallback also failed:', fallbackError);
        }
        
        return null;
    }
}
