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
        // Vector/Stack共通操作
        ("GET", "Vector/Stack: Get element at position (0-indexed). Stack op is equivalent to PICK.", "Vector/Stack"),
        ("INSERT", "Vector/Stack: Insert element at position.", "Vector/Stack"),
        ("REPLACE", "Vector/Stack: Replace element at position.", "Vector/Stack"),
        ("REMOVE", "Vector/Stack: Remove element at position. Stack op is equivalent to DROP.", "Vector/Stack"),
        ("LENGTH", "Vector/Stack: Get vector/stack length. Stack op is equivalent to DEPTH.", "Vector/Stack"),
        ("TAKE", "Vector/Stack: Take first N elements and create a new vector.", "Vector/Stack"),
        ("CONCAT", "Vector/Stack: Concatenate N vectors into one.", "Vector/Stack"),
        ("REVERSE", "Vector/Stack: Reverse elements. Stack op is equivalent to SWAP/ROT.", "Vector/Stack"),
        ("LEVEL", "Vector/Stack: Flatten a nested vector or expand N vectors on stack.", "Vector/Stack"),
        ("SPLIT", "Splits a vector into elements on the stack.", "Vector/Stack"),

        // 算術演算
        ("+", "Vector/Stack: Add N vectors element-wise.", "Arithmetic"),
        ("-", "Vector/Stack: Subtract N vectors element-wise.", "Arithmetic"),
        ("*", "Vector/Stack: Multiply N vectors element-wise.", "Arithmetic"),
        ("/", "Vector/Stack: Divide N vectors element-wise.", "Arithmetic"),
        
        // 比較演算 (変更なし)
        ("=", "Vector equality test", "Comparison"),
        ("<", "Vector less than test", "Comparison"),
        ("<=", "Vector less than or equal test", "Comparison"),
        (">", "Vector greater than test", "Comparison"),
        (">=", "Vector greater than or equal test", "Comparison"),
        
        // 論理演算 (変更なし)
        ("AND", "Vector logical AND", "Logic"),
        ("OR", "Vector logical OR", "Logic"),
        ("NOT", "Vector logical NOT", "Logic"),
        
        // 制御構造 (変更なし)
        (":", "Conditional execution. Usage: condition : action", "Control"),
        (";", "Alternative to ':' for conditional execution", "Control"),
        ("TIMES", "Execute custom word N times. Usage: 'WORD' [ n ] TIMES", "Control"),
        ("WAIT", "Execute custom word after delay. Usage: 'WORD' [ ms ] WAIT", "Control"),
        ("EVAL", "Evaluate a vector or N stack items as code.", "Control"),

        // 高階関数
        ("MAP", "Apply word to each element of a vector or N stack items.", "HigherOrder"),
        ("FILTER", "Filter elements of a vector or N stack items.", "HigherOrder"),
        ("REDUCE", "Fold elements of a vector or N stack items.", "HigherOrder"),
        ("EACH", "Execute word for each element of a vector or N stack items (for side-effects).", "HigherOrder"),
        
        // 入出力 (変更なし)
        ("PRINT", "Print vector value", "IO"),
        
        // オーディオ
        ("AUDIO", "Play audio sequence. Usage: [ notes ] AUDIO", "Audio"),
        
        // システム (変更なし)
        ("DEF", "Define new word. Usage: body 'NAME' DEF or body 'NAME' 'DESCRIPTION' DEF", "System"),
        ("DEL", "Delete word. Usage: 'WORD_NAME' DEL", "System"),
        ("?", "Load word definition into editor. Usage: 'WORD' ?", "System"),
        ("RESET", "Reset all memory and database", "System"),
    ]
}

pub fn get_builtin_detail(name: &str) -> String {
    match name {
        "GET" => r#"# GET - 要素の取得 (Vector/Stack)

## 説明
Vector内部の要素、またはスタック上のVectorを取得します。

### デフォルト動作 (対象: Vectorの要素)
[Vector] [Index] GET -> [Element]
Vectorから指定位置(0オリジン)の要素を取得します。

### 拡張動作 (対象: スタック)
[Index] GET -> [Vector]
スタックの指定位置のVectorをコピーしてスタックトップに積みます。
伝統的な `PICK` に相当します。`[0] GET` は `DUP` と同じです。

## 例
# Vector操作
[10 20 30] [1] GET    # -> [20]

# スタック操作 (DUP)
[10] [0] GET          # -> [10] [10]

# スタック操作 (OVER)
[10] [20] [1] GET      # -> [10] [20] [10]"#.to_string(),

        "REMOVE" => r#"# REMOVE - 要素の削除 (Vector/Stack)

## 説明
Vector内部の要素、またはスタック上のVectorを削除します。

### デフォルト動作 (対象: Vectorの要素)
[Vector] [Index] REMOVE -> [ModifiedVector]
Vectorから指定位置(0オリジン)の要素を削除します。

### 拡張動作 (対象: スタック)
[Index] REMOVE
スタックの指定位置のVectorを削除します。
伝統的な `DROP` に相当します (`[0] REMOVE`)。

## 例
# Vector操作
[1 2 3] [1] REMOVE   # -> [1 3]

# スタック操作 (DROP)
[10] [20] [0] REMOVE   # -> [10]"#.to_string(),

        "REVERSE" => r#"# REVERSE - 要素の反転 (Vector/Stack)

## 説明
Vector内部の要素、またはスタック上の複数のVectorを反転させます。

### デフォルト動作 (対象: Vectorの要素)
[Vector] REVERSE -> [ReversedVector]
Vectorの要素を逆順にします。

### 拡張動作 (対象: スタック)
[V1] [V2]...[VN] [N] REVERSE
スタック上のN個のVectorの順序を逆転させます。
`[2] REVERSE` は `SWAP`、`[3] REVERSE` は `ROT` に相当します。

## 例
# Vector操作
[1 2 3] REVERSE      # -> [3 2 1]

# スタック操作 (SWAP)
[10] [20] [2] REVERSE  # -> [20] [10]

# スタック操作 (ROT)
[10] [20] [30] [3] REVERSE # -> [30] [20] [10]"#.to_string(),

        _ => format!("# {}\n\nこの組み込みワードの詳細な説明は、新しい設計に基づき更新中です。", name)
    }
}
