// The playground's wall-clock guard on one run, and the refusal it raises.
//
// Kept apart from the worker pool that enforces it because the guard is also
// what the host *discloses* (the profile badge lists it beside the
// interpreter's own ceilings) and what the diagnosis for a stopped run is
// written from — neither of which should have to construct a worker pool, and
// neither of which runs in a browser during the tests.

// Per-task wall-clock cap on worker execution. The recursion guard returns an
// AjisaiError immediately for blown-stack programs; this is the second line of
// defence for "still running" non-recursive loops that neither hit the
// execution-step cap fast enough nor produce a recursion error. Set well above
// the longest legitimate run so a legal program never trips it.
export const EXECUTION_TIMEOUT_MS = 5_000;

/**
 * A run stopped by the wall-clock guard rather than by anything the
 * interpreter decided.
 *
 * Distinguished by type because it is the one refusal that carries no
 * diagnosis from the language: the worker is terminated where it stands, so
 * the Rust side never builds one and never gets to name a ceiling. A host that
 * cannot tell this apart from an ordinary failure can only print the sentence
 * and leave the reader to guess whether their program is wrong or merely slow.
 */
export class ExecutionTimeoutError extends Error {
    readonly limitMs: number;

    constructor(limitMs: number) {
        super(`Execution timed out after ${limitMs} ms`);
        this.name = 'ExecutionTimeoutError';
        this.limitMs = limitMs;
    }
}
