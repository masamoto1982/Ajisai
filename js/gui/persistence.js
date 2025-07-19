export class Persistence {
    constructor() {
        this.autoSaveEnabled = true;
        this.db = null;
    }

    async init() {
        try {
            await this.initDatabase();
            this.setupDatabaseListeners();
            
            // WASMロード後に自動読み込み
            if (window.ajisaiInterpreter) {
                await this.loadDatabaseData();
            } else {
                window.addEventListener('wasmLoaded', async () => {
                    await this.loadDatabaseData();
                });
            }
        } catch (error) {
            console.error('Failed to initialize persistence:', error);
        }
    }

    async initDatabase() {
        if (!window.AjisaiDB) {
            throw new Error('AjisaiDB is not defined');
        }
        
        await window.AjisaiDB.open();
        console.log('Database initialized successfully');
    }

    setupDatabaseListeners() {
        // SAVE-DBワード実行時
        window.addEventListener('ajisai-save-db', async () => {
            await this.saveToDatabase();
        });
        
        // LOAD-DBワード実行時
        window.addEventListener('ajisai-load-db', async () => {
            await this.loadFromDatabase();
        });
    }

    async saveCurrentState() {
        if (!this.autoSaveEnabled || !window.ajisaiInterpreter) return;
        
        try {
            const state = {
                stack: window.ajisaiInterpreter.get_stack(),
                register: window.ajisaiInterpreter.get_register(),
                tables: {},
                customWords: []
            };
            
            // テーブルを収集
            const tableNames = window.ajisaiInterpreter.get_all_tables();
            for (const name of tableNames) {
                const tableData = window.ajisaiInterpreter.load_table(name);
                if (tableData) {
                    state.tables[name] = {
                        schema: tableData[0],
                        records: tableData[1]
                    };
                }
            }
            
            // カスタムワードを収集
            const customWordsInfo = window.ajisaiInterpreter.get_custom_words_info();
            for (const wordData of customWordsInfo) {
                if (Array.isArray(wordData)) {
                    const name = wordData[0];
                    const description = wordData[1] || null;
                    const definition = window.ajisaiInterpreter.get_word_definition(name);
                    state.customWords.push({ name, definition, description });
                }
            }
            
            await window.AjisaiDB.saveAllState(state.tables, {
                stack: state.stack,
                register: state.register,
                customWords: state.customWords
            });
            
            window.dispatchEvent(new CustomEvent('persistence-complete', {
                detail: { message: 'State saved automatically' }
            }));
        } catch (error) {
            console.error('Failed to save state:', error);
        }
    }

    async loadDatabaseData() {
        try {
            // テーブルを読み込み
            const tableNames = await window.AjisaiDB.getAllTableNames();
            for (const tableName of tableNames) {
                const tableData = await window.AjisaiDB.loadTable(tableName);
                if (tableData) {
                    window.ajisaiInterpreter.save_table(
                        tableName,
                        tableData.schema,
                        tableData.records
                    );
                }
            }
            
            // インタープリタの状態を読み込み
            const state = await window.AjisaiDB.loadInterpreterState();
            if (state) {
                if (state.stack && state.stack.length > 0) {
                    window.ajisaiInterpreter.restore_stack(state.stack);
                }
                
                if (state.register !== null && state.register !== undefined) {
                    window.ajisaiInterpreter.restore_register(state.register);
                }
                
                if (state.customWords && state.customWords.length > 0) {
                    for (const wordInfo of state.customWords) {
                        const name = wordInfo.name || wordInfo[0];
                        const definition = wordInfo.definition;
                        const description = wordInfo.description || wordInfo[1] || null;
                        
                        if (definition) {
                            window.ajisaiInterpreter.restore_word(name, definition, description);
                        }
                    }
                }
            }
            
            window.dispatchEvent(new CustomEvent('persistence-complete', {
                detail: { message: 'Database loaded successfully' }
            }));
        } catch (error) {
            console.error('Failed to load database:', error);
        }
    }

    async saveToDatabase() {
        // SAVE-DBコマンド用
        await this.saveCurrentState();
    }

    async loadFromDatabase() {
        // LOAD-DBコマンド用
        await this.loadDatabaseData();
    }
}
