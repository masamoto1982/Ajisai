// js/gui/execution-controller.ts

import { WORKER_MANAGER } from '../workers/worker-manager';
import type { AjisaiInterpreter, ExecuteResult, CustomWord } from '../wasm-types';

export class ExecutionController {
    private gui: any;
    private interpreter: AjisaiInterpreter;
    private stepMode: {
        active: boolean;
        tokens: string[];
        currentIndex: number;
    } = {
        active: false,
        tokens: [],
        currentIndex: 0
    };

    constructor(gui: any, interpreter: AjisaiInterpreter) {
        this.gui = gui;
        this.interpreter = interpreter;
    }

    async runCode(code: string): Promise<void> {
        if (!code) return;

        // 通常実行が開始されたらステップモードをリセット
        this.resetStepMode();

        if (code.trim().toUpperCase() === 'RESET') {
            await this.executeReset();
            return;
        }

        try {
            // モバイル表示を実行モードに切り替え
            this.gui.mobile.updateView('execution');
            
            this.gui.display.showInfo('Executing...', false);
            
            const currentState = {
                stack: this.interpreter.get_stack(),
                customWords: this.getCustomWords(),
            };
            
            const result = await WORKER_MANAGER.execute(code, currentState);

            this.updateInterpreterStateFromResult(result);

            if (result.inputHelper) {
                // 入力支援ワードの結果: エディタをクリアしてテキストを挿入
                this.gui.editor.clear(false);
                this.gui.editor.insertText(result.inputHelper);
                this.gui.display.showInfo('Input helper text inserted.');
                // 入力モードに戻す
                this.gui.mobile.updateView('input');
            } else if (result.definition_to_load) {
                this.gui.editor.setValue(result.definition_to_load);
                const wordName = code.replace("?", "").trim();
                this.gui.display.showInfo(`Loaded definition for ${wordName}.`);
                // 定義ロード時は入力モードに戻す
                this.gui.mobile.updateView('input');
            } else if (result.status === 'OK' && !result.error) {
                this.gui.display.showExecutionResult(result);
                this.gui.editor.clear(false); // ビューを切り替えずにエディタをクリア
                // エディタクリア後も実行モードを維持（モバイルで結果を確認できるように）
            } else {
                this.gui.display.showError(result.message || 'Unknown error');
            }
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

            // ステップモードをリセット
            this.resetStepMode();

            await WORKER_MANAGER.resetAllWorkers();

            const result = this.interpreter.reset();

            if (result.status === 'OK' && !result.error) {
                this.gui.display.showOutput(result.output || 'RESET executed');
                this.gui.editor.clear();
                this.gui.display.showInfo('🔄 RESET: All memory cleared.', true);
                // リセット後は入力モードに戻す
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
    
    private getCustomWords(): CustomWord[] {
        const customWordsInfo = this.interpreter.get_custom_words_info();
        return customWordsInfo.map(wordData => {
            const name = wordData[0];
            const description = wordData[1];
            const definition = this.interpreter.get_word_definition(name);
            return { name, definition, description };
        });
    }
    
    private updateInterpreterStateFromResult(result: ExecuteResult): void {
        if (!result || result.error) return;

        try {
            // Workerの実行結果がメインスレッドの状態を上書きする
            this.interpreter.reset(); // まずリセット
            if (result.stack) {
                this.interpreter.restore_stack(result.stack);
            }
            if (result.customWords) {
                this.interpreter.restore_custom_words(result.customWords);
            }
        } catch (error) {
            console.error('[ExecController] Failed to sync state from result:', error);
            this.gui.display.showError(error as Error);
        }
    }

    // ステップ実行モード用メソッド
    async executeStep(): Promise<void> {
        if (!this.stepMode.active) {
            // ステップ実行モード開始
            const code = this.gui.editor.getValue();
            if (!code) return;

            // トークンに分割（空白、改行、タブで分割）
            this.stepMode.tokens = code.split(/\s+/).filter((token: string) => token.length > 0);
            this.stepMode.currentIndex = 0;
            this.stepMode.active = true;

            if (this.stepMode.tokens.length === 0) {
                this.gui.display.showInfo('No code to execute.', true);
                this.resetStepMode();
                return;
            }

            this.gui.display.showInfo(
                `🔍 Step mode started. ${this.stepMode.tokens.length} tokens to execute. (Ctrl+Enter to continue)`,
                true
            );

            // 最初のトークンを実行
            await this.executeNextToken();
        } else {
            // 次のトークンを実行
            await this.executeNextToken();
        }
    }

    private async executeNextToken(): Promise<void> {
        if (this.stepMode.currentIndex >= this.stepMode.tokens.length) {
            this.gui.display.showInfo('✅ Step mode completed.', true);
            this.resetStepMode();
            return;
        }

        const token = this.stepMode.tokens[this.stepMode.currentIndex]!;
        const remaining = this.stepMode.tokens.length - this.stepMode.currentIndex - 1;

        try {
            this.gui.display.showInfo(
                `▶️ Step ${this.stepMode.currentIndex + 1}/${this.stepMode.tokens.length}: "${token}" (${remaining} remaining)`,
                false
            );

            const currentState = {
                stack: this.interpreter.get_stack(),
                customWords: this.getCustomWords(),
            };

            const result = await WORKER_MANAGER.execute(token, currentState);
            this.updateInterpreterStateFromResult(result);

            if (result.status === 'OK' && !result.error) {
                this.gui.display.showExecutionResult(result);
            } else {
                this.gui.display.showError(result.message || 'Unknown error');
                this.resetStepMode();
            }

            this.stepMode.currentIndex++;

            if (this.stepMode.currentIndex >= this.stepMode.tokens.length) {
                this.gui.display.showInfo('✅ Step mode completed.', true);
                this.resetStepMode();
            }

        } catch (error) {
            console.error('[ExecController] Step execution failed:', error);
            if (error instanceof Error && error.message.includes('aborted')) {
                this.gui.display.showInfo('Step execution aborted by user.', true);
            } else {
                this.gui.display.showError(error as Error);
            }
            this.resetStepMode();
        }

        this.gui.updateAllDisplays();
        await this.gui.persistence.saveCurrentState();
    }

    private resetStepMode(): void {
        this.stepMode.active = false;
        this.stepMode.tokens = [];
        this.stepMode.currentIndex = 0;
    }

    isStepModeActive(): boolean {
        return this.stepMode.active;
    }

    abortExecution(): void {
        // ステップモードをリセット
        if (this.stepMode.active) {
            this.resetStepMode();
            this.gui.display.showInfo('Step mode aborted.', true);
        }
    }
}
