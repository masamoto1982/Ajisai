// js/main.js

import { GUI_INSTANCE } from './gui/main.js';
import { initWasm } from './wasm-loader.js';

/**
 * アプリケーションのメインエントリーポイント
 */
async function main() {
    try {
        console.log('Application starting...');

        // 1. WASMモジュールの初期化を待つ
        const wasm = await initWasm();
        if (!wasm) {
            throw new Error('WASM initialization failed. Application cannot start.');
        }
        window.AjisaiWasm = wasm;
        console.log('WASM loaded and initialized successfully.');

        // 2. Ajisaiインタープリタを作成し、グローバルに公開
        window.ajisaiInterpreter = new window.AjisaiWasm.AjisaiInterpreter();
        console.log('Ajisai interpreter created.');
        
        // 3. GUIを初期化（この時点でajisaiInterpreterは利用可能）
        // GUI.init()は同期的にDOM要素のキャッシュとイベントリスナーの設定を行う
        GUI_INSTANCE.init();
        console.log('GUI initialized.');

        // 4. データベースから非同期でデータを読み込み、完了後にGUIを更新
        await GUI_INSTANCE.persistence.loadDatabaseData();
        GUI_INSTANCE.updateAllDisplays(); // データベース読み込み後に表示を完全に更新
        GUI_INSTANCE.display.showInfo('Ready.'); // 準備完了を通知

    } catch (error) {
        console.error('An error occurred during application startup:', error);
        // ユーザー向けのエラー表示
        const outputDisplay = document.getElementById('output-display');
        if (outputDisplay) {
            outputDisplay.textContent = `アプリケーションの起動に失敗しました: ${error.message}`;
        }
    }
}

document.addEventListener('DOMContentLoaded', async () => {
    try {
        await window.initWasm();
        console.log('WASM module initialized.');

        await window.persistence.loadDatabaseData(); // DBのロードを待つ
        console.log('Database data loaded.');

        window.gui.updateAllDisplays(); // DBロード後にUIを更新
        console.log('Initial display updated.');

        // 定期的な処理
        setInterval(() => {
            window.ajisaiInterpreter.cleanup_expired_entries();
            window.gui.updateAllDisplays();
        }, 10000);

    } catch (error) {
        console.error("Error during initialization:", error);
    }
});
