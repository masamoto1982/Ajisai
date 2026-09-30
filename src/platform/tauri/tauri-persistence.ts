import type {
    InterpreterStateSnapshot,
    Persistence,
    StoredInterpreterState
} from '../platform-adapter';

// Resolved only at runtime inside the Tauri WebView; `@vite-ignore` keeps the
// web build from trying to bundle the Tauri SDK.
const dynamicImport = (specifier: string): Promise<any> =>
    import(/* @vite-ignore */ specifier);

// The file's one field. A `tables` array sat beside it for years without
// anything ever writing an entry; a file that still carries one is read for
// its state and rewritten without it.
interface StoredData {
    interpreterState: StoredInterpreterState | null;
}

const STATE_FILE = 'ajisai-state.json';

const cloneEmptyData = (): StoredData => ({
    interpreterState: null
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
        interpreterState: parsed.interpreterState ?? null
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
            await writeStoredData(cloneEmptyData());
            await this.migrateFromIndexedDb(() => webPersistenceModule.default.exportInterpreterState()).catch((error) => {
                console.warn('Failed to migrate IndexedDB data into Tauri storage:', error);
            });
        }

        this.opened = true;
    }

    private async migrateFromIndexedDb(
        exportWebState: () => Promise<StoredInterpreterState | null>
    ): Promise<void> {
        const interpreterState = await exportWebState();
        if (!interpreterState) {
            return;
        }

        await writeStoredData({ interpreterState });
    }

    async saveInterpreterState(state: InterpreterStateSnapshot): Promise<void> {
        await this.open();
        await writeStoredData({
            interpreterState: {
                key: 'interpreter_state',
                ...state,
                updatedAt: new Date().toISOString()
            }
        });
    }

    async loadInterpreterState(): Promise<InterpreterStateSnapshot | null> {
        const state = await this.exportInterpreterState();
        if (!state) {
            return null;
        }

        return {
            stateVersion: Number(state.stateVersion),
            stackSnapshot: state.stackSnapshot as InterpreterStateSnapshot['stackSnapshot'],
            userWords: state.userWords as InterpreterStateSnapshot['userWords'],
            activeDictionarySheet: state.activeDictionarySheet
        };
    }

    async clearAll(): Promise<void> {
        await this.open();
        await writeStoredData(cloneEmptyData());
    }

    async exportInterpreterState(): Promise<StoredInterpreterState | null> {
        await this.open();
        const current = await readStoredData();
        return current.interpreterState;
    }
}
