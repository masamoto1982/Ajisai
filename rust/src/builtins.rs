// rust/src/builtins.rs

use std::collections::{HashMap, HashSet};
use crate::types::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    for (name, description, _) in get_builtin_definitions() {
        dictionary.insert(name.to_string(), WordDefinition {
            lines: vec![],
            is_builtin: true,
            description: Some(description.to_string()),
            dependencies: HashSet::new(),
            original_source: None,
        });
    }
}

pub fn get_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // 位置指定操作（0オリジン）
        ("GET", "Get element at index. Usage: [ vector ] [ index ] GET", "Vector Operations"),
        ("INSERT", "Insert element at index. Usage: [ vector ] [ index ] [ element ] INSERT", "Vector Operations"),
        ("REPLACE", "Replace element at index. Usage: [ vector ] [ index ] [ element ] REPLACE", "Vector Operations"),
        ("REMOVE", "Remove element at index. Usage: [ vector ] [ index ] REMOVE", "Vector Operations"),
        
        // 量指定操作（1オリジン）
        ("LENGTH", "Get vector length", "Vector Operations"),
        ("TAKE", "Take N elements. Usage: [ vector ] [ N ] TAKE", "Vector Operations"),
        
        // Vector構造操作
        ("SPLIT", "Split vector. Usage: [ vector ] [ index ] SPLIT", "Vector Operations"),
        ("CONCAT", "Concatenate vectors. Usage: [ v1 ] [ v2 ] CONCAT", "Vector Operations"),
        ("REVERSE", "Reverse vector. Usage: [ vector ] REVERSE", "Vector Operations"),
        ("LEVEL", "Flatten nested vector. Usage: [ vector ] LEVEL", "Vector Operations"),
        
        // 算術演算
        ("+", "Add two numbers", "Arithmetic"),
        ("-", "Subtract two numbers", "Arithmetic"),
        ("*", "Multiply two numbers", "Arithmetic"),
        ("/", "Divide two numbers", "Arithmetic"),
        
        // 比較演算
        ("=", "Equal comparison", "Comparison"),
        ("<", "Less than", "Comparison"),
        ("<=", "Less than or equal", "Comparison"),
        (">", "Greater than", "Comparison"),
        (">=", "Greater than or equal", "Comparison"),
        
        // 論理演算
        ("AND", "Logical AND", "Logic"),
        ("OR", "Logical OR", "Logic"),
        ("NOT", "Logical NOT", "Logic"),
        
        // 制御構造
        (":", "Guard separator. Usage: condition : action : condition : action : default", "Control"),
        (";", "Synonym for : (guard separator)", "Control"),
        
        // 高階関数
        ("MAP", "Apply word to each element. Usage: [ data ] 'WORD' MAP or ... [ N ] 'WORD' STACK MAP", "Higher-Order"),
        ("FILTER", "Filter elements using word. Usage: [ data ] 'WORD' FILTER", "Higher-Order"),
        
        // 入出力
        ("PRINT", "Print top element", "I/O"),
        
        // カスタムワード管理
        ("DEF", "Define a custom word. Usage: (definition block) 'NAME' DEF", "Word Management"),
        ("DEL", "Delete a custom word. Usage: 'NAME' DEL", "Word Management"),
        ("?", "Look up word definition. Usage: 'NAME' ?", "Word Management"),
    ]
}

pub fn get_builtin_detail(name: &str) -> String {
    let definitions = get_builtin_definitions();
    for (word_name, description, category) in definitions {
        if word_name == name {
            return format!("Built-in Word: {}\nCategory: {}\nDescription: {}", name, category, description);
        }
    }
    format!("No detailed information available for '{}'", name)
}
