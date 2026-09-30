import { scanAtoms } from './source-text';

// ── Splitting source into steps ─────────────────────────────────────────────
// The pieces step mode executes, with each piece's position in the source.
//
// The unit is a *balanced* piece of source, not a whitespace-separated atom.
// Step mode runs each piece on its own against the persisted interpreter
// state, so a piece that cannot stand alone cannot be stepped: `[ 42 ]` is the
// idiomatic scalar, and splitting it on whitespace would hand the interpreter
// a bare `[`.

// One piece of source that step mode can execute on its own, with where it
// sits in the source. The offsets travel with the text rather than being
// recomputed later: a piece's text can repeat, so searching the source for it
// would land on the wrong occurrence.
export interface StepToken {
    readonly text: string;
    readonly start: number;
    readonly end: number;
}

// Split `code` into the balanced pieces step mode executes, in order.
//
// A piece is one atom at bracket depth zero, or a whole `[ ... ]` group with
// everything nested inside it. Interior whitespace and line breaks are
// preserved, because the piece is executed as the source text it is: a
// multi-line vector is one value and stepping through it half-built would
// execute something the program never contains. Comments are dropped: a
// comment is not a step.
//
// Malformed source is deliberately *not* repaired here. An unclosed `[` yields
// one final piece running to the end of the source, a stray `]` yields a
// piece of its own, and an unclosed string runs to the end; either way the
// interpreter reports the real source error against the real text, which is
// the same error a plain run would give.
export const tokenizeWithOffsets = (code: string): StepToken[] => {
    const pieces: StepToken[] = [];
    const all = scanAtoms(code).filter((atom) => atom.kind !== 'comment');
    let index = 0;
    while (index < all.length) {
        const first = all[index]!;
        let depth = 0;
        let last = first;
        do {
            const atom = all[index]!;
            if (atom.text === '[') depth += 1;
            else if (atom.text === ']') depth -= 1;
            last = atom;
            index += 1;
        } while (depth > 0 && index < all.length);
        pieces.push({ text: code.slice(first.start, last.end), start: first.start, end: last.end });
    }
    return pieces;
};

// ── Running the steps ───────────────────────────────────────────────────────

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

const INITIAL_STATE: StepState = {
    active: false,
    tokens: [],
    currentIndex: 0
};

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

    let state: StepState = INITIAL_STATE;

    const isActive = (): boolean => state.active;

    // Only a highlight step mode drew is taken back. Run and Reset call this
    // too, to end any step mode in progress, and collapsing the selection
    // unconditionally moved the caret to the start of the text on every one
    // of them — so a Run that failed left its text in place but the caret at
    // the top of it.
    const reset = (): void => {
        const wasActive = state.active;
        state = INITIAL_STATE;
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

        state = { active: true, tokens, currentIndex: 0 };

        await executeNextToken(`[STEP] Step mode: ${tokens.length} steps (Ctrl+Enter to continue)`);
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

        state = { ...state, currentIndex: state.currentIndex + 1 };
        if (state.currentIndex >= state.tokens.length) {
            showInfo('[DONE] Step mode completed', true);
            reset();
        }
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
