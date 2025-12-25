// js/gui/step-executor.ts - ステップ実行管理

import { WORKER_MANAGER } from '../workers/worker-manager';
import type { AjisaiInterpreter, CustomWord } from '../wasm-types';

interface StepState {
    active: boolean;
    tokens: string[];
    currentIndex: number;
}

export class StepExecutor {
    private gui: any;
    private interpreter: AjisaiInterpreter;
    private state: StepState = {
        active: false,
        tokens: [],
        currentIndex: 0
    };

    constructor(gui: any, interpreter: AjisaiInterpreter) {
        this.gui = gui;
        this.interpreter = interpreter;
    }

    isActive(): boolean {
        return this.state.active;
    }

    reset(): void {
        this.state = { active: false, tokens: [], currentIndex: 0 };
    }

    async executeStep(): Promise<void> {
        if (!this.state.active) {
            await this.startStepMode();
        } else {
            await this.executeNextToken();
        }
    }

    abort(): void {
        if (this.state.active) {
            this.reset();
            this.gui.display.showInfo('Step mode aborted.', true);
        }
    }

    private async startStepMode(): Promise<void> {
        const code = this.gui.editor.getValue();
        if (!code) return;

        this.state.tokens = code.split(/\s+/).filter((token: string) => token.length > 0);
        this.state.currentIndex = 0;
        this.state.active = true;

        if (this.state.tokens.length === 0) {
            this.gui.display.showInfo('No code to execute.', true);
            this.reset();
            return;
        }

        this.gui.display.showInfo(
            `[STEP] Step mode started. ${this.state.tokens.length} tokens to execute. (Ctrl+Enter to continue)`,
            true
        );

        await this.executeNextToken();
    }

    private async executeNextToken(): Promise<void> {
        if (this.state.currentIndex >= this.state.tokens.length) {
            this.gui.display.showInfo('[DONE] Step mode completed.', true);
            this.reset();
            return;
        }

        const token = this.state.tokens[this.state.currentIndex]!;
        const remaining = this.state.tokens.length - this.state.currentIndex - 1;

        try {
            this.gui.display.showInfo(
                `[>] Step ${this.state.currentIndex + 1}/${this.state.tokens.length}: "${token}" (${remaining} remaining)`,
                false
            );

            const currentState = {
                stack: this.interpreter.get_stack(),
                customWords: this.getCustomWords(),
            };

            const result = await WORKER_MANAGER.execute(token, currentState);
            this.updateInterpreterState(result);

            if (result.status === 'OK' && !result.error) {
                this.gui.display.showExecutionResult(result);
            } else {
                this.gui.display.showError(result.message || 'Unknown error');
                this.reset();
            }

            this.state.currentIndex++;

            if (this.state.currentIndex >= this.state.tokens.length) {
                this.gui.display.showInfo('[DONE] Step mode completed.', true);
                this.reset();
            }
        } catch (error) {
            console.error('[StepExecutor] Step execution failed:', error);
            if (error instanceof Error && error.message.includes('aborted')) {
                this.gui.display.showInfo('Step execution aborted by user.', true);
            } else {
                this.gui.display.showError(error as Error);
            }
            this.reset();
        }

        this.gui.updateAllDisplays();
        await this.gui.persistence.saveCurrentState();
    }

    private getCustomWords(): CustomWord[] {
        const customWordsInfo = this.interpreter.get_custom_words_info();
        return customWordsInfo.map(wordData => {
            const name = wordData[0];
            const description = wordData[1];
            const definition = this.interpreter.get_word_definition(name);
            return { name, definition, description };
        });
    }

    private updateInterpreterState(result: any): void {
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
            console.error('[StepExecutor] Failed to sync state:', error);
            this.gui.display.showError(error as Error);
        }
    }
}
