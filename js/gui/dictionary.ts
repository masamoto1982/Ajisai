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
    private builtinWords: WordInfo[] = [
        { name: '+', description: '加算 ( a b -- a+b )' },
        { name: '-', description: '減算 ( a b -- a-b )' },
        { name: '*', description: '乗算 ( a b -- a*b )' },
        { name: '/', description: '除算 ( a b -- a/b )' },
        { name: '=', description: '等しい ( a b -- bool )' },
        { name: '>', description: 'より大きい ( a b -- bool )' },
        { name: '>=', description: '以上 ( a b -- bool )' },
        { name: '<', description: 'より小さい ( a b -- bool )' },
        { name: '<=', description: '以下 ( a b -- bool )' },
        { name: 'NOT', description: '論理否定 ( bool -- bool )' },
        { name: 'AND', description: '論理積 ( bool bool -- bool )' },
        { name: 'OR', description: '論理和 ( bool bool -- bool )' },
        { name: 'DUP', description: 'スタックトップを複製 ( a -- a a )' },
        { name: 'DROP', description: 'スタックトップを削除 ( a -- )' },
        { name: 'SWAP', description: '上位2つを交換 ( a b -- b a )' },
        { name: 'OVER', description: '2番目をコピー ( a b -- a b a )' },
        { name: 'ROT', description: '3番目を最上位へ ( a b c -- b c a )' },
        { name: 'NIP', description: '2番目を削除 ( a b -- b )' },
        { name: '>R', description: 'スタックからレジスタへ移動 ( a -- )' },
        { name: 'R>', description: 'レジスタからスタックへ移動 ( -- a )' },
        { name: 'R@', description: 'レジスタの値をコピー ( -- a )' },
        { name: 'R+', description: 'レジスタとの加算 ( a -- a+r )' },
        { name: 'R-', description: 'レジスタとの減算 ( a -- a-r )' },
        { name: 'R*', description: 'レジスタとの乗算 ( a -- a*r )' },
        { name: 'R/', description: 'レジスタとの除算 ( a -- a/r )' },
        { name: 'LENGTH', description: 'ベクトルの長さ ( vec -- n )' },
        { name: 'HEAD', description: '最初の要素 ( vec -- elem )' },
        { name: 'TAIL', description: '最初以外の要素 ( vec -- vec\' )' },
        { name: 'CONS', description: '要素を先頭に追加 ( elem vec -- vec\' )' },
        { name: 'APPEND', description: '要素を末尾に追加 ( vec elem -- vec\' )' },
        { name: 'REVERSE', description: 'ベクトルを逆順に ( vec -- vec\' )' },
        { name: 'NTH', description: 'N番目の要素を取得 ( n vec -- elem )' },
        { name: 'UNCONS', description: 'ベクトルを分解 ( vec -- elem vec\' )' },
        { name: 'EMPTY?', description: 'ベクトルが空か ( vec -- bool )' },
        { name: 'IF', description: '条件分岐 ( bool quot quot -- ... )' },
        { name: 'CALL', description: 'Quotationを実行 ( quot -- ... )' },
        { name: 'DEF', description: 'カスタムワードを定義 ( quot str -- )' },
        { name: 'AMNESIA', description: 'IndexedDBを初期化 ( -- )' },
        { name: 'DEL', description: 'カスタムワードを削除 ( str -- )' },
        { name: 'NIL?', description: 'nilかどうか ( a -- bool )' },
        { name: 'NOT-NIL?', description: 'nilでないか ( a -- bool )' },
        { name: 'KNOWN?', description: 'nil以外の値か ( a -- bool )' },
        { name: 'DEFAULT', description: 'nilならデフォルト値 ( a b -- a|b )' },
        { name: '.', description: '値を出力 ( a -- )' },
        { name: 'PRINT', description: '値を出力 ( a -- a )' },
        { name: 'CR', description: '改行を出力 ( -- )' },
        { name: 'SPACE', description: 'スペースを出力 ( -- )' },
        { name: 'SPACES', description: 'N個のスペース ( n -- )' },
        { name: 'EMIT', description: '文字を出力 ( n -- )' }
    ];

    init(elements: DictionaryElements, onWordClick: (word: string) => void): void {
        this.elements = elements;
        this.onWordClick = onWordClick;
    }

    renderBuiltinWords(): void {
        this.renderWordButtons(this.elements.builtinWordsDisplay, this.builtinWords, false);
    }

    // js/gui/dictionary.ts の updateCustomWords メソッドを修正

updateCustomWords(customWordsInfo: Array<[string, string | null, boolean]>): void {
    const words: WordInfo[] = (customWordsInfo || []).map(wordData => ({
        name: wordData[0],
        description: wordData[1] || this.decodeWordName(wordData[0]) || wordData[0],  // 優先順位を変更
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
            button.title = wordInfo.name;  // フォールバック
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
