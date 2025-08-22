// rust/src/builtins.rs (漢字一文字完全版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // 算術演算（記号・7個）
    register_builtin(dictionary, "+", "加算 ( a b -- a+b )", "Arithmetic");
    register_builtin(dictionary, "-", "減算 ( a b -- a-b )", "Arithmetic");
    register_builtin(dictionary, "*", "乗算 ( a b -- a*b )", "Arithmetic");
    register_builtin(dictionary, "/", "除算 ( a b -- a/b )", "Arithmetic");
    register_builtin(dictionary, ">", "より大きい ( a b -- bool )", "Comparison");
    register_builtin(dictionary, ">=", "以上 ( a b -- bool )", "Comparison");
    register_builtin(dictionary, "=", "等しい ( a b -- bool )", "Comparison");
    
    // 論理・存在（漢字・5個）
    register_builtin(dictionary, "否", "論理否定 ( bool -- bool )", "Logic");
    register_builtin(dictionary, "且", "論理積 ( bool bool -- bool )", "Logic");
    register_builtin(dictionary, "或", "論理和 ( bool bool -- bool )", "Logic");
    register_builtin(dictionary, "無", "nilかどうかをチェック ( a -- bool )", "Logic");
    register_builtin(dictionary, "有", "nilでないかをチェック ( a -- bool )", "Logic");

    // Vector操作（漢字・24個）
    register_builtin(dictionary, "頭", "先頭要素を取得 ( vec -- elem )", "Vector");
    register_builtin(dictionary, "尾", "末尾群を取得 ( vec -- vec' )", "Vector");
    register_builtin(dictionary, "接", "先頭に接続 ( elem vec -- vec' )", "Vector");
    register_builtin(dictionary, "離", "先頭から分離 ( vec -- elem vec' )", "Vector");
    register_builtin(dictionary, "追", "末尾に追加 ( vec elem -- vec' )", "Vector");
    register_builtin(dictionary, "除", "末尾から除去 ( vec -- vec' elem )", "Vector");
    register_builtin(dictionary, "複", "値を複製 ( a -- a a )", "Vector");
    register_builtin(dictionary, "選", "条件選択 ( a b condition -- result )", "Vector");
    register_builtin(dictionary, "数", "要素数を取得 ( vec -- n )", "Vector");
    register_builtin(dictionary, "在", "位置アクセス ( n vec -- elem )", "Vector");
    register_builtin(dictionary, "行", "実行 ( value -- )", "Vector");
    
    // 新Vector操作（13個）
    register_builtin(dictionary, "結", "Vector結合 ( vec1 vec2 -- vec )", "Vector");
    register_builtin(dictionary, "切", "指定位置で分割 ( vec n -- vec1 vec2 )", "Vector");
    register_builtin(dictionary, "反", "順序反転 ( vec -- vec' )", "Vector");
    register_builtin(dictionary, "挿", "指定位置に挿入 ( vec n elem -- vec' )", "Vector");
    register_builtin(dictionary, "消", "指定位置削除 ( vec n -- vec' elem )", "Vector");
    register_builtin(dictionary, "探", "要素検索 ( vec elem -- index/nil )", "Vector");
    register_builtin(dictionary, "含", "含有チェック ( vec elem -- bool )", "Vector");
    register_builtin(dictionary, "換", "要素置換 ( vec n elem -- vec' old_elem )", "Vector");
    register_builtin(dictionary, "抽", "条件抽出 ( vec predicate -- vec' )", "Vector");
    register_builtin(dictionary, "変", "要素変換 ( vec transform -- vec' )", "Vector");
    register_builtin(dictionary, "畳", "畳込処理 ( vec operation -- result )", "Vector");
    register_builtin(dictionary, "並", "ソート ( vec -- vec' )", "Vector");
    register_builtin(dictionary, "空", "空判定 ( vec -- bool )", "Vector");
    
    // 制御・システム（漢字・4個）
    register_builtin(dictionary, "定", "ワードを定義 ( vec str -- )", "Control");
    register_builtin(dictionary, "削", "ワードを削除 ( str -- )", "Control");
    register_builtin(dictionary, "跳", "条件付き跳躍 ( condition target -- )", "Control");
    register_builtin(dictionary, "忘", "全データを消去 ( -- )", "System");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str, category: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: Some(category.to_string()),
    });
}
