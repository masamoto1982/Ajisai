// js/gui/main.ts (AMNESIA機能対応)

import { Display } from './display';
import { Dictionary } from './dictionary';
import { Editor } from './editor';
import { MobileHandler } from './mobile';
import { Persistence } from './persistence';
import { TestRunner } from './test';  
import type { AjisaiInterpreter, ExecuteResult, StepResult } from '../wasm-types';

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
    outputDisplay: HTMLElement;
    workspaceDisplay: HTMLElement;
    builtinWordsDisplay: HTMLElement;
    customWordsDisplay: HTMLElement;
    inputArea: HTMLElement;
    outputArea: HTMLElement;
    workspaceArea: HTMLElement;
    dictionaryArea: HTMLElement;
}

export class GUI {
    display: Display;
    dictionary: Dictionary;
    editor: Editor;
    mobile: MobileHandler;
    persistence: Persistence;
    testRunner: TestRunner;

    public elements: GUIElements = {} as GUIElements;
    private mode: 'input' | 'execution' = 'input';
    private stepMode: boolean = false;

    constructor() {
        this.display = new Display();
        this.dictionary = new Dictionary();
        this.editor = new Editor();
        this.mobile = new MobileHandler();
        this.persistence = new Persistence(this);
        this.testRunner = new TestRunner(this);
        console.log('GUI constructor: TestRunner initialized');
    }

    init(): void {
        console.log('GUI.init() called');
        this.cacheElements();

        // 各モジュールの初期化
        this.display.init({
            outputDisplay: this.elements.outputDisplay,
            workspaceDisplay: this.elements.workspaceDisplay,
        });
        
        this.dictionary.init({
            builtinWordsDisplay: this.elements.builtinWordsDisplay,
            customWordsDisplay: this.elements.customWordsDisplay
        }, (word: string) => this.insertWord(word));
        
        this.editor.init(this.elements.codeInput);
        
        this.mobile.init({
            inputArea: this.elements.inputArea,
            outputArea: this.elements.outputArea,
            workspaceArea: this.elements.workspaceArea,
            dictionaryArea: this.elements.dictionaryArea
        });
        
        this.persistence.init();

        // イベントリスナーの設定
        this.setupEventListeners();

        // 初期表示
        this.dictionary.renderBuiltinWords();
        this.updateAllDisplays();
        this.mobile.updateView(this.mode);
    }

    private cacheElements(): void {
        console.log('Caching elements...');
        this.elements = {
            codeInput: document.getElementById('code-input') as HTMLTextAreaElement,
            runBtn: document.getElementById('run-btn') as HTMLButtonElement,
            clearBtn: document.getElementById('clear-btn') as HTMLButtonElement,
            testBtn: document.getElementById('test-btn') as HTMLButtonElement,
            outputDisplay: document.getElementById('output-display')!,
            workspaceDisplay: document.getElementById('workspace-display')!,
            builtinWordsDisplay: document.getElementById('builtin-words-display')!,
            customWordsDisplay: document.getElementById('custom-words-display')!,
            inputArea: document.querySelector('.input-area')!,
            outputArea: document.querySelector('.output-area')!,
            workspaceArea: document.querySelector('.workspace-area')!,
            dictionaryArea: document.querySelector('.dictionary-area')!
        };

        if (!this.elements.testBtn) {
            console.error('Test button not found in DOM!');
        } else {
            console.log('Test button found:', this.elements.testBtn);
        }
    }
    
