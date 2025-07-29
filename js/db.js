// IndexedDB操作のラッパー
const DB = {
    dbName: 'AjisaiDB',
    version: 2,  // バージョンを上げる
    storeName: 'tables',
    stateStoreName: 'interpreter_state',  // 新しいストア
    db: null,

    // データベースを開く
    async open() {
        console.log('Opening IndexedDB...');
        
        // IndexedDBがサポートされているかチェック
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
                const db = event.target.result;
                
                // テーブル用のストア
                if (!db.objectStoreNames.contains(this.storeName)) {
                    console.log(`Creating store: ${this.storeName}`);
                    db.createObjectStore(this.storeName, { keyPath: 'name' });
                }
                
                // インタープリタの状態用のストア
                if (!db.objectStoreNames.contains(this.stateStoreName)) {
                    console.log(`Creating store: ${this.stateStoreName}`);
                    db.createObjectStore(this.stateStoreName, { keyPath: 'key' });
                }
                
                console.log('IndexedDB stores created successfully');
            };
        });
    },

    // テーブルを保存
    async saveTable(name, schema, records) {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName], 'readwrite');
            const store = transaction.objectStore(this.storeName);
            
            const tableData = {
                name: name,
                schema: schema,
                records: records,
                updatedAt: new Date().toISOString()
            };
            
            const request = store.put(tableData);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    },

    // テーブルを読み込み
    async loadTable(name) {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName], 'readonly');
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
    },

    // すべてのテーブル名を取得
    async getAllTableNames() {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName], 'readonly');
            const store = transaction.objectStore(this.storeName);
            
            const request = store.getAllKeys();
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
    },

    // テーブルを削除
    async deleteTable(name) {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName], 'readwrite');
            const store = transaction.objectStore(this.storeName);
            
            const request = store.delete(name);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    },

    // インタープリタの状態を保存
    async saveInterpreterState(state) {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.stateStoreName], 'readwrite');
            const store = transaction.objectStore(this.stateStoreName);
            
            const stateData = {
                key: 'interpreter_state',
                stack: state.stack,
                register: state.register,
                customWords: state.customWords,
                updatedAt: new Date().toISOString()
            };
            
            const request = store.put(stateData);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    },

    // インタープリタの状態を読み込み
    async loadInterpreterState() {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.stateStoreName], 'readonly');
            const store = transaction.objectStore(this.stateStoreName);
            
            const request = store.get('interpreter_state');
            request.onsuccess = () => {
                const result = request.result;
                if (result) {
                    resolve({
                        stack: result.stack,
                        register: result.register,
                        customWords: result.customWords
                    });
                } else {
                    resolve(null);
                }
            };
            request.onerror = () => reject(request.error);
        });
    },

    // すべての状態を保存（テーブル + インタープリタ状態）
    async saveAllState(tables, interpreterState) {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName, this.stateStoreName], 'readwrite');
            
            // テーブルを保存
            const tableStore = transaction.objectStore(this.storeName);
            for (const [name, data] of Object.entries(tables)) {
                tableStore.put({
                    name: name,
                    schema: data.schema,
                    records: data.records,
                    updatedAt: new Date().toISOString()
                });
            }
            
            // インタープリタ状態を保存
            const stateStore = transaction.objectStore(this.stateStoreName);
            stateStore.put({
                key: 'interpreter_state',
                ...interpreterState,
                updatedAt: new Date().toISOString()
            });
            
            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error);
        });
    },

    // 全データをエクスポート
    async exportAll() {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName, this.stateStoreName], 'readonly');
            
            const result = {
                tables: [],
                interpreterState: null
            };
            
            // テーブルを取得
            const tableStore = transaction.objectStore(this.storeName);
            const tableRequest = tableStore.getAll();
            
            tableRequest.onsuccess = () => {
                result.tables = tableRequest.result;
                
                // インタープリタ状態を取得
                const stateStore = transaction.objectStore(this.stateStoreName);
                const stateRequest = stateStore.get('interpreter_state');
                
                stateRequest.onsuccess = () => {
                    result.interpreterState = stateRequest.result;
                    resolve(result);
                };
            };
            
            tableRequest.onerror = () => reject(tableRequest.error);
        });
    },

    // データをインポート
    async importAll(data) {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName, this.stateStoreName], 'readwrite');
            
            // 既存データをクリア
            const tableStore = transaction.objectStore(this.storeName);
            const stateStore = transaction.objectStore(this.stateStoreName);
            
            tableStore.clear();
            stateStore.clear();
            
            // テーブルを挿入
            if (data.tables && data.tables.length > 0) {
                for (const table of data.tables) {
                    tableStore.put(table);
                }
            }
            
            // インタープリタ状態を挿入
            if (data.interpreterState) {
                stateStore.put(data.interpreterState);
            }
            
            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error);
        });
    },

    async clearAll() {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName, this.stateStoreName], 'readwrite');
            
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
};

// グローバルに公開（即座に実行）
window.AjisaiDB = DB;

// デバッグ用
console.log('AjisaiDB initialized:', window.AjisaiDB);

// IndexedDBの基本テスト関数を追加
DB.test = async function() {
    try {
        console.log('Starting IndexedDB test...');
        
        // データベースを開く
        await this.open();
        console.log('✓ Database opened successfully');
        
        // テストテーブルを保存
        await this.saveTable('test_table', ['id', 'name'], [
            [1, 'Test Record 1'],
            [2, 'Test Record 2']
        ]);
        console.log('✓ Test table saved successfully');
        
        // テストテーブルを読み込み
        const loadedTable = await this.loadTable('test_table');
        console.log('✓ Test table loaded:', loadedTable);
        
        // テーブル一覧を取得
        const tableNames = await this.getAllTableNames();
        console.log('✓ Table names retrieved:', tableNames);
        
        // テストテーブルを削除
        await this.deleteTable('test_table');
        console.log('✓ Test table deleted successfully');
        
        console.log('IndexedDB test completed successfully!');
        return true;
    } catch (error) {
        console.error('IndexedDB test failed:', error);
        return false;
    }
};
