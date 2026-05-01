const dynamicImport = (specifier: string): Promise<any> =>
    (0, eval)(`import(${JSON.stringify(specifier)})`);

import type {
    ExportData,
    InterpreterStateSnapshot,
    Persistence,
    TablePayload
} from '../platform-adapter';

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

    await writeTextFile(STATE_FILE, JSON.stringify(data, null, 2), { baseDir: BaseDirectory.AppData });
}

export class TauriPersistence implements Persistence {
    private opened = false;

    async open(): Promise<void> {
        if (this.opened) {
            return;
        }

        const [{ exists, BaseDirectory }, webPersistenceModule] = await Promise.all([
            dynamicImport('@tauri-apps/plugin-fs'),
            import('../web/web-persistence')
        ]);

        const alreadyExists = await exists(STATE_FILE, { baseDir: BaseDirectory.AppData });
        if (!alreadyExists) {
            await writeStoredData(EMPTY_DATA);
            await this.migrateFromIndexedDb(() => webPersistenceModule.default.exportAll()).catch((error) => {
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
            stack: state.stack,
            userWords: state.userWords,
            importedModules: state.importedModules,
            demoWordsVersion: state.demoWordsVersion,
            updatedAt: new Date().toISOString()
        };
        await writeStoredData(current);
    }

    async loadInterpreterState(): Promise<InterpreterStateSnapshot | null> {
        await this.open();
        const current = await readStoredData();
        const state = current.interpreterState;
        if (!state) {
            return null;
        }

        return {
            stack: state.stack as InterpreterStateSnapshot['stack'],
            userWords: state.userWords as InterpreterStateSnapshot['userWords'],
            importedModules: state.importedModules as InterpreterStateSnapshot['importedModules'],
            demoWordsVersion: state.demoWordsVersion
        };
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
