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
    private gui: any;

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
        window.addEventListener('ajisai-reset', async () => {
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
                
                if (state.customWords && state.customWords.length > 0) {
                    // 依存関係を考慮した順序で復元
                    await this.restoreWordsInDependencyOrder(state.customWords);
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

    private async restoreWordsInDependencyOrder(customWords: CustomWord[]): Promise<void> {
        console.log('[DEBUG] Starting word restoration with dependency order');
        
        // 全てのワードを一度に復元
        for (const word of customWords) {
            if (!word || !word.name || !word.definition) {
                console.error(`[DEBUG] Skipping invalid word:`, word);
                continue;
            }
            
            try {
                const description: string | undefined = word.description === null ? undefined : word.description;
                await window.ajisaiInterpreter.restore_word(
                    word.name, 
                    word.definition,
                    description
                );
                console.log(`[DEBUG] Restored word: ${word.name}`);
            } catch (error) {
                console.error(`[DEBUG] Failed to restore word ${word.name}:`, error);
            }
        }
        
        // 復元完了後に依存関係を再構築
        console.log('[DEBUG] Rebuilding dependencies...');
        const result = window.ajisaiInterpreter.rebuild_dependencies();
        if (result.status === 'OK') {
            console.log('[DEBUG] Dependencies rebuilt successfully');
        } else {
            console.error('[DEBUG] Failed to rebuild dependencies:', result.message);
        }
        
        console.log('[DEBUG] Word restoration completed');
        
        // 復元完了後にGUIを更新
        setTimeout(() => {
            if (this.gui) {
                this.gui.updateAllDisplays();
            }
        }, 100);
    }
    
    private extractDependencies(definition: string): string[] {
        const dependencies: string[] = [];
        // 簡単なワード名抽出（実際のトークナイザと同等の処理が必要）
        const words = definition.match(/[A-Z_][A-Z0-9_]*/g) || [];
        for (const word of words) {
            if (!this.isBuiltinWord(word)) {
                dependencies.push(word);
            }
        }
        return [...new Set(dependencies)]; // 重複除去
    }
    
    private isCustomWord(customWords: CustomWord[], wordName: string): boolean {
        return customWords.some(w => w && w.name === wordName);
    }
    
    private isBuiltinWord(wordName: string): boolean {
        const builtins = [
            'GET', 'INSERT', 'REPLACE', 'REMOVE', 'LENGTH', 'TAKE', 'DROP', 'SPLIT',
            'DUP', 'SWAP', 'ROT', 'CONCAT', 'REVERSE',
            '+', '-', '*', '/', '=', '<', '<=', '>', '>=', 'AND', 'OR', 'NOT',
            'PRINT', 'DEF', 'DEL', 'RESET', 'GOTO'
        ];
        return builtins.includes(wordName);
    }
}
