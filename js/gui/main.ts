// js/gui/main.ts

import { Display } from './display';
import { Dictionary } from './dictionary';
import { Editor } from './editor';
import { MobileHandler } from './mobile';
import { Persistence } from './persistence';
import { TestRunner } from './test';
import { WORKER_MANAGER } from '../workers/worker-manager';
import { ExecutionController } from './execution-controller';
import type { AjisaiInterpreter } from '../wasm-types';

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

interface GUIElements {
    codeInput: HTMLTextAreaElement;
    runBtn: HTMLButtonElement;
    clearBtn: HTMLButtonElement;
    testBtn: HTMLButtonElement;
    exportBtn: HTMLButtonElement;
    importBtn: HTMLButtonElement;
    outputDisplay: HTMLElement;
    stackDisplay: HTMLElement;
    builtinWordsDisplay: HTMLElement;
    customWordsDisplay: HTMLElement;
    inputArea: HTMLElement;
    outputArea: HTMLElement;
    stackArea: HTMLElement;
    dictionaryArea: HTMLElement;
}

export class GUI {
    display: Display;
    dictionary: Dictionary;
    editor: Editor;
    mobile: MobileHandler;
    persistence: Persistence;
    testRunner: TestRunner;
    executionController!: ExecutionController;

    public elements: GUIElements = {} as GUIElements;

    constructor() {
        this.display = new Display();
        this.dictionary = new Dictionary();
        this.editor = new Editor();
        this.mobile = new MobileHandler();
        this.persistence = new Persistence(this);
        this.testRunner = new TestRunner(this);
    }

    async init(): Promise<void> {
        console.log('[GUI] Initializing GUI...');
        
        this.cacheElements();
        this.executionController = new ExecutionController(this, window.ajisaiInterpreter);

        this.display.init({
            outputDisplay: this.elements.outputDisplay,
            stackDisplay: this.elements.stackDisplay,
        });
        
        this.dictionary.init({
            builtinWordsDisplay: this.elements.builtinWordsDisplay,
            customWordsDisplay: this.elements.customWordsDisplay
        }, (word: string) => this.editor.insertWord(word), this);
        
        // エディタにGUIインスタンスを渡す
        this.editor.init(this.elements.codeInput, this);
        
        this.mobile.init({
            inputArea: this.elements.inputArea,
            outputArea: this.elements.outputArea,
            stackArea: this.elements.stackArea,
            dictionaryArea: this.elements.dictionaryArea
        });
        await this.persistence.init();

        this.setupEventListeners();
        this.dictionary.renderBuiltinWords();
        this.updateAllDisplays();
        
        // 初期表示は入力モード
        this.mobile.updateView('input');

        await this.initializeWorkers();
        
        console.log('[GUI] GUI initialization completed');
    }

    private async initializeWorkers(): Promise<void> {
        try {
            this.display.showInfo('Initializing parallel execution system...');
            await WORKER_MANAGER.init();
            this.display.showInfo('Parallel execution system ready.', true);
        } catch (error) {
            console.error('[GUI] Failed to initialize workers:', error);
            this.display.showError(new Error(`Failed to initialize parallel execution: ${error}`));
        }
    }

    private cacheElements(): void {
        this.elements = {
            codeInput: document.getElementById('code-input') as HTMLTextAreaElement,
            runBtn: document.getElementById('run-btn') as HTMLButtonElement,
            clearBtn: document.getElementById('clear-btn') as HTMLButtonElement,
            testBtn: document.getElementById('test-btn') as HTMLButtonElement,
            exportBtn: document.getElementById('export-btn') as HTMLButtonElement,
            importBtn: document.getElementById('import-btn') as HTMLButtonElement,
            outputDisplay: document.getElementById('output-display')!,
            stackDisplay: document.getElementById('stack-display')!,
            builtinWordsDisplay: document.getElementById('builtin-words-display')!,
            customWordsDisplay: document.getElementById('custom-words-display')!,
            inputArea: document.querySelector('.input-area')!,
            outputArea: document.querySelector('.output-area')!,
            stackArea: document.querySelector('.stack-area')!,
            dictionaryArea: document.querySelector('.dictionary-area')!
        };
    }
    
    private setupEventListeners(): void {
        this.elements.runBtn.addEventListener('click', () => this.executionController.runCode(this.editor.getValue()));
        this.elements.clearBtn.addEventListener('click', () => this.editor.clear());
        
        this.elements.testBtn?.addEventListener('click', () => {
            // テスト実行時は実行モードに切り替え
            this.mobile.updateView('execution');
            this.testRunner.runAllTests();
        });
        
        this.elements.exportBtn?.addEventListener('click', () => this.persistence.exportCustomWords());
        this.elements.importBtn?.addEventListener('click', () => this.persistence.importCustomWords());

        this.elements.codeInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && e.shiftKey) {
                e.preventDefault();
                this.executionController.runCode(this.editor.getValue());
            }
        });
        
        window.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                WORKER_MANAGER.abortAll();
                e.preventDefault();
                e.stopImmediatePropagation();
            }
            // Ctrl+Alt+Enterでリセットを実行
            if (e.key === 'Enter' && e.ctrlKey && e.altKey) {
                if (confirm('Are you sure you want to reset the system?')) {
                    this.executionController.executeReset();
                }
                e.preventDefault();
                e.stopImmediatePropagation();
            }
        }, true);
        
        // モバイルでOutputエリアやStackエリアをタップしたら入力モードに戻す
        if (this.mobile.isMobile()) {
            this.elements.outputArea.addEventListener('click', () => {
                this.mobile.updateView('input');
                this.editor.focus();
            });
            
            this.elements.stackArea.addEventListener('click', () => {
                this.mobile.updateView('input');
                this.editor.focus();
            });
        }
    }

    updateAllDisplays(): void {
        if (!window.ajisaiInterpreter) return;
        try {
            this.display.updateStack(window.ajisaiInterpreter.get_stack());
            this.dictionary.updateCustomWords(window.ajisaiInterpreter.get_custom_words_info());
        } catch (error) {
            console.error('Failed to update display:', error);
            this.display.showError(new Error('Failed to update display.'));
        }
    }
}

export const GUI_INSTANCE = new GUI();
