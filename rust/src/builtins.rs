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
        // 入力支援
        ("'", "Insert single quote", "Input Helper"),
        ("[]", "Insert empty vector brackets", "Input Helper"),
        
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
        
        // 制御構造（ガード）
        (":", "Guard separator for conditional execution. Usage: condition : action : condition : action : default", "Control"),
        (";", "Synonym for : (guard separator)", "Control"),
        ("TIMES", "Execute custom word N times. Usage: 'WORD' [ n ] TIMES", "Control"),
        ("WAIT", "Execute custom word after delay. Usage: 'WORD' [ ms ] WAIT", "Control"),

        // 高階関数
        ("MAP", "Apply word to each element. Usage: [ data ] 'WORD' MAP or ... [ N ] 'WORD' STACK MAP", "Higher-Order"),
        ("FILTER", "Filter elements using word. Usage: [ data ] 'WORD' FILTER", "Higher-Order"),
        ("REDUCE", "Reduce vector using word. Usage: [ data ] 'WORD' REDUCE", "Higher-Order"),

        // スタック操作
        ("DROP", "Remove top element from stack", "Stack"),
        ("DUP", "Duplicate top element", "Stack"),
        ("SWAP", "Swap top two elements", "Stack"),
        ("OVER", "Copy second element to top", "Stack"),
        ("ROT", "Rotate top three elements", "Stack"),
        ("DEPTH", "Get current stack depth", "Stack"),
        ("CLEAR", "Clear the stack", "Stack"),
        
        // 型変換・検査
        ("TO_STRING", "Convert to string", "Type"),
        ("TO_NUMBER", "Convert to number", "Type"),
        ("TYPE", "Get type of value", "Type"),
        
        // 入出力
        ("PRINT", "Print top element", "I/O"),
        ("PRINTLN", "Print top element with newline", "I/O"),
        
        // カスタムワード管理
        ("DEF", "Define a custom word. Usage: (definition block) 'NAME' DEF", "Word Management"),
        ("DEL", "Delete a custom word. Usage: 'NAME' DEL", "Word Management"),
        ("LOOKUP", "Look up word definition. Usage: 'NAME' LOOKUP", "Word Management"),
        ("WORDS", "List all defined words", "Word Management"),
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
```

---

## 実装の要点

1. **`Token::GuardSeparator`の再追加**: `:` と `;` を区別するためのトークン型を復活させました。

2. **トークナイザでの処理**: `parse_single_char_tokens` 関数で `:` と `;` を `GuardSeparator` トークンとして認識します。

3. **ガード構造の実行ロジック**:
   - `execute_guard_structure`: ガードセパレータでトークン列を分割し、条件:処理のペアを順次評価
   - `split_by_guard_separator`: トークン列をガードセパレータで分割
   - `is_condition_true`: スタックトップの値を真偽値として評価

4. **実行フロー**:
```
   条件1 : 処理1 : 条件2 : 処理2 : デフォルト処理
