// js/gui/dictionary.ts

interface WordInfo {
    name: string;
    description?: string | null;
    protected?: boolean;
}

interface DictionaryElements {
    builtinWordsDisplay: HTMLElement;
    customWordsDisplay: HTMLElement;
}

export class Dictionary {
    private elements!: DictionaryElements;
    private onWordClick!: (word: string) => void;

    init(elements: DictionaryElements, onWordClick: (word: string) => void): void {
        this.elements = elements;
        this.onWordClick = onWordClick;
    }

    renderBuiltinWords(): void {
        if (!window.ajisaiInterpreter) return;
        
        try {
            const categorizedWords = window.ajisaiInterpreter.get_builtin_words_by_category();
            this.renderCategorizedWords(this.elements.builtinWordsDisplay, categorizedWords);
        } catch (error) {
            console.error('Failed to render builtin words:', error);
        }
    }

private renderCategorizedWords(container: HTMLElement, categorizedWords: any): void {
    container.innerHTML = '';
    
    for (const [category, words] of Object.entries(categorizedWords)) {
        const categorySection = document.createElement('div');
        categorySection.className = 'word-category';
        
        const wordsContainer = document.createElement('div');
        wordsContainer.style.marginBottom = '1rem';
        
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

    updateCustomWords(customWordsInfo: Array<[string, string | null, boolean]>): void {
        const words: WordInfo[] = (customWordsInfo || []).map(wordData => ({
            name: wordData[0],
            description: wordData[1] || this.decodeWordName(wordData[0]) || wordData[0],
            protected: wordData[2] || false
        }));
        this.renderWordButtons(this.elements.customWordsDisplay, words, true);
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

    private renderWordButtons(container: HTMLElement, words: WordInfo[], isCustom: boolean): void {
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
            
            if (!isCustom) {
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
