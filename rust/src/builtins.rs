// rust/src/builtins.rs (ソースコード順対応版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // ソースコード記述順で登録（カテゴリ分けなし）
    
    // 算術演算司書（要求順）
    register_builtin(dictionary, "+", "加算を行う司書です。例: 3 4 + → 7");
    register_builtin(dictionary, "/", "除算を行う司書です。例: 15 3 / → 5");
    register_builtin(dictionary, "*", "乗算を行う司書です。例: 6 7 * → 42");
    register_builtin(dictionary, "-", "減算を行う司書です。例: 10 3 - → 7");
    register_builtin(dictionary, "=", "2つの値が等しいかを判定する司書です。例: 5 5 = → true");
    register_builtin(dictionary, ">=", "左辺が右辺以上かを判定する司書です。例: 5 5 >= → true");
    register_builtin(dictionary, ">", "左辺が右辺より大きいかを判定する司書です。例: 5 3 > → true");
    register_builtin(dictionary, "AND", "論理積を計算する司書です");
    register_builtin(dictionary, "OR", "論理和を計算する司書です");
    register_builtin(dictionary, "NOT", "論理否定を計算する司書です");
    
    // 書籍・頁操作司書（要求順）
    register_builtin(dictionary, "頁", "書籍の特定ページを指定する司書です。例: [ 1 2 3 ] 1 頁 → 2");
    register_builtin(dictionary, "頁数", "書籍の総ページ数を取得する司書です。例: [ 1 2 3 ] 頁数 → 3");
    register_builtin(dictionary, "冊", "書籍コレクション内の特定の冊を取得する司書です");
    register_builtin(dictionary, "冊数", "書籍コレクションの総冊数を取得する司書です");
    register_builtin(dictionary, "挿入", "指定位置にページを挿入する司書です。頁と組み合わせて使用");
    register_builtin(dictionary, "置換", "指定位置のページを置換する司書です。頁と組み合わせて使用");
    register_builtin(dictionary, "削除", "コンテキストに応じて削除または破棄を行う司書です");
    register_builtin(dictionary, "合併", "2つの書籍を結合する司書です。例: [ 1 2 ] [ 3 4 ] 合併 → [ 1 2 3 4 ]");
    register_builtin(dictionary, "分離", "書籍を2つに分ける司書です。頁と組み合わせて使用");
    
    // 司書管理司書
    register_builtin(dictionary, "雇用", "新しい部署を作り司書を雇用します（DEF相当）");
    register_builtin(dictionary, "解雇", "部署を解散し司書を解雇します（DEL相当）");
    register_builtin(dictionary, "交代", "同一部署内で司書交代します（GOTO相当）");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: None, // カテゴリは削除
    });
}
