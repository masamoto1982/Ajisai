// js/main.js

import { GUI_INSTANCE } from './gui/main.js';
import { initWasm } from './wasm-loader.js';

/**
 * アプリケーションのメインエントリーポイント
 */
async function main() {
    try {
        console.log('Application starting...');

        // WASM読み込みを一時的に無効化してデバッグ
        console.log('WASM loading temporarily disabled for debugging');
        
        // モックインタープリタを作成
        window.ajisaiInterpreter = {
            execute: () => ({ status: 'OK', output: 'WASM disabled for debugging' }),
            get_stack: () => [],
            get_register: () => null,
            get_custom_words_info: () => [],
            init_step: () => 'OK',
            step: () => ({ hasMore: false, output: '', position: 0, total: 0 })
        };
        console.log('Mock interpreter created.');
        
        // GUIを初期化
        GUI_INSTANCE.init();
        console.log('GUI initialized.');

        // データベース読み込みをスキップ
        console.log('Database loading skipped for debugging');
        
        // 基本的な表示更新
        GUI_INSTANCE.updateAllDisplays();
        GUI_INSTANCE.display.showInfo('Ready (Debug mode - WASM disabled).');

    } catch (error) {
        console.error('An error occurred during application startup:', error);
        // ユーザー向けのエラー表示
        const outputDisplay = document.getElementById('output-display');
        if (outputDisplay) {
            outputDisplay.textContent = `アプリケーションの起動に失敗しました: ${error.message}`;
        }
    }
}

// アプリケーションの実行開始
document.addEventListener('DOMContentLoaded', main);
