import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import type {
    AjisaiInterpreter,
    ProtocolDiagnosis,
    ExecuteResult
} from '../wasm-interpreter-types';
import {
    createExecutionSnapshot,
    collectUserWords,
    syncInterpreterState,
    resolveExecutionException
} from './interpreter-execution-utils';
import { createStepExecutor, StepExecutor } from './step-executor';
import { detectExecutionSurfaceChanges } from './execution-surface-changes';
import type { ViewMode } from './mobile-view-switcher';
import type { ExecutionSurfaceChanges } from './gui-layout-state';

export interface ExecutionCallbacks {
    readonly extractEditorValue: () => string;
    readonly clearEditor: (switchView?: boolean) => void;
    readonly updateEditorValue: (value: string) => void;
    readonly insertEditorText: (text: string) => void;
    readonly showInfo: (text: string, append: boolean) => void;
    readonly showError: (error: Error | string) => void;
    readonly showExecutionResult: (result: ExecuteResult) => void;
    readonly updateDisplays: (executedCode?: string) => void;
    readonly saveState: () => Promise<void>;
    readonly fullReset: () => Promise<void>;
    readonly updateView: (mode: ViewMode) => void;
    readonly updateAfterExecution: (changes: ExecutionSurfaceChanges) => void;
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
        updateView,
        updateAfterExecution
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
        const lastDiagnosis: ProtocolDiagnosis | undefined = result.errorFlowTrace
            ?.map((event) => event.diagnosis)
            .filter((d): d is ProtocolDiagnosis => Boolean(d))
            .at(-1);
        if (lastDiagnosis) {
            const whereLabel = lastDiagnosis.where.word ?? lastDiagnosis.where.kind;
            const lines = [
                `[DIAGNOSIS] ${lastDiagnosis.summary}`,
                `Q1 when: ${lastDiagnosis.when}`,
                `Q2 where: ${whereLabel}`,
                `Q3 why: ${lastDiagnosis.why}`,
                ...lastDiagnosis.nextChecks.map(
                    (check) => `next: ${check.label} - ${check.detail}`
                )
            ];
            showInfo(lines.join('\n'), true);
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

        let executionChanges: ExecutionSurfaceChanges | null = null;

        try {
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
            // Read the post-execution surfaces back from the SAME interpreter
            // instance (already updated by syncInterpreterState) so they are
            // compared like-for-like with the pre-execution snapshot. Comparing
            // against the worker's `result` instead skews the dictionary
            // comparison across instances and misfires on every run.
            executionChanges = detectExecutionSurfaceChanges(
                currentState,
                {
                    stack: interpreter.collect_stack(),
                    userWords: collectUserWords(interpreter),
                    importedModules: interpreter.collect_imported_modules()
                },
                result
            );

        } catch (error) {
            resolveExecutionException('ExecController', error, showInfo, showError);
        }

        updateDisplays(code);
        if (executionChanges) {
            updateAfterExecution(executionChanges);
        }
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
