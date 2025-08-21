// js/gui.ts (簡素化版)

// 後方互換性のためのエクスポート
import { GUI_INSTANCE } from './gui/main';

// グローバルに公開
(window as any).GUI = GUI_INSTANCE;

// エクスポート
export { GUI_INSTANCE };

// DOMContentLoadedで初期化
document.addEventListener('DOMContentLoaded', () => {
    GUI_INSTANCE.init();
});

// WASM初期化完了時にインタープリタを作成
window.addEventListener('wasmLoaded', () => {
    if ((window as any).AjisaiWasm) {
        (window as any).ajisaiInterpreter = new (window as any).AjisaiWasm.AjisaiInterpreter();
        console.log('Ajisai interpreter initialized');
    }
});
