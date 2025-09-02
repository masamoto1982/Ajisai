// rust/src/builtins.rs (NOP追加、EVAL削除)

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
        // 算術・論理演算（> と >= を削除）
        ("+", "Addition operator"),
        ("/", "Division operator"), 
        ("*", "Multiplication operator"),
        ("-", "Subtraction operator"),
        ("=", "Equality test"),
        ("<=", "Less than or equal test"),
        ("<", "Less than test"),
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
        ("DROP", "Drop first N elements"),
        ("REPEAT", "Repeat element N times"),
        ("SPLIT", "Split vector by sizes"),
        
        // その他
        ("CONCAT", "Concatenate vectors"),
        ("GOTO", "Conditional code block execution"),
        ("DEF", "Define new word"),
        ("DEL", "Delete word"),
        ("NOP", "No operation - do nothing"), // EVAL → NOP に変更
        
        // 補助ワード
        ("CODE", "Code block marker"),
        ("DEFAULT", "Default code block marker"),
    ]
}
