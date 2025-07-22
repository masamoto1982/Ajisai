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
        window.addEventListener('ajisai-save-db', async () => {
            console.log('SAVE-DB command caught.');
            this.gui.display.showInfo('Saving database via SAVE-DB command...');
            try {
                // 現在はテーブル機能無効化のため、空のテーブルで保存
                await window.AjisaiDB.saveAllState({}, {});
                this.gui.display.showInfo('Database saved via SAVE-DB.', true);
            } catch(error) {
                this.gui.display.showError(error);
            }
        });
        
        window.addEventListener('ajisai-load-db', async () => {
            console.log('LOAD-DB command caught.');
            this.gui.display.showInfo('Loading database via LOAD-DB command...');
            await this.loadDatabaseData(true); // isCommand = true
            this.gui.updateAllDisplays();
            this.gui.display.showInfo('Database loaded via LOAD-DB (tables disabled).', true);
        });
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

    async loadDatabaseData(isCommand = false) {
        if (!window.ajisaiInterpreter) return;
        
        try {
            console.log('Table loading disabled.');

            // LOAD-DBコマンドの時はスタックやレジスタは復元しない
            if (isCommand) return;

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
