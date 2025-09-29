import { Display } from './display';
import { Dictionary } from './dictionary';
import { Editor } from './editor';
import { MobileHandler } from './mobile';
import { Persistence } from './persistence';
import { TestRunner } from './test';
import { WORKER_MANAGER } from '../workers/worker-manager';
import { PARALLEL_EXECUTOR } from '../workers/parallel-executor';
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

    public elements: GUIElements = {} as GUIElements;
    private mode: 'input' | 'execution' = 'input';
    private workerInitialized = false;

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

        this.display.init({
            outputDisplay: this.elements.outputDisplay,
            stackDisplay: this.elements.stackDisplay,
        });
        
        this.dictionary.init({
            builtinWordsDisplay: this.elements.builtinWordsDisplay,
            customWordsDisplay: this.elements.customWordsDisplay
        }, (word: string) => this.insertWord(word), this);
        
        this.editor.init(this.elements.codeInput);
        
        this.mobile.init({
            inputArea: this.elements.inputArea,
            outputArea: this.elements.outputArea,
            stackArea: this.elements.stackArea,
            dictionaryArea: this.elements.dictionaryArea
        });
        
        this.persistence.init();

        this.setupEventListeners();
        this.dictionary.renderBuiltinWords();
        this.updateAllDisplays();
        this.mobile.updateView(this.mode);

        // Initialize Workers
        await this.initializeWorkers();
        
        console.log('[GUI] GUI initialization completed');
    }

    private async initializeWorkers(): Promise<void> {
        try {
            console.log('[GUI] Initializing worker system...');
            this.display.showInfo('Initializing parallel execution system...');
            
            await WORKER_MANAGER.init();
            this.workerInitialized = true;
            
            console.log('[GUI] Worker system initialized successfully');
            this.display.showInfo('Parallel execution system ready.', true);
            
        } catch (error) {
            console.error('[GUI] Failed to initialize workers:', error);
            this.display.showError(`Failed to initialize parallel execution: ${error}`);
            
            // Fall back to main thread execution
            this.workerInitialized = false;
            this.display.showInfo('Falling back to main thread execution.', true);
        }
    }

    private cacheElements(): void {
        this.elements = {
            codeInput: document.getElementById('code-input') as HTMLTextAreaElement,
            runBtn: document.getElementById('run-btn') as HTMLButtonElement,
            clearBtn: document.getElementById('clear-btn') as HTMLButtonElement,
            testBtn: document.getElementById('test-btn') as HTMLButtonElement,
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
            }
        });

        this.elements.stackArea.addEventListener('click', () => {
            if (this.mobile.isMobile() && this.mode === 'execution') {
                this.setMode('input');
            }
        });

        window.addEventListener('resize', () => this.mobile.updateView(this.mode));
        
        // Global escape key handler (higher priority)
        window.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                console.log('[GUI] Escape key detected - emergency stop');
                this.emergencyStop();
                e.preventDefault();
                e.stopImmediatePropagation();
            }
        }, true);
    }

    private emergencyStop(): void {
        console.log('[GUI] Emergency stop initiated');
        this.display.showInfo('🛑 EMERGENCY STOP - All executions aborted', true);
        
        if (this.workerInitialized) {
            PARALLEL_EXECUTOR.abortAll();
        }
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
        console.log('[GUI] Executing code via workers');
        this.display.showInfo('Executing...', true);
        
        let result: ExecuteResult;
        
        if (this.workerInitialized) {
            // プログレスコールバックを設定
            result = await WORKER_MANAGER.execute(code, (progressResult) => {
                console.log('[GUI] Progress callback:', progressResult);
                // 各ステップの結果を追加モードで表示
                if (progressResult.output) {
                    this.display.appendExecutionResult(progressResult);  // 🆕 追加モードで表示
                }
                // スタック表示を更新
                this.updateAllDisplays();
            });
        } else {
            // Fallback to main thread
            result = await window.ajisaiInterpreter.execute(code) as ExecuteResult;
        }

        if (result.definition_to_load) {
            this.editor.setValue(result.definition_to_load);
            const wordName = code.replace("?", "").trim();
            this.display.showInfo(`Loaded definition for ${wordName}.`);
        } else if (result.status === 'OK' && !result.error) {
            // プログレッシブ実行でない場合のみ通常表示
            if (!result.isProgressive) {
                this.display.showExecutionResult(result);
            }
            this.editor.clear();

            if (this.mobile.isMobile()) {
                this.setMode('execution');
            }
        } else if (result.status === 'COMPLETED') {
            // Progressive execution completed
            this.display.showInfo('Progressive execution completed.', true);
            this.editor.clear();

            if (this.mobile.isMobile()) {
                this.setMode('execution');
            }
        } else {
            this.display.showError(result.message || 'Unknown error');
        }
    } catch (error) {
        console.error('[GUI] Code execution failed:', error);
        
        if (error instanceof Error && error.message.includes('aborted')) {
            this.display.showInfo('Execution aborted by user.', true);
        } else {
            this.display.showError(error as Error);
        }
    }

    this.updateAllDisplays();
    await this.persistence.saveCurrentState();

    if (!code.trim().endsWith("?")) {
        this.display.showInfo('State saved.', true);
    }
}

    private async executeStepByStep(): Promise<void> {
        const code = this.editor.getValue();
        if (!code) return;

        try {
            console.log('[GUI] Executing step-by-step via workers');
            
            let result: ExecuteResult;
            
            if (this.workerInitialized) {
                result = await WORKER_MANAGER.executeStep(code);
            } else {
                result = window.ajisaiInterpreter.execute_step(code) as ExecuteResult;
            }
            
            if (result.status === 'OK' && !result.error) {
                this.display.showExecutionResult(result);
                
                if (!result.hasMore) {
                    this.display.showInfo('Step execution completed.', true);
                    this.editor.clear();
                } else {
                    const progressInfo = result.debugOutput || 'Step completed';
                    this.display.showInfo(`${progressInfo}. Press Ctrl+Enter for next step.`, true);
                }
                
                if (this.mobile.isMobile()) {
                    this.setMode('execution');
                }
            } else {
                this.display.showError(result.message || 'Unknown error');
            }
        } catch (error) {
            console.error('[GUI] Step execution failed:', error);
            
            if (error instanceof Error && error.message.includes('aborted')) {
                this.display.showInfo('Step execution aborted by user.', true);
            } else {
                this.display.showError(error as Error);
            }
        }
        
        this.updateAllDisplays();
        await this.persistence.saveCurrentState();
    }

    private async executeReset(): Promise<void> {
        try {
            console.log('[GUI] Executing reset');
            
            // Abort all worker tasks first
            if (this.workerInitialized) {
                PARALLEL_EXECUTOR.abortAll();
            }
            
            const result = window.ajisaiInterpreter.reset() as ExecuteResult;
            
            if (result.status === 'OK' && !result.error) {
                this.display.showOutput(result.output || 'RESET executed');
                this.editor.clear();
                
                if (this.mobile.isMobile()) {
                    this.setMode('execution');
                }
                
                this.updateAllDisplays();
                this.display.showInfo('🔄 RESET: All memory cleared and database reset.', true);
            } else {
                this.display.showError(result.message || 'RESET execution failed');
            }
        } catch (error) {
            console.error('[GUI] Reset execution failed:', error);
            this.display.showError(error as Error);
        }
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
            this.display.updateStack(window.ajisaiInterpreter.get_stack());
            this.dictionary.updateCustomWords(window.ajisaiInterpreter.get_custom_words_info());
            
            // Show worker status
            if (this.workerInitialized) {
                const status = WORKER_MANAGER.getStatus();
                console.log(`[GUI] Worker status: ${status.activeJobs} active, ${status.queuedJobs} queued, ${status.workers} workers`);
            }
        } catch (error) {
            console.error('Failed to update display:', error);
            this.display.showError('Failed to update display.');
        }
    }

    cleanup(): void {
        console.log('[GUI] Cleaning up...');
        
        if (this.workerInitialized) {
            WORKER_MANAGER.terminate();
        }
        
        console.log('[GUI] Cleanup completed');
    }
}

export const GUI_INSTANCE = new GUI();

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    GUI_INSTANCE.cleanup();
});
