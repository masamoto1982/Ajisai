// rust/src/builtins.rs (新司書体系版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // 算術演算司書（4名）
    register_builtin(dictionary, "+", "☆", "Arithmetic");
    register_builtin(dictionary, "-", "☆", "Arithmetic");
    register_builtin(dictionary, "*", "☆", "Arithmetic");
    register_builtin(dictionary, "/", "☆", "Arithmetic");
    
    // 比較判定司書（3名）
    register_builtin(dictionary, ">", "☆", "Comparison");
    register_builtin(dictionary, ">=", "☆", "Comparison");
    register_builtin(dictionary, "=", "☆", "Comparison");
    
    // 書籍操作司書（9名）
    register_builtin(dictionary, "頁", "☆", "BookOps");
    register_builtin(dictionary, "頁数", "☆", "BookOps");
    register_builtin(dictionary, "挿入", "☆", "BookOps");
    register_builtin(dictionary, "置換", "☆", "BookOps");
    register_builtin(dictionary, "削除", "☆", "BookOps");
    register_builtin(dictionary, "合併", "☆", "BookOps");
    register_builtin(dictionary, "分離", "☆", "BookOps");
    register_builtin(dictionary, "待機", "☆", "BookOps");
    register_builtin(dictionary, "複製", "☆", "BookOps");
    register_builtin(dictionary, "破棄", "☆", "BookOps");
    
    // 司書管理司書（3名）
    register_builtin(dictionary, "雇用", "☆", "Management");
    register_builtin(dictionary, "解雇", "☆", "Management");
    register_builtin(dictionary, "交代", "☆", "Management");
    
    // 後方互換性のためのエイリアス
    register_builtin(dictionary, "DEF", "☆", "Management");
    register_builtin(dictionary, "DEL", "☆", "Management");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str, category: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: Some(category.to_string()),
    });
}
