import type { UserWord, Value } from '../wasm-interpreter-types';

export interface InterpreterStateSnapshot {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules?: string[];
    readonly demoWordsVersion?: number;
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

export interface PlatformAdapter {
    readonly persistence: Persistence;
    readonly fileIO: FileIO;
    readonly runtime: Runtime;
}
