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
        const remaining = [...customWords];
        const restored = new Set<string>();
        let maxIterations = customWords.length * 2; // 循環依存回避
        
        while (remaining.length > 0 && maxIterations > 0) {
            let progressMade = false;
            
            for (let i = remaining.length - 1; i >= 0; i--) {
                const word = remaining[i];
                if (!word.name || !word.definition) {
                    remaining.splice(i, 1);
                    progressMade = true;
                    continue;
                }
                
                // この単語の依存関係をチェック
                const dependencies = this.extractDependencies(word.definition);
                const canRestore = dependencies.every(dep => restored.has(dep) || !this.isCustomWord(customWords, dep));
                
                if (canRestore) {
                    try {
                        window.ajisaiInterpreter.restore_word(
                            word.name, 
                            word.definition, 
                            word.description
                        );
                        restored.add(word.name);
                        remaining.splice(i, 1);
                        progressMade = true;
                        console.log(`Restored word: ${word.name}`);
                    } catch (error) {
                        console.error(`Failed to restore word ${word.name}:`, error);
                        remaining.splice(i, 1);
                        progressMade = true;
                    }
                }
            }
            
            if (!progressMade) {
                // 循環依存または解決不可能な依存関係
                console.warn('Cannot resolve all word dependencies. Remaining words:', 
                    remaining.map(w => w.name));
                // 残りの単語を強制的に復元を試行
                for (const word of remaining) {
                    try {
                        window.ajisaiInterpreter.restore_word(
                            word.name, 
                            word.definition, 
                            word.description
                        );
                        console.log(`Force restored word: ${word.name}`);
                    } catch (error) {
                        console.error(`Failed to force restore word ${word.name}:`, error);
                    }
                }
                break;
            }
            
            maxIterations--;
        }
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
        return customWords.some(w => w.name === wordName);
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
