// js/gui/persistence.js

export class Persistence {
    constructor(gui) {
        this.gui = gui; // GUIのメインインスタンスへの参照
    }

    async init() {
        try {
            await window.AjisaiDB.open();
            console.log('Database initialized successfully for Persistence.');
            this.setupDatabaseListeners();
        } catch (error) {
            console.error('Failed to initialize persistence database:', error);
        }
    }

    setupDatabaseListeners() {
        // SAVE-DB/LOAD-DBコマンドは削除されました
        // 自動保存機能のみが動作します
    }

    async saveCurrentState() {
        if (!window.ajisaiInterpreter) return;
        
        try {
            const tables = {}; // テーブル機能は無効化

            const customWordsInfo = window.ajisaiInterpreter.get_custom_words_info();
            const customWords = customWordsInfo.map(wordData => ({
                name: wordData[0],
                description: wordData[1],
                definition: window.ajisaiInterpreter.get_word_definition ? 
                    window.ajisaiInterpreter.get_word_definition(wordData[0]) : null
            }));

            const interpreterState = {
                stack: window.ajisaiInterpreter.get_stack(),
                register: window.ajisaiInterpreter.get_register(),
                customWords: customWords,
            };

            await window.AjisaiDB.saveAllState(tables, interpreterState);
            console.log('State saved automatically.');
        } catch (error) {
            console.error('Failed to auto-save state:', error);
        }
    }

    async loadDatabaseData() {
        if (!window.ajisaiInterpreter) return;
        
        try {
            console.log('Loading saved interpreter state...');

            const state = await window.AjisaiDB.loadInterpreterState();
            if (state) {
                if (state.stack) window.ajisaiInterpreter.restore_stack(state.stack);
                if (state.register) window.ajisaiInterpreter.restore_register(state.register);
                if (state.customWords) {
                    for (const word of state.customWords) {
                        if (word.name && word.definition) {
                            window.ajisaiInterpreter.restore_word(word.name, word.definition, word.description);
                        }
                    }
                }
                console.log('Interpreter state restored.');
            }
        } catch (error) {
            console.error('Failed to load database data:', error);
            if (this.gui) {
                this.gui.display.showError(error);
            }
        }
    }
}
