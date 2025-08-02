// js/main.ts

import { GUI_INSTANCE } from './gui/main';
import { initWasm } from './wasm-loader';
import type { WasmModule, AjisaiInterpreter } from './wasm-types';

declare global {
    interface Window {
        HolonWasm: WasmModule;
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

/**
 * アプリケーションのメインエントリーポイント
 */
async function main(): Promise<void> {
    try {
        console.log('Application starting...');

        // 1. WASMモジュールの初期化を待つ
        const wasm = await initWasm();
        if (!wasm) {
            throw new Error('WASM initialization failed. Application cannot start.');
        }
        window.HolonWasm = wasm;
        console.log('WASM loaded and initialized successfully.');

        // 2. Ajisaiインタープリタを作成し、グローバルに公開
        window.ajisaiInterpreter = new window.HolonWasm.AjisaiInterpreter();
        console.log('Ajisai interpreter created.');
        
        // 3. GUIを初期化（この時点でajisaiInterpreterは利用可能）
        GUI_INSTANCE.init();
        console.log('GUI initialized.');

        // 4. データベースから非同期でデータを読み込み、完了後にGUIを更新
        await GUI_INSTANCE.persistence.loadDatabaseData();
        GUI_INSTANCE.updateAllDisplays();
        GUI_INSTANCE.display.showInfo('Ready.');

    } catch (error) {
        console.error('An error occurred during application startup:', error);
        // ユーザー向けのエラー表示
        const outputDisplay = document.getElementById('output-display');
        if (outputDisplay) {
            outputDisplay.textContent = `アプリケーションの起動に失敗しました: ${(error as Error).message}`;
        }
    }
}

// アプリケーションの実行開始
document.addEventListener('DOMContentLoaded', main);
