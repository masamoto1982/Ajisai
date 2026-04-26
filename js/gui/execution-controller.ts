import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import type { AjisaiInterpreter, ExecuteResult } from '../wasm-interpreter-types';
import {
    createExecutionSnapshot,
    syncInterpreterState,
    resolveExecutionException
} from './interpreter-execution-utils';
import { createStepExecutor, StepExecutor } from './step-executor';
import type { ViewMode } from './mobile-view-switcher';

export interface ExecutionCallbacks {
    readonly extractEditorValue: () => string;
    readonly clearEditor: (switchView?: boolean) => void;
    readonly updateEditorValue: (value: string) => void;
    readonly insertEditorText: (text: string) => void;
    readonly showInfo: (text: string, append: boolean) => void;
    readonly showError: (error: Error | string) => void;
    readonly showExecutionResult: (result: ExecuteResult) => void;
    readonly updateDisplays: () => void;
    readonly saveState: () => Promise<void>;
    readonly fullReset: () => Promise<void>;
    readonly updateView: (mode: ViewMode) => void;
}

export interface ExecutionController {
    readonly executeCode: (code: string) => Promise<void>;
    readonly executeReset: () => Promise<void>;
    readonly executeStep: () => Promise<void>;
    readonly checkIsStepModeActive: () => boolean;
    readonly abortExecution: () => void;
}

const checkIsResetCommand = (code: string): boolean =>
    code.trim().toUpperCase() === 'RESET';

export const createExecutionController = (
    interpreter: AjisaiInterpreter,
    callbacks: ExecutionCallbacks
): ExecutionController => {
    const {
        extractEditorValue,
        clearEditor,
        updateEditorValue,
        insertEditorText,
        showInfo,
        showError,
        showExecutionResult,
        updateDisplays,
        saveState,
        fullReset,
        updateView
    } = callbacks;

    const stepExecutor: StepExecutor = createStepExecutor(interpreter, {
        extractEditorValue,
        showInfo,
        showError,
        showExecutionResult,
        updateDisplays,
        saveState
    });

    const applyExecutionResult = (result: ExecuteResult, code: string): void => {
        if (result.hedgedTrace && result.hedgedTrace.length > 0) {
            showInfo(`[HEDGED] ${result.hedgedTrace.join(' | ')}`, true);
        }
        if (result.hedgedWinner) {
            showInfo(`[HEDGED-WINNER] ${result.hedgedWinner}`, true);
        }
        if (result.hedgedFallbackReason) {
            showInfo(`[HEDGED-FALLBACK] ${result.hedgedFallbackReason}`, true);
        }
        if (result.hedgedCancelled && result.hedgedCancelled.length > 0) {
            showInfo(`[HEDGED-CANCEL] ${result.hedgedCancelled.join(', ')}`, true);
        }
        if (result.inputHelper) {
            clearEditor(false);
            insertEditorText(result.inputHelper);
            showInfo('Input helper inserted', false);
            updateView('input');
        } else if (result.definition_to_load) {
            updateEditorValue(result.definition_to_load);
            const wordName = code.replace(/\?|LOOKUP/gi, "").trim();
            showInfo(`Showing definition: ${wordName}`, false);
            updateView('input');
        } else if (result.status === 'OK' && !result.error) {
            showExecutionResult(result);
            clearEditor(false);
        } else {
            showError(result.message || 'Unknown error');
        }
    };

    const executeCode = async (code: string): Promise<void> => {
        if (!code) return;

        stepExecutor.reset();

        if (checkIsResetCommand(code)) {
            await executeReset();
            return;
        }

        try {
            updateView('output');
            showInfo('Executing...', false);

            const currentState = createExecutionSnapshot(interpreter);
            const result = await WORKER_MANAGER.execute(code, currentState);

            try {
                syncInterpreterState(interpreter, result);
            } catch (error) {
                console.error('[ExecController] Failed to sync state:', error);
                showError(error as Error);
            }

            applyExecutionResult(result, code);

        } catch (error) {
            resolveExecutionException('ExecController', error, showInfo, showError);
        }

        updateDisplays();
        await saveState();
    };

    const executeReset = async (): Promise<void> => {
        try {
            console.log('[ExecController] Executing full reset');
            stepExecutor.reset();
            await WORKER_MANAGER.resetAllWorkers();
            const result = interpreter.reset();

            if (result.status === 'OK' && !result.error) {
                clearEditor(true);
                await fullReset();
                updateView('input');
            } else {
                showError(result.message || 'RESET execution failed');
            }
        } catch (error) {
            console.error('[ExecController] Reset failed:', error);
            showError(error as Error);
        }
    };

    const executeStep = async (): Promise<void> => {
        await stepExecutor.executeStep();
    };

    const checkIsStepModeActive = (): boolean => stepExecutor.isActive();

    const abortExecution = (): void => {
        stepExecutor.abort();
    };

    return {
        executeCode,
        executeReset,
        executeStep,
        checkIsStepModeActive,
        abortExecution
    };
};
