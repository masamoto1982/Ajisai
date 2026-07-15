import type { AjisaiInterpreter, ExecutionMode, UserWord, Value } from '../wasm-interpreter-types';

export interface SerialInboxEntry {
    readonly portId: string;
    readonly bytes: number[];
    readonly disconnected?: boolean;
}

export interface InterpreterSnapshot {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules: string[];
    readonly executionMode: ExecutionMode;
    /** Host-received serial bytes to inject before this run (SERIAL@READ). */
    readonly serialInbox?: SerialInboxEntry[];
    /**
     * Host override for the execution step budget (water level, SPEC §5.3).
     * A positive integer; omitted keeps the interpreter default (100,000).
     * Runtime safety control, not a language semantic.
     */
    readonly stepLimit?: number;
}

export const createInterpreterSnapshot = (snapshot: {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules?: string[];
    readonly executionMode?: ExecutionMode;
    readonly serialInbox?: SerialInboxEntry[];
    readonly stepLimit?: number;
}): InterpreterSnapshot => ({
    stack: snapshot.stack,
    userWords: snapshot.userWords,
    importedModules: snapshot.importedModules ?? [],
    executionMode: snapshot.executionMode ?? "greedy",
    serialInbox: snapshot.serialInbox,
    stepLimit: snapshot.stepLimit
});

export const applyInterpreterSnapshot = (
    interpreter: AjisaiInterpreter,
    snapshot?: Partial<InterpreterSnapshot> | null
): void => {
    interpreter.reset();
    if (!snapshot) return;

    if (snapshot.importedModules?.length) {
        interpreter.restore_imported_modules(snapshot.importedModules);
    }
    if (snapshot.stack) {
        interpreter.restore_stack(snapshot.stack);
    }
    if (snapshot.userWords) {
        interpreter.restore_user_words(snapshot.userWords);
    }
    if (snapshot.executionMode) {
        interpreter.set_execution_mode(snapshot.executionMode);
    }
    // Untrusted partial snapshot: only a positive finite integer is a valid
    // budget; anything else keeps the interpreter default (the wasm side
    // ignores non-positive values as a second line of defence).
    if (typeof snapshot.stepLimit === 'number'
        && Number.isInteger(snapshot.stepLimit)
        && snapshot.stepLimit > 0) {
        interpreter.set_max_execution_steps(snapshot.stepLimit);
    }
    // The parameter is an explicitly partial/untrusted snapshot, so validate
    // each serial entry instead of trusting its shape: a non-array inbox, a
    // null entry, a non-string portId or missing/non-array bytes previously
    // threw a TypeError (`Uint8Array.from(null)` / non-iterable) and aborted
    // the whole restore. Malformed entries are skipped.
    if (Array.isArray(snapshot.serialInbox)) {
        for (const entry of snapshot.serialInbox) {
            if (!entry || typeof entry !== 'object') continue;
            const { portId, bytes, disconnected } = entry as SerialInboxEntry;
            if (typeof portId !== 'string' || !Array.isArray(bytes)) continue;
            // update_serial_inbox clears the disconnected flag, so mark after.
            interpreter.update_serial_inbox(portId, Uint8Array.from(bytes));
            if (disconnected) {
                interpreter.mark_serial_disconnected(portId);
            }
        }
    }
};
