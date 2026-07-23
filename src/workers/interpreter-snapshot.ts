import type { AjisaiInterpreter, ExecutionMode, UserWord, Value } from '../wasm-interpreter-types';

export interface SerialInboxEntry {
    readonly portId: string;
    readonly bytes: number[];
    readonly disconnected?: boolean;
}

export interface InterpreterSnapshot {
    readonly stack: Value[];
    // Lossless stack snapshot (opaque string from `snapshot_stack`) preferred
    // over the observation-format `stack` on restore. Reusing the lossy
    // observation format for the worker round-trip silently changed exact
    // values on every execution — a CodeBlock came back as nil, √2 as its
    // rational approximation. `stack` is retained for the downgrade path
    // (a wasm bundle predating `restore_stack_snapshot`). See SPEC §2.3 and
    // docs/dev/external-evaluation-response-strategy.md (P0).
    readonly stackSnapshot?: string;
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
    readonly stackSnapshot?: string;
    readonly userWords: UserWord[];
    readonly importedModules?: string[];
    readonly executionMode?: ExecutionMode;
    readonly serialInbox?: SerialInboxEntry[];
    readonly stepLimit?: number;
}): InterpreterSnapshot => ({
    stack: snapshot.stack,
    stackSnapshot: snapshot.stackSnapshot,
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
    // Phase 5: a session reset reinitializes the session but keeps the
    // cross-reset compiled-artifact cache, so an unchanged user word's compiled
    // plan is reused across runs instead of recompiled. Reuse is content-identity
    // keyed and observationally transparent; fall back to the full reset against
    // a wasm bundle that predates the API.
    if (typeof interpreter.reset_session === 'function') {
        interpreter.reset_session();
    } else {
        interpreter.reset();
    }
    if (!snapshot) return;

    if (snapshot.importedModules?.length) {
        interpreter.restore_imported_modules(snapshot.importedModules);
    }
    // Prefer the lossless snapshot so exact values (CodeBlock, ExactScalar)
    // survive the worker round-trip; fall back to the observation-format stack
    // for a wasm bundle that predates `restore_stack_snapshot` (SPEC §2.3).
    if (typeof snapshot.stackSnapshot === 'string'
        && typeof interpreter.restore_stack_snapshot === 'function') {
        interpreter.restore_stack_snapshot(snapshot.stackSnapshot);
    } else if (snapshot.stack) {
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
