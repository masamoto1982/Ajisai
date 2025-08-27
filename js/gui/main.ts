// js/gui/main.ts (LPL対応)

import { Display } from './display';
import { Librarians } from './librarians';  // Dictionary → Librarians
import { Editor } from './editor';
import { MobileHandler } from './mobile';
import { Persistence } from './persistence';
import { TestRunner } from './test';  
import type { LPLInterpreter, ExecuteResult, StepResult } from '../wasm-types';

declare global {
    interface Window {
        lplInterpreter: LPLInterpreter;
    }
}

interface GUIElements {
    codeInput: HTMLTextAreaElement;
    runBtn: HTMLButtonElement;
    clearBtn: HTMLButtonElement;
    testBtn: HTMLButtonElement;
    outputDisplay: HTMLElement;
    bookshelfDisplay: HTMLElement;
    permanentLibrariansDisplay: HTMLElement;
    temporaryLibrariansDisplay: HTMLElement;
    inputArea: HTMLElement;
    outputArea: HTMLElement;
    bookshelfArea: HTMLElement;
    librariansArea: HTMLElement;
}

export class GUI {
    display: Display;
    librarians: Librarians;  // dictionary → librarians
    editor: Editor;
    mobile: MobileHandler;
    persistence: Persistence;
    testRunner: TestRunner;

    public elements: GUIElements = {} as GUIElements;
    private mode: 'input' | 'execution' = 'input';
    private stepMode: boolean = false;

    constructor() {
        this.display = new Display();
        this.librarians = new Librarians();  // Dictionary → Librarians
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
            bookshelfDisplay: this.elements.bookshelfDisplay,
        });
        
        this.librarians.init({
            permanentLibrariansDisplay: this.elements.permanentLibrariansDisplay,
            temporaryLibrariansDisplay: this.elements.temporaryLibrariansDisplay
        }, (word: string) => this.insertWord(word));
        
        this.editor.init(this.elements.codeInput);
        
        this.mobile.init({
            inputArea: this.elements.inputArea,
            outputArea: this.elements.outputArea,
            bookshelfArea: this.elements.bookshelfArea,
            librariansArea: this.elements.librariansArea
        });
        
        this.persistence.init();

        // イベントリスナーの設定
        this.setupEventListeners();

        // 初期表示
        this.librarians.renderPermanentLibrarians();
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
            bookshelfDisplay: document.getElementById('bookshelf-display')!,
            permanentLibrariansDisplay: document.getElementById('permanent-librarians-display')!,
            temporaryLibrariansDisplay: document.getElementById('temporary-librarians-display')!,
            inputArea: document.querySelector('.input-area')!,
            outputArea: document.querySelector('.output-area')!,
            bookshelfArea: document.querySelector('.bookshelf-area')!,
            librariansArea: document.querySelector('.librarians-area')!
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

        this.elements.bookshelfArea.addEventListener('click', () => {
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
            const result = window.lplInterpreter.execute(code) as ExecuteResult;
            
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
            const result = window.lplInterpreter.amnesia() as ExecuteResult;
            
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
            const message = window.lplInterpreter.init_step(code);
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
            const result = window.lplInterpreter.step() as StepResult;
            
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
        
        if (!window.lplInterpreter) {
            console.error('lplInterpreter not available');
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
        if (!window.lplInterpreter) return;
        try {
            this.display.updateBookshelf(window.lplInterpreter.get_bookshelf());
            this.librarians.updateTemporaryLibrarians(window.lplInterpreter.get_custom_words_info());
        } catch (error) {
            console.error('Failed to update display:', error);
            this.display.showError('Failed to update display.');
        }
    }
}

export const GUI_INSTANCE = new GUI();
