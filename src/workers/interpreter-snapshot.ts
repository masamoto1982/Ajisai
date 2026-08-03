import type { AjisaiInterpreter, UserWord, Value } from '../wasm-interpreter-types';

export interface SerialInboxEntry {
    readonly portId: string;
    readonly bytes: number[];
    readonly disconnected?: boolean;
}

export interface InterpreterSnapshot {
    // The observation-format stack, carried for display on the main thread.
    readonly stack: Value[];
    // The lossless snapshot (opaque string from `snapshot_stack`) and the only
    // format the worker round-trip restores from. Reusing the lossy observation
    // format silently changed exact values on every execution — a CodeBlock
    // came back as nil, √2 as its rational approximation. See SPEC §2.3.
    readonly stackSnapshot?: string;
    readonly userWords: UserWord[];
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
    readonly stackSnapshot?: string;
    readonly userWords: UserWord[];
    readonly serialInbox?: SerialInboxEntry[];
    readonly stepLimit?: number;
}): InterpreterSnapshot => ({
    stack: snapshot.stack,
    stackSnapshot: snapshot.stackSnapshot,
    userWords: snapshot.userWords,
    serialInbox: snapshot.serialInbox,
    stepLimit: snapshot.stepLimit
});

export const applyInterpreterSnapshot = (
    interpreter: AjisaiInterpreter,
    snapshot?: Partial<InterpreterSnapshot> | null
): void => {
    // A session reset reinitializes the session but keeps the cross-reset
    // compiled-artifact cache, so an unchanged user word's compiled plan is
    // reused across runs instead of recompiled. Reuse is content-identity keyed
    // and observationally transparent.
    interpreter.reset_session();
    if (!snapshot) return;

    // The lossless snapshot is the only accepted stack format, so exact values
    // (CodeBlock, ExactScalar) survive the worker round-trip (SPEC §2.3). A
    // snapshot without one restores an empty stack rather than silently
    // downgrading through the observation format.
    if (typeof snapshot.stackSnapshot === 'string') {
        interpreter.restore_stack_snapshot(snapshot.stackSnapshot);
    }
    if (snapshot.userWords) {
        interpreter.restore_user_words(snapshot.userWords);
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
