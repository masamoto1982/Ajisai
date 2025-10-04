// js/main.ts

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
    console.log('[Main] Starting Ajisai application...');
    
    try {
        console.log('[Main] Initializing WASM...');
        const wasm = await initWasm();
        if (!wasm) {
            throw new Error('WASM initialization failed. Application cannot start.');
        }
        window.AjisaiWasm = wasm;

        console.log('[Main] Creating main thread interpreter...');
        window.ajisaiInterpreter = new window.AjisaiWasm.AjisaiInterpreter();
        
        console.log('[Main] Initializing GUI...');
        await GUI_INSTANCE.init();

        console.log('[Main] Loading database data...');
        await GUI_INSTANCE.persistence.loadDatabaseData();
        
        GUI_INSTANCE.updateAllDisplays();
        GUI_INSTANCE.display.showInfo('Ready. Press Esc for emergency stop.');

        console.log('[Main] Application initialization completed successfully');

    } catch (error) {
        console.error('[Main] Application startup failed:', error);
        const outputDisplay = document.getElementById('output-display');
        if (outputDisplay) {
            outputDisplay.innerHTML = `
                <span style="color: #dc3545; font-weight: bold;">
                    Application startup failed: ${(error as Error).message}
                </span>
            `;
        }
    }
}

document.addEventListener('DOMContentLoaded', main);
