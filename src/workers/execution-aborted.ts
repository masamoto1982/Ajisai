/** A run stopped from outside (Escape), before the interpreter could answer. */
export class ExecutionAbortedError extends Error {
    constructor() {
        super('Execution aborted');
        this.name = 'ExecutionAbortedError';
    }
}
