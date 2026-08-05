// How much of a stack value the Stack area draws.
//
// The interpreter's materialization ceiling bounds what a generative Word may
// build, not what the host can draw. `[ 1 500000 ] RANGE` sits well inside that
// ceiling and is an ordinary, correct program — and rendering it drew one DOM
// node per element, locking the browser tab for tens of seconds with no way to
// abort, clear the editor, or read the result. A safety mechanism that stops
// the interpreter and then hands the host an unbounded drawing job has only
// moved where the program hangs.
//
// So the Stack area draws a bounded prefix of any collection and says, in
// place, how many elements it left out. This is presentation, in the same class
// as the execution step limit: a host control, not a language constraint. The
// value on the stack is whole and unmodified, every Word still sees all of it,
// and `LENGTH` remains the way to ask how long it really is.

/** Elements drawn from any one collection before the rest is summarized. */
export const MAX_RENDERED_ELEMENTS_PER_COLLECTION = 100;

/**
 * Elements drawn across the whole Stack area in one render. A per-collection
 * cap alone still admits a stack of ten thousand short vectors, so the budget
 * is global to the render and every collection draws from it.
 */
export const MAX_RENDERED_ELEMENTS_PER_STACK = 2_000;

export interface RenderBudget {
    remaining: number;
}

export const createRenderBudget = (
    total: number = MAX_RENDERED_ELEMENTS_PER_STACK
): RenderBudget => ({ remaining: Math.max(0, total) });

export interface CollectionRenderPlan {
    /** Leading elements to draw. */
    readonly shown: number;
    /** Elements summarized instead of drawn; 0 means the collection is complete. */
    readonly elided: number;
}

/**
 * Decide how much of a `length`-element collection to draw, and charge the
 * drawn part to the shared budget. A nested collection costs one element in its
 * parent and then draws its own children from the same budget, so the total
 * node count of one render stays bounded whatever the nesting looks like.
 */
export const planCollectionRender = (length: number, budget: RenderBudget): CollectionRenderPlan => {
    const safeLength = Number.isFinite(length) && length > 0 ? Math.floor(length) : 0;
    const shown = Math.min(safeLength, MAX_RENDERED_ELEMENTS_PER_COLLECTION, budget.remaining);
    budget.remaining -= shown;
    return { shown, elided: safeLength - shown };
};

/**
 * The marker that stands in for the undrawn tail. It is deliberately not
 * Ajisai-shaped — a reader must never mistake it for part of the value — and it
 * states the exact count, so the display never implies the collection ends
 * where the drawing does.
 */
export const formatElision = (elided: number): string => `… ${elided} more`;
