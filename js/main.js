// js/main.js

import { GUI_INSTANCE } from './gui/main.js';
import { initWasm } from './wasm-loader.js';

async function main() {
    try {
        // 1. WASMモジュールの初期化を待つ
        const wasm = await initWasm();
        if (!wasm) {
            console.error('WASM initialization failed. Aborting application startup.');
            // ユーザーにエラーメッセージを表示する処理などをここに追加
            return;
        }
        window.HolonWasm = wasm;
        console.log('WASM loaded and initialized successfully.');

        // 2. Ajisaiインタープリタを作成
        window.ajisaiInterpreter = new window.HolonWasm.AjisaiInterpreter();
        console.log('Ajisai interpreter created.');

        // 3. GUIを初期化（この時点でajisaiInterpreterは利用可能）
        GUI_INSTANCE.init();
        console.log('GUI initialized.');
        
        // 4. (オプション) データベースからのデータ読み込み完了を待ってから最終表示更新
        // Persistence.init()内でデータロードまで完了させるように改修するとより堅牢になります。
        // await GUI_INSTANCE.persistence.loadDatabaseData();
        // GUI_INSTANCE.updateDisplay();

    } catch (error) {
        console.error('An error occurred during application startup:', error);
        // ユーザー向けのエラー表示
        const outputDisplay = document.getElementById('output-display');
        if (outputDisplay) {
            outputDisplay.textContent = 'アプリケーションの起動に失敗しました。詳細はコンソールを確認してください。';
        }
    }
}

// アプリケーションの実行開始
main();
