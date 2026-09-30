import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import type { AjisaiInterpreter, ExecuteResult } from '../wasm-interpreter-types';
import {
    createExecutionSnapshot,
    collectUserWords,
    describeFailedRunOutput,
    isFailure,
    syncInterpreterState,
    resolveExecutionException
} from './interpreter-execution-utils';
import { renderDiagnosisReport } from './diagnosis-report';
import { toError } from './to-error';
import { createStepExecutor, StepExecutor } from './step-executor';
import {
    checkRunLeftOwnNil,
    detectExecutionSurfaceChanges,
    type ExecutionStateView
} from './execution-surface-changes';
import { EXECUTION_TIMEOUT_MS } from '../workers/execution-timeout';
import type { ViewMode } from './mobile-view-switcher';
import type { ExecutionSurfaceChanges } from './gui-layout-state';

// What one trip through the worker came back with: the interpreter's result,
// or the exception that stopped the run before it could answer (the wall-clock
// guard, Abort, a broken worker). `before` is the snapshot the run was handed.
type ExecutionOutcome =
    | { readonly before: ExecutionStateView; readonly result: ExecuteResult }
    | { readonly error: unknown };

// A run that never answered changed nothing but the report in Output.
const OUTPUT_ONLY_CHANGE: ExecutionSurfaceChanges = {
    outputChanged: true,
    stackChanged: false,
    dictionaryChanged: false
};

const RUN_STATUS_TEXT =
    `Running… Escape stops it; the playground stops it after ${EXECUTION_TIMEOUT_MS / 1000} s.`;

export interface ExecutionCallbacks {
    readonly extractEditorValue: () => string;
    readonly clearEditor: (switchView?: boolean) => void;
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
    // Say on the Input surface that a run is in progress, or (`null`) that
    // none is. Input is the surface a run is started from, so it is the one
    // on screen while the run has not answered yet.
    readonly showRunStatus: (text: string | null) => void;
}

export interface ExecutionController {
    readonly executeCode: (code: string) => Promise<void>;
    readonly executeReset: () => Promise<void>;
    readonly executeStep: () => Promise<void>;
    readonly checkIsStepModeActive: () => boolean;
    readonly abortExecution: () => void;
    // Look `name` up in the dictionary and show the answer. Bound to
    // `Ctrl+Alt+L`, which supplies the word under the cursor — see
    // `lookupWord` below for why the answer always goes to Output.
    readonly lookupWord: (name: string) => void;
}

