// js/gui/main.js

import { Display } from './display.js';
import { Dictionary } from './dictionary.js';
import { Editor } from './editor.js';
import { Stepper } from './stepper.js';
import { MobileHandler } from './mobile.js';
import { Persistence } from './persistence.js';

// GUIクラスの定義
export class GUI {
    constructor() {
        this.display = new Display();
        this.dictionary = new Dictionary();
        this.editor = new Editor();
        this.stepper = new Stepper();
        this.mobile = new MobileHandler();
        this.persistence = new Persistence();
        this.elements = {};
    }

    init() {
        console.log('GUI.init() called');
        
        // DOM要素の取得
        this.elements = {
            codeInput: document.getElementById('code-input'),
            runBtn: document.getElementById('run-btn'),
            clearBtn: document.getElementById('clear-btn'),
            outputDisplay: document.getElementById('output-display'),
            stackDisplay: document.getElementById('stack-display'),
            registerDisplay: document.getElementById('register-display'),
            builtinWordsDisplay: document.getElementById('builtin-words-display'),
            customWordsDisplay: document.getElementById('custom-words-display'),
            inputArea: document.querySelector('.input-area'),
            outputArea: document.querySelector('.output-area'),
            memoryArea: document.querySelector('.memory-area'),
            dictionaryArea: document.querySelector('.dictionary-area')
        };

        // 各モジュールの初期化
        this.display.init(this.elements);
        this.dictionary.init(this.elements);
        this.editor.init(this.elements.codeInput);
        this.stepper.init();
        this.mobile.init(this.elements);
        this.persistence.init();

        // イベントリスナーの設定
        this.setupEventListeners();

        // 初期表示
        this.updateDisplay();
        
        // 組み込みワードを表示
        this.dictionary.renderBuiltinWords();
        
        console.log('GUI initialization complete');
    }

    setupEventListeners() {
        // 実行ボタン
        this.elements.runBtn.addEventListener('click', () => this.run());
        
        // クリアボタン
        this.elements.clearBtn.addEventListener('click', () => this.clear());
        
        // キーボードショートカット
        this.elements.codeInput.addEventListener('keydown', (e) => {
            if (e.ctrlKey && e.key === 'Enter') {
                e.preventDefault();
                this.run();
            }
        });

        // ワード挿入イベント
        window.addEventListener('insert-word', (e) => {
            this.editor.insertWord(e.detail.word + ' ');
        });

        // WASMロード完了時
        window.addEventListener('wasmLoaded', () => {
            this.updateDisplay();
        });

        // 永続化完了通知
        window.addEventListener('persistence-complete', (e) => {
            console.log(e.detail.message);
            this.updateDisplay();
        });

        // ステップモード変更
        window.addEventListener('step-mode-changed', (e) => {
            this.elements.runBtn.textContent = e.detail.active ? 'Step' : 'Run';
        });
    }

    async run() {
        const code = this.editor.getValue();
        if (!code) return;

        if (!window.ajisaiInterpreter) {
            this.display.showError('WASM not loaded yet. Please wait...');
            return;
        }

        try {
            if (this.stepper.isActive()) {
                await this.stepper.step();
            } else if (this.shouldUseStepMode()) {
                await this.stepper.start(code, (result) => {
                    this.display.showStepInfo(result);
                    this.updateDisplay();
                });
            } else {
                const result = window.ajisaiInterpreter.execute(code);
                this.display.showOutput(result.output || 'OK');
                this.updateDisplay();
                await this.persistence.saveCurrentState();
            }
        } catch (error) {
            this.display.showError(error);
            this.stepper.reset();
        }
    }

    clear() {
        this.editor.clear();
        this.editor.focus();
    }

    shouldUseStepMode() {
        return event.ctrlKey || event.metaKey;
    }

    updateDisplay() {
        if (!window.ajisaiInterpreter) return;

        try {
            // スタックの更新
            const stack = window.ajisaiInterpreter.get_stack();
            this.display.updateStack(stack);

            // レジスタの更新
            const register = window.ajisaiInterpreter.get_register();
            this.display.updateRegister(register);

            // カスタムワードの更新
            const customWords = window.ajisaiInterpreter.get_custom_words_info();
            this.dictionary.updateCustomWords(customWords);
        } catch (error) {
            console.error('Failed to update display:', error);
        }
    }
}

// GUIインスタンスを作成してエクスポート
export const GUI_INSTANCE = new GUI();

// グローバルに公開（後方互換性のため）
window.GUI = GUI_INSTANCE;