    private setupEventListeners(): void {
        console.log('Setting up event listeners...');
        
        this.elements.runBtn.addEventListener('click', () => this.runCode());
        this.elements.clearBtn.addEventListener('click', () => this.editor.clear());
        
        if (this.elements.testBtn) {
            console.log('Adding test button event listener');
            this.elements.testBtn.addEventListener('click', () => {
                console.log('Test button clicked!');
                this.runTests();
            });
        } else {
            console.error('Cannot add event listener: test button not found');
        }

        this.elements.codeInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                if (e.shiftKey) {
                    // Shift+Enter: 通常実行
                    e.preventDefault();
                    this.runCode();
                } else if (e.ctrlKey && e.altKey) {
                    // Ctrl+Alt+Enter: AMNESIA実行
                    e.preventDefault();
                    this.executeAmnesia();
                } else if (e.ctrlKey) {
                    // Ctrl+Enter: ステップ実行
                    e.preventDefault();
                    this.startStepExecution();
                }
            } else if (e.key === ' ' && this.stepMode) {
                // スペース: ステップ実行中の次のステップ
                e.preventDefault();
                this.executeNextStep();
            } else if (e.key === 'Escape' && this.stepMode) {
                // Escape: ステップ実行終了
                e.preventDefault();
                this.endStepExecution();
            }
        });

        this.elements.workspaceArea.addEventListener('click', () => {
            if (this.mobile.isMobile() && this.mode === 'execution') {
                this.setMode('input');
            }
        });

        window.addEventListener('resize', () => this.mobile.updateView(this.mode));
    }

    private setMode(newMode: 'input' | 'execution'): void {
        this.mode = newMode;
        this.mobile.updateView(this.mode);
    }

    private insertWord(word: string): void {
        this.editor.insertWord(word);
    }
    
    private async runCode(): Promise<void> {
        const code = this.editor.getValue();
        if (!code) return;

        try {
            const result = window.ajisaiInterpreter.execute(code) as ExecuteResult;
            
            if (result.status === 'OK' && !result.error) {
                this.display.showOutput(result.output || 'OK');
                this.editor.clear();
                
                if (this.mobile.isMobile()) {
                    this.setMode('execution');
                }
            } else {
                this.display.showError(result.message || 'Unknown error');
            }
        } catch (error) {
            this.display.showError(error as Error);
        }
        
        this.updateAllDisplays();
        await this.persistence.saveCurrentState();
        this.display.showInfo('State saved.', true);
    }

    private async executeAmnesia(): Promise<void> {
        try {
            console.log('Executing AMNESIA...');
            const result = window.ajisaiInterpreter.amnesia() as ExecuteResult;
            
            if (result.status === 'OK' && !result.error) {
                this.display.showOutput(result.output || 'AMNESIA executed');
                this.editor.clear();
                
                if (this.mobile.isMobile()) {
                    this.setMode('execution');
                }
                
                // 表示を更新
                this.updateAllDisplays();
                this.display.showInfo('🧠 AMNESIA: All memory cleared and database reset.', true);
            } else {
                this.display.showError(result.message || 'AMNESIA execution failed');
            }
        } catch (error) {
            this.display.showError(error as Error);
        }
    }

    private startStepExecution(): void {
        const code = this.editor.getValue();
        if (!code) return;

        try {
            const message = window.ajisaiInterpreter.init_step(code);
            this.stepMode = true;
            this.display.showInfo(`Step mode started: ${message}\nPress Space to step, Escape to end.`);
            
            if (this.mobile.isMobile()) {
                this.setMode('execution');
            }
        } catch (error) {
            this.display.showError(error as Error);
        }
    }

    private executeNextStep(): void {
        if (!this.stepMode) return;

        try {
            const result = window.ajisaiInterpreter.step() as StepResult;
            
            if (result.error) {
                this.display.showError(result.output || 'Step execution error');
                this.endStepExecution();
                return;
            }

            this.display.showOutput(result.output || 'Step executed');
            
            if (result.hasMore) {
                const progress = result.position && result.total 
                    ? ` (${result.position}/${result.total})`
                    : '';
                this.display.showInfo(`Step completed${progress}. Press Space for next step, Escape to end.`, true);
            } else {
                this.display.showInfo('Step execution completed.', true);
                this.endStepExecution();
            }
            
            this.updateAllDisplays();
        } catch (error) {
            this.display.showError(error as Error);
            this.endStepExecution();
        }
    }

    private endStepExecution(): void {
        this.stepMode = false;
        this.display.showInfo('Step mode ended.', true);
    }

    private async runTests(): Promise<void> {
        console.log('runTests called');
        
        if (!window.ajisaiInterpreter) {
            console.error('ajisaiInterpreter not available');
            this.display.showError('Interpreter not available');
            return;
        }

        try {
            console.log('Starting test runner...');
            await this.testRunner.runAllTests();
            this.updateAllDisplays();
            
            if (this.mobile.isMobile()) {
                this.setMode('execution');
            }
            
            console.log('Tests completed');
        } catch (error) {
            console.error('Error running tests:', error);
            this.display.showError(error as Error);
        }
    }

    updateAllDisplays(): void {
        if (!window.ajisaiInterpreter) return;
        try {
            this.display.updateWorkspace(window.ajisaiInterpreter.get_workspace());
            this.dictionary.updateCustomWords(window.ajisaiInterpreter.get_custom_words_info());
        } catch (error) {
            console.error('Failed to update display:', error);
            this.display.showError('Failed to update display.');
        }
    }
}

export const GUI_INSTANCE = new GUI();
