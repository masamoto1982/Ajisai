// js/gui/persistence.js

export class Persistence {
    constructor(gui) {
        this.gui = gui;
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
        window.addEventListener('ajisai-amnesia', async () => {
            console.log('AMNESIA command caught.');
            this.gui.display.showInfo('Clearing all database...');
            try {
                // IndexedDBを完全にクリア
                await window.AjisaiDB.clearAll();
                // インタープリタをリセット
                window.ajisaiInterpreter.reset();
                this.gui.updateAllDisplays();
                this.gui.display.showInfo('All memory has been cleared.', true);
            } catch(error) {
                this.gui.display.showError(error);
            }
        });
    }

    async saveCurrentState() {
        if (!window.ajisaiInterpreter) return;
        
        try {
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

            await window.AjisaiDB.saveInterpreterState(interpreterState);
            console.log('State saved automatically.');
        } catch (error) {
            console.error('Failed to auto-save state:', error);
        }
    }

    async loadDatabaseData(isCommand = false) {
    if (!window.ajisaiInterpreter) return;
    
    try {
        if (isCommand) return;

        const state = await window.AjisaiDB.loadInterpreterState();
        if (state) {
            if (state.stack) window.ajisaiInterpreter.restore_stack(state.stack);
            if (state.register) window.ajisaiInterpreter.restore_register(state.register);
            if (state.customWords) {
                for (const word of state.customWords) {
                    if (word.name && word.definition) {
                        try {
                            // 直接復元（DEFを使わない）
                            window.ajisaiInterpreter.restore_word(
                                word.name, 
                                word.definition, 
                                word.description
                            );
                        } catch (error) {
                            console.error(`Failed to restore word ${word.name}:`, error);
                        }
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
