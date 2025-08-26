// js/gui/editor.ts (シンタックスハイライト対応版)

interface TokenInfo {
    type: string;
    value: string;
    start: number;
    end: number;
}

export class Editor {
    private element!: HTMLTextAreaElement;
    private highlightElement!: HTMLDivElement;
    private container!: HTMLDivElement;

    init(element: HTMLTextAreaElement): void {
        this.element = element;
        this.setupSyntaxHighlighting();
        this.setupEventListeners();
        
        if (this.element.value.trim() === '') {
            this.element.value = '';
        }
        
        // 初回ハイライト
        this.updateSyntaxHighlighting();
    }

    private setupSyntaxHighlighting(): void {
        // エディタコンテナを作成
        this.container = document.createElement('div');
        this.container.className = 'editor-container';
        
        // 元のテキストエリアを包む
        const parent = this.element.parentElement!;
        parent.insertBefore(this.container, this.element);
        this.container.appendChild(this.element);
        
        // ハイライト用のdivを作成
        this.highlightElement = document.createElement('div');
        this.highlightElement.className = 'syntax-highlight';
        this.container.insertBefore(this.highlightElement, this.element);
        
        // テキストエリアを透明にする
        this.element.style.background = 'transparent';
        this.element.style.color = 'transparent';
        this.element.style.caretColor = '#333';
    }

    private setupEventListeners(): void {
        // リアルタイムハイライト更新
        this.element.addEventListener('input', () => {
            this.updateSyntaxHighlighting();
        });
        
        // スクロール同期
        this.element.addEventListener('scroll', () => {
            this.highlightElement.scrollTop = this.element.scrollTop;
            this.highlightElement.scrollLeft = this.element.scrollLeft;
        });
        
        // キーダウンは既存のまま
        this.element.addEventListener('keydown', (e) => this.handleKeyDown(e));
    }

    private updateSyntaxHighlighting(): void {
        if (!window.ajisaiInterpreter) return;
        
        const text = this.element.value;
        if (!text.trim()) {
            this.highlightElement.innerHTML = '';
            return;
        }
        
        try {
            const tokens = window.ajisaiInterpreter.tokenize_with_positions(text) as TokenInfo[];
            this.applyHighlighting(text, tokens);
        } catch (error) {
            console.error('Tokenization error:', error);
            this.highlightElement.textContent = text;
        }
    }

    private applyHighlighting(text: string, tokens: TokenInfo[]): void {
        let highlightedText = '';
        let lastEnd = 0;
        
        // トークンを位置順にソート
        const sortedTokens = tokens.sort((a, b) => a.start - b.start);
        
        for (const token of sortedTokens) {
            // トークン間の非認識テキストを追加
            if (token.start > lastEnd) {
                highlightedText += this.escapeHtml(text.slice(lastEnd, token.start));
            }
            
            // トークンを着色して追加
            const tokenClass = this.getTokenClass(token);
            const tokenText = this.escapeHtml(text.slice(token.start, token.end));
            highlightedText += `<span class="${tokenClass}">${tokenText}</span>`;
            
            lastEnd = token.end;
        }
        
        // 残りのテキストを追加
        if (lastEnd < text.length) {
            highlightedText += this.escapeHtml(text.slice(lastEnd));
        }
        
        this.highlightElement.innerHTML = highlightedText;
    }

    private getTokenClass(token: TokenInfo): string {
        if (token.type === 'symbol') {
            // シンボルの場合、辞書での状態をチェック
            const wordInfo = this.getWordInfo(token.value);
            if (wordInfo.isBuiltin || wordInfo.isProtected) {
                return 'token-builtin';
            } else if (wordInfo.exists) {
                return 'token-custom';
            } else {
                return 'token-unknown';
            }
        }
        
        return `token-${token.type.replace('-', '-')}`;
    }

    private getWordInfo(word: string): { exists: boolean; isBuiltin: boolean; isProtected: boolean } {
        if (!window.ajisaiInterpreter) {
            return { exists: false, isBuiltin: false, isProtected: false };
        }
        
        try {
            // 組み込みワードかチェック
            const builtinWords = window.ajisaiInterpreter.get_builtin_words_info();
            const isBuiltin = Array.isArray(builtinWords) && 
                builtinWords.some((w: any) => Array.isArray(w) && w[0] === word);
            
            if (isBuiltin) {
                return { exists: true, isBuiltin: true, isProtected: true };
            }
            
            // カスタムワードかチェック
            const customWordsInfo = window.ajisaiInterpreter.get_custom_words_info();
            const customWordInfo = Array.isArray(customWordsInfo) && 
                customWordsInfo.find((w: any) => Array.isArray(w) && w[0] === word);
            
            if (customWordInfo) {
                return { 
                    exists: true, 
                    isBuiltin: false, 
                    isProtected: customWordInfo[2] || false 
                };
            }
            
            return { exists: false, isBuiltin: false, isProtected: false };
        } catch (error) {
            console.error('Error getting word info:', error);
            return { exists: false, isBuiltin: false, isProtected: false };
        }
    }

    private escapeHtml(text: string): string {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    handleKeyDown(_event: KeyboardEvent): void {
        // 特別な処理なし - 普通のテキストエディタとして動作
    }

    getValue(): string {
        return this.element.value.trim();
    }

    setValue(value: string): void {
        this.element.value = value;
        this.updateSyntaxHighlighting();
    }

    clear(): void {
        this.element.value = '';
        this.updateSyntaxHighlighting();
        this.element.focus();
    }

    insertWord(word: string): void {
        const start = this.element.selectionStart;
        const end = this.element.selectionEnd;
        const text = this.element.value;
        
        this.element.value = text.substring(0, start) + word + text.substring(end);
        
        const newPos = start + word.length;
        this.element.selectionStart = newPos;
        this.element.selectionEnd = newPos;
        
        this.updateSyntaxHighlighting();
        this.element.focus();
    }

    focus(): void {
        this.element.focus();
    }
}
