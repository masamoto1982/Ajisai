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

// Service Worker Registration for PWA/Offline support
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('/service-worker.js')
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
                            if (GUI_INSTANCE && GUI_INSTANCE.display) {
                                GUI_INSTANCE.display.showInfo('新しいバージョンが利用可能です。ページを再読み込みしてください。', true);
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

// オフライン/オンライン状態の監視
const offlineIndicator = document.getElementById('offline-indicator');

function updateOnlineStatus() {
    if (navigator.onLine) {
        console.log('[Main] Online');
        if (offlineIndicator) offlineIndicator.style.display = 'none';
        if (GUI_INSTANCE && GUI_INSTANCE.display) {
            GUI_INSTANCE.display.showInfo('オンラインに戻りました', true);
        }
    } else {
        console.log('[Main] Offline');
        if (offlineIndicator) offlineIndicator.style.display = 'inline';
        if (GUI_INSTANCE && GUI_INSTANCE.display) {
            GUI_INSTANCE.display.showInfo('⚠ オフラインモードで動作中', true);
        }
    }
}

window.addEventListener('online', updateOnlineStatus);
window.addEventListener('offline', updateOnlineStatus);

// 初期状態をチェック
window.addEventListener('load', () => {
    updateOnlineStatus();
});

document.addEventListener('DOMContentLoaded', main);
