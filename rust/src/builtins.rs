use std::collections::{HashMap, HashSet};
use crate::types::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    for (name, description, _) in get_builtin_definitions() {
        dictionary.insert(name.to_string(), WordDefinition {
            lines: vec![],
            is_builtin: true,
            description: Some(description.to_string()),
            dependencies: HashSet::new(),
            original_source: None, // 🆕 追加
        });
    }
}

pub fn get_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("GET", "Get element at position (0-indexed)", "Position"),
        ("INSERT", "Insert element at position", "Position"),
        ("REPLACE", "Replace element at position", "Position"),
        ("REMOVE", "Remove element at position", "Position"),
        ("LENGTH", "Get vector length", "Quantity"),
        ("TAKE", "Take first N elements", "Quantity"),
        ("DROP", "Drop first N elements", "Quantity"),
        ("SPLIT", "Split vector by sizes", "Quantity"),
        ("DUP", "Duplicate top workspace element", "Workspace"),
        ("SWAP", "Swap top two workspace elements", "Workspace"),
        ("ROT", "Rotate top three workspace elements", "Workspace"),
        ("CONCAT", "Concatenate vectors", "Vector"),
        ("REVERSE", "Reverse vector elements", "Vector"),
        ("+", "Vector addition", "Arithmetic"),
        ("-", "Vector subtraction", "Arithmetic"),
        ("*", "Vector multiplication", "Arithmetic"),
        ("/", "Vector division", "Arithmetic"),
        ("=", "Vector equality test", "Comparison"),
        ("<", "Vector less than test", "Comparison"),
        ("<=", "Vector less than or equal test", "Comparison"),
        (">", "Vector greater than test", "Comparison"),
        (">=", "Vector greater than or equal test", "Comparison"),
        ("AND", "Vector logical AND", "Logic"),
        ("OR", "Vector logical OR", "Logic"),
        ("NOT", "Vector logical NOT", "Logic"),
        ("PRINT", "Print vector value", "IO"),
        ("DEF", "Define new word", "System"),
        ("DEL", "Delete word", "System"),
        ("RESET", "Reset all memory and database", "System"),
        ("GOTO", "( N -- ) Jump to N-th line in custom word (1-indexed)", "System"),
        ("?", "Load word definition into editor. Usage: 'WORD' ?", "System"),
    ]
}
