import type {
    InterpreterStateSnapshot,
    Persistence,
    StoredInterpreterState
} from '../platform-adapter';

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

// One record, under one key, in one object store. A second store, `tables`,
// was created, cleared, exported and migrated for years without anything
// ever writing a row to it; version 5 drops it.
const STATE_STORE = 'interpreter_state';
const STATE_KEY = 'interpreter_state';
const RETIRED_TABLES_STORE = 'tables';

class WebPersistence implements Persistence {
    private dbName = 'AjisaiDB';
    private version = 5;
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

                if (db.objectStoreNames.contains(RETIRED_TABLES_STORE)) {
                    db.deleteObjectStore(RETIRED_TABLES_STORE);
                }

                if (!db.objectStoreNames.contains(STATE_STORE)) {
                    db.createObjectStore(STATE_STORE, { keyPath: 'key' });
                }
            };
        });

        await this.openPromise;
    }

    async saveInterpreterState(state: InterpreterStateSnapshot): Promise<void> {
        if (!this.db) await this.open();

        return withObjectStore(this.db!, STATE_STORE, 'readwrite', async store => {
            const stateData: StoredInterpreterState = {
                key: STATE_KEY,
                ...state,
                updatedAt: new Date().toISOString()
            };
            await promisifyRequest(store.put(stateData));
        });
    }

    async loadInterpreterState(): Promise<InterpreterStateSnapshot | null> {
        const result = await this.exportInterpreterState();
        if (!result) {
            return null;
        }
        return {
            stateVersion: Number(result.stateVersion),
            stackSnapshot: result.stackSnapshot as InterpreterStateSnapshot['stackSnapshot'],
            userWords: result.userWords as InterpreterStateSnapshot['userWords'],
            activeDictionarySheet: result.activeDictionarySheet
        };
    }

    async clearAll(): Promise<void> {
        if (!this.db) await this.open();

        return withObjectStore(this.db!, STATE_STORE, 'readwrite', async store => {
            await promisifyRequest(store.clear());
        });
    }

    async exportInterpreterState(): Promise<StoredInterpreterState | null> {
        if (!this.db) await this.open();

        return withObjectStore(this.db!, STATE_STORE, 'readonly', async store =>
            ((await promisifyRequest(store.get(STATE_KEY))) as StoredInterpreterState | undefined) ?? null
        );
    }
}

const DB = new WebPersistence();

export default DB;
