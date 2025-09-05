// js/gui/dictionary.ts (純粋Vector操作言語版)

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
            const builtinWords = window.ajisaiInterpreter.get_builtin_words_info();
            this.renderBuiltinWordsWithGroups(this.elements.builtinWordsDisplay, builtinWords);
        } catch (error) {
            console.error('Failed to render builtin words:', error);
        }
    }

    private renderBuiltinWordsWithGroups(container: HTMLElement, builtinWords: any[]): void {
        container.innerHTML = '';
        
        // 純粋Vector操作言語版のグループ分け
        const positionWords = ['GET', 'INSERT', 'REPLACE', 'REMOVE'];
        const quantityWords = ['LENGTH', 'TAKE', 'DROP', 'REPEAT', 'SPLIT'];
        const workspaceWords = ['DUP', 'SWAP', 'ROT'];
        const vectorWords = ['CONCAT', 'REVERSE'];
        const arithmeticWords = ['+', '-', '*', '/'];
        const comparisonWords = ['=', '<', '<=', '>', '>='];
        const logicWords = ['AND', 'OR', 'NOT'];
        const ioWords = ['PRINT'];
        const systemWords = ['DEF', 'DEL', 'RESET'];
        
        const groups = [
            positionWords, quantityWords, workspaceWords, vectorWords,
            arithmeticWords, comparisonWords, logicWords, ioWords, systemWords
        ];
        
        groups.forEach((group, groupIndex) => {
            group.forEach(wordName => {
                const wordData = builtinWords.find((item: any[]) => item[0] === wordName);
                if (wordData) {
                    const button = document.createElement('button');
                    button.textContent = wordData[0];
                    button.className = 'word-button builtin';
                    button.title = wordData[1] || wordData[0];
                    
                    button.addEventListener('click', () => {
                        if (this.onWordClick) {
                            this.onWordClick(wordData[0]);
                        }
                    });
                    
                    container.appendChild(button);
                }
            });
            
            if (groupIndex < groups.length - 1) {
                const lineBreak = document.createElement('br');
                container.appendChild(lineBreak);
            }
        });
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
                if (part === 'LT') return '<';
                if (part === 'LE') return '<=';
                if (part === 'GT') return '>';
                if (part === 'GE') return '>=';
                if (part === 'EQ') return '=';
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
