// rust/src/builtins.rs (BRANCH_IF/BRANCH_END削除、EXECUTE_CONDITIONS追加)

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
        // スタック操作
        ("DUP", "Duplicate top item"),
        ("DROP", "Remove top item"),
        ("SWAP", "Swap top two items"),
        ("OVER", "Copy second item to top"),
        ("ROT", "Rotate top three items"),
        
        // 算術・論理演算（> と >= を復活）
        ("+", "Addition operator"),
        ("/", "Division operator"), 
        ("*", "Multiplication operator"),
        ("-", "Subtraction operator"),
        ("=", "Equality test"),
        ("<=", "Less than or equal test"),
        ("<", "Less than test"),
        (">=", "Greater than or equal test"),
        (">", "Greater than test"),
        ("AND", "Logical AND"),
        ("OR", "Logical OR"),
        ("NOT", "Logical NOT"),
        
        // 位置指定操作（0オリジン）
        ("NTH", "Get element at position (0-indexed)"),
        ("INSERT", "Insert element at position"),
        ("REPLACE", "Replace element at position"),
        ("REMOVE", "Remove element at position"),
        
        // 量指定操作（1オリジン）
        ("LENGTH", "Get vector length"),
        ("TAKE", "Take first N elements"),
        ("DROP", "Drop first N elements"), // 注意: DROPが重複するので調整必要
        ("REPEAT", "Repeat element N times"),
        ("SPLIT", "Split vector by sizes"),
        
        // その他
        ("CONCAT", "Concatenate vectors"),
        ("DEF", "Define new word"),
        ("DEL", "Delete word"),
        ("NOP", "No operation - do nothing"),
        
        // 条件分岐制御（内部使用）
        ("CONDITIONAL_BRANCH", "Internal: conditional branch execution"),
    ]
}
