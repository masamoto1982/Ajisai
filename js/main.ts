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

        GUI_INSTANCE.updateAllDisplays();

        // GUI初期化完了後にオンライン状態を設定
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

// Service Worker Registration for PWA/Offline support
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('./service-worker.js')
            .then(registration => {
                console.log('[Main] Service Worker registered:', registration.scope);

                // 更新チェック
                registration.addEventListener('updatefound', () => {
                    const newWorker = registration.installing;
                    console.log('[Main] New service worker found');

                    newWorker?.addEventListener('statechange', () => {
                        if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
                            // 新しいバージョンが利用可能
                            console.log('[Main] New version available');
                            try {
                                const display = GUI_INSTANCE.getDisplay();
                                display.showInfo('新しいバージョンが利用可能です。ページを再読み込みしてください。', true, 'New version available. Please reload.');
                            } catch {
                                // GUI not yet initialized
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

// オフライン/オンライン状態の監視（GUI初期化後に呼ばれる）
function setupOnlineStatusMonitoring(): void {
    const offlineIndicator = document.getElementById('offline-indicator');
    let isInitialCheck = true;

    function updateOnlineStatus() {
        if (navigator.onLine) {
            console.log('[Main] Online');
            if (offlineIndicator) offlineIndicator.style.display = 'none';
            // 初回チェック時はオンラインメッセージを表示しない（サンプルワードメッセージを維持）
            if (!isInitialCheck) {
                try {
                    const display = GUI_INSTANCE.getDisplay();
                    display.showInfo('オンラインに復帰しました', true, 'Back online');
                } catch {
                    // GUI not yet initialized
                }
            }
        } else {
            console.log('[Main] Offline');
            if (offlineIndicator) offlineIndicator.style.display = 'inline';
            try {
                const display = GUI_INSTANCE.getDisplay();
                display.showInfo('オフラインモードで動作中', true, 'Offline mode');
            } catch {
                // GUI not yet initialized
            }
        }
        isInitialCheck = false;
    }

    window.addEventListener('online', updateOnlineStatus);
    window.addEventListener('offline', updateOnlineStatus);

    // 初期状態をチェック
    updateOnlineStatus();
}

document.addEventListener('DOMContentLoaded', main);
