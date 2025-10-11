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
        ("GET", "Get element at position (0-indexed)", "Position"),
        ("INSERT", "Insert element at position", "Position"),
        ("REPLACE", "Replace element at position", "Position"),
        ("REMOVE", "Remove element at position", "Position"),
        
        // 量指定操作（1オリジン）
        ("LENGTH", "Get vector length", "Quantity"),
        ("TAKE", "Take first N elements", "Quantity"),
        
        // Vector構造操作
        ("SPLIT", "Splits a vector. With arguments, it splits into specified sizes. Without arguments, it slices into single-element vectors.", "Vector"),
        ("CONCAT", "Concatenate vectors. Default is 2. Specify count with an argument. Negative count reverses order.", "Vector"),
        ("REVERSE", "Reverse vector elements", "Vector"),
        ("LEVEL", "Flatten a nested vector", "Vector"),

        // 算術演算
        ("+", "Element-wise vector addition or Reduce N stack items.", "Arithmetic"),
        ("-", "Element-wise vector subtraction or Reduce N stack items.", "Arithmetic"),
        ("*", "Element-wise vector multiplication or Reduce N stack items.", "Arithmetic"),
        ("/", "Element-wise vector division or Reduce N stack items.", "Arithmetic"),
        
        // 比較演算
        ("=", "Vector equality test", "Comparison"),
        ("<", "Vector less than test", "Comparison"),
        ("<=", "Vector less than or equal test", "Comparison"),
        (">", "Vector greater than test", "Comparison"),
        (">=", "Vector greater than or equal test", "Comparison"),
        
        // 論理演算
        ("AND", "Vector logical AND", "Logic"),
        ("OR", "Vector logical OR", "Logic"),
        ("NOT", "Vector logical NOT", "Logic"),
        
        // 制御構造
        (":", "Conditional execution. Usage: condition : action", "Control"),
        (";", "Alternative to ':' for conditional execution", "Control"),
        ("TIMES", "Execute custom word N times. Usage: 'WORD' [ n ] TIMES", "Control"),
        ("WAIT", "Execute custom word after delay. Usage: 'WORD' [ ms ] WAIT", "Control"),

        // 高階関数
        ("MAP", "Apply word to each element. Usage: [ data ] 'WORD' MAP or ... [ N ] 'WORD' STACK MAP", "Higher-Order"),
        ("FILTER", "Filter elements using word. Usage: [ data ] 'WORD' FILTER or ... [ N ] 'WORD' STACK FILTER", "Higher-Order"),

        // 入出力
        ("PRINT", "Print vector value", "IO"),
        
        // オーディオ
        ("AUDIO", "Play audio sequence. Usage: [ notes ] AUDIO", "Audio"),
        
        // システム
        ("DEF", "Define new word. Usage: body 'NAME' DEF or body 'NAME' 'DESCRIPTION' DEF", "System"),
        ("DEL", "Delete word. Usage: 'WORD_NAME' DEL", "System"),
        ("?", "Load word definition into editor. Usage: 'WORD' ?", "System"),
        ("RESET", "Reset all memory and database", "System"),
        ("STACK", "Set operation target to the whole stack.", "System"),
        ("STACKTOP", "Set operation target to the top of the stack (default).", "System"),
        ("EVAL", "Evaluate code from vector. STACKTOP: eval single vector. STACK: concatenate and eval all vectors", "System"),
    ]
}

pub fn get_builtin_detail(name: &str) -> String {
    match name {
        "EVAL" => r#"# EVAL - コード評価

## 説明
ベクトル内の要素を連結してコードとして評価・実行します。
操作対象により2つの動作モードがあります。

## 使用法

### STACKTOPモード（デフォルト）
スタックトップのベクトルを評価し、結果を元のスタックに追加します。
