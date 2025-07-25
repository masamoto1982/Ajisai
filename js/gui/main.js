// js/gui/main.js

import { Display } from './display.js';
import { Dictionary } from './dictionary.js';
import { Editor } from './editor.js';
import { Stepper } from './stepper.js';
import { MobileHandler } from './mobile.js';
import { Persistence } from './persistence.js';

// GUIクラスの定義
class GUI {
    constructor() {
        this.display = new Display();
        this.dictionary = new Dictionary();
        this.editor = new Editor();
        this.stepper = new Stepper();
        this.mobile = new MobileHandler();
        this.persistence = new Persistence(this);

        this.elements = {};
        this.mode = 'input';
        this.stepMode = false;
    }

    init() {
        console.log('GUI.init() called');
        this.cacheElements();

        // 各モジュールの初期化
        this.display.init(this.elements);
        // ★★★ 修正点 ★★★
        // ワードクリック時にスペースを追加しないように修正
        this.dictionary.init(this.elements, (word) => this.editor.insertWord(word));
        this.editor.init(this.elements.codeInput);
        this.stepper.init(() => window.ajisaiInterpreter);
        this.mobile.init(this.elements);
        this.persistence.init();

        this.setupEventListeners();

        // 初期表示
        this.dictionary.renderBuiltinWords();
        this.updateAllDisplays();
        this.mobile.updateView(this.mode);
    }

    cacheElements() {
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
    }
    
    setupEventListeners() {
    this.elements.runBtn.addEventListener('click', () => this.runNormal());
    this.elements.clearBtn.addEventListener('click', () => this.editor.clear());

    this.elements.codeInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            if (e.shiftKey) {
                e.preventDefault();
                this.runNormal();
            } else if (e.ctrlKey) {
                e.preventDefault();
                this.runStep();
            }
        }
    });

    this.elements.memoryArea.addEventListener('click', () => {
        if (this.mobile.isMobile() && this.mode === 'execution') {
            this.setMode('input');
        }
    });

    window.addEventListener('resize', () => this.mobile.updateView(this.mode));
    
    // タイムアウトチェック用のタイマーを設定（10秒ごとに実行）
    setInterval(() => {
        if (window.ajisaiInterpreter) {
            window.ajisaiInterpreter.cleanup_expired_entries();
            this.updateAllDisplays();
        }
    }, 10000); // 10秒ごとにチェック
}

    setMode(newMode) {
        this.mode = newMode;
        this.mobile.updateView(this.mode);
    }
    
    async runNormal() {
        const code = this.editor.getValue();
        if (!code) return;

        this.stepMode = false;
        this.updateRunButton();

        try {
            const result = window.ajisaiInterpreter.execute(code);
            if (result.status === 'OK') {
                this.display.showOutput(result.output || 'OK');
                this.editor.clear();
                if (this.mobile.isMobile()) {
                    this.setMode('execution');
                }
            } else {
                this.display.showError(result);
            }
        } catch (error) {
            this.display.showError(error);
        }
        
        this.updateAllDisplays();
        await this.persistence.saveCurrentState();
        this.display.showInfo('State saved.', true);
    }

    async runStep() {
        const code = this.editor.getValue();
        if (!code && !this.stepMode) return;

        try {
            if (!this.stepMode) {
                const result = await this.stepper.start(code);
                if (result.ok) {
                    this.stepMode = true;
                    this.updateRunButton();
                    await this.continueStep();
                } else {
                    this.display.showError(result.error);
                }
            } else {
                await this.continueStep();
            }
        } catch(error) {
            this.display.showError(error);
            this.resetStepMode();
        }
    }

    async continueStep() {
        const result = await this.stepper.step();
        
        if (result.output) {
            this.display.showOutput(result.output);
        }

        if (result.hasMore) {
            this.display.showInfo(`Step ${result.position}/${result.total}: Press Ctrl+Enter to continue...`);
        } else {
            this.display.showInfo('Step execution completed.');
            this.resetStepMode();
        }
        
        this.updateAllDisplays();
    }
    
    resetStepMode() {
        this.stepMode = false;
        this.stepper.reset();
        this.updateRunButton();
    }

    updateRunButton() {
        this.elements.runBtn.textContent = this.stepMode ? 'Step' : 'Run';
    }

    updateAllDisplays() {
        if (!window.ajisaiInterpreter) return;
        try {
            this.display.updateStack(window.ajisaiInterpreter.get_stack());
            this.display.updateRegister(window.ajisaiInterpreter.get_register());
            this.dictionary.updateCustomWords(window.ajisaiInterpreter.get_custom_words_info());
        } catch (error) {
            console.error('Failed to update display:', error);
            this.display.showError('Failed to update display.');
        }
    }
}

// GUIインスタンスを作成してエクスポート
export const GUI_INSTANCE = new GUI();
