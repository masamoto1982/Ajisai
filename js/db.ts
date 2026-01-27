// js/db.ts

interface TableData {
    name: string;
    schema: any;
    records: any;
    updatedAt: string;
}

interface InterpreterState {
    key: string;
    stack: any;
    customWords: any;
    updatedAt: string;
}

interface ExportData {
    tables: TableData[];
    interpreterState: InterpreterState | null;
}

class AjisaiDB {
    private dbName = 'AjisaiDB';
    private version = 4;
    private storeName = 'tables';
    private stateStoreName = 'interpreter_state';
    private db: IDBDatabase | null = null;
    private openPromise: Promise<IDBDatabase> | null = null;

    async open(): Promise<IDBDatabase> {
        // 既に開いている場合はそのまま返す
        if (this.db) {
            return this.db;
        }

        // 初期化中の場合は同じPromiseを返す（競合防止）
        if (this.openPromise) {
            return this.openPromise;
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

        return this.openPromise;
    }

    async saveTable(name: string, schema: any, records: any): Promise<void> {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.storeName], 'readwrite');
            const store = transaction.objectStore(this.storeName);
            
            const tableData: TableData = {
                name,
                schema,
                records,
                updatedAt: new Date().toISOString()
            };
            
            const request = store.put(tableData);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    async loadTable(name: string): Promise<{ schema: any; records: any } | null> {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.storeName], 'readonly');
            const store = transaction.objectStore(this.storeName);
            
            const request = store.get(name);
            request.onsuccess = () => {
                const result = request.result;
                if (result) {
                    resolve({ schema: result.schema, records: result.records });
                } else {
                    resolve(null);
                }
            };
            request.onerror = () => reject(request.error);
        });
    }

    async getAllTableNames(): Promise<string[]> {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.storeName], 'readonly');
            const store = transaction.objectStore(this.storeName);
            
            const request = store.getAllKeys();
            request.onsuccess = () => resolve(request.result as string[]);
            request.onerror = () => reject(request.error);
        });
    }

    async deleteTable(name: string): Promise<void> {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.storeName], 'readwrite');
            const store = transaction.objectStore(this.storeName);
            
            const request = store.delete(name);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    async saveInterpreterState(state: Omit<InterpreterState, 'key' | 'updatedAt'>): Promise<void> {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.stateStoreName], 'readwrite');
            const store = transaction.objectStore(this.stateStoreName);
            
            const stateData: InterpreterState = {
                key: 'interpreter_state',
                ...state,
                updatedAt: new Date().toISOString()
            };
            
            const request = store.put(stateData);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    async loadInterpreterState(): Promise<Omit<InterpreterState, 'key' | 'updatedAt'> | null> {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.stateStoreName], 'readonly');
            const store = transaction.objectStore(this.stateStoreName);
            
            const request = store.get('interpreter_state');
            request.onsuccess = () => {
                const result = request.result;
                if (result) {
                    resolve({
                        stack: result.stack,
                        customWords: result.customWords
                    });
                } else {
                    resolve(null);
                }
            };
            request.onerror = () => reject(request.error);
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
                result.tables = tableRequest.result;
                
                const stateStore = transaction.objectStore(this.stateStoreName);
                const stateRequest = stateStore.get('interpreter_state');
                
                stateRequest.onsuccess = () => {
                    result.interpreterState = stateRequest.result;
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

    async test(): Promise<boolean> {
        try {
            await this.open();
            
            await this.saveTable('test_table', ['id', 'name'], [
                [1, 'Test Record 1'],
                [2, 'Test Record 2']
            ]);
            
            await this.loadTable('test_table');
            await this.getAllTableNames();
            await this.deleteTable('test_table');
            
            return true;
        } catch (error) {
            console.error('IndexedDB test failed:', error);
            return false;
        }
    }
}

const DB = new AjisaiDB();

declare global {
    interface Window {
        AjisaiDB: AjisaiDB;
    }
}

window.AjisaiDB = DB;

export default DB;