export const createExecutionController = (
    interpreter: AjisaiInterpreter,
    callbacks: ExecutionCallbacks
): ExecutionController => {
    const {
        extractEditorValue,
        clearEditor,
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
        updateAfterExecution,
        showRunStatus
    } = callbacks;

    // Answer a lookup from the dictionary for `name`, without running
    // anything. Always answers to the Output area, whether `name` is a Core
    // word (its reference text) or a User word (its reconstructed `DEF`
    // source, shown as read-only reference rather than loaded for editing).
    //
    // The trigger is `Ctrl+Alt+L` at the cursor, which can be anywhere inside
    // a program still being written, so overwriting the Input area here would
    // risk unsaved work; Output is the only destination that is always safe.
    const lookupWord = (name: string): void => {
        if (!name) return;
        const found = interpreter.resolve_host_lookup(name);
        // An unknown word is an answer too, and it goes where every answer
        // goes (spec/gui-semantics.md, Lookup).
        if (found) {
            showDocumentation(found.text);
        } else {
            showError(`Unknown word: ${name}`);
        }
        updateView('output');
    };

    // The word that failed, where in the source it failed, the stack depth at
    // that point, and what to check — written *under* the error rather than
    // before it, because `showError` clears the area and would erase anything
    // drawn ahead of it.
    //
    // Selecting the diagnosis is this controller's job; presenting one belongs
    // to `renderDiagnosisReport`, which every diagnosis in the playground goes
    // through (see that module's header).
    const describeDiagnosis = (
        result: ExecuteResult
    ): { readonly text: string; readonly aboutNil: boolean } | null => {
        // An ERROR's diagnosis is the result's own; its trace event says only
        // where the stack stood when the Word was called.
        if (result.diagnosis) {
            const event = result.errorFlowTrace
                ?.filter((candidate) => candidate.kind === 'wordError')
                .at(-1);
            return {
                text: renderDiagnosisReport(result.diagnosis, { stackLenBefore: event?.stackLenBefore }),
                aboutNil: false
            };
        }
        // A reasoned NIL is one of the three outcomes, not a failure, and the
        // trace event that produced it is the only place its diagnosis lives —
        // so the presentation can follow the language instead of reporting
        // every NIL as though something went wrong.
        const event = result.errorFlowTrace
            ?.filter((candidate) => candidate.kind === 'nilProduced' && Boolean(candidate.diagnosis))
            .at(-1);
        if (!event?.diagnosis) return null;
        return {
            text: renderDiagnosisReport(event.diagnosis, { stackLenBefore: event.stackLenBefore }),
            aboutNil: true
        };
    };

    // The report half of a run's answer: what it printed or the error it
    // stopped with, then the diagnosis. A "Why NIL" is offered only for a NIL
    // this run left on the stack (`leftOwnNil`); the trace also names the Word
    // that merely left an older NIL on top, which is not this run's to explain.
    const reportExecutionResult = (result: ExecuteResult, leftOwnNil: boolean): void => {
        const diagnosis = describeDiagnosis(result);
        if (isFailure(result)) {
            // Keep whatever the run printed before it failed: the host reports
            // it on the error path, and the error is written below it.
            showError(result.message || 'Unknown error', describeFailedRunOutput(result));
            // A run that failed: its diagnosis is the report, so it is open.
            if (diagnosis) showInfo(diagnosis.text, true);
            return;
        }
        showExecutionResult(result);
        if (!diagnosis) return;
        if (!diagnosis.aboutNil) {
            showInfo(diagnosis.text, true);
        } else if (leftOwnNil) {
            showFoldedInfo('Why NIL', diagnosis.text);
        }
    };

    // Everything a run's answer does to the page, in one place: the report in
    // Output, its diagnosis or "Why NIL", the redraw, the move to the surfaces
    // the run changed, and the save. Run and Step both come through here, and
    // so does a run that never answered — stopped by the wall-clock guard or by
    // Abort — so each of them is reported and surfaced the same way.
    //
    // The post-run surfaces are read back from the SAME interpreter instance
    // (already updated by syncInterpreterState) so they are compared
    // like-for-like with the snapshot the run was handed. Comparing against the
    // worker's `result` instead skews the dictionary comparison across
    // instances and misfires on every run.
    const applyExecutionResult = async (
        context: string,
        outcome: ExecutionOutcome,
        clearEditorOnSuccess: boolean
    ): Promise<boolean> => {
        let changes: ExecutionSurfaceChanges = OUTPUT_ONLY_CHANGE;
        let succeeded = false;

        try {
            if ('error' in outcome) throw outcome.error;
            const { before, result } = outcome;
            try {
                syncInterpreterState(interpreter, result);
            } catch (error) {
                console.error(`[${context}] Failed to sync state:`, error);
                showError(toError(error));
            }
            const after: ExecutionStateView = {
                stack: interpreter.collect_stack(),
                userWords: collectUserWords(interpreter)
            };
            reportExecutionResult(result, checkRunLeftOwnNil(before, after));
            succeeded = !isFailure(result);
            if (succeeded && clearEditorOnSuccess) clearEditor(false);
            changes = detectExecutionSurfaceChanges(before, after, result);
        } catch (error) {
            resolveExecutionException(context, error, showInfo, showError);
        }

        updateDisplays();
        updateAfterExecution(changes);
        await saveState();
        return succeeded;
    };

    // One execution path: a run is one task on one worker, and its answer —
    // or the exception that stood in for one — goes to `applyExecutionResult`.
    const executeSource = async (
        context: string,
        code: string,
        clearEditorOnSuccess: boolean
    ): Promise<boolean> => {
        let outcome: ExecutionOutcome;
        showRunStatus(RUN_STATUS_TEXT);
        try {
            const before = createExecutionSnapshot(interpreter);
            outcome = { before, result: await WORKER_MANAGER.execute(code, before) };
        } catch (error) {
            outcome = { error };
        } finally {
            showRunStatus(null);
        }
        return applyExecutionResult(context, outcome, clearEditorOnSuccess);
    };

    const stepExecutor: StepExecutor = createStepExecutor({
        extractEditorValue,
        showInfo,
        highlightSourceRange,
        executeSource: (code) => executeSource('StepExecutor', code, false)
    });

    const executeCode = async (code: string): Promise<void> => {
        if (!code) return;

        stepExecutor.reset();
        showInfo('Executing...', false);
        await executeSource('ExecController', code, true);
    };

    const executeReset = async (): Promise<void> => {
        try {
            console.log('[ExecController] Executing full reset');
            stepExecutor.reset();
            await WORKER_MANAGER.resetAllWorkers();
            const result = interpreter.reset();

            if (!isFailure(result)) {
                clearEditor(true);
                await fullReset();
                updateView('input');
            } else {
                showError(result.message || 'RESET execution failed');
            }
        } catch (error) {
            console.error('[ExecController] Reset failed:', error);
            showError(toError(error));
        }
    };

    const executeStep = async (): Promise<void> => {
        await stepExecutor.executeStep();
    };

    const checkIsStepModeActive = (): boolean => stepExecutor.isActive();

    // Abort stops the run where it stands — the worker carrying it is
    // replaced, exactly as the wall-clock guard does — and the run answers
    // through `applyExecutionResult` like any other that did not complete.
    const abortExecution = (): void => {
        WORKER_MANAGER.abortAll();
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
