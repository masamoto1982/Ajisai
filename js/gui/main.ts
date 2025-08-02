// js/gui/main.ts

import { Display } from './display';
import { Dictionary } from './dictionary';
import { Editor } from './editor';
import { Stepper } from './stepper';
import { MobileHandler } from './mobile';
import { Persistence } from './persistence';
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
    outputDisplay: HTMLElement;
    stackDisplay: HTMLElement;
    registerDisplay: HTMLElement;
    builtinWordsDisplay: HTMLElement;
    customWordsDisplay: HTMLElement;
    inputArea: HTMLElement;
    outputArea: HTMLElement;
    memoryArea: HTMLElement;
    dictionaryArea: HTMLElement;
}

export class GUI {
    display: Display;
    dictionary: Dictionary;
    editor: Editor;
    stepper: Stepper;
    mobile: MobileHandler;
    persistence: Persistence;

    private elements: GUIElements = {} as GUIElements;
    private mode: 'input' | 'execution' = 'input';
    private stepMode = false;

    constructor() {
        this.display = new Display();
        this.dictionary = new Dictionary();
        this.editor = new Editor();
        this.stepper = new Stepper();
        this.mobile = new MobileHandler();
        this.persistence = new Persistence(this);
    }

    init(): void {
        console.log('GUI.init() called');
        this.cacheElements();

        // 各モジュールの初期化
        this.display.init({
            outputDisplay: this.elements.outputDisplay,
            stackDisplay: this.elements.stackDisplay,
            registerDisplay: this.elements.registerDisplay
        });
        
        this.dictionary.init({
            builtinWordsDisplay: this.elements.builtinWordsDisplay,
            customWordsDisplay: this.elements.customWordsDisplay
        }, (word) => this.insertWord(word));
        
        this.editor.init(this.elements.codeInput);
        this.stepper.init(() => window.ajisaiInterpreter);
        
        this.mobile.init({
            inputArea: this.elements.inputArea,
            outputArea: this.elements.outputArea,
            memoryArea: this.elements.memoryArea,
            dictionaryArea: this.elements.dictionaryArea
        });
        
        this.persistence.init();

        this.setupEventListeners();

        // 初期表示
        this.dictionary.renderBuiltinWords();
        this.updateAllDisplays();
        this.mobile.updateView(this.mode);
    }

    private cacheElements(): void {
        this.elements = {
            codeInput: document.getElementById('code-input') as HTMLTextAreaElement,
            runBtn: document.getElementById('run-btn') as HTMLButtonElement,
            clearBtn: document.getElementById('clear-btn') as HTMLButtonElement,
            outputDisplay: document.getElementById('output-display')!,
            stackDisplay: document.getElementById('stack-display')!,
            registerDisplay: document.getElementById('register-display')!,
            builtinWordsDisplay: document.getElementById('builtin-words-display')!,
            customWordsDisplay: document.getElementById('custom-words-display')!,
            inputArea: document.querySelector('.input-area')!,
            outputArea: document.querySelector('.output-area')!,
            memoryArea: document.querySelector('.memory-area')!,
            dictionaryArea: document.querySelector('.dictionary-area')!
        };
    }
    
    private setupEventListeners(): void {
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
    }

    private setMode(newMode: 'input' | 'execution'): void {
        this.mode = newMode;
        this.mobile.updateView(this.mode);
    }

    private insertWord(word: string): void {
        this.editor.insertWord(word);
    }
    
    private async runNormal(): Promise<void> {
        const code = this.editor.getValue();
        if (!code) return;

        this.stepMode = false;
        this.updateRunButton();

        try {
            const result = window.ajisaiInterpreter.execute(code) as ExecuteResult;
            if (result.status === 'OK') {
                this.display.showOutput(result.output || 'OK');
                
                if (result.autoNamed && result.autoNamedWord) {
                    this.editor.setValue(result.autoNamedWord);
                } else if (!result.autoNamed) {
                    this.editor.clear();
                }
                
                if (this.mobile.isMobile()) {
                    this.setMode('execution');
                }
            } else {
                this.display.showError(result);
            }
        } catch (error) {
            this.display.showError(error as Error);
        }
        
        this.updateAllDisplays();
        await this.persistence.saveCurrentState();
        this.display.showInfo('State saved.', true);
    }

    private async runStep(): Promise<void> {
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
                    this.display.showError(result.error || 'Unknown error');
                }
            } else {
                await this.continueStep();
            }
        } catch(error) {
            this.display.showError(error as Error);
            this.resetStepMode();
        }
    }

    private async continueStep(): Promise<void> {
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
    
    private resetStepMode(): void {
        this.stepMode = false;
        this.stepper.reset();
        this.updateRunButton();
    }

    private updateRunButton(): void {
        this.elements.runBtn.textContent = this.stepMode ? 'Step' : 'Run';
    }

    updateAllDisplays(): void {
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
