// 後方互換性のためのエクスポート
import { GUI } from './gui/main.js';

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
