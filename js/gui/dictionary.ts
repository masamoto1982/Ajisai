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
    private gui: any;

    init(elements: DictionaryElements, onWordClick: (word: string) => void, gui?: any): void {
        this.elements = elements;
        this.onWordClick = onWordClick;
        this.gui = gui;
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
        
        const groups: { [key: string]: any[] } = {};
        
        builtinWords.forEach((wordData: any[]) => {
            const category = wordData[2] || 'Other';
            const group = groups[category];
            if (group) {
                group.push(wordData);
            } else {
                groups[category] = [wordData];
            }
        });

        Object.keys(groups).sort().forEach((category) => {
            const groupContainer = document.createElement('div');
            
            const categoryWords = groups[category];
            if (categoryWords) {
                categoryWords.forEach((wordData: any[]) => {
                    const button = document.createElement('button');
                    button.textContent = wordData[0];
                    button.className = 'word-button builtin';
                    button.title = wordData[1] || wordData[0];
                    
                    button.addEventListener('click', () => {
                        if (this.onWordClick) {
                            this.onWordClick(wordData[0]);
                        }
                    });
                    groupContainer.appendChild(button);
                });
            }
            
            container.appendChild(groupContainer);
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
            
            let titleText = `Name: ${wordInfo.name}`;
            if (wordInfo.description) {
                titleText += `\n\nDescription:\n${wordInfo.description}`;
            }
            button.title = titleText;
            
            if (!isCustom) {
                button.classList.add('builtin');
            } else {
                if (wordInfo.protected) {
                    button.classList.add('dependency');
                } else {
                    button.classList.add('non-dependency');
                }
                
                button.addEventListener('contextmenu', (e) => {
                    e.preventDefault();
                    this.confirmAndDeleteWord(wordInfo.name);
                });
            }
            
            button.addEventListener('click', () => {
                if (this.onWordClick) {
                    this.onWordClick(wordInfo.name);
                }
            });
            
            container.appendChild(button);
        });
    }

    private confirmAndDeleteWord(wordName: string): void {
        if (confirm(`Delete word '${wordName}'?`)) {
            try {
                const result = window.ajisaiInterpreter.execute(`'${wordName}' DEL`);
                if (result.status === 'ERROR') {
                    alert(`Failed to delete word: ${result.message}`);
                } else {
                    if (this.gui) {
                        this.gui.updateAllDisplays();
                        this.gui.persistence.saveCurrentState();
                        this.gui.display.showInfo(`Word '${wordName}' deleted.`, true);
                    }
                }
            } catch (error) {
                alert(`Error deleting word: ${error}`);
            }
        }
    }
}
