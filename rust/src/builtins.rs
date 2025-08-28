// rust/src/builtins.rs (順序保持版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // ソースコード記述順で登録
    let builtin_definitions = get_builtin_definitions();
    
    for (name, description) in builtin_definitions {
        dictionary.insert(name.to_string(), WordDefinition {
            tokens: vec![],
            is_builtin: true,
            description: Some(description.to_string()),
            category: None,
        });
    }
}

// 組み込みワードの定義を順序付きで返す
pub fn get_builtin_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        // 算術演算司書（要求順）
        ("+", "加算を行う司書です。例: 3 4 + → 7"),
        ("/", "除算を行う司書です。例: 15 3 / → 5"),
        ("*", "乗算を行う司書です。例: 6 7 * → 42"),
        ("-", "減算を行う司書です。例: 10 3 - → 7"),
        ("=", "2つの値が等しいかを判定する司書です。例: 5 5 = → true"),
        (">=", "左辺が右辺以上かを判定する司書です。例: 5 5 >= → true"),
        (">", "左辺が右辺より大きいかを判定する司書です。例: 5 3 > → true"),
        ("AND", "論理積を計算する司書です"),
        ("OR", "論理和を計算する司書です"),
        ("NOT", "論理否定を計算する司書です"),
        
        // 書籍・頁操作司書（要求順）
        ("頁", "書籍の特定ページを指定する司書です。例: [ 1 2 3 ] 1 頁 → 2"),
        ("頁数", "書籍の総ページ数を取得する司書です。例: [ 1 2 3 ] 頁数 → 3"),
        ("巻", "巻は未実装です（将来機能）"),
        ("巻数", "巻数は未実装です（将来機能）"),
        ("冊", "書籍コレクション内の特定の冊を取得する司書です"),
        ("冊数", "書籍コレクションの総冊数を取得する司書です"),
        ("挿入", "指定位置にページを挿入する司書です。頁と組み合わせて使用"),
        ("置換", "指定位置のページを置換する司書です。頁と組み合わせて使用"),
        ("削除", "コンテキストに応じて削除または破棄を行う司書です"),
        ("合併", "2つの書籍を結合する司書です。例: [ 1 2 ] [ 3 4 ] 合併 → [ 1 2 3 4 ]"),
        ("分離", "書籍を2つに分ける司書です。頁と組み合わせて使用"),
        
        // 司書管理司書
        ("雇用", "新しい部署を作り司書を雇用します（DEF相当）"),
        ("解雇", "部署を解散し司書を解雇します（DEL相当）"),
        ("交代", "同一部署内で司書交代します（GOTO相当）"),
    ]
}
