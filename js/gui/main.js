import { Editor } from './editor.js';
import { Display } from './display.js';
import { Dictionary } from './dictionary.js';
import { MobileHandler } from './mobile.js';
import { Persistence } from './persistence.js';
import { Stepper } from './stepper.js';

export class GUI {
    constructor() {
        this.mode = 'input';
        this.elements = {};
        
        // サブモジュール
        this.editor = new Editor();
        this.display = new Display();
        this.dictionary = new Dictionary();
        this.mobile = new MobileHandler();
        this.persistence = new Persistence();
        this.stepper = new Stepper();
    }

    init() {
        this.cacheElements();
        
        // 各モジュールを初期化
        this.editor.init(this.elements.codeInput);
        this.display.init({
            outputDisplay: this.elements.outputDisplay,
            stackDisplay: this.elements.stackDisplay,
            registerDisplay: this.elements.registerDisplay
        });
        this.dictionary.init({
            builtinWordsDisplay: this.elements.builtinWordsDisplay,
            customWordsDisplay: this.elements.customWordsDisplay
        });
        this.mobile.init({
            workspacePanel: this.elements.workspacePanel,
            statePanel: this.elements.statePanel,
            inputArea: this.elements.inputArea,
            outputArea: this.elements.outputArea,
            memoryArea: this.elements.memoryArea,
            dictionaryArea: this.elements.dictionaryArea
        });
        this.stepper.init();
        
        this.setupEventListeners();
        this.setupModuleListeners();
        
        // 初期表示
        this.display.updateStack([]);
        this.display.updateRegister(null);
        this.dictionary.renderBuiltinWords();
        this.dictionary.renderCustomWords([]);
        
        // 永続化の初期化
        this.persistence.init();
    }

    cacheElements() {
        this.elements = {
            workspacePanel: document.getElementById('workspace-panel'),
            statePanel: document.getElementById('state-panel'),
            inputArea: document.querySelector('.input-area'),
            outputArea: document.querySelector('.output-area'),
            memoryArea: document.querySelector('.memory-area'),
            dictionaryArea: document.querySelector('.dictionary-area'),
            codeInput: document.getElementById('code-input'),
            outputDisplay: document.getElementById('output-display'),
            stackDisplay: document.getElementById('stack-display'),
            registerDisplay: document.getElementById('register-display'),
            builtinWordsDisplay: document.getElementById('builtin-words-display'),
            customWordsDisplay: document.getElementById('custom-words-display')
        };
    }

    setupEventListeners() {
        // Runボタン
        document.getElementById('run-btn').addEventListener('click', () => {
            this.executeCode();
        });
        
        // Clearボタン
        document.getElementById('clear-btn').addEventListener('click', () => {
            this.editor.clear();
        });
        
        // キーボードショートカット
        this.elements.codeInput.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
                if (event.shiftKey) {
                    event.preventDefault();
                    this.executeCode();
                } else if (event.ctrlKey) {
                    event.preventDefault();
                    this.handleStepExecution();
                }
            }
        });
        
        // モバイル用のメモリエリアタッチ
        this.elements.memoryArea.addEventListener('click', () => {
            if (this.mobile.isMobile() && this.mode === 'execution') {
                this.setMode('input');
            }
        });
        
        // ウィンドウリサイズ
        window.addEventListener('resize', () => {
            this.mobile.updateView(this.mode);
        });
    }

    setupModuleListeners() {
        // エディタからのワード挿入イベント
        window.addEventListener('insert-word', (event) => {
            this.editor.insertWord(event.detail.word);
        });
        
        // ステップ実行の状態変化
        window.addEventListener('step-mode-changed', (event) => {
            if (event.detail.active) {
                this.display.showOutput('Step mode: Press Ctrl+Enter to continue...');
            }
        });
        
        // 永続化完了通知
        window.addEventListener('persistence-complete', (event) => {
            console.log(event.detail.message);
        });
    }

    async executeCode() {
        const code = this.editor.getValue();
        if (!code) return;
        
        // ステップモードを終了
        this.stepper.reset();
        
        if (!window.ajisaiInterpreter) {
            this.display.showError('WASM not loaded');
            return;
        }
        
        try {
            const result = window.ajisaiInterpreter.execute(code);
            this.handleExecutionResult(result);
        } catch (error) {
            this.display.showError(error);
        }
    }

    handleExecutionResult(result) {
        if (result.status === 'OK') {
            this.display.showOutput(result.output || 'OK');
            this.updateInterpreterState();
            this.editor.clear();
            
            if (this.mobile.isMobile()) {
                this.setMode('execution');
            }
            
            // 自動保存
            this.persistence.saveCurrentState();
        } else {
            this.display.showOutput(result);
        }
    }

    updateInterpreterState() {
        const stack = window.ajisaiInterpreter.get_stack();
        const register = window.ajisaiInterpreter.get_register();
        const customWordsInfo = window.ajisaiInterpreter.get_custom_words_info();
        
        this.display.updateStack(stack);
        this.display.updateRegister(register);
        this.dictionary.updateCustomWords(customWordsInfo);
    }

    handleStepExecution() {
        if (!this.stepper.isActive()) {
            const code = this.editor.getValue();
            this.stepper.start(code, (result) => {
                this.display.showStepInfo(result);
                this.updateInterpreterState();
                
                if (!result.hasMore) {
                    this.editor.clear();
                    if (this.mobile.isMobile()) {
                        this.setMode('execution');
                    }
                }
            });
        } else {
            this.stepper.step();
        }
    }

    setMode(mode) {
        this.mode = mode;
        this.mobile.updateView(mode);
    }
}

// グローバルに公開
window.GUI = new GUI();
