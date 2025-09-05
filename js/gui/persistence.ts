// js/gui/persistence.ts

import type { AjisaiInterpreter, Value } from '../wasm-types';

interface CustomWord {
    name: string;
    description: string | null;
    definition: string | null;
}

interface InterpreterState {
    workspace: Value[];
    customWords: CustomWord[];
}

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

export class Persistence {
    private gui: any; // GUI型の循環参照を避けるため any を使用

    constructor(gui: any) {
        this.gui = gui;
    }

    async init(): Promise<void> {
        try {
            await window.AjisaiDB.open();
            console.log('Database initialized successfully for Persistence.');
            this.setupDatabaseListeners();
        } catch (error) {
            console.error('Failed to initialize persistence database:', error);
        }
    }

    private setupDatabaseListeners(): void {
        window.addEventListener('ajisai-reset', async () => {  // AMNESIA → RESET
    console.log('RESET command caught.');
    this.gui.display.showInfo('Clearing all database...');
    try {
        await window.AjisaiDB.clearAll();
        this.gui.updateAllDisplays();
        this.gui.display.showInfo('All memory has been cleared.', true);
    } catch(error) {
        this.gui.display.showError(error as Error);
    }
});
    }

    async saveCurrentState(): Promise<void> {
        if (!window.ajisaiInterpreter) return;
        
        try {
            const customWordsInfo = window.ajisaiInterpreter.get_custom_words_info();
            const customWords: CustomWord[] = customWordsInfo.map(wordData => ({
                name: wordData[0],
                description: wordData[1],
                definition: window.ajisaiInterpreter.get_word_definition ? 
                    window.ajisaiInterpreter.get_word_definition(wordData[0]) : null
            }));

            const interpreterState: InterpreterState = {
                workspace: window.ajisaiInterpreter.get_workspace(),
                customWords: customWords,
            };

            await window.AjisaiDB.saveInterpreterState(interpreterState);
            console.log('State saved automatically.');
        } catch (error) {
            console.error('Failed to auto-save state:', error);
        }
    }

    async loadDatabaseData(isCommand = false): Promise<void> {
        if (!window.ajisaiInterpreter) return;
        
        try {
            if (isCommand) return;

            const state = await window.AjisaiDB.loadInterpreterState();
            if (state) {
                if (state.workspace) window.ajisaiInterpreter.restore_workspace(state.workspace);
                if (state.customWords) {
                    for (const word of state.customWords) {
                        if (word.name && word.definition) {
                            try {
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
                this.gui.display.showError(error as Error);
            }
        }
    }
}
