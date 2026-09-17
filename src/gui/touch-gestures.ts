// Pure recognizers for the touch gestures the mobile presentation uses.
//
// The counting used to live inline in `gui-event-bindings.ts` as a bare
// `tapCount` / `lastTapAt` pair updated from `touchend`, which meant every
// `touchend` counted as a tap: the end of a drag-to-select inside the editor,
// the two ends of a pinch, the release of a swipe. Three of those within the
// interval ran the program. A tap is a touch that goes down and comes up in
// the same place, and a run of taps is a sequence of those close together in
// both time and space — that is what this module states, once, for both the
// editor's triple-tap and the Stack/Output double-taps to share.
//
// Thresholds are device tuning, not semantics (Portability Profiles
// "Presentation Profile": gesture thresholds and tap counts have the same
// standing as the execution step limit), so they are parameters here and the
// shipped values live with the bindings that choose them.

export interface GesturePoint {
    readonly x: number;
    readonly y: number;
}

export interface MultiTapOptions {
    /** How long after a tap a further tap still continues the same run. */
    readonly intervalMs: number;
    /** How far a tap may land from the run's first tap and still continue it. */
    readonly movementTolerancePx: number;
}

export interface MultiTapRecognizer {
    /**
     * Register one completed tap and answer how many taps the current run now
     * holds. A tap too late, or too far from where the run started, begins a
     * new run and answers 1.
     */
    readonly registerTap: (point: GesturePoint, at: number) => number;
    /** Abandon the current run; the next tap starts a new one. */
    readonly reset: () => void;
}

export const measureDistance = (from: GesturePoint, to: GesturePoint): number =>
    Math.hypot(to.x - from.x, to.y - from.y);

/**
 * Whether a touch that went down at `from` and came up at `to` stayed put
 * enough to read as a tap rather than as a drag (a text selection, a swipe,
 * a scroll).
 */
export const checkIsStationary = (
    from: GesturePoint,
    to: GesturePoint,
    tolerancePx: number
): boolean => measureDistance(from, to) <= tolerancePx;

export const createMultiTapRecognizer = (options: MultiTapOptions): MultiTapRecognizer => {
    // The anchor is the run's *first* tap, not its previous one: measuring
    // against the previous tap lets a run drift across the screen a tolerance
    // at a time, which is a drag with pauses in it, not a multi-tap.
    let anchor: GesturePoint | null = null;
    let count = 0;
    let lastTapAt = 0;

    const reset = (): void => {
        anchor = null;
        count = 0;
        lastTapAt = 0;
    };

    const registerTap = (point: GesturePoint, at: number): number => {
        const continuesRun = anchor !== null
            && at - lastTapAt <= options.intervalMs
            && checkIsStationary(anchor, point, options.movementTolerancePx);

        if (continuesRun) {
            count += 1;
        } else {
            anchor = point;
            count = 1;
        }

        lastTapAt = at;
        return count;
    };

    return { registerTap, reset };
};

export interface SwipeOptions {
    /** Minimum horizontal travel before a drag reads as a swipe. */
    readonly thresholdPx: number;
}

/**
 * The horizontal direction of a drag, or `null` when it travelled mostly
 * vertically (a scroll) or not far enough to mean anything.
 */
export const detectSwipeDirection = (
    from: GesturePoint,
    to: GesturePoint,
    options: SwipeOptions
): 'left' | 'right' | null => {
    const deltaX = to.x - from.x;
    const deltaY = to.y - from.y;

    if (Math.abs(deltaX) <= Math.abs(deltaY)) return null;
    if (Math.abs(deltaX) <= options.thresholdPx) return null;

    return deltaX > 0 ? 'right' : 'left';
};

// Elements whose own drag gesture belongs to them rather than to the layout.
// A horizontal drag inside the editor is a text selection, inside a search
// field a caret move, over the suggestion panel a scroll through the list —
// none of them is a request to leave the surface, and the panel-cycling swipe
// used to take all three because it listened on `document.body` and never
// looked at where the touch started.
export const SWIPE_EXEMPT_SELECTOR = 'textarea, input, select, option, [contenteditable="true"], .editor-suggestions';

export const checkIsSwipeExempt = (target: EventTarget | null): boolean =>
    target instanceof Element && target.closest(SWIPE_EXEMPT_SELECTOR) !== null;
