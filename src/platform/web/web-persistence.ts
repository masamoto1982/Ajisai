import type {
    ExportData,
    InterpreterStateSnapshot,
    Persistence,
    TablePayload
} from '../platform-adapter';

interface TableData {
    name: string;
    schema: unknown;
    records: unknown;
    updatedAt: string;
}

interface InterpreterState {
    key: string;
    stack: unknown;
    userWords: unknown;
    importedModules?: unknown;
    demoWordsVersion?: number;
    updatedAt: string;

    customWords?: unknown;
    sampleWordsVersion?: number;
}

const promisifyRequest = <T>(request: IDBRequest<T>): Promise<T> =>
    new Promise((resolve, reject) => {
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });

const withObjectStore = <T>(
    db: IDBDatabase,
    storeName: string,
    mode: IDBTransactionMode,
    action: (store: IDBObjectStore, transaction: IDBTransaction) => Promise<T>
): Promise<T> => {
    const transaction = db.transaction([storeName], mode);
    const store = transaction.objectStore(storeName);
    return action(store, transaction);
};

class WebPersistence implements Persistence {
    private dbName = 'AjisaiDB';
    private version = 4;
    private storeName = 'tables';
    private stateStoreName = 'interpreter_state';
    private db: IDBDatabase | null = null;
    private openPromise: Promise<IDBDatabase> | null = null;

    async open(): Promise<void> {
        if (this.db) {
            return;
        }

        if (this.openPromise) {
            await this.openPromise;
            return;
        }

        if (!window.indexedDB) {
            throw new Error('IndexedDB is not supported in this browser');
        }

        this.openPromise = new Promise<IDBDatabase>((resolve, reject) => {
            const request = indexedDB.open(this.dbName, this.version);

            request.onerror = () => {
                this.openPromise = null;
                reject(request.error);
            };

            request.onsuccess = () => {
                this.db = request.result;
                this.openPromise = null;
                resolve(this.db);
            };

            request.onupgradeneeded = (event) => {
                const db = (event.target as IDBOpenDBRequest).result;

                if (!db.objectStoreNames.contains(this.storeName)) {
                    db.createObjectStore(this.storeName, { keyPath: 'name' });
                }

                if (!db.objectStoreNames.contains(this.stateStoreName)) {
                    db.createObjectStore(this.stateStoreName, { keyPath: 'key' });
                }
            };
        });

        await this.openPromise;
    }

    async saveTable(name: string, schema: unknown, records: unknown): Promise<void> {
        if (!this.db) await this.open();

        return withObjectStore(this.db!, this.storeName, 'readwrite', async store => {
            const tableData: TableData = {
                name,
                schema,
                records,
                updatedAt: new Date().toISOString()
            };
            await promisifyRequest(store.put(tableData));
        });
    }

    async loadTable(name: string): Promise<TablePayload | null> {
        if (!this.db) await this.open();

        return withObjectStore(this.db!, this.storeName, 'readonly', async store => {
            const result = await promisifyRequest(store.get(name));
            return result ? { schema: result.schema, records: result.records } : null;
        });
    }

    async collectTableNames(): Promise<string[]> {
        if (!this.db) await this.open();

        return withObjectStore(this.db!, this.storeName, 'readonly', async store =>
            (await promisifyRequest(store.getAllKeys())) as string[]
        );
    }

    async deleteTable(name: string): Promise<void> {
        if (!this.db) await this.open();

        return withObjectStore(this.db!, this.storeName, 'readwrite', async store => {
            await promisifyRequest(store.delete(name));
        });
    }

    async saveInterpreterState(state: InterpreterStateSnapshot): Promise<void> {
        if (!this.db) await this.open();

        return withObjectStore(this.db!, this.stateStoreName, 'readwrite', async store => {
            const stateData: InterpreterState = {
                key: 'interpreter_state',
                ...state,
                updatedAt: new Date().toISOString()
            };
            await promisifyRequest(store.put(stateData));
        });
    }

    async loadInterpreterState(): Promise<InterpreterStateSnapshot | null> {
        if (!this.db) await this.open();

        return withObjectStore(this.db!, this.stateStoreName, 'readonly', async store => {
            const result = await promisifyRequest(store.get('interpreter_state'));
            if (!result) {
                return null;
            }
            return {
                stack: result.stack as InterpreterStateSnapshot['stack'],
                userWords: (result.userWords ?? result.customWords) as InterpreterStateSnapshot['userWords'],
                importedModules: result.importedModules as InterpreterStateSnapshot['importedModules'],
                demoWordsVersion: result.demoWordsVersion ?? result.sampleWordsVersion
            };
        });
    }

    async clearAll(): Promise<void> {
        if (!this.db) await this.open();

        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.storeName, this.stateStoreName], 'readwrite');

            const tableStore = transaction.objectStore(this.storeName);
            const stateStore = transaction.objectStore(this.stateStoreName);

            tableStore.clear();
            stateStore.clear();

            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error);
        });
    }

    async exportAll(): Promise<ExportData> {
        if (!this.db) await this.open();

        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.storeName, this.stateStoreName], 'readonly');

            const result: ExportData = {
                tables: [],
                interpreterState: null
            };

            const tableStore = transaction.objectStore(this.storeName);
            const tableRequest = tableStore.getAll();

            tableRequest.onsuccess = () => {
                result.tables = tableRequest.result as ExportData['tables'];

                const stateStore = transaction.objectStore(this.stateStoreName);
                const stateRequest = stateStore.get('interpreter_state');

                stateRequest.onsuccess = () => {
                    result.interpreterState = stateRequest.result as ExportData['interpreterState'];
                    resolve(result);
                };
            };

            tableRequest.onerror = () => reject(tableRequest.error);
        });
    }

    async importAll(data: ExportData): Promise<void> {
        if (!this.db) await this.open();

        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.storeName, this.stateStoreName], 'readwrite');

            const tableStore = transaction.objectStore(this.storeName);
            const stateStore = transaction.objectStore(this.stateStoreName);

            tableStore.clear();
            stateStore.clear();

            if (data.tables && data.tables.length > 0) {
                for (const table of data.tables) {
                    tableStore.put(table);
                }
            }

            if (data.interpreterState) {
                stateStore.put(data.interpreterState);
            }

            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error);
        });
    }
}

const DB = new WebPersistence();

export { WebPersistence };
export default DB;
