// js/gui/persistence.ts

import type { LPLInterpreter, Value } from '../wasm-types';  // AjisaiInterpreter → LPLInterpreter

interface CustomWord {
    name: string;
    description: string | null;
    definition: string | null;
}

interface InterpreterState {
    bookshelf: Value[];  // workspace → bookshelf
    customWords: CustomWord[];
}

declare global {
    interface Window {
        lplInterpreter: LPLInterpreter;  // ajisaiInterpreter → lplInterpreter
    }
}

export class Persistence {
    private gui: any; // GUI型の循環参照を避けるため any を使用

    constructor(gui: any) {
        this.gui = gui;
    }

    async init(): Promise<void> {
        try {
            await window.LPLDB.open();  // AjisaiDB → LPLDB
            console.log('Database initialized successfully for Persistence.');
            this.setupDatabaseListeners();
        } catch (error) {
            console.error('Failed to initialize persistence database:', error);
        }
    }

    private setupDatabaseListeners(): void {
        window.addEventListener('lpl-amnesia', async () => {  // ajisai-amnesia → lpl-amnesia
            console.log('AMNESIA command caught.');
            this.gui.display.showInfo('Clearing all database...');
            try {
                await window.LPLDB.clearAll();  // AjisaiDB → LPLDB
                window.lplInterpreter.reset();  // ajisaiInterpreter → lplInterpreter
                this.gui.updateAllDisplays();
                this.gui.display.showInfo('All memory has been cleared.', true);
            } catch(error) {
                this.gui.display.showError(error as Error);
            }
        });
    }

    async saveCurrentState(): Promise<void> {
        if (!window.lplInterpreter) return;  // ajisaiInterpreter → lplInterpreter
        
        try {
            const customWordsInfo = window.lplInterpreter.get_custom_words_info();  // ajisai → lpl
            const customWords: CustomWord[] = customWordsInfo.map(wordData => ({
                name: wordData[0],
                description: wordData[1],
                definition: window.lplInterpreter.get_word_definition ? 
                    window.lplInterpreter.get_word_definition(wordData[0]) : null
            }));

            const interpreterState: InterpreterState = {
                bookshelf: window.lplInterpreter.get_bookshelf(),  // workspace → bookshelf, ajisai → lpl
                customWords: customWords,
            };

            await window.LPLDB.saveInterpreterState(interpreterState);  // AjisaiDB → LPLDB
            console.log('State saved automatically.');
        } catch (error) {
            console.error('Failed to auto-save state:', error);
        }
    }

    async loadDatabaseData(isCommand = false): Promise<void> {
        if (!window.lplInterpreter) return;  // ajisaiInterpreter → lplInterpreter
        
        try {
            if (isCommand) return;

            const state = await window.LPLDB.loadInterpreterState();  // AjisaiDB → LPLDB
            if (state) {
                if (state.bookshelf) window.lplInterpreter.restore_bookshelf(state.bookshelf);  // workspace → bookshelf, ajisai → lpl
                if (state.customWords) {
                    for (const word of state.customWords) {
                        if (word.name && word.definition) {
                            try {
                                window.lplInterpreter.restore_word(  // ajisai → lpl
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
