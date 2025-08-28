// rust/src/builtins.rs
use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // 1. 基礎演算司書（4名）
    register_builtin(dictionary, "+", "二つの値を加算", "Basic");
    register_builtin(dictionary, "-", "二つの値を減算", "Basic");  
    register_builtin(dictionary, "*", "二つの値を乗算", "Basic");
    register_builtin(dictionary, "/", "二つの値を除算", "Basic");
    
    // 2. 比較演算司書（5名）
    register_builtin(dictionary, ">", "左が右より大きいか判定", "Compare");
    register_builtin(dictionary, ">=", "左が右以上か判定", "Compare");
    register_builtin(dictionary, "=", "二つの値が等しいか判定", "Compare");
    register_builtin(dictionary, "<", "左が右より小さいか判定", "Compare");
    register_builtin(dictionary, "<=", "左が右以下か判定", "Compare"); 
    
    // 3. 論理演算司書（3名）
    register_builtin(dictionary, "AND", "論理積を計算", "Logic");
    register_builtin(dictionary, "OR", "論理和を計算", "Logic");
    register_builtin(dictionary, "NOT", "論理否定を計算", "Logic");
    
    // 4. 書籍操作司書（10名）- 「冊」追加、「破棄」削除
    register_builtin(dictionary, "頁", "指定ページを取得", "BookOps");
    register_builtin(dictionary, "頁数", "総ページ数を取得", "BookOps");
    register_builtin(dictionary, "冊", "指定された冊（書籍）を取得", "BookOps");
    register_builtin(dictionary, "挿入", "指定位置に要素を挿入", "BookOps");
    register_builtin(dictionary, "置換", "指定位置の要素を置換", "BookOps");
    register_builtin(dictionary, "削除", "指定位置の要素を削除、または要素全体を削除", "BookOps");
    register_builtin(dictionary, "合併", "二つの書籍を結合", "BookOps");
    register_builtin(dictionary, "分離", "書籍を分割", "BookOps");
    register_builtin(dictionary, "待機", "何もしない", "BookOps");
    register_builtin(dictionary, "複製", "書籍を複製", "BookOps");
    // 「破棄」は削除 - DROPの機能は「削除」に統合
    
    // 5. 司書管理司書（3名）
    register_builtin(dictionary, "雇用", "司書を雇用する司書", "Management");
    register_builtin(dictionary, "解雇", "司書を解雇する司書", "Management");
    register_builtin(dictionary, "交代", "司書を交代させる司書", "Management");
}

fn register_builtin(
    dictionary: &mut HashMap<String, WordDefinition>, 
    name: &str,
    description: &str, 
    category: &str
) {
    let definition = WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: Some(category.to_string()),
        hidden: Some(false),
        english_name: None,
        japanese_name: None,
    };
    
    dictionary.insert(name.to_string(), definition);
}
