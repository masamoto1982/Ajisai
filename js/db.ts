// js/db.ts

interface TableData {
    name: string;
    schema: any;
    records: any;
    updatedAt: string;
}

interface InterpreterState {
    key: string;
    workspace: any;  // stack → workspace
    customWords: any;
    updatedAt: string;
}

interface ExportData {
    tables: TableData[];
    interpreterState: InterpreterState | null;
}

class AjisaiDB {
    private dbName = 'AjisaiDB';
    private version = 4;  // バージョンアップ（workspace対応）
    private storeName = 'tables';
    private stateStoreName = 'interpreter_state';
    private db: IDBDatabase | null = null;

    async open(): Promise<IDBDatabase> {
        console.log('Opening IndexedDB...');
        
        if (!window.indexedDB) {
            throw new Error('IndexedDB is not supported in this browser');
        }
        
        return new Promise((resolve, reject) => {
            const request = indexedDB.open(this.dbName, this.version);
            
            request.onerror = () => {
                console.error('IndexedDB open error:', request.error);
                reject(request.error);
            };
            
            request.onsuccess = () => {
                this.db = request.result;
                console.log('IndexedDB opened successfully');
                resolve(this.db);
            };
            
            request.onupgradeneeded = (event) => {
                console.log('IndexedDB upgrade needed, creating stores...');
                const db = (event.target as IDBOpenDBRequest).result;
                
                if (!db.objectStoreNames.contains(this.storeName)) {
                    console.log(`Creating store: ${this.storeName}`);
                    db.createObjectStore(this.storeName, { keyPath: 'name' });
                }
                
                if (!db.objectStoreNames.contains(this.stateStoreName)) {
                    console.log(`Creating store: ${this.stateStoreName}`);
                    db.createObjectStore(this.stateStoreName, { keyPath: 'key' });
                }
                
                console.log('IndexedDB stores created successfully');
            };
        });
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
                    resolve({
                        schema: result.schema,
                        records: result.records
                    });
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
                        workspace: result.workspace,  // stack → workspace
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
            
            transaction.oncomplete = () => {
                console.log('All data cleared from IndexedDB');
                resolve();
            };
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
            console.log('Starting IndexedDB test...');
            
            await this.open();
            console.log('✓ Database opened successfully');
            
            await this.saveTable('test_table', ['id', 'name'], [
                [1, 'Test Record 1'],
                [2, 'Test Record 2']
            ]);
            console.log('✓ Test table saved successfully');
            
            const loadedTable = await this.loadTable('test_table');
            console.log('✓ Test table loaded:', loadedTable);
            
            const tableNames = await this.getAllTableNames();
            console.log('✓ Table names retrieved:', tableNames);
            
            await this.deleteTable('test_table');
            console.log('✓ Test table deleted successfully');
            
            console.log('IndexedDB test completed successfully!');
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

console.log('AjisaiDB initialized:', DB);

export default DB;
