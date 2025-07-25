// js/gui/dictionary.js

export class Dictionary {
    init(elements, onWordClick) {
        this.elements = elements;
        this.onWordClick = onWordClick;
        // データを元のフラットな配列構造に戻す
        this.builtinWords = [
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
            { name: 'NTH', description: 'N番目の要素を取得（負数は末尾から） ( n vec -- elem )' },
            { name: 'UNCONS', description: 'ベクトルを先頭要素と残りに分解 ( vec -- elem vec\' )' },
            { name: 'EMPTY?', description: 'ベクトルが空かチェック ( vec -- bool )' },
            { name: 'DEF', description: '新しいワードを定義 ( ... str -- )' },
            { name: 'DEL', description: 'カスタムワードを削除 ( str -- )' },
            { name: 'IF', description: '条件分岐 ( bool vec vec -- ... )' },
            { name: 'CALL', description: 'Quotationを実行 ( quot -- ... )' },
            { name: 'DEL', description: 'カスタムワードを削除 ( str -- )' },
            { name: 'NIL?', description: 'nilかどうかをチェック ( a -- bool )' },
            { name: 'NOT-NIL?', description: 'nilでないかをチェック ( a -- bool )' },
            { name: 'KNOWN?', description: 'nil以外の値かチェック ( a -- bool )' },
            { name: 'DEFAULT', description: 'nilならデフォルト値を使用 ( a b -- a | nil b -- b )' },
            { name: '.', description: '値を出力してドロップ ( a -- )' },
            { name: 'PRINT', description: '値を出力（ドロップしない） ( a -- a )' },
            { name: 'CR', description: '改行を出力 ( -- )' },
            { name: 'SPACE', description: 'スペースを出力 ( -- )' },
            { name: 'SPACES', description: 'N個のスペースを出力 ( n -- )' },
            { name: 'EMIT', description: '文字コードを文字として出力 ( n -- )' }
        ];
    }

    renderBuiltinWords() {
        // カテゴリ分けのロジックを削除し、直接renderWordButtonsを呼び出す
        this.renderWordButtons(this.elements.builtinWordsDisplay, this.builtinWords, false);
    }

    updateCustomWords(customWordsInfo) {
        const words = (customWordsInfo || []).map(wordData => ({
            name: wordData[0],
            description: wordData[1] || null,
            protected: wordData[2] || false
        }));
        this.renderWordButtons(this.elements.customWordsDisplay, words, true);
    }

    renderWordButtons(container, words, isCustom) {
        // 描画の前に必ずコンテナをクリアする
        container.innerHTML = '';

        words.forEach(wordInfo => {
            const button = document.createElement('button');
            button.textContent = wordInfo.name;
            button.className = 'word-button';
            button.title = wordInfo.description || wordInfo.name;
            
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
