// IndexedDB操作のラッパー
const DB = {
    dbName: 'AjisaiDB',
    version: 2,
    stateStoreName: 'interpreter_state',
    db: null,

    // データベースを開く
    async open() {
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
                const db = event.target.result;
                
                // インタープリタの状態用のストア
                if (!db.objectStoreNames.contains(this.stateStoreName)) {
                    console.log(`Creating store: ${this.stateStoreName}`);
                    db.createObjectStore(this.stateStoreName, { keyPath: 'key' });
                }
                
                console.log('IndexedDB stores created successfully');
            };
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

    // 状態を保存
    async saveAllState(tables, interpreterState) {
        if (!this.db) await this.open();
        
        // インタープリタ状態のみを保存
        return this.saveInterpreterState(interpreterState);
    }
};

// グローバルに公開
window.AjisaiDB = DB;

console.log('AjisaiDB initialized:', window.AjisaiDB);
