

import { GUI_INSTANCE } from './gui/gui-application';
import { initWasm } from './wasm-module-loader';
import './indexeddb-user-word-store';
import type { WasmModule, AjisaiInterpreter } from './wasm-interpreter-types';

declare const __AJISAI_CHANGE_NOTE__: string;

declare global {
    interface Window {
        AjisaiWasm: WasmModule;
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

function formatTimestamp(date: Date): string {
    const year = date.getFullYear();
    const month = `${date.getMonth() + 1}`.padStart(2, '0');
    const day = `${date.getDate()}`.padStart(2, '0');
    const hours = `${date.getHours()}`.padStart(2, '0');
    const minutes = `${date.getMinutes()}`.padStart(2, '0');
    return `${year}${month}${day}${hours}${minutes}`;
}

function setBuildVersionLabel(): void {
    const versionElement = document.querySelector<HTMLElement>('.version');
    if (!versionElement) return;

    const timestamp = formatTimestamp(new Date());
    versionElement.textContent = `ver.${timestamp} (${__AJISAI_CHANGE_NOTE__})`;
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

        GUI_INSTANCE.updateAllDisplays();


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



function setupLogoTouchQR(): void {
    const logoSwap = document.querySelector<HTMLElement>('.logo-swap');
    if (!logoSwap) return;

    let hideTimer: ReturnType<typeof setTimeout> | null = null;

    logoSwap.addEventListener('touchstart', (e) => {
        e.preventDefault();

        if (hideTimer !== null) {
            clearTimeout(hideTimer);
            hideTimer = null;
        }

        logoSwap.classList.add('qr-active');

        hideTimer = setTimeout(() => {
            logoSwap.classList.remove('qr-active');
            hideTimer = null;
        }, 3000);
    }, { passive: false });
}

document.addEventListener('DOMContentLoaded', () => {
    setBuildVersionLabel();
    setupLogoTouchQR();
    main();
});
