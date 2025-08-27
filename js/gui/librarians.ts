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

export class Librarians {
    private elements!: LibrariansElements;
    private onWordClick!: (word: string) => void;

    init(elements: LibrariansElements, onWordClick: (word: string) => void): void {
        this.elements = elements;
        this.onWordClick = onWordClick;
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

    private renderCategorizedWords(container: HTMLElement, categorizedWords: any): void {
        container.innerHTML = '';
        
        for (const [_, words] of Object.entries(categorizedWords)) {
            const categorySection = document.createElement('div');
            categorySection.className = 'word-category';
            
            const wordsContainer = document.createElement('div');
            
            (words as any[]).forEach(wordData => {
                const button = document.createElement('button');
                button.textContent = wordData[0];
                button.className = 'word-button builtin';
                button.title = wordData[1] || wordData[0];
                
                button.addEventListener('click', () => {
                    if (this.onWordClick) {
                        this.onWordClick(wordData[0]);
                    }
                });
                
                wordsContainer.appendChild(button);
            });
            
            categorySection.appendChild(wordsContainer);
            container.appendChild(categorySection);
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
        // W_で始まるタイムスタンプ形式の自動生成名は処理しない
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
                // 演算子の復号化
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
            
            // ホバー時のタイトル設定
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
