// rust/src/builtins.rs

use std::collections::HashMap;
use crate::types::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    for (name, description) in get_builtin_definitions() {
        dictionary.insert(name.to_string(), WordDefinition {
            lines: vec![],
            is_builtin: true,
            description: Some(description.to_string()),
        });
    }
}

pub fn get_builtin_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("GET", "Get element at position (0-indexed)"),
        ("INSERT", "Insert element at position"),
        ("REPLACE", "Replace element at position"),
        ("REMOVE", "Remove element at position"),
        ("LENGTH", "Get vector length"),
        ("TAKE", "Take first N elements"),
        ("DROP", "Drop first N elements"),
        ("SPLIT", "Split vector by sizes"),
        ("DUP", "Duplicate top workspace element"),
        ("SWAP", "Swap top two workspace elements"),
        ("ROT", "Rotate top three workspace elements"),
        ("CONCAT", "Concatenate vectors"),
        ("REVERSE", "Reverse vector elements"),
        ("+", "Vector addition"),
        ("-", "Vector subtraction"),
        ("*", "Vector multiplication"),
        ("/", "Vector division"),
        ("=", "Vector equality test"),
        ("<", "Vector less than test"),
        ("<=", "Vector less than or equal test"),
        (">", "Vector greater than test"),
        (">=", "Vector greater than or equal test"),
        ("AND", "Vector logical AND"),
        ("OR", "Vector logical OR"),
        ("NOT", "Vector logical NOT"),
        ("PRINT", "Print vector value"),
        ("DEF", "Define new word"),
        ("DEL", "Delete word"),
        ("RESET", "Reset all memory and database"),
        ("GOTO", "( N -- ) Jump to N-th line in custom word (1-indexed)"),
    ]
}
