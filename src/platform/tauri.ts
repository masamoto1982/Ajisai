// The Tauri host: one JSON document in the app-data directory for
// persistence, and the native dialogs and filesystem for file I/O.

import type {
    ExportData,
    FileIO,
    InterpreterStateSnapshot,
    OpenResult,
    Persistence,
    SaveResult,
    TablePayload
} from './index';
import { WEB_PERSISTENCE, formatJsonDocument, readInterpreterStateSnapshot } from './web';

// Resolved only at runtime inside the Tauri WebView; `@vite-ignore` keeps the
// web build from trying to bundle the Tauri SDK.
const dynamicImport = (specifier: string): Promise<any> =>
    import(/* @vite-ignore */ specifier);

interface StoredData {
    interpreterState: ExportData['interpreterState'];
    tables: ExportData['tables'];
}

const STATE_FILE = 'ajisai-state.json';

const EMPTY_DATA: StoredData = {
    interpreterState: null,
    tables: []
};

const cloneEmptyData = (): StoredData => ({
    interpreterState: null,
    tables: []
});

async function readStoredData(): Promise<StoredData> {
    const [{ readTextFile, exists, BaseDirectory }] = await Promise.all([
        dynamicImport('@tauri-apps/plugin-fs')
    ]);

    const fileExists = await exists(STATE_FILE, { baseDir: BaseDirectory.AppData });
    if (!fileExists) {
        return cloneEmptyData();
    }

    const raw = await readTextFile(STATE_FILE, { baseDir: BaseDirectory.AppData });
    const parsed = JSON.parse(raw) as Partial<StoredData>;

    return {
        interpreterState: parsed.interpreterState ?? null,
        tables: Array.isArray(parsed.tables) ? parsed.tables : []
    };
}

async function writeStoredData(data: StoredData): Promise<void> {
    const [{ writeTextFile, BaseDirectory }] = await Promise.all([
        dynamicImport('@tauri-apps/plugin-fs')
    ]);

    await writeTextFile(STATE_FILE, formatJsonDocument(data), { baseDir: BaseDirectory.AppData });
}

export class TauriPersistence implements Persistence {
    private opened = false;

    async open(): Promise<void> {
        if (this.opened) {
            return;
        }

        const [{ exists, BaseDirectory }] = await Promise.all([
            dynamicImport('@tauri-apps/plugin-fs')
        ]);

        const alreadyExists = await exists(STATE_FILE, { baseDir: BaseDirectory.AppData });
        if (!alreadyExists) {
            await writeStoredData(EMPTY_DATA);
            // A first launch inherits whatever the web playground had stored in
            // this WebView's IndexedDB.
            await this.migrateFromIndexedDb(() => WEB_PERSISTENCE.exportAll()).catch((error) => {
                console.warn('Failed to migrate IndexedDB data into Tauri storage:', error);
            });
        }

        this.opened = true;
    }

    private async migrateFromIndexedDb(exportWebData: () => Promise<ExportData>): Promise<void> {
        const data = await exportWebData();
        const hasState = !!data.interpreterState;
        const hasTables = Array.isArray(data.tables) && data.tables.length > 0;

        if (!hasState && !hasTables) {
            return;
        }

        await writeStoredData({
            interpreterState: data.interpreterState,
            tables: data.tables
        });
    }

    async saveInterpreterState(state: InterpreterStateSnapshot): Promise<void> {
        await this.open();
        const current = await readStoredData();
        current.interpreterState = {
            key: 'interpreter_state',
            stateVersion: state.stateVersion,
            stack: state.stack,
            stackSnapshot: state.stackSnapshot,
            userWords: state.userWords,
            activeDictionarySheet: state.activeDictionarySheet,
            updatedAt: new Date().toISOString()
        };
        await writeStoredData(current);
    }

    async loadInterpreterState(): Promise<InterpreterStateSnapshot | null> {
        await this.open();
        const current = await readStoredData();
        return readInterpreterStateSnapshot(current.interpreterState);
    }

    async saveTable(name: string, schema: unknown, records: unknown): Promise<void> {
        await this.open();
        const current = await readStoredData();
        const nextTables = current.tables.filter((table) => table.name !== name);
        nextTables.push({
            name,
            schema,
            records,
            updatedAt: new Date().toISOString()
        });
        current.tables = nextTables;
        await writeStoredData(current);
    }

    async loadTable(name: string): Promise<TablePayload | null> {
        await this.open();
        const current = await readStoredData();
        const table = current.tables.find((entry) => entry.name === name);
        return table ? { schema: table.schema, records: table.records } : null;
    }

    async collectTableNames(): Promise<string[]> {
        await this.open();
        const current = await readStoredData();
        return current.tables.map((entry) => entry.name);
    }

    async deleteTable(name: string): Promise<void> {
        await this.open();
        const current = await readStoredData();
        current.tables = current.tables.filter((entry) => entry.name !== name);
        await writeStoredData(current);
    }

    async clearAll(): Promise<void> {
        await this.open();
        await writeStoredData(cloneEmptyData());
    }

    async exportAll(): Promise<ExportData> {
        await this.open();
        const current = await readStoredData();
        return {
            tables: [...current.tables],
            interpreterState: current.interpreterState
        };
    }

    async importAll(data: ExportData): Promise<void> {
        await this.open();
        await writeStoredData({
            tables: Array.isArray(data.tables) ? data.tables : [],
            interpreterState: data.interpreterState ?? null
        });
    }
}

export class TauriFileIO implements FileIO {
    async saveJson(defaultName: string, data: unknown): Promise<SaveResult> {
        const [{ save }, { writeTextFile }] = await Promise.all([
            dynamicImport('@tauri-apps/plugin-dialog'),
            dynamicImport('@tauri-apps/plugin-fs')
        ]);

        const path = await save({
            defaultPath: defaultName,
            filters: [{ name: 'JSON', extensions: ['json'] }]
        });

        if (!path) {
            throw new Error('Save cancelled');
        }

        await writeTextFile(path, formatJsonDocument(data));
        return { filename: defaultName };
    }

    async openJsonFile(): Promise<OpenResult | null> {
        const [{ open }, { readTextFile }] = await Promise.all([
            dynamicImport('@tauri-apps/plugin-dialog'),
            dynamicImport('@tauri-apps/plugin-fs')
        ]);

        const selected = await open({
            multiple: false,
            filters: [{ name: 'JSON', extensions: ['json'] }]
        });

        if (!selected || Array.isArray(selected)) {
            return null;
        }

        const text = await readTextFile(selected);
        const filename = selected.split(/[\\/]/).pop() ?? 'import.json';
        return { filename, text };
    }
}
