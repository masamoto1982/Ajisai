// js/main.js

import { GUI_INSTANCE } from './gui/main.js';
import { initWasm } from './wasm-loader.js';

/**
 * DOM（ページの構造）の読み込みが完了したときに実行されるメインの処理です。
 * HTMLの解析が終わった直後にこの中のコードが動きます。
 */
document.addEventListener('DOMContentLoaded', () => {
    console.log('DOMContentLoaded - initializing GUI');
    // インポートしたGUIインスタンスの初期化メソッド `init()` を呼び出します。
    // これにより、画面のボタンや表示エリアなどの設定が行われます。
    GUI_INSTANCE.init();
});

/**
 * WASMモジュールの読み込みが完了したことを知らせる 'wasmLoaded' イベントを監視します。
 * このイベントは、WASMの非同期読み込みが成功した後に発火します。
 */
window.addEventListener('wasmLoaded', () => {
    // グローバル変数 `window.HolonWasm` にWASMモジュールが格納されていることを確認します。
    if (window.HolonWasm) {
        // Ajisaiのインタープリタ（コード解釈・実行エンジン）を作成し、
        // グローバル変数 `window.ajisaiInterpreter` に格納します。
        // これにより、アプリケーションのどこからでもインタープリタにアクセスできるようになります。
        window.ajisaiInterpreter = new window.HolonWasm.AjisaiInterpreter();
        console.log('Ajisai interpreter initialized');
    }
});

/**
 * WASMモジュールの初期化を非同期で開始します。
 * この処理はページの読み込みと並行して行われます。
 */
initWasm().then(wasm => {
    // 読み込みが成功した場合
    if (wasm) {
        // 読み込んだWASMモジュールをグローバル変数 `window.HolonWasm` に格納します。
        window.HolonWasm = wasm;
        console.log('WASM loaded successfully from main.js');

        // 'wasmLoaded' という名前のカスタムイベントを作成し、
        // アプリケーション全体にWASMの準備ができたことを通知します。
        window.dispatchEvent(new Event('wasmLoaded'));
    }
}).catch(error => {
    // 読み込みに失敗した場合は、コンソールに警告メッセージを表示します。
    // これにより、WASMがなくても限定的ながら動作を継続できる可能性があります。
    console.warn('WASM loading failed, continuing without WASM:', error);
});
