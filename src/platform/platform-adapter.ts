import type { UserWord, Value } from '../wasm-interpreter-types';

export interface InterpreterStateSnapshot {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules?: string[];
    readonly demoWordsVersion?: number;
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
        readonly stack: unknown;
        readonly userWords: unknown;
        readonly importedModules?: unknown;
        readonly demoWordsVersion?: number;
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
    readonly version: string;
    readonly buildTimestamp: string;
    onReady(callback: () => void): void;
}

export interface SerialPortInfo {
    readonly portId: string;
    readonly label?: string;
}

/** Received bytes drained from one port, plus its host-reported connection state. */
export interface SerialInboxData {
    readonly portId: string;
    readonly bytes: number[];
    readonly disconnected: boolean;
}

/**
 * Host serial-port capability. The interpreter core never calls this directly;
 * it emits `SERIAL:` commands that the output dispatcher forwards here. The
 * interface is shaped for the stricter web case (capability detection,
 * user-gesture-driven access) so a native Tauri backend satisfies it trivially.
 *
 * Phase 1 covers the outbound methods. `drainInbox` is the Phase-2 receive
 * seam: it returns the bytes received since the previous call, to be injected
 * into the next execution snapshot.
 */
export interface SerialAdapter {
    /** Capability detection: false when the host has no serial API. */
    readonly available: boolean;
    /** Prompt the user to grant a port (web requires a user gesture). */
    requestAccess(): Promise<SerialPortInfo | null>;
    listPorts(): Promise<SerialPortInfo[]>;
    open(portId: string): Promise<void>;
    configure(portId: string, options: { readonly baudRate: number }): Promise<void>;
    write(portId: string, bytes: Uint8Array): Promise<void>;
    flush(portId: string): Promise<void>;
    /** Received bytes for one port since the last drain. */
    drainInbox(portId: string): Uint8Array;
    /** Drain every open port's received bytes, for injection into a run's snapshot. */
    drainAllInboxes(): SerialInboxData[];
    close(portId: string): Promise<void>;
}

export interface PlatformAdapter {
    readonly persistence: Persistence;
    readonly fileIO: FileIO;
    readonly runtime: Runtime;
    readonly serial: SerialAdapter;
}
