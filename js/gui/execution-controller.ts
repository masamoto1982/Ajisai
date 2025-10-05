// js/gui/execution-controller.ts

import { WORKER_MANAGER } from '../workers/worker-manager';
import type { AjisaiInterpreter, ExecuteResult, CustomWord } from '../wasm-types';

export class ExecutionController {
    private gui: any;
    private interpreter: AjisaiInterpreter;

    constructor(gui: any, interpreter: AjisaiInterpreter) {
        this.gui = gui;
        this.interpreter = interpreter;
    }

    async runCode(code: string): Promise<void> {
        if (!code) return;

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

            if (result.definition_to_load) {
                this.gui.editor.setValue(result.definition_to_load);
                const wordName = code.replace("?", "").trim();
                this.gui.display.showInfo(`Loaded definition for ${wordName}.`);
                // 定義ロード時は入力モードに戻す
                this.gui.mobile.updateView('input');
            } else if (result.status === 'OK' && !result.error) {
                this.gui.display.showExecutionResult(result);
                this.gui.editor.clear();
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
}
