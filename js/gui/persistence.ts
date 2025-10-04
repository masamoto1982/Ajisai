// js/gui/persistence.ts

import type { AjisaiInterpreter, Value, CustomWord } from '../wasm-types';
import type DB from '../db';

interface InterpreterState {
    stack: Value[];
    customWords: CustomWord[];
}

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
        AjisaiDB: typeof DB;
    }
}

export class Persistence {
    private gui: any;

    constructor(gui: any) {
        this.gui = gui;
    }

    async init(): Promise<void> {
        try {
            await window.AjisaiDB.open();
            console.log('Database initialized successfully for Persistence.');
        } catch (error) {
            console.error('Failed to initialize persistence database:', error);
        }
    }

    async saveCurrentState(): Promise<void> {
        if (!window.ajisaiInterpreter) return;
        
        try {
            const customWordsInfo = window.ajisaiInterpreter.get_custom_words_info();
            const customWords: CustomWord[] = customWordsInfo.map(wordData => ({
                name: wordData[0],
                description: wordData[1],
                definition: window.ajisaiInterpreter.get_word_definition(wordData[0])
            }));

            const interpreterState: InterpreterState = {
                stack: window.ajisaiInterpreter.get_stack(),
                customWords: customWords,
            };

            await window.AjisaiDB.saveInterpreterState(interpreterState);
            console.log('State saved automatically.');
        } catch (error) {
            console.error('Failed to auto-save state:', error);
        }
    }

    async loadDatabaseData(): Promise<void> {
        if (!window.ajisaiInterpreter) return;
        
        try {
            const state = await window.AjisaiDB.loadInterpreterState();
            if (state) {
                if (state.stack) {
                    window.ajisaiInterpreter.restore_stack(state.stack);
                }
                
                if (state.customWords && state.customWords.length > 0) {
                    await window.ajisaiInterpreter.restore_custom_words(state.customWords);
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

    exportCustomWords(): void {
        if (!window.ajisaiInterpreter) {
            this.gui.display.showError('Interpreter not available');
            return;
        }

        const customWordsInfo = window.ajisaiInterpreter.get_custom_words_info();
        const exportData: CustomWord[] = customWordsInfo.map(wordData => {
            const name = wordData[0];
            const description = wordData[1];
            const definition = window.ajisaiInterpreter.get_word_definition(name);
            return { name, definition, description };
        });

        const jsonString = JSON.stringify(exportData, null, 2);
        const blob = new Blob([jsonString], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const filename = `ajisai_words_${timestamp}.json`;

        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        
        this.gui.display.showInfo(`Custom words exported as ${filename}.`, true);
    }

    importCustomWords(): void {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = '.json';

        input.onchange = async (e) => {
            const file = (e.target as HTMLInputElement).files?.[0];
            if (!file) return;

            const reader = new FileReader();
            reader.onload = async (event) => {
                try {
                    const jsonString = event.target?.result as string;
                    const importedWords = JSON.parse(jsonString) as CustomWord[];

                    if (!Array.isArray(importedWords)) {
                        throw new Error('Invalid file format. Expected an array of words.');
                    }
                    
                    await window.ajisaiInterpreter.restore_custom_words(importedWords);
                    
                    this.gui.updateAllDisplays();
                    await this.saveCurrentState();
                    this.gui.display.showInfo(`${importedWords.length} custom words imported and saved.`, true);

                } catch (error) {
                    this.gui.display.showError(error as Error);
                }
            };
            reader.readAsText(file);
        };

        input.click();
    }
}
