import { initWasm } from '../wasm-module-loader';
import { createAjisaiRuntime } from './ajisai-runtime';
import type { AjisaiRuntime } from './ajisai-runtime-types';

export const createAjisaiRuntimeFromWasm = async (): Promise<AjisaiRuntime> => {
    const wasm = await initWasm();
    if (!wasm) {
        throw new Error('WASM initialization failed.');
    }

    const interpreter = new wasm.AjisaiInterpreter();
    return createAjisaiRuntime(interpreter);
};
