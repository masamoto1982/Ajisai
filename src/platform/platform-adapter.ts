import type { UserWord, Value } from '../wasm-interpreter-types';

export interface InterpreterStateSnapshot {
    // Format identifier of the persisted document; see STATE_FORMAT_VERSION and
    // InterpreterState in gui/interpreter-state-persistence.ts.
    readonly stateVersion: number;
    // The observation-format stack, persisted for display only.
    readonly stack: Value[];
    // The lossless stack snapshot (opaque string) restore reads (SPEC §2.3).
    readonly stackSnapshot: string;
    readonly userWords: UserWord[];
    readonly activeDictionarySheet?: string;
    readonly activeUserDictionary?: string;
}

export interface TablePayload {
    readonly schema: unknown;
    readonly records: unknown;
}

export interface ExportData {
    tables: Array<{
        readonly name: string;
        readonly schema: unknown;
        readonly records: unknown;
        readonly updatedAt: string;
    }>;
    interpreterState: {
        readonly key: string;
        readonly stateVersion?: unknown;
        readonly stack: unknown;
        readonly stackSnapshot?: unknown;
        readonly userWords: unknown;
        readonly activeDictionarySheet?: string;
        readonly activeUserDictionary?: string;
        readonly updatedAt: string;
    } | null;
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
    saveTable(name: string, schema: unknown, records: unknown): Promise<void>;
    loadTable(name: string): Promise<TablePayload | null>;
    collectTableNames(): Promise<string[]>;
    deleteTable(name: string): Promise<void>;
    clearAll(): Promise<void>;
    exportAll(): Promise<ExportData>;
    importAll(data: ExportData): Promise<void>;
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
 * Host-configurable execution water levels (SPECIFICATION.html §5.3).
 * These are runtime safety controls, not language semantics: a host may
 * raise or lower them without changing what any program means, and
 * conformance never depends on a particular value.
 */
export interface ExecutionConfig {
    /**
     * Execution step budget for one run. Positive integer; `undefined`
     * keeps the interpreter default (100,000).
     */
    readonly stepLimit?: number;
}



export interface PlatformAdapter {
    readonly persistence: Persistence;
    readonly fileIO: FileIO;
    readonly runtime: Runtime;
    /**
     * Where a platform surfaces host execution settings (§5.3 water levels).
     * Both current adapters return the empty config (all defaults); a Tauri
     * settings store or a web host embedding the playground fills this in.
     */
    readonly executionConfig: ExecutionConfig;
}
