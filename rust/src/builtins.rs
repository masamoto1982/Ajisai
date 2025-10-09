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
        // Specifiers
        ("STACK", "Sets the operation target to the stack itself.", "Specifier"),
        ("STACKTOP", "Sets the operation target to the top vector's elements (default).", "Specifier"),

        // Vector/Stack Operations
        ("GET", "Gets an element from the target (stack or vector).", "Vector/Stack"),
        ("INSERT", "Inserts an element into the target.", "Vector/Stack"),
        ("REPLACE", "Replaces an element in the target.", "Vector/Stack"),
        ("REMOVE", "Removes an element from the target.", "Vector/Stack"),
        ("LENGTH", "Gets the length of the target.", "Vector/Stack"),
        ("TAKE", "Takes N elements from the target.", "Vector/Stack"),
        ("CONCAT", "Concatenates vectors.", "Vector/Stack"),
        ("REVERSE", "Reverses the target.", "Vector/Stack"),
        ("LEVEL", "Flattens the target.", "Vector/Stack"),
        ("SPLIT", "Splits a vector's elements onto the stack.", "Vector/Stack"),

        // Arithmetic
        ("+", "Performs element-wise addition.", "Arithmetic"),
        ("-", "Performs element-wise subtraction.", "Arithmetic"),
        ("*", "Performs element-wise multiplication.", "Arithmetic"),
        ("/", "Performs element-wise division.", "Arithmetic"),
        
        // Comparison (Unaffected by new design)
        ("=", "Vector equality test", "Comparison"),
        ("<", "Vector less than test", "Comparison"),
        ("<=", "Vector less than or equal test", "Comparison"),
        (">", "Vector greater than test", "Comparison"),
        (">=", "Vector greater than or equal test", "Comparison"),
        
        // Logic (Unaffected by new design)
        ("AND", "Vector logical AND", "Logic"),
        ("OR", "Vector logical OR", "Logic"),
        ("NOT", "Vector logical NOT", "Logic"),
        
        // Control Flow
        (":", "Conditional execution. Usage: condition : action", "Control"),
        (";", "Alternative to ':' for conditional execution", "Control"),
        ("TIMES", "Execute custom word N times.", "Control"),
        ("WAIT", "Execute custom word after delay.", "Control"),
        ("EVAL", "Evaluates the target as code.", "Control"),

        // Higher-Order Functions
        ("MAP", "Applies a word to each element of the target.", "HigherOrder"),
        ("FILTER", "Filters the target based on a predicate word.", "HigherOrder"),
        ("REDUCE", "Reduces the target to a single value.", "HigherOrder"),
        ("EACH", "Applies a word to each element for side effects.", "HigherOrder"),
        
        // I/O
        ("PRINT", "Print vector value", "IO"),
        ("AUDIO", "Play audio sequence", "Audio"),
        
        // System
        ("DEF", "Define new word.", "System"),
        ("DEL", "Delete word.", "System"),
        ("?", "Load word definition into editor.", "System"),
        ("RESET", "Reset all memory and database.", "System"),
    ]
}

pub fn get_builtin_detail(name: &str) -> String {
    // This function can be expanded to provide detailed help for each word,
    // explaining the difference between STACK and STACKTOP operations.
    format!("# {}\n\nDetailed documentation for this word is pending.", name)
}
