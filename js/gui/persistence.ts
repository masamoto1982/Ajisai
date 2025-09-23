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

    // js/gui/persistence.ts の restoreWordsInDependencyOrder メソッド完全修正版

private async restoreWordsInDependencyOrder(customWords: CustomWord[]): Promise<void> {
    console.log('[DEBUG] Starting word restoration with dependency order');
    
    // まず全てのワードを定義のみで復元（依存関係は後で構築）
    const simpleWords: CustomWord[] = [];
    const complexWords: CustomWord[] = [];
    
    // シンプルなワード（他のカスタムワードに依存しない）と複雑なワードを分離
    for (const word of customWords) {
        if (!word || !word.name || !word.definition) continue;
        
        const dependencies = this.extractDependencies(word.definition);
        const hasCustomDependencies = dependencies.some(dep => this.isCustomWord(customWords, dep));
        
        if (hasCustomDependencies) {
            complexWords.push(word);
        } else {
            simpleWords.push(word);
        }
    }
    
    // まずシンプルなワードを復元
    console.log(`[DEBUG] Restoring ${simpleWords.length} simple words first`);
for (const word of simpleWords) {
    try {
        // definitionがnullでないことを確認
        if (!word.definition) {
            console.error(`[DEBUG] Skipping word ${word.name}: no definition`);
            continue;
        }
        // descriptionがnullでない場合のみ文字列として扱う
        const description: string | undefined = word.description === null ? undefined : word.description;
        await window.ajisaiInterpreter.restore_word(
            word.name, 
            word.definition,  // ここでword.definitionは確実にstring
            description
        );
        console.log(`[DEBUG] Restored simple word: ${word.name}`);
    } catch (error) {
        console.error(`[DEBUG] Failed to restore simple word ${word.name}:`, error);
    }
}
    
    // 次に複雑なワードを依存関係順で復元
    console.log(`[DEBUG] Restoring ${complexWords.length} complex words with dependency order`);
    const remaining = [...complexWords];
    const restored = new Set<string>(simpleWords.map(w => w.name));
    let maxIterations = complexWords.length * 3; // より多くの反復を許可
    
    while (remaining.length > 0 && maxIterations > 0) {
        let progressMade = false;
        
        for (let i = remaining.length - 1; i >= 0; i--) {
            const word = remaining[i];
            if (!word || !word.name || !word.definition) {
                remaining.splice(i, 1);
                progressMade = true;
                continue;
            }
            
            // この単語の依存関係をチェック
            const dependencies = this.extractDependencies(word.definition);
            const canRestore = dependencies.every(dep => 
                restored.has(dep) || !this.isCustomWord(customWords, dep)
            );
            
            if (canRestore) {
    try {
        // definitionがnullでないことを確認
        if (!word.definition) {
            console.error(`[DEBUG] Skipping word ${word.name}: no definition`);
            remaining.splice(i, 1);
            progressMade = true;
            continue;
        }
        // descriptionがnullでない場合のみ文字列として扱う
        const description: string | undefined = word.description === null ? undefined : word.description;
        await window.ajisaiInterpreter.restore_word(
            word.name, 
            word.definition,  // ここでword.definitionは確実にstring
            description
        );
        restored.add(word.name);
        remaining.splice(i, 1);
        progressMade = true;
        console.log(`[DEBUG] Restored complex word: ${word.name}`);
    } catch (error) {
        console.error(`[DEBUG] Failed to restore complex word ${word.name}:`, error);
        remaining.splice(i, 1);
        progressMade = true;
    }
}
        }
        
        if (!progressMade) {
            console.warn('[DEBUG] Cannot resolve remaining word dependencies:', 
                remaining.map(w => w?.name || 'unknown'));
            
            // 残りの単語を強制的に復元
            for (const word of remaining) {
    if (word && word.name && word.definition) {
        try {
            // definitionがnullでない場合のみ処理（上のif文で既にチェック済み）
            // descriptionがnullでない場合のみ文字列として扱う
            const description: string | undefined = word.description === null ? undefined : word.description;
            await window.ajisaiInterpreter.restore_word(
                word.name, 
                word.definition,  // ここでword.definitionは確実にstring
                description
            );
            console.log(`[DEBUG] Force restored word: ${word.name}`);
        } catch (error) {
            console.error(`[DEBUG] Failed to force restore word ${word.name}:`, error);
        }
    }
}
            break;
        }
        
        maxIterations--;
    }
    
    console.log('[DEBUG] Word restoration completed');
    
    // 復元完了後にGUIを更新して依存関係の状態を確認
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
