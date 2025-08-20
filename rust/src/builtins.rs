// rust/src/builtins.rs (完全版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // Stack操作
    register_builtin(dictionary, "DUP", "スタックトップを複製 ( a -- a a )", "Stack");
    register_builtin(dictionary, "DROP", "スタックトップを削除 ( a -- )", "Stack");
    register_builtin(dictionary, "SWAP", "上位2つを交換 ( a b -- b a )", "Stack");
    register_builtin(dictionary, "OVER", "2番目をコピー ( a b -- a b a )", "Stack");
    register_builtin(dictionary, "ROT", "3番目を最上位へ ( a b c -- b c a )", "Stack");
    register_builtin(dictionary, "NIP", "2番目を削除 ( a b -- b )", "Stack");
    
    // Arithmetic
    register_builtin(dictionary, "+", "加算 ( a b -- a+b )", "Arithmetic");
    register_builtin(dictionary, "-", "減算 ( a b -- a-b )", "Arithmetic");
    register_builtin(dictionary, "*", "乗算 ( a b -- a*b )", "Arithmetic");
    register_builtin(dictionary, "/", "除算 ( a b -- a/b )", "Arithmetic");
    
    // Comparison
    register_builtin(dictionary, ">", "より大きい ( a b -- bool )", "Comparison");
    register_builtin(dictionary, ">=", "以上 ( a b -- bool )", "Comparison");
    register_builtin(dictionary, "=", "等しい ( a b -- bool )", "Comparison");
    register_builtin(dictionary, "<", "より小さい ( a b -- bool )", "Comparison");
    register_builtin(dictionary, "<=", "以下 ( a b -- bool )", "Comparison");

    // Logic
    register_builtin(dictionary, "NOT", "論理否定 ( bool -- bool )", "Logic");
    register_builtin(dictionary, "AND", "論理積 ( bool bool -- bool )", "Logic");
    register_builtin(dictionary, "OR", "論理和 ( bool bool -- bool )", "Logic");

    // Vector
    register_builtin(dictionary, "LENGTH", "ベクトルの長さ ( vec -- n )", "Vector");
    register_builtin(dictionary, "HEAD", "最初の要素 ( vec -- elem )", "Vector");
    register_builtin(dictionary, "TAIL", "最初以外の要素 ( vec -- vec' )", "Vector");
    register_builtin(dictionary, "CONS", "要素を先頭に追加 ( elem vec -- vec' )", "Vector");
    register_builtin(dictionary, "APPEND", "要素をベクトルの末尾に追加 ( vec elem -- vec' )", "Vector");
    register_builtin(dictionary, "REVERSE", "ベクトルを逆順に ( vec -- vec' )", "Vector");
    register_builtin(dictionary, "NTH", "N番目の要素を取得 ( n vec -- elem )", "Vector");
    register_builtin(dictionary, "UNCONS", "ベクトルを分解 ( vec -- elem vec' )", "Vector");
    register_builtin(dictionary, "EMPTY?", "ベクトルが空かチェック ( vec -- bool )", "Vector");
    
    // Quotation
    register_builtin(dictionary, "CALL", "クオーテーションを実行 ( quot -- )", "Quotation");
    
    // Control
    register_builtin(dictionary, "DEL", "カスタムワードを削除 ( str -- )", "Control");
    register_builtin(dictionary, "DEF", "カスタムワードを定義 ( quot str -- )", "Control");
    register_builtin(dictionary, "LEAP", "条件付き絶対ジャンプ ( condition word -- )", "Control");

    // Nil
    register_builtin(dictionary, "NIL?", "nilかどうかをチェック ( a -- bool )", "Nil");
    register_builtin(dictionary, "NOT-NIL?", "nilでないかをチェック ( a -- bool )", "Nil");
    register_builtin(dictionary, "KNOWN?", "nil以外の値かチェック ( a -- bool )", "Nil");
    register_builtin(dictionary, "DEFAULT", "nilならデフォルト値を使用 ( a | b -- a | b )", "Nil");

    // Output
    register_builtin(dictionary, "SHOW", "値を出力してドロップ ( a -- )", "Output");
    register_builtin(dictionary, "NEWL", "改行を出力 ( -- )", "Output");
    register_builtin(dictionary, "SPCE", "スペースを出力 ( -- )", "Output");
    register_builtin(dictionary, "SPCS", "N個のスペースを出力 ( n -- )", "Output");
    register_builtin(dictionary, "CHAR", "文字コードを文字として出力 ( n -- )", "Output");
    
    // Database
    register_builtin(dictionary, "AMNESIA", "IndexedDBを初期化 ( -- )", "Database");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str, category: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: Some(category.to_string()),
    });
}
