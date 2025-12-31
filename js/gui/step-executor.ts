// js/gui/step-executor.ts - ステップ実行管理（関数型スタイル）

import { WORKER_MANAGER } from '../workers/worker-manager';
import type { AjisaiInterpreter, CustomWord, ExecuteResult } from '../wasm-types';

// ============================================================
// 型定義
// ============================================================

export interface StepState {
    readonly active: boolean;
    readonly tokens: readonly string[];
    readonly currentIndex: number;
}

export interface StepExecutorCallbacks {
    readonly getEditorValue: () => string;
    readonly showInfo: (text: string, append: boolean) => void;
    readonly showError: (error: Error | string) => void;
    readonly showExecutionResult: (result: ExecuteResult) => void;
    readonly updateDisplays: () => void;
    readonly saveState: () => Promise<void>;
}

export interface StepExecutor {
    readonly isActive: () => boolean;
    readonly reset: () => void;
    readonly executeStep: () => Promise<void>;
    readonly abort: () => void;
    readonly getState: () => StepState;
}

// ============================================================
// 純粋関数
// ============================================================

/**
 * 初期状態を生成
 */
const createInitialState = (): StepState => ({
    active: false,
    tokens: [],
    currentIndex: 0
});

/**
 * コードをトークンに分割
 */
const tokenize = (code: string): string[] =>
    code.split(/\s+/).filter(token => token.length > 0);

/**
 * ステップモード開始時の状態を生成
 */
const createActiveState = (tokens: string[]): StepState => ({
    active: true,
    tokens,
    currentIndex: 0
});

/**
 * 次のトークンへ進んだ状態を生成
 */
const advanceState = (state: StepState): StepState => ({
    ...state,
    currentIndex: state.currentIndex + 1
});

/**
 * ステップ情報のメッセージを生成
 */
const formatStepMessage = (
    currentIndex: number,
    totalTokens: number,
    token: string
): string => {
    const remaining = totalTokens - currentIndex - 1;
    return `[>] Step ${currentIndex + 1}/${totalTokens}: "${token}" (${remaining} remaining)`;
};

/**
 * カスタムワードを取得
 */
const getCustomWords = (interpreter: AjisaiInterpreter): CustomWord[] => {
    const customWordsInfo = interpreter.get_custom_words_info();
    return customWordsInfo.map(wordData => ({
        name: wordData[0],
        definition: interpreter.get_word_definition(wordData[0]),
        description: wordData[1]
    }));
};

/**
 * 実行状態をインタープリタに反映
 */
const syncInterpreterState = (
    interpreter: AjisaiInterpreter,
    result: ExecuteResult
): void => {
    if (!result || result.error) return;

    interpreter.reset();
    if (result.stack) {
        interpreter.restore_stack(result.stack);
    }
    if (result.customWords) {
        interpreter.restore_custom_words(result.customWords);
    }
};

// ============================================================
// ファクトリ関数: StepExecutor作成
// ============================================================

export const createStepExecutor = (
    interpreter: AjisaiInterpreter,
    callbacks: StepExecutorCallbacks
): StepExecutor => {
    const {
        getEditorValue,
        showInfo,
        showError,
        showExecutionResult,
        updateDisplays,
        saveState
    } = callbacks;

    // 状態（クロージャで保持）
    let state = createInitialState();

    // アクティブかどうか
    const isActive = (): boolean => state.active;

    // リセット
    const reset = (): void => {
        state = createInitialState();
    };

    // 中断
    const abort = (): void => {
        if (state.active) {
            reset();
            showInfo('Step mode aborted', true);
        }
    };

    // 状態取得
    const getState = (): StepState => ({ ...state });

    // ステップモード開始
    const startStepMode = async (): Promise<void> => {
        const code = getEditorValue();
        if (!code) return;

        const tokens = tokenize(code);

        if (tokens.length === 0) {
            showInfo('No code', true);
            return;
        }

        state = createActiveState(tokens);

        showInfo(`[STEP] Step mode: ${tokens.length} tokens (Ctrl+Enter to continue)`, true);

        await executeNextToken();
    };

    // 次のトークンを実行
    const executeNextToken = async (): Promise<void> => {
        if (state.currentIndex >= state.tokens.length) {
            showInfo('[DONE] Step mode completed', true);
            reset();
            return;
        }

        const token = state.tokens[state.currentIndex]!;

        try {
            showInfo(
                formatStepMessage(state.currentIndex, state.tokens.length, token),
                false
            );

            const currentState = {
                stack: interpreter.get_stack(),
                customWords: getCustomWords(interpreter),
            };

            const result = await WORKER_MANAGER.execute(token, currentState);

            try {
                syncInterpreterState(interpreter, result);
            } catch (error) {
                console.error('[StepExecutor] Failed to sync state:', error);
                showError(error as Error);
            }

            if (result.status === 'OK' && !result.error) {
                showExecutionResult(result);
            } else {
                showError(result.message || 'Unknown error');
                reset();
                updateDisplays();
                await saveState();
                return;
            }

            state = advanceState(state);

            if (state.currentIndex >= state.tokens.length) {
                showInfo('[DONE] Step mode completed', true);
                reset();
            }

        } catch (error) {
            console.error('[StepExecutor] Step execution failed:', error);
            if (error instanceof Error && error.message.includes('aborted')) {
                showInfo('Step execution aborted', true);
            } else {
                showError(error as Error);
            }
            reset();
        }

        updateDisplays();
        await saveState();
    };

    // ステップ実行（開始または続行）
    const executeStep = async (): Promise<void> => {
        if (!state.active) {
            await startStepMode();
        } else {
            await executeNextToken();
        }
    };

    return {
        isActive,
        reset,
        executeStep,
        abort,
        getState
    };
};

// 純粋関数をエクスポート（テスト用）
export const stepExecutorUtils = {
    createInitialState,
    tokenize,
    createActiveState,
    advanceState,
    formatStepMessage,
    getCustomWords
};
