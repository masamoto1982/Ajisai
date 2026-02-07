// js/gui/execution-controller.ts

import { WORKER_MANAGER } from '../workers/worker-manager';
import type { AjisaiInterpreter, ExecuteResult, CustomWord } from '../wasm-types';
import { createStepExecutor, StepExecutor } from './step-executor';

export interface ExecutionCallbacks {
    readonly getEditorValue: () => string;
    readonly clearEditor: (switchView?: boolean) => void;
    readonly setEditorValue: (value: string) => void;
    readonly insertEditorText: (text: string) => void;
    readonly showInfo: (text: string, append: boolean) => void;
    readonly showError: (error: Error | string) => void;
    readonly showExecutionResult: (result: ExecuteResult) => void;
    readonly updateDisplays: () => void;
    readonly saveState: () => Promise<void>;
    readonly fullReset: () => Promise<void>;
    readonly updateView: (mode: 'input' | 'execution') => void;
}

export interface ExecutionController {
    readonly runCode: (code: string) => Promise<void>;
    readonly executeReset: () => Promise<void>;
    readonly executeStep: () => Promise<void>;
    readonly isStepModeActive: () => boolean;
    readonly abortExecution: () => void;
}

const getCustomWords = (interpreter: AjisaiInterpreter): CustomWord[] => {
    const customWordsInfo = interpreter.get_custom_words_info();
    return customWordsInfo.map(wordData => ({
        name: wordData[0],
        definition: interpreter.get_word_definition(wordData[0]),
        description: wordData[1]
    }));
};

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

        // Worker 側で削除されたエクステンションワードをメインスレッドからも削除する。
        // reset() は全エクステンションを再登録するが、Worker の結果に含まれない
        // ワードは Worker 側で削除されたことを意味する。
        const workerWordNames = new Set(result.customWords.map(w => w.name.toUpperCase()));
        const mainWords = interpreter.get_custom_words_info();
        for (const [name] of mainWords) {
            if (!workerWordNames.has(name.toUpperCase())) {
                interpreter.remove_word(name);
            }
        }
    }
};

const isResetCommand = (code: string): boolean =>
    code.trim().toUpperCase() === 'RESET';

const isAbortError = (error: Error): boolean =>
    error.message.includes('aborted');

export const createExecutionController = (
    interpreter: AjisaiInterpreter,
    callbacks: ExecutionCallbacks
): ExecutionController => {
    const {
        getEditorValue,
        clearEditor,
        setEditorValue,
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
        getEditorValue,
        showInfo,
        showError,
        showExecutionResult,
        updateDisplays,
        saveState
    });

    const handleResult = (result: ExecuteResult, code: string): void => {
        if (result.inputHelper) {
            clearEditor(false);
            insertEditorText(result.inputHelper);
            showInfo('Input helper inserted', false);
            updateView('input');
        } else if (result.definition_to_load) {
            setEditorValue(result.definition_to_load);
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

    const runCode = async (code: string): Promise<void> => {
        if (!code) return;

        stepExecutor.reset();

        if (isResetCommand(code)) {
            await executeReset();
            return;
        }

        try {
            updateView('execution');
            showInfo('Executing...', false);

            const currentState = {
                stack: interpreter.get_stack(),
                customWords: getCustomWords(interpreter),
            };

            const result = await WORKER_MANAGER.execute(code, currentState);

            try {
                syncInterpreterState(interpreter, result);
            } catch (error) {
                console.error('[ExecController] Failed to sync state:', error);
                showError(error as Error);
            }

            handleResult(result, code);

        } catch (error) {
            console.error('[ExecController] Code execution failed:', error);
            if (error instanceof Error && isAbortError(error)) {
                showInfo('Execution aborted', true);
            } else {
                showError(error as Error);
            }
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

    const isStepModeActive = (): boolean => stepExecutor.isActive();

    const abortExecution = (): void => {
        stepExecutor.abort();
    };

    return {
        runCode,
        executeReset,
        executeStep,
        isStepModeActive,
        abortExecution
    };
};

export const executionControllerUtils = {
    getCustomWords,
    syncInterpreterState,
    isResetCommand,
    isAbortError
};
