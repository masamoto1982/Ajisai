// js/gui.js

// 後方互換性のためのエクスポート
import { GUI_INSTANCE } from './gui/main.js';

// グローバルに公開
window.GUI = GUI_INSTANCE;

// エクスポート
export { GUI_INSTANCE as GUI };
