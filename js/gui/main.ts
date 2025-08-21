// js/gui/main.ts (Vector中心設計版)

import { Display } from './display';
import { Dictionary } from './dictionary';
import { Editor } from './editor';
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
    workspaceDisplay: HTMLElement;  // stackDisplay → workspaceDisplay
    builtinWordsDisplay: HTMLElement;
    customWordsDisplay: HTMLElement;
    inputArea: HTMLElement;
    outputArea: HTMLElement;
    workspaceArea: HTMLElement;     // stackArea → workspaceArea
    dictionaryArea: HTMLElement;
}

export class GUI {
    display: Display;
    dictionary: Dictionary;
    editor: Editor;
    mobile: MobileHandler;
    persistence: Persistence;

    private elements: GUIElements = {} as GUIElements;
    private mode: 'input' | 'execution' = 'input';

    constructor() {
        this.display = new Display();
        this.dictionary = new Dictionary();
        this.editor = new Editor();
        this.mobile = new MobileHandler();
        this.persistence = new Persistence(this);
    }

    init(): void {
        console.log('GUI.init() called');
        this.cacheElements();

        this.display.init({
            outputDisplay: this.elements.outputDisplay,
            workspaceDisplay: this.elements.workspaceDisplay,  // 変更
        });
        
        this.dictionary.init({
            builtinWordsDisplay: this.elements.builtinWordsDisplay,
            customWordsDisplay: this.elements.customWordsDisplay
        }, (word) => this.insertWord(word));
        
        this.editor.init(this.elements.codeInput);
        
        this.mobile.init({
            inputArea: this.elements.inputArea,
            outputArea: this.elements.outputArea,
            workspaceArea: this.elements.workspaceArea,  // 変更
            dictionaryArea: this.elements.dictionaryArea
        });
        
        this.persistence.init();

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
            workspaceDisplay: document.getElementById('workspace-display')!,  // 変更
            builtinWordsDisplay: document.getElementById('builtin-words-display')!,
            customWordsDisplay: document.getElementById('custom-words-display')!,
            inputArea: document.querySelector('.input-area')!,
            outputArea: document.querySelector('.output-area')!,
            workspaceArea: document.querySelector('.workspace-area')!,  // 変更
            dictionaryArea: document.querySelector('.dictionary-area')!
        };
    }
    
    private setupEventListeners(): void {
        this.elements.runBtn.addEventListener('click', () => this.runCode());
        this.elements.clearBtn.addEventListener('click', () => this.editor.clear());

        this.elements.codeInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && e.shiftKey) {
                e.preventDefault();
                this.runCode();
            }
        });

        this.elements.workspaceArea.addEventListener('click', () => {  // 変更
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

    updateAllDisplays(): void {
        if (!window.ajisaiInterpreter) return;
        try {
            this.display.updateWorkspace(window.ajisaiInterpreter.get_workspace());  // 変更
            this.dictionary.updateCustomWords(window.ajisaiInterpreter.get_custom_words_info());
        } catch (error) {
            console.error('Failed to update display:', error);
            this.display.showError('Failed to update display.');
        }
    }
}

export const GUI_INSTANCE = new GUI();
