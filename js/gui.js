// js/gui.js

// 後方互換性のためのエクスポート
import { GUI_INSTANCE } from './gui/main.js';

// グローバルに公開
window.GUI = GUI_INSTANCE;

// エクスポート
export { GUI_INSTANCE as GUI };

// DOMContentLoadedで初期化
document.addEventListener('DOMContentLoaded', () => {
    window.GUI.init();
});

// WASM初期化完了時にインタープリタを作成
window.addEventListener('wasmLoaded', () => {
    if (window.HolonWasm) {
        window.ajisaiInterpreter = new window.HolonWasm.AjisaiInterpreter();
        console.log('Ajisai interpreter initialized');
    }
});
