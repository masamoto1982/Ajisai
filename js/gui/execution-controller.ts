import type { ExecuteResult } from '../wasm-interpreter-types';
import type { AjisaiRuntime } from '../core/ajisai-runtime-types';
import { createStepExecutor, StepExecutor } from './step-executor';
import type { ViewMode } from './mobile-view-switcher';
import type { ExecutionService } from '../application/execution-service';

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

const isAbortError = (error: Error): boolean =>
    error.message.includes('aborted');

const resolveExecutionException = (
    error: unknown,
    showInfo: (text: string, append: boolean) => void,
    showError: (error: Error | string) => void
): void => {
    console.error('[ExecController] Code execution failed:', error);
    if (error instanceof Error && isAbortError(error)) {
        showInfo('Execution aborted', true);
        return;
    }
    showError(error as Error);
};

export const createExecutionController = (
    interpreter: AjisaiRuntime,
    executionService: ExecutionService,
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
        if (result.inputHelper) {
            clearEditor(false);
            insertEditorText(result.inputHelper);
            showInfo('Input helper inserted', false);
            updateView('input');
        } else if (result.definition_to_load) {
            updateEditorValue(result.definition_to_load);
            const wordName = code.replace('?', '').trim();
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

            const result = await executionService.executeCode(code);
            applyExecutionResult(result, code);

        } catch (error) {
            resolveExecutionException(error, showInfo, showError);
        }

        updateDisplays();
        await saveState();
    };

    const executeReset = async (): Promise<void> => {
        try {
            stepExecutor.reset();
            const result = await executionService.executeReset();

            if (result.status === 'OK' && !result.error) {
                clearEditor(true);
                await fullReset();
                updateView('input');
            } else {
                showError(result.message || 'RESET execution failed');
            }
        } catch (error) {
            showError(error as Error);
        }
    };

    const executeStep = async (): Promise<void> => {
        await stepExecutor.executeStep();
    };

    const checkIsStepModeActive = (): boolean => stepExecutor.isActive();

    const abortExecution = (): void => {
        stepExecutor.abort();
        executionService.abortExecution();
    };

    return {
        executeCode,
        executeReset,
        executeStep,
        checkIsStepModeActive,
        abortExecution
    };
};

export const executionControllerUtils = {
    checkIsResetCommand,
    isAbortError,
    resolveExecutionException
};
