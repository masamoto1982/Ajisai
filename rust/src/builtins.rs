use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // スタック操作
    register_builtin(dictionary, "DUP", "スタックトップを複製 ( a -- a a )");
    register_builtin(dictionary, "DROP", "スタックトップを削除 ( a -- )");
    register_builtin(dictionary, "SWAP", "上位2つを交換 ( a b -- b a )");
    register_builtin(dictionary, "OVER", "2番目をコピー ( a b -- a b a )");
    register_builtin(dictionary, "ROT", "3番目を最上位へ ( a b c -- b c a )");
    register_builtin(dictionary, "NIP", "2番目を削除 ( a b -- b )");
    
    // レジスタ操作
    register_builtin(dictionary, ">R", "スタックからレジスタへ移動 ( a -- )");
    register_builtin(dictionary, "R>", "レジスタからスタックへ移動 ( -- a )");
    register_builtin(dictionary, "R@", "レジスタの値をコピー ( -- a )");
    register_builtin(dictionary, "R+", "レジスタとの加算 ( a -- a+r )");
    register_builtin(dictionary, "R-", "レジスタとの減算 ( a -- a-r )");
    register_builtin(dictionary, "R*", "レジスタとの乗算 ( a -- a*r )");
    register_builtin(dictionary, "R/", "レジスタとの除算 ( a -- a/r )");
    
    // ベクトル操作
    register_builtin(dictionary, "LENGTH", "ベクトルの長さ ( vec -- n )");
    register_builtin(dictionary, "HEAD", "最初の要素 ( vec -- elem )");
    register_builtin(dictionary, "TAIL", "最初以外の要素 ( vec -- vec' )");
    register_builtin(dictionary, "CONS", "要素を先頭に追加 ( elem vec -- vec' )");
    register_builtin(dictionary, "APPEND", "要素をベクトルの末尾に追加 ( vec elem -- vec' )");
    register_builtin(dictionary, "REVERSE", "ベクトルを逆順に ( vec -- vec' )");
    register_builtin(dictionary, "NTH", "N番目の要素を取得（負数は末尾から） ( n vec -- elem )");
    
    // スタックベース反復サポート（再帰の構成要素）
    register_builtin(dictionary, "UNCONS", "ベクトルを先頭要素と残りに分解 ( vec -- elem vec' )");
    register_builtin(dictionary, "EMPTY?", "ベクトルが空かチェック ( vec -- bool )");
    
    // 制御構造
    register_builtin(dictionary, "DEF", "新しいワードを定義 ( vec str -- )");
    register_builtin(dictionary, "IF", "条件分岐 ( bool vec vec -- ... )");
    register_builtin(dictionary, "CALL", "Quotationを実行 ( quot -- ... )");  // 新規追加
    
    // 辞書操作
    register_builtin(dictionary, "DEL", "カスタムワードを削除 ( str -- )");
    
    // 算術演算子（暗黙の反復対応）
    register_builtin(dictionary, "+", "加算 - 暗黙の反復対応 ( a b -- a+b )");
    register_builtin(dictionary, "-", "減算 - 暗黙の反復対応 ( a b -- a-b )");
    register_builtin(dictionary, "*", "乗算 - 暗黙の反復対応 ( a b -- a*b )");
    register_builtin(dictionary, "/", "除算 - 暗黙の反復対応 ( a b -- a/b )");
    
    // 比較演算子（暗黙の反復対応）
    register_builtin(dictionary, ">", "より大きい - 暗黙の反復対応 ( a b -- bool )");
    register_builtin(dictionary, ">=", "以上 - 暗黙の反復対応 ( a b -- bool )");
    register_builtin(dictionary, "=", "等しい ( a b -- bool )");
    register_builtin(dictionary, "<", "より小さい - 暗黙の反復対応 ( a b -- bool )");
    register_builtin(dictionary, "<=", "以下 - 暗黙の反復対応 ( a b -- bool )");

    // 論理演算子（暗黙の反復対応、三値論理対応）
    register_builtin(dictionary, "NOT", "論理否定 - 暗黙の反復対応 ( bool -- bool )");
    register_builtin(dictionary, "AND", "論理積 - 三値論理対応 ( bool bool -- bool )");
    register_builtin(dictionary, "OR", "論理和 - 三値論理対応 ( bool bool -- bool )");

    // Nil関連
    register_builtin(dictionary, "NIL?", "nilかどうかをチェック ( a -- bool )");
    register_builtin(dictionary, "NOT-NIL?", "nilでないかをチェック ( a -- bool )");
    register_builtin(dictionary, "KNOWN?", "nil以外の値かチェック（NOT-NIL?のエイリアス） ( a -- bool )");
    register_builtin(dictionary, "DEFAULT", "nilならデフォルト値を使用 ( a b -- a | nil b -- b )");

    // データベース操作 (一時的にコメントアウト - Vector機能完成後に再有効化予定)
    /*
    register_builtin(dictionary, "TABLE", "テーブルをスタックに載せる ( str -- table )");
    register_builtin(dictionary, "TABLE-CREATE", "新しいテーブルを作成 ( vec str -- )");
    register_builtin(dictionary, "FILTER", "条件でレコードをフィルタ ( table vec -- table' )");
    register_builtin(dictionary, "PROJECT", "指定カラムを選択 ( table vec -- table' )");
    register_builtin(dictionary, "INSERT", "レコードを挿入 ( record str -- )");
    register_builtin(dictionary, "UPDATE", "レコードを更新 ( table vec -- )");
    register_builtin(dictionary, "DELETE", "レコードを削除 ( table -- )");
    register_builtin(dictionary, "TABLES", "テーブル名をパターンで検索 ( str -- vec )");
    register_builtin(dictionary, "TABLES-INFO", "全テーブルの詳細情報を表示 ( -- )");
    register_builtin(dictionary, "TABLE-INFO", "指定テーブルの情報を表示 ( str -- )");
    register_builtin(dictionary, "TABLE-SIZE", "テーブルのレコード数を取得 ( str -- n )");
    */
    // データベース永続化機能は残す（IndexedDB連携のため）
    register_builtin(dictionary, "SAVE-DB", "データベースを保存 ( -- )");
    register_builtin(dictionary, "LOAD-DB", "データベースを読み込み ( -- )");

    // ワイルドカード・パターンマッチング
    register_builtin(dictionary, "MATCH?", "ワイルドカードマッチング ( str str -- bool )");
    register_builtin(dictionary, "WILDCARD", "ワイルドカードパターンを作成 ( str -- pattern )");

    // 出力
    register_builtin(dictionary, ".", "値を出力してドロップ ( a -- )");
    register_builtin(dictionary, "PRINT", "値を出力（ドロップしない） ( a -- a )");
    register_builtin(dictionary, "CR", "改行を出力 ( -- )");
    register_builtin(dictionary, "SPACE", "スペースを出力 ( -- )");
    register_builtin(dictionary, "SPACES", "N個のスペースを出力 ( n -- )");
    register_builtin(dictionary, "EMIT", "文字コードを文字として出力 ( n -- )");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
    });
}
