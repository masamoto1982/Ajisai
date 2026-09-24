import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import type {
    AjisaiInterpreter,
    ProtocolDiagnosis,
    ExecuteResult
} from '../wasm-interpreter-types';
import {
    createExecutionSnapshot,
    collectUserWords,
    describeFailedRunOutput,
    syncInterpreterState,
    resolveExecutionException
} from './interpreter-execution-utils';
import { renderDiagnosisReport } from './diagnosis-report';
import { createStepExecutor, StepExecutor } from './step-executor';
import { detectExecutionSurfaceChanges } from './execution-surface-changes';
import type { ViewMode } from './mobile-view-switcher';
import type { ExecutionSurfaceChanges } from './gui-layout-state';

export interface ExecutionCallbacks {
    readonly extractEditorValue: () => string;
    readonly clearEditor: (switchView?: boolean) => void;
    readonly insertEditorText: (text: string) => void;
    readonly showInfo: (text: string, append: boolean) => void;
    readonly showFoldedInfo: (label: string, text: string) => void;
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
    /// Look `name` up in the dictionary and show the answer. Bound to
    /// `Ctrl+Alt+L`, which supplies the word under the cursor — see
    /// `lookupWord` below for why the answer always goes to Output.
    readonly lookupWord: (name: string) => void;
}

export const createExecutionController = (
    interpreter: AjisaiInterpreter,
    callbacks: ExecutionCallbacks
): ExecutionController => {
    const {
        extractEditorValue,
        clearEditor,
        insertEditorText,
        showInfo,
        showFoldedInfo,
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

    /// Answer a lookup from the dictionary for `name`, without running
    /// anything. Always answers to the Output area, whether `name` is a Core
    /// word (its reference text) or a User word (its reconstructed `DEF`
    /// source, shown as read-only reference rather than loaded for editing).
    ///
    /// This used to load a User word's `DEF` into the Input area instead, back
    /// when a lookup was typed on its own throwaway line and running it was
    /// the trigger — replacing that one line cost nothing. The trigger is now
    /// `Ctrl+Alt+L` at the cursor, which can be anywhere inside a program
    /// still being written, so overwriting the Input area here would risk
    /// unsaved work; Output is the only destination that is always safe.
    const lookupWord = (name: string): void => {
        if (!name) return;
        const found = interpreter.resolve_host_lookup(name);
        if (!found) {
            showError(`Unknown word: ${name}`);
            return;
        }
        showDocumentation(found.text);
        updateView('output');
    };

    const stepExecutor: StepExecutor = createStepExecutor(interpreter, {
        extractEditorValue,
        showInfo,
        highlightSourceRange,
        showError,
        showExecutionResult,
        updateDisplays,
        saveState
    });

    // The word that failed, where in the source it failed, the stack depth at
    // that point, and what to check — written *under* the error rather than
    // before it. This block used to run first, and `showError` then cleared the
    // area, so the one message that named the failing word was drawn and
    // immediately erased: what survived was a bare "Error: Stack underflow"
    // with nothing to say where.
    //
    // Selecting the diagnosis is this controller's job; presenting one belongs
    // to `renderDiagnosisReport`, which every diagnosis in the playground goes
    // through (see that module's header).
    const describeDiagnosis = (
        result: ExecuteResult
    ): { readonly text: string; readonly aboutNil: boolean } | null => {
        const event = result.errorFlowTrace
            ?.filter((candidate) => Boolean(candidate.diagnosis))
            .at(-1);
        const diagnosis: ProtocolDiagnosis | undefined = event?.diagnosis;
        if (!diagnosis) return null;
        return {
            text: renderDiagnosisReport(diagnosis, { stackLenBefore: event?.stackLenBefore }),
            // A reasoned NIL is one of the three outcomes, not a failure. The
            // trace says which this was, so the presentation can follow the
            // language instead of reporting every NIL as though something
            // went wrong.
            aboutNil: event?.kind === 'nilProduced'
        };
    };

    const applyExecutionResult = (result: ExecuteResult): void => {
        const diagnosis = describeDiagnosis(result);
        if (result.inputHelper) {
            clearEditor(false);
            insertEditorText(result.inputHelper);
            showInfo('Input helper inserted', false);
            updateView('input');
        } else if (result.status === 'OK' && !result.error) {
            showExecutionResult(result);
            clearEditor(false);
        } else {
            // Keep whatever the run printed before it failed: the host reports
            // it on the error path, and the error is written below it.
            showError(result.message || 'Unknown error', describeFailedRunOutput(result));
            // A run that failed: its diagnosis is the report, so it is open.
            if (diagnosis) showInfo(diagnosis.text, true);
            return;
        }
        if (!diagnosis) return;
        if (diagnosis.aboutNil) {
            showFoldedInfo('Why NIL', diagnosis.text);
        } else {
            showInfo(diagnosis.text, true);
        }
    };

    const executeCode = async (code: string): Promise<void> => {
        if (!code) return;

        stepExecutor.reset();

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

            applyExecutionResult(result);
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
        abortExecution,
        lookupWord
    };
};
