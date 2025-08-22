// rust/src/builtins.rs (完全版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // 算術演算（記号）
    register_builtin(dictionary, "+", "加算 ( a b -- a+b )", "Arithmetic");
    register_builtin(dictionary, "-", "減算 ( a b -- a-b )", "Arithmetic");
    register_builtin(dictionary, "*", "乗算 ( a b -- a*b )", "Arithmetic");
    register_builtin(dictionary, "/", "除算 ( a b -- a/b )", "Arithmetic");
    register_builtin(dictionary, ">", "より大きい ( a b -- bool )", "Comparison");
    register_builtin(dictionary, ">=", "以上 ( a b -- bool )", "Comparison");
    register_builtin(dictionary, "=", "等しい ( a b -- bool )", "Comparison");
    
    // 論理演算
    register_builtin(dictionary, "NOT", "論理否定 ( bool -- bool )", "Logic");
    register_builtin(dictionary, "AND", "論理積 ( bool bool -- bool )", "Logic");
    register_builtin(dictionary, "OR", "論理和 ( bool bool -- bool )", "Logic");
    
    // 存在チェック
    register_builtin(dictionary, "NIL?", "nilかどうかをチェック ( a -- bool )", "Logic");
    register_builtin(dictionary, "SOME?", "nilでないかをチェック ( a -- bool )", "Logic");

    // Vector操作
    register_builtin(dictionary, "HEAD", "先頭要素を取得 ( vec -- elem )", "Vector");
    register_builtin(dictionary, "TAIL", "末尾群を取得 ( vec -- vec' )", "Vector");
    register_builtin(dictionary, "CONS", "先頭に接続 ( elem vec -- vec' )", "Vector");
    register_builtin(dictionary, "UNCONS", "先頭から分離 ( vec -- elem vec' )", "Vector");
    register_builtin(dictionary, "APPEND", "末尾に追加 ( vec elem -- vec' )", "Vector");
    register_builtin(dictionary, "REMOVE_LAST", "末尾から除去 ( vec -- vec' elem )", "Vector");
    register_builtin(dictionary, "CLONE", "値を複製 ( a -- a a )", "Vector");
    register_builtin(dictionary, "SELECT", "条件選択 ( a b condition -- result )", "Vector");
    register_builtin(dictionary, "LENGTH", "要素数を取得 ( vec -- n )", "Vector");
    register_builtin(dictionary, "COUNT", "要素数を取得 ( vec -- n )", "Vector");
    register_builtin(dictionary, "AT", "位置アクセス ( n vec -- elem )", "Vector");
    register_builtin(dictionary, "NTH", "位置アクセス ( n vec -- elem )", "Vector");
    register_builtin(dictionary, "DO", "実行 ( value -- )", "Vector");
    
    // 制御・定義
    register_builtin(dictionary, "DEF", "ワードを定義 ( vec str -- )", "Control");
    register_builtin(dictionary, "DEL", "ワードを削除 ( str -- )", "Control");
    register_builtin(dictionary, "LEAP", "条件付き跳躍 ( condition target -- )", "Control");
    
    // システム
    register_builtin(dictionary, "AMNESIA", "全データを消去 ( -- )", "System");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str, category: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: Some(category.to_string()),
    });
}
