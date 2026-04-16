

import { GUI_INSTANCE } from './gui/gui-application';
import { initWasm } from './wasm-module-loader';
import './indexeddb-user-word-store';
import type { WasmModule, AjisaiInterpreter } from './wasm-interpreter-types';

declare const __AJISAI_BUILD_VERSION__: string;

declare global {
    interface Window {
        AjisaiWasm: WasmModule;
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

function setBuildVersionLabel(): void {
    const versionElement = document.querySelector<HTMLElement>('.version');
    if (!versionElement) return;

    versionElement.textContent = `ver.${__AJISAI_BUILD_VERSION__}`;
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


        setupOnlineStatusMonitoring();

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


if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('./service-worker.js')
            .then(registration => {
                console.log('[Main] Service Worker registered:', registration.scope);


                registration.addEventListener('updatefound', () => {
                    const newWorker = registration.installing;
                    console.log('[Main] New service worker found');

                    newWorker?.addEventListener('statechange', () => {
                        if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {

                            console.log('[Main] New version available');
                            try {
                                const display = GUI_INSTANCE.extractDisplay();
                                display.renderInfo('New version available. Please reload.', true);
                            } catch {

                            }
                        }
                    });
                });
            })
            .catch(error => {
                console.error('[Main] Service Worker registration failed:', error);
            });
    });
}


function setupOnlineStatusMonitoring(): void {
    const offlineIndicator = document.getElementById('offline-indicator');
    let isInitialCheck = true;

    function updateOnlineStatus() {
        if (navigator.onLine) {
            console.log('[Main] Online');
            if (offlineIndicator) offlineIndicator.style.display = 'none';

            if (!isInitialCheck) {
                try {
                    const display = GUI_INSTANCE.extractDisplay();
                    display.renderInfo('Online mode', true);
                } catch {

                }
            }
        } else {
            console.log('[Main] Offline');
            if (offlineIndicator) offlineIndicator.style.display = 'inline';
            try {
                const display = GUI_INSTANCE.extractDisplay();
                display.renderInfo('Offline mode', true);
            } catch {

            }
        }
        isInitialCheck = false;
    }

    window.addEventListener('online', updateOnlineStatus);
    window.addEventListener('offline', updateOnlineStatus);


    updateOnlineStatus();
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
