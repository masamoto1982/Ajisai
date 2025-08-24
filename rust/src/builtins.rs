// rust/src/builtins.rs (新しいワード体系)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // 対象指定ワード（2個）
    register_builtin(dictionary, "頁", "ベクトル内の特定のページ（インデックス）を指定します。例: [ 1 2 3 4 5 ] 2 頁", "Target");
    register_builtin(dictionary, "頁数", "ベクトルの総ページ数（要素数）を指定します。例: [ 1 2 3 4 5 ] 頁数", "Target");
    
    // 基本操作ワード（6個）
    register_builtin(dictionary, "取得", "指定された対象を取得します。例: [ 1 2 3 4 5 ] 2 頁 取得 → 3", "Operation");
    register_builtin(dictionary, "挿入", "指定位置に要素を挿入します。例: [ 1 2 3 ] 1 頁 9 挿入 → [ 1 9 2 3 ]", "Operation");
    register_builtin(dictionary, "置換", "指定位置の要素を置き換えます。例: [ 1 2 3 ] 1 頁 9 置換 → [ 1 9 3 ]", "Operation");
    register_builtin(dictionary, "削除", "指定位置の要素を削除します。例: [ 1 2 3 ] 1 頁 削除 → [ 1 3 ]", "Operation");
    register_builtin(dictionary, "合併", "2つのベクトルを結合します。例: [ 1 2 ] [ 3 4 ] 合併 → [ 1 2 3 4 ]", "Operation");
    register_builtin(dictionary, "分離", "ベクトルを指定位置で分離します。例: [ 1 2 3 4 ] 2 頁 分離 → [ 1 2 ] [ 3 4 ]", "Operation");
    
    // 定義/削除ワード（2個）
    register_builtin(dictionary, "DEF", "新しいカスタムワードを定義します。例: [ 複 * ] \"平方\" DEF", "Control");
    register_builtin(dictionary, "DEL", "カスタムワードを削除します。例: \"平方\" DEL", "Control");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str, category: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: Some(category.to_string()),
    });
}
