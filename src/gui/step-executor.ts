import { tokenizeWithOffsets, type StepToken } from './step-tokens';

interface StepState {
    readonly active: boolean;
    readonly tokens: readonly StepToken[];
    readonly currentIndex: number;
}

export interface StepExecutorCallbacks {
    readonly extractEditorValue: () => string;
    readonly showInfo: (text: string, append: boolean) => void;
    // Show the reader where execution has got to, by selecting the token that
    // is about to run, so following a run never means counting tokens by eye
    // against the source. Called with an empty range when step mode ends.
    readonly highlightSourceRange: (start: number, end: number) => void;
    // Run one step's text on the path Run takes, which reports its answer,
    // diagnosis included, and surfaces what it changed. Resolves whether the
    // step completed.
    readonly executeSource: (code: string) => Promise<boolean>;
}

export interface StepExecutor {
    readonly isActive: () => boolean;
    readonly reset: () => void;
    readonly executeStep: () => Promise<void>;
    readonly abort: () => void;
}

const createInitialState = (): StepState => ({
    active: false,
    tokens: [],
    currentIndex: 0
});

const createActiveState = (tokens: StepToken[]): StepState => ({
    active: true,
    tokens,
    currentIndex: 0
});

const advanceState = (state: StepState): StepState => ({
    ...state,
    currentIndex: state.currentIndex + 1
});

// A step's text can span lines — a multi-line vector is one step — and a
// status line that wrapped mid-vector would undo the point of showing it. The
// editor highlight carries the exact range, so the message only needs enough
// of the text to recognise which step it is.
const STEP_LABEL_LIMIT = 40;

const formatStepMessage = (
    currentIndex: number,
    totalSteps: number,
    step: string
): string => {
    const remaining = totalSteps - currentIndex - 1;
    const collapsed = step.replace(/\s+/g, ' ');
    const label =
        collapsed.length > STEP_LABEL_LIMIT
            ? `${collapsed.slice(0, STEP_LABEL_LIMIT - 1)}…`
            : collapsed;
    return `[>] Step ${currentIndex + 1}/${totalSteps}: "${label}" (${remaining} remaining)`;
};

export const createStepExecutor = (callbacks: StepExecutorCallbacks): StepExecutor => {
    const {
        extractEditorValue,
        showInfo,
        highlightSourceRange,
        executeSource
    } = callbacks;

    let state = createInitialState();

    const isActive = (): boolean => state.active;

    // Only a highlight step mode drew is taken back. Run and Reset call this
    // too, to end any step mode in progress, and collapsing the selection
    // unconditionally moved the caret to the start of the text on every one
    // of them — so a Run that failed left its text in place but the caret at
    // the top of it.
    const reset = (): void => {
        const wasActive = state.active;
        state = createInitialState();
        if (wasActive) highlightSourceRange(0, 0);
    };

    const abort = (): void => {
        if (state.active) {
            reset();
            showInfo('Step mode aborted', true);
        }
    };

    const startStepMode = async (): Promise<void> => {
        const code = extractEditorValue();
        if (!code) return;

        const tokens = tokenizeWithOffsets(code);

        if (tokens.length === 0) {
            showInfo('No code', true);
            return;
        }

        state = createActiveState(tokens);

        await executeNextToken(`[STEP] Step mode: ${tokens.length} steps (Ctrl+Enter to continue)`);
    };

    const finish = (): void => {
        showInfo('[DONE] Step mode completed', true);
        reset();
    };

    // The status lines are written *after* the step's answer: the answer
    // replaces the Output area, so a line written ahead of it was erased by
    // the very step it described.
    const executeNextToken = async (preface?: string): Promise<void> => {
        const token = state.tokens[state.currentIndex]!;
        const message = formatStepMessage(state.currentIndex, state.tokens.length, token.text);

        // Mark the token before running it, so the highlight always shows
        // what is *about* to happen rather than what just did.
        highlightSourceRange(token.start, token.end);

        const completed = await executeSource(token.text);

        // Aborted while the step was running: `abort` has already ended step
        // mode and said so.
        if (!state.active) return;

        if (preface) showInfo(preface, true);
        showInfo(message, true);

        if (!completed) {
            reset();
            return;
        }

        state = advanceState(state);
        if (state.currentIndex >= state.tokens.length) finish();
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
        abort
    };
};
