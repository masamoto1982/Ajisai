// rust/src/builtins.rs (新司書体系版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // 算術演算司書（4名）
    register_builtin(dictionary, "+", "加算を行う司書です。例: 3 4 + → 7", "Arithmetic");
    register_builtin(dictionary, "-", "減算を行う司書です。例: 10 3 - → 7", "Arithmetic");
    register_builtin(dictionary, "*", "乗算を行う司書です。例: 6 7 * → 42", "Arithmetic");
    register_builtin(dictionary, "/", "除算を行う司書です。例: 15 3 / → 5", "Arithmetic");
    
    // 比較判定司書（3名）
    register_builtin(dictionary, ">", "左辺が右辺より大きいかを判定する司書です。例: 5 3 > → true", "Comparison");
    register_builtin(dictionary, ">=", "左辺が右辺以上かを判定する司書です。例: 5 5 >= → true", "Comparison");
    register_builtin(dictionary, "=", "2つの値が等しいかを判定する司書です。例: 5 5 = → true", "Comparison");
    
    // 書籍操作司書（9名）
    register_builtin(dictionary, "頁", "書籍の特定ページを指定する司書です。例: [ 1 2 3 ] 1 頁 → 2", "BookOps");
    register_builtin(dictionary, "頁数", "書籍の総ページ数を取得する司書です。例: [ 1 2 3 ] 頁数 → 3", "BookOps");
    register_builtin(dictionary, "挿入", "指定位置にページを挿入する司書です。頁と組み合わせて使用", "BookOps");
    register_builtin(dictionary, "置換", "指定位置のページを置換する司書です。頁と組み合わせて使用", "BookOps");
    register_builtin(dictionary, "削除", "指定位置のページを削除する司書です。頁と組み合わせて使用", "BookOps");
    register_builtin(dictionary, "合併", "2つの書籍を結合する司書です。例: [ 1 2 ] [ 3 4 ] 合併 → [ 1 2 3 4 ]", "BookOps");
    register_builtin(dictionary, "分離", "書籍を2つに分ける司書です。頁と組み合わせて使用", "BookOps");
    register_builtin(dictionary, "待機", "何も処理しない司書です（pass文相当）", "BookOps");
    register_builtin(dictionary, "複製", "書籍を複製する司書です。例: [ 1 2 3 ] 複製 → [ 1 2 3 ] [ 1 2 3 ]", "BookOps");
    register_builtin(dictionary, "破棄", "書籍を破棄する司書です。例: [ 1 2 3 ] 破棄 → (空)", "BookOps");
    
    // 司書管理司書（3名）
    register_builtin(dictionary, "雇用", "新しい部署を作り司書を雇用します（DEF相当）", "Management");
    register_builtin(dictionary, "解雇", "部署を解散し司書を解雇します（DEL相当）", "Management");
    register_builtin(dictionary, "交代", "同一部署内で司書交代します（GOTO相当）", "Management");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str, category: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: Some(category.to_string()),
    });
}
