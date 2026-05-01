
import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import type { AjisaiInterpreter, ExecuteResult } from '../wasm-interpreter-types';
import {
    createExecutionSnapshot,
    syncInterpreterState,
    resolveExecutionException
} from './interpreter-execution-utils';

export interface StepState {
    readonly active: boolean;
    readonly tokens: readonly string[];
    readonly currentIndex: number;
}

export interface StepExecutorCallbacks {
    readonly extractEditorValue: () => string;
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
    readonly extractState: () => StepState;
}

const createInitialState = (): StepState => ({
    active: false,
    tokens: [],
    currentIndex: 0
});

const tokenize = (code: string): string[] =>
    code.split(/\s+/).filter(token => token.length > 0);

const createActiveState = (tokens: string[]): StepState => ({
    active: true,
    tokens,
    currentIndex: 0
});

const advanceState = (state: StepState): StepState => ({
    ...state,
    currentIndex: state.currentIndex + 1
});

const formatStepMessage = (
    currentIndex: number,
    totalTokens: number,
    token: string
): string => {
    const remaining = totalTokens - currentIndex - 1;
    return `[>] Step ${currentIndex + 1}/${totalTokens}: "${token}" (${remaining} remaining)`;
};

export const createStepExecutor = (
    interpreter: AjisaiInterpreter,
    callbacks: StepExecutorCallbacks
): StepExecutor => {
    const {
        extractEditorValue,
        showInfo,
        showError,
        showExecutionResult,
        updateDisplays,
        saveState
    } = callbacks;

    let state = createInitialState();

    const isActive = (): boolean => state.active;

    const reset = (): void => {
        state = createInitialState();
    };

    const abort = (): void => {
        if (state.active) {
            reset();
            showInfo('Step mode aborted', true);
        }
    };

    const extractState = (): StepState => ({ ...state });

    const startStepMode = async (): Promise<void> => {
        const code = extractEditorValue();
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

            const currentState = createExecutionSnapshot(interpreter);
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
            resolveExecutionException('StepExecutor', error, showInfo, showError);
            reset();
        }

        updateDisplays();
        await saveState();
    };

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
        extractState
    };
};
