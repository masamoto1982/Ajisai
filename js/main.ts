import { GUI_INSTANCE } from './gui/main';
import { initWasm } from './wasm-loader';
import './db';
import type { WasmModule, AjisaiInterpreter } from './wasm-types';

declare global {
    interface Window {
        AjisaiWasm: WasmModule;
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

async function main(): Promise<void> {
    try {
        const wasm = await initWasm();
        if (!wasm) {
            throw new Error('WASM initialization failed. Application cannot start.');
        }
        window.AjisaiWasm = wasm;

        window.ajisaiInterpreter = new window.AjisaiWasm.AjisaiInterpreter();
        
        GUI_INSTANCE.init();

        await GUI_INSTANCE.persistence.loadDatabaseData();
        GUI_INSTANCE.updateAllDisplays();
        GUI_INSTANCE.display.showInfo('Ready.');

    } catch (error) {
        console.error('An error occurred during application startup:', error);
        const outputDisplay = document.getElementById('output-display');
        if (outputDisplay) {
            outputDisplay.textContent = `Application startup failed: ${(error as Error).message}`;
        }
    }
}

document.addEventListener('DOMContentLoaded', main);
