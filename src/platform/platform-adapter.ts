import type { UserWord } from '../wasm-interpreter-types';

// The persisted session document. `stateVersion` and the version rule are
// gui/interpreter-state-persistence.ts's (`STATE_FORMAT_VERSION`); this is
// the one shape both stores write and read.
export interface InterpreterStateSnapshot {
    readonly stateVersion: number;
    // The lossless stack snapshot (opaque string) restore reads (LANG.OBSERVATION.FIREWALL).
    readonly stackSnapshot: string;
    readonly userWords: UserWord[];
    readonly activeDictionarySheet?: string;
}

// The record as a store holds it: the snapshot under its key, dated. Read
// back as untrusted — a hand edit or an older build can have left any field
// in any shape — so the fields a reader looks at are typed loosely and
// checked where they are read.
export interface StoredInterpreterState {
    readonly key: string;
    readonly stateVersion?: unknown;
    readonly stackSnapshot?: unknown;
    readonly userWords: unknown;
    readonly activeDictionarySheet?: string;
    readonly updatedAt: string;
}

export interface OpenResult {
    readonly filename: string;
    readonly text: string;
}

export interface SaveResult {
    readonly filename: string;
}

export interface Persistence {
    open(): Promise<void>;
    saveInterpreterState(state: InterpreterStateSnapshot): Promise<void>;
    loadInterpreterState(): Promise<InterpreterStateSnapshot | null>;
    clearAll(): Promise<void>;
    /** The stored record as it is, for one store to hand another (the Tauri migration). */
    exportInterpreterState(): Promise<StoredInterpreterState | null>;
}

export interface FileIO {
    saveJson(defaultName: string, data: unknown): Promise<SaveResult>;
    openJsonFile(): Promise<OpenResult | null>;
}

export interface Runtime {
    readonly kind: 'web' | 'tauri';
    readonly buildTimestamp: string;
    onReady(callback: () => void): void;
}

/**
 * Host-configurable execution water levels (LANG.MACHINE.LIMITS).
 * These are runtime safety controls, not language semantics: a host may
 * raise or lower them without changing what any program means, and
 * conformance never depends on a particular value.
 */
export interface ExecutionConfig {
    /**
     * Execution step budget for one run. Positive integer; `undefined`
     * keeps the interpreter's own default (`DEFAULT_MAX_EXECUTION_STEPS`,
     * derived from the host time budget — see that constant's doc comment
     * for the derivation; the value is not restated on this side).
     */
    readonly stepLimit?: number;
}



export interface PlatformAdapter {
    readonly persistence: Persistence;
    readonly fileIO: FileIO;
    readonly runtime: Runtime;
    /**
     * Where a platform surfaces host execution settings (LANG.MACHINE.LIMITS water levels).
     * Both current adapters return the empty config (all defaults); a Tauri
     * settings store or a web host embedding the playground fills this in.
     */
    readonly executionConfig: ExecutionConfig;
}
