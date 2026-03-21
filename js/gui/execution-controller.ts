// js/gui/execution-controller.ts

import { WORKER_MANAGER } from '../workers/worker-manager';
import type { AjisaiInterpreter, ExecuteResult, CustomWord } from '../wasm-types';
import { createStepExecutor, StepExecutor } from './step-executor';
import type { ViewMode } from './mobile';

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

interface ExecutionSnapshot {
    readonly stack: ReturnType<AjisaiInterpreter['collect_stack']>;
    readonly customWords: CustomWord[];
    readonly importedModules: string[];
}

const collectCustomWords = (interpreter: AjisaiInterpreter): CustomWord[] => {
    const customWordsInfo = interpreter.collect_idiolect_words_info();
    return customWordsInfo.map(wordData => ({
        name: wordData[0],
        definition: interpreter.lookup_word_definition(wordData[0]),
        description: wordData[1]
    }));
};

const restoreInterpreterState = (
    interpreter: AjisaiInterpreter,
    result: ExecuteResult
): void => {
    if (!result || result.error) return;

    interpreter.reset();
    if (result.importedModules?.length) {
        interpreter.restore_imported_modules(result.importedModules);
    }
    if (result.stack) {
        interpreter.restore_stack(result.stack);
    }
    if (result.customWords) {
        interpreter.restore_idiolect(result.customWords);
    }
};

const checkIsResetCommand = (code: string): boolean =>
    code.trim().toUpperCase() === 'RESET';

const isAbortError = (error: Error): boolean =>
    error.message.includes('aborted');

const createExecutionSnapshot = (interpreter: AjisaiInterpreter): ExecutionSnapshot => ({
    stack: interpreter.collect_stack(),
    customWords: collectCustomWords(interpreter),
    importedModules: interpreter.collect_imported_modules()
});

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
        if (result.inputHelper) {
            clearEditor(false);
            insertEditorText(result.inputHelper);
            showInfo('Input helper inserted', false);
            updateView('input');
        } else if (result.definition_to_load) {
            updateEditorValue(result.definition_to_load);
            const wordName = code.replace("?", "").trim();
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
                restoreInterpreterState(interpreter, result);
            } catch (error) {
                console.error('[ExecController] Failed to sync state:', error);
                showError(error as Error);
            }

            applyExecutionResult(result, code);

        } catch (error) {
            resolveExecutionException(error, showInfo, showError);
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

export const executionControllerUtils = {
    collectCustomWords,
    restoreInterpreterState,
    checkIsResetCommand,
    isAbortError,
    createExecutionSnapshot,
    resolveExecutionException
};
