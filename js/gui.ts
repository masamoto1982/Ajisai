// js/gui.ts

// 後方互換性のためのエクスポート
import { GUI_INSTANCE } from './gui/main';

// グローバルに公開
(window as any).GUI = GUI_INSTANCE;

// エクスポート
export { GUI_INSTANCE as GUI };

// DOMContentLoadedで初期化
document.addEventListener('DOMContentLoaded', () => {
    (window as any).GUI.init();
});

// WASM初期化完了時にインタープリタを作成
window.addEventListener('wasmLoaded', () => {
    if ((window as any).HolonWasm) {
        (window as any).ajisaiInterpreter = new (window as any).HolonWasm.AjisaiInterpreter();
        console.log('Ajisai interpreter initialized');
    }
});
