// rust/src/builtins.rs
use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // 1. 基礎演算司書（4名）
    register_dual_builtin(dictionary, "+", "ADD", "二つの値を加算", "Basic");
    register_dual_builtin(dictionary, "-", "SUB", "二つの値を減算", "Basic");  
    register_dual_builtin(dictionary, "*", "MUL", "二つの値を乗算", "Basic");
    register_dual_builtin(dictionary, "/", "DIV", "二つの値を除算", "Basic");
    
    // 2. 比較演算司書（5名）
    register_dual_builtin(dictionary, ">", "GT", "左が右より大きいか判定", "Compare");
    register_dual_builtin(dictionary, ">=", "GE", "左が右以上か判定", "Compare");
    register_dual_builtin(dictionary, "=", "EQ", "二つの値が等しいか判定", "Compare");
    register_dual_builtin(dictionary, "<", "LT", "左が右より小さいか判定", "Compare");
    register_dual_builtin(dictionary, "<=", "LE", "左が右以下か判定", "Compare"); 
    
    // 3. 論理演算司書（3名）- 日本語名を追加
    register_dual_builtin(dictionary, "かつ", "AND", "論理積を計算", "Logic");
    register_dual_builtin(dictionary, "または", "OR", "論理和を計算", "Logic");
    register_dual_builtin(dictionary, "でない", "NOT", "論理否定を計算", "Logic");
    
    // 4. 書籍操作司書（10名）
    register_dual_builtin(dictionary, "頁", "PAGE", "指定ページを取得", "BookOps");
    register_dual_builtin(dictionary, "頁数", "LENGTH", "総ページ数を取得", "BookOps");
    register_dual_builtin(dictionary, "挿入", "INSERT", "指定位置にページを挿入", "BookOps");
    register_dual_builtin(dictionary, "置換", "REPLACE", "指定ページを置換", "BookOps");
    register_dual_builtin(dictionary, "削除", "DELETE", "指定ページを削除", "BookOps");
    register_dual_builtin(dictionary, "合併", "MERGE", "二つの書籍を結合", "BookOps");
    register_dual_builtin(dictionary, "分離", "SPLIT", "書籍を分割", "BookOps");
    register_dual_builtin(dictionary, "待機", "WAIT", "何もしない", "BookOps");
    register_dual_builtin(dictionary, "複製", "DUP", "書籍を複製", "BookOps");
    register_dual_builtin(dictionary, "破棄", "DROP", "書籍を破棄", "BookOps");
    
    // 5. 司書管理司書（3名）
    register_dual_builtin(dictionary, "雇用", "HIRE", "司書を雇用する司書", "Management");
    register_dual_builtin(dictionary, "解雇", "FIRE", "司書を解雇する司書", "Management");
    register_dual_builtin(dictionary, "交代", "HANDOVER", "司書を交代させる司書", "Management");
}

fn register_dual_builtin(
    dictionary: &mut HashMap<String, WordDefinition>, 
    japanese: &str, 
    english: &str, 
    description: &str, 
    category: &str
) {
    let definition = WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: Some(category.to_string()),
        hidden: Some(false),
        english_name: Some(english.to_string()),
        japanese_name: Some(japanese.to_string()),
    };
    
    // 日本語名で登録（メイン）
    dictionary.insert(japanese.to_string(), definition.clone());
    
    // 英語名でもアクセス可能にする（内部用）
    dictionary.insert(english.to_string(), definition);
}
