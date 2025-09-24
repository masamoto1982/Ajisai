import { Display } from './display';
import { Dictionary } from './dictionary';
import { Editor } from './editor';
import { MobileHandler } from './mobile';
import { Persistence } from './persistence';
import { TestRunner } from './test';  
import type { AjisaiInterpreter, ExecuteResult } from '../wasm-types';

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
    }

    init(): void {
        this.cacheElements();

        this.display.init({
            outputDisplay: this.elements.outputDisplay,
            workspaceDisplay: this.elements.workspaceDisplay,
        });
        
        this.dictionary.init({
            builtinWordsDisplay: this.elements.builtinWordsDisplay,
            customWordsDisplay: this.elements.customWordsDisplay
        }, (word: string) => this.insertWord(word), this);
        
        this.editor.init(this.elements.codeInput);
        
        this.mobile.init({
            inputArea: this.elements.inputArea,
            outputArea: this.elements.outputArea,
            workspaceArea: this.elements.workspaceArea,
            dictionaryArea: this.elements.dictionaryArea
        });
        
        this.persistence.init();

        this.setupEventListeners();
        this.dictionary.renderBuiltinWords();
        this.updateAllDisplays();
        this.mobile.updateView(this.mode);
    }

    private cacheElements(): void {
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
    }
    
    private setupEventListeners(): void {
        this.elements.runBtn.addEventListener('click', () => this.runCode());
        this.elements.clearBtn.addEventListener('click', () => this.editor.clear());
        
        if (this.elements.testBtn) {
            this.elements.testBtn.addEventListener('click', () => {
                this.runTests();
            });
        }

        this.elements.codeInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                if (e.shiftKey) {
                    e.preventDefault();
                    this.runCode();
                } else if (e.ctrlKey && e.altKey) {
                    e.preventDefault();
                    this.executeReset();
                } else if (e.ctrlKey) {
                    e.preventDefault();
                    this.executeStepByStep();
                }
            } else if (e.key === 'Escape' && this.stepMode) {
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
                this.display.showExecutionResult(result);
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

    private async executeStepByStep(): Promise<void> {
        const code = this.editor.getValue();
        if (!code) return;

        try {
            const result = window.ajisaiInterpreter.execute_step(code) as ExecuteResult;
            
            if (result.status === 'OK' && !result.error) {
                this.display.showExecutionResult(result);
                
                if (!result.hasMore) {
                    this.stepMode = false;
                    this.display.showInfo('Step execution completed.', true);
                    this.editor.clear();
                } else {
                    this.stepMode = true;
                    const progressInfo = result.debugOutput || 'Step completed';
                    this.display.showInfo(`${progressInfo}. Press Ctrl+Enter for next step.`, true);
                }
                
                if (this.mobile.isMobile()) {
                    this.setMode('execution');
                }
            } else {
                this.display.showError(result.message || 'Unknown error');
                this.stepMode = false;
            }
        } catch (error) {
            this.display.showError(error as Error);
            this.stepMode = false;
        }
        
        this.updateAllDisplays();
        await this.persistence.saveCurrentState();
    }

    private async executeReset(): Promise<void> {
        try {
            const result = window.ajisaiInterpreter.reset() as ExecuteResult;
            
            if (result.status === 'OK' && !result.error) {
                this.display.showOutput(result.output || 'RESET executed');
                this.editor.clear();
                this.stepMode = false;
                
                if (this.mobile.isMobile()) {
                    this.setMode('execution');
                }
                
                this.updateAllDisplays();
                this.display.showInfo('🔄 RESET: All memory cleared and database reset.', true);
            } else {
                this.display.showError(result.message || 'RESET execution failed');
            }
        } catch (error) {
            this.display.showError(error as Error);
        }
    }

    private endStepExecution(): void {
        this.stepMode = false;
        this.display.showInfo('Step mode ended.', true);
    }

    private async runTests(): Promise<void> {
        if (!window.ajisaiInterpreter) {
            this.display.showError('Interpreter not available');
            return;
        }

        try {
            await this.testRunner.runAllTests();
            this.updateAllDisplays();
            
            if (this.mobile.isMobile()) {
                this.setMode('execution');
            }
        } catch (error) {
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
