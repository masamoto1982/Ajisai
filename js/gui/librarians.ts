// js/gui/librarians.ts
interface LibrarianInfo {
    name: string;
    description?: string | null;
    protected?: boolean;
}

interface LibrariansElements {
    permanentLibrariansDisplay: HTMLElement;
    temporaryLibrariansDisplay: HTMLElement;
}

type LanguageMode = 'japanese' | 'english';

export class Librarians {
    private elements!: LibrariansElements;
    private onWordClick!: (word: string) => void;
    private languageMode: LanguageMode = 'japanese'; // デフォルトは日本語

    init(elements: LibrariansElements, onWordClick: (word: string) => void): void {
        this.elements = elements;
        this.onWordClick = onWordClick;
    }

    setLanguageMode(mode: LanguageMode): void {
        this.languageMode = mode;
        this.renderPermanentLibrarians(); // 再描画
    }

    getLanguageMode(): LanguageMode {
        return this.languageMode;
    }

    renderPermanentLibrarians(): void {
        if (!window.lplInterpreter) return;
        
        try {
            const categorizedWords = window.lplInterpreter.get_builtin_words_by_category();
            this.renderCategorizedWords(this.elements.permanentLibrariansDisplay, categorizedWords);
        } catch (error) {
            console.error('Failed to render permanent librarians:', error);
        }
    }

    private getCategoryOrder(): string[] {
        return [
            'Basic',      // 基礎演算
            'Compare',    // 比較演算  
            'Logic',      // 論理演算
            'BookOps',    // 書籍操作
            'Management'  // 司書管理
        ];
    }

    private renderCategorizedWords(container: HTMLElement, categorizedWords: any): void {
        container.innerHTML = '';
        
        const categoryOrder = this.getCategoryOrder();
        
        for (const categoryKey of categoryOrder) {
            const words = categorizedWords[categoryKey];
            if (!words || words.length === 0) continue;
            
            // グループ名は表示しない - 直接ボタンを配置
            const wordsContainer = document.createElement('div');
            wordsContainer.style.marginBottom = '0.75rem';
            
            (words as any[]).forEach(wordData => {
                const button = document.createElement('button');
                
                // 言語モードに応じて表示名を決定
                const displayName = this.getDisplayName(wordData[0]);
                button.textContent = displayName;
                button.className = 'word-button builtin';
                button.title = wordData[1] || displayName;
                
                button.addEventListener('click', () => {
                    if (this.onWordClick) {
                        // クリック時は実際のワード名（辞書のキー）を渡す
                        this.onWordClick(wordData[0]);
                    }
                });
                
                wordsContainer.appendChild(button);
            });
            
            container.appendChild(wordsContainer);
        }
    }

    private getDisplayName(originalName: string): string {
    if (this.languageMode === 'english') {
        // 英語モード：日本語名を英語名に変換
        const englishMapping: Record<string, string> = {
            '頁': 'PAGE', '頁数': 'LENGTH', '冊': 'BOOK', // 「冊」追加
            '挿入': 'INSERT', '置換': 'REPLACE', '削除': 'DELETE',
            '合併': 'MERGE', '分離': 'SPLIT', '待機': 'WAIT', '複製': 'DUP',
            // '破棄': 'DROP', // 削除
            '雇用': 'HIRE', '解雇': 'FIRE', '交代': 'HANDOVER'
        };
        
        return englishMapping[originalName] || originalName;
    } else {
        // 日本語モード：そのまま表示
        return originalName;
    }
}

    updateTemporaryLibrarians(customWordsInfo: Array<[string, string | null, boolean]>): void {
        const words: LibrarianInfo[] = (customWordsInfo || []).map(wordData => ({
            name: wordData[0],
            description: wordData[1] || this.decodeWordName(wordData[0]) || wordData[0],
            protected: wordData[2] || false
        }));
        this.renderWordButtons(this.elements.temporaryLibrariansDisplay, words, true);
    }

    private decodeWordName(name: string): string | null {
        if (name.match(/^W_[0-9A-F]+$/)) {
            return null;
        }
        
        if (name.includes('_')) {
            const parts = name.split('_');
            const decoded = parts.map(part => {
                if (part === 'VSTART') return '[';
                if (part === 'VEND') return ']';
                if (part === 'BSTART') return '{';
                if (part === 'BEND') return '}';
                if (part === 'NIL') return 'nil';
                if (part.startsWith('STR_')) return `"${part.substring(4).replace(/_/g, ' ')}"`;
                if (part === 'ADD') return '+';
                if (part === 'SUB') return '-';
                if (part === 'MUL') return '*';
                if (part === 'DIV') return '/';
                if (part === 'GT') return '>';
                if (part === 'GE') return '>=';
                if (part === 'EQ') return '=';
                if (part === 'LT') return '<';
                if (part === 'LE') return '<=';
                if (part === 'AND') return 'and';
                if (part === 'OR') return 'or';
                if (part === 'NOT') return 'not';
                return part.toLowerCase();
            }).join(' ');
            return `≈ ${decoded}`;
        }
        return null;
    }

    private renderWordButtons(container: HTMLElement, words: LibrarianInfo[], isTemporary: boolean): void {
        container.innerHTML = '';

        words.forEach(wordInfo => {
            const button = document.createElement('button');
            button.textContent = wordInfo.name;
            button.className = 'word-button';
            
            if (wordInfo.description) {
                button.title = wordInfo.description;
            } else {
                button.title = wordInfo.name;
            }
            
            if (!isTemporary) {
                button.classList.add('builtin');
            } else if (wordInfo.protected) {
                button.classList.add('protected');
            } else {
                button.classList.add('deletable');
            }
            
            button.addEventListener('click', () => {
                if (this.onWordClick) {
                    this.onWordClick(wordInfo.name);
                }
            });
            
            container.appendChild(button);
        });
    }
}
