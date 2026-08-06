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
    readonly highlightSourceRange: (start: number, end: number) => void;
    readonly showDocumentation: (text: string) => void;
    readonly showError: (error: Error | string, precedingOutput?: string) => void;
    readonly showExecutionResult: (result: ExecuteResult) => void;
    readonly updateDisplays: () => void;
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
        highlightSourceRange,
        showDocumentation,
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
        highlightSourceRange,
        showError,
        showExecutionResult,
        updateDisplays,
        saveState
    });

    const evidence = (entries: readonly string[] | undefined, key: string): string | null => {
        const hit = entries?.find((entry) => entry.startsWith(`${key}=`));
        return hit ? hit.slice(key.length + 1) : null;
    };

    // The word that failed, where in the source it failed, the stack depth at
    // that point, and what to check — written *under* the error rather than
    // before it. This block used to run first, and `showError` then cleared the
    // area, so the one message that named the failing word was drawn and
    // immediately erased: what survived was a bare "Error: Stack underflow"
    // with nothing to say where.
    const describeDiagnosis = (result: ExecuteResult): string | null => {
        const event = result.errorFlowTrace
            ?.filter((candidate) => Boolean(candidate.diagnosis))
            .at(-1);
        const diagnosis: ProtocolDiagnosis | undefined = event?.diagnosis;
        if (!diagnosis) return null;

        const where = diagnosis.where.word
            ? `${diagnosis.where.word} (${diagnosis.where.kind})`
            : diagnosis.where.kind;
        const depth =
            event && typeof event.stackLenBefore === 'number'
                ? `, stack depth ${event.stackLenBefore}`
                : '';
        // Where in the source the run was when it failed. The host records it
        // as evidence — the same `key=value` channel `stackLenBefore` uses —
        // so nothing about the protocol had to change to carry it.
        const at = evidence(diagnosis.evidence, 'sourceLine')
            ? ` at line ${evidence(diagnosis.evidence, 'sourceLine')}, column ${evidence(
                  diagnosis.evidence,
                  'sourceColumn'
              )}`
            : '';
        return [
            `[DIAGNOSIS] ${diagnosis.summary}`,
            `Q1 when: ${diagnosis.when}`,
            `Q2 where: ${where}${at}${depth}`,
            `Q3 why: ${diagnosis.why}`,
            ...diagnosis.nextChecks.map((check) => `next: ${check.label} - ${check.detail}`)
        ].join('\n');
    };

    const applyExecutionResult = (result: ExecuteResult, code: string): void => {
        const diagnosis = describeDiagnosis(result);
        if (result.inputHelper) {
            clearEditor(false);
            insertEditorText(result.inputHelper);
            showInfo('Input helper inserted', false);
            updateView('input');
        } else if (result.documentation) {
            // Reference text for a Core Word is read, not edited, so it goes to
            // the output area. It used to be written into the editor, where
            // several screens of prose replaced whatever the user had typed;
            // `'ADD' ?` in the middle of writing a program lost the program.
            showDocumentation(result.documentation);
            updateView('output');
        } else if (result.definition_to_load) {
            // A User Word's reconstructed `DEF` *is* meant for the editor: the
            // point of looking one up is to edit it and define it again. What it
            // replaces is the `?` line that just ran.
            updateEditorValue(result.definition_to_load);
            const wordName = code.replace(/\?|LOOKUP/gi, "").trim();
            showInfo(`Showing definition: ${wordName}`, false);
            updateView('input');
        } else if (result.status === 'OK' && !result.error) {
            showExecutionResult(result);
            clearEditor(false);
        } else {
            // Keep whatever the run printed before it failed: the host reports
            // it on the error path, and the error is written below it.
            showError(result.message || 'Unknown error', result.output || '');
            if (diagnosis) showInfo(diagnosis, true);
            return;
        }
        if (diagnosis) showInfo(diagnosis, true);
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
                    userWords: collectUserWords(interpreter)
                },
                result
            );

        } catch (error) {
            resolveExecutionException('ExecController', error, showInfo, showError);
        }

        updateDisplays();
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
