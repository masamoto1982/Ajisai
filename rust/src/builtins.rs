// rust/src/builtins.rs (純粋Vector操作言語版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
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

pub fn get_builtin_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        // 位置指定操作（0オリジン）
        ("GET", "Get element at position (0-indexed)"),
        ("INSERT", "Insert element at position"),
        ("REPLACE", "Replace element at position"),
        ("REMOVE", "Remove element at position"),
        
        // 量指定操作（1オリジン）
        ("LENGTH", "Get vector length"),
        ("TAKE", "Take first N elements"),
        ("DROP", "Drop first N elements"),
        ("REPEAT", "Repeat element N times"),
        ("SPLIT", "Split vector by sizes"),
        
        // ワークスペース操作
        ("DUP", "Duplicate top workspace element"),
        ("SWAP", "Swap top two workspace elements"),
        ("ROT", "Rotate top three workspace elements"),
        
        // Vector構造操作
        ("CONCAT", "Concatenate vectors"),
        ("REVERSE", "Reverse vector elements"),
        
        // 算術演算
        ("+", "Vector addition"),
        ("-", "Vector subtraction"),
        ("*", "Vector multiplication"),
        ("/", "Vector division"),
        
        // 比較演算
        ("=", "Vector equality test"),
        ("<", "Vector less than test"),
        ("<=", "Vector less than or equal test"),
        (">", "Vector greater than test"),
        (">=", "Vector greater than or equal test"),
        
        // 論理演算
        ("AND", "Vector logical AND"),
        ("OR", "Vector logical OR"),
        ("NOT", "Vector logical NOT"),
        
        // 入出力
        ("PRINT", "Print vector value"),
        
        // ワード管理・システム
        ("DEF", "Define new word"),
        ("DEL", "Delete word"),
        ("RESET", "Reset all memory and database"),
        
        // 条件分岐制御
        ("IF_SELECT", "Select action based on condition"),
    ]
}
