// js/gui/dictionary.js

export class Dictionary {
    init(elements, onWordClick) {
        this.elements = elements;
        this.onWordClick = onWordClick;
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
            { name: 'LENGTH', description: 'ベクトルの長さ ( vec -- n )' },
            { name: 'HEAD', description: '最初の要素 ( vec -- elem )' },
            { name: 'TAIL', description: '最初以外の要素 ( vec -- vec\' )' },
            { name: 'CONS', description: '要素を先頭に追加 ( elem vec -- vec\' )' },
            { name: 'APPEND', description: '要素を末尾に追加 ( vec elem -- vec\' )' },
            { name: 'REVERSE', description: 'ベクトルを逆順に ( vec -- vec\' )' },
            { name: 'NTH', description: 'N番目の要素を取得（負数は末尾から） ( n vec -- elem )' },
            { name: 'UNCONS', description: 'ベクトルを先頭要素と残りに分解 ( vec -- elem vec\' )' },
            { name: 'EMPTY?', description: 'ベクトルが空かチェック ( vec -- bool )' },
            { name: 'DEF', description: '新しいワードを定義 ( vec str -- )' },
            { name: 'IF', description: '条件分岐 ( bool vec vec -- ... )' },
            { name: 'CALL', description: 'Quotationを実行 ( quot -- ... )' },
            { name: 'DEL', description: 'カスタムワードを削除 ( str -- )' },
            { name: 'NIL?', description: 'nilかどうかをチェック ( a -- bool )' },
            { name: 'NOT-NIL?', description: 'nilでないかをチェック ( a -- bool )' },
            { name: 'KNOWN?', description: 'nil以外の値かチェック ( a -- bool )' },
            { name: 'DEFAULT', description: 'nilならデフォルト値を使用 ( a b -- a | nil b -- b )' },
            { name: 'TABLE', description: 'テーブルをスタックに載せる ( str -- table )' },
            { name: 'TABLE-CREATE', description: '新しいテーブルを作成 ( vec str -- )' },
            { name: 'FILTER', description: '条件でレコードをフィルタ ( table vec -- table\' )' },
            { name: 'PROJECT', description: '指定カラムを選択 ( table vec -- table\' )' },
            { name: 'INSERT', description: 'レコードを挿入 ( record str -- )' },
            { name: 'UPDATE', description: 'レコードを更新 ( table vec -- )' },
            { name: 'DELETE', description: 'レコードを削除 ( table -- )' },
            { name: 'TABLES', description: 'テーブル名をパターンで検索 ( str -- vec )' },
            { name: 'TABLES-INFO', description: '全テーブルの詳細情報を表示 ( -- )' },
            { name: 'TABLE-INFO', description: '指定テーブルの情報を表示 ( str -- )' },
            { name: 'TABLE-SIZE', description: 'テーブルのレコード数を取得 ( str -- n )' },
            { name: 'SAVE-DB', description: 'データベースを保存 ( -- )' },
            { name: 'LOAD-DB', description: 'データベースを読み込み ( -- )' },
            { name: 'MATCH?', description: 'ワイルドカードマッチング ( str str -- bool )' },
            { name: 'WILDCARD', description: 'ワイルドカードパターンを作成 ( str -- pattern )' },
            { name: '.', description: '値を出力してドロップ ( a -- )' },
            { name: 'PRINT', description: '値を出力（ドロップしない） ( a -- a )' },
            { name: 'CR', description: '改行を出力 ( -- )' },
            { name: 'SPACE', description: 'スペースを出力 ( -- )' },
            { name: 'SPACES', description: 'N個のスペースを出力 ( n -- )' },
            { name: 'EMIT', description: '文字コードを文字として出力 ( n -- )' }
        ];
    }

    renderBuiltinWords() {
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
