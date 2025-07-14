// IndexedDB操作のラッパー
const DB = {
    dbName: 'AjisaiDB',
    version: 1,
    storeName: 'tables',
    db: null,

    // データベースを開く
    async open() {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open(this.dbName, this.version);
            
            request.onerror = () => reject(request.error);
            request.onsuccess = () => {
                this.db = request.result;
                resolve(this.db);
            };
            
            request.onupgradeneeded = (event) => {
                const db = event.target.result;
                if (!db.objectStoreNames.contains(this.storeName)) {
                    db.createObjectStore(this.storeName, { keyPath: 'name' });
                }
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

    // 全データをエクスポート
    async exportAll() {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName], 'readonly');
            const store = transaction.objectStore(this.storeName);
            
            const request = store.getAll();
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
    },

    // データをインポート
    async importAll(data) {
        if (!this.db) await this.open();
        
        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction([this.storeName], 'readwrite');
            const store = transaction.objectStore(this.storeName);
            
            // 既存データをクリア
            store.clear();
            
            // 新しいデータを挿入
            let count = 0;
            for (const table of data) {
                const request = store.put(table);
                request.onsuccess = () => {
                    count++;
                    if (count === data.length) {
                        resolve();
                    }
                };
                request.onerror = () => reject(request.error);
            }
            
            if (data.length === 0) {
                resolve();
            }
        });
    }
};

// グローバルに公開
window.AjisaiDB = DB;
