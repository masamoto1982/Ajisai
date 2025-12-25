// js/gui/execution-controller.ts - 実行制御

import { WORKER_MANAGER } from '../workers/worker-manager';
import type { AjisaiInterpreter, ExecuteResult, CustomWord } from '../wasm-types';
import { StepExecutor } from './step-executor';

export class ExecutionController {
    private gui: any;
    private interpreter: AjisaiInterpreter;
    private stepExecutor: StepExecutor;

    constructor(gui: any, interpreter: AjisaiInterpreter) {
        this.gui = gui;
        this.interpreter = interpreter;
        this.stepExecutor = new StepExecutor(gui, interpreter);
    }

    async runCode(code: string): Promise<void> {
        if (!code) return;

        this.stepExecutor.reset();

        if (code.trim().toUpperCase() === 'RESET') {
            await this.executeReset();
            return;
        }

        try {
            this.gui.mobile.updateView('execution');
            this.gui.display.showInfo('Executing...', false);

            const currentState = {
                stack: this.interpreter.get_stack(),
                customWords: this.getCustomWords(),
            };

            const result = await WORKER_MANAGER.execute(code, currentState);
            this.updateInterpreterState(result);
            this.handleResult(result, code);
        } catch (error) {
            console.error('[ExecController] Code execution failed:', error);
            if (error instanceof Error && error.message.includes('aborted')) {
                this.gui.display.showInfo('Execution aborted by user.', true);
            } else {
                this.gui.display.showError(error as Error);
            }
        }

        this.gui.updateAllDisplays();
        await this.gui.persistence.saveCurrentState();
    }

    async executeReset(): Promise<void> {
        try {
            console.log('[ExecController] Executing reset');
            this.stepExecutor.reset();
            await WORKER_MANAGER.resetAllWorkers();

            const result = this.interpreter.reset();

            if (result.status === 'OK' && !result.error) {
                this.gui.display.showOutput(result.output || 'RESET executed');
                this.gui.editor.clear();
                this.gui.display.showInfo('RESET: All memory cleared.', true);
                this.gui.mobile.updateView('input');
            } else {
                this.gui.display.showError(result.message || 'RESET execution failed');
            }
        } catch (error) {
            console.error('[ExecController] Reset failed:', error);
            this.gui.display.showError(error as Error);
        }
        this.gui.updateAllDisplays();
        await this.gui.persistence.saveCurrentState();
    }

    async executeStep(): Promise<void> {
        await this.stepExecutor.executeStep();
    }

    isStepModeActive(): boolean {
        return this.stepExecutor.isActive();
    }

    abortExecution(): void {
        this.stepExecutor.abort();
    }

    private getCustomWords(): CustomWord[] {
        const customWordsInfo = this.interpreter.get_custom_words_info();
        return customWordsInfo.map(wordData => ({
            name: wordData[0],
            definition: this.interpreter.get_word_definition(wordData[0]),
            description: wordData[1]
        }));
    }

    private updateInterpreterState(result: ExecuteResult): void {
        if (!result || result.error) return;

        try {
            this.interpreter.reset();
            if (result.stack) {
                this.interpreter.restore_stack(result.stack);
            }
            if (result.customWords) {
                this.interpreter.restore_custom_words(result.customWords);
            }
        } catch (error) {
            console.error('[ExecController] Failed to sync state:', error);
            this.gui.display.showError(error as Error);
        }
    }

    private handleResult(result: ExecuteResult, code: string): void {
        if (result.inputHelper) {
            this.gui.editor.clear(false);
            this.gui.editor.insertText(result.inputHelper);
            this.gui.display.showInfo('Input helper text inserted.');
            this.gui.mobile.updateView('input');
        } else if (result.definition_to_load) {
            this.gui.editor.setValue(result.definition_to_load);
            const wordName = code.replace("?", "").trim();
            this.gui.display.showInfo(`Loaded definition for ${wordName}.`);
            this.gui.mobile.updateView('input');
        } else if (result.status === 'OK' && !result.error) {
            this.gui.display.showExecutionResult(result);
            this.gui.editor.clear(false);
        } else {
            this.gui.display.showError(result.message || 'Unknown error');
        }
    }
}
