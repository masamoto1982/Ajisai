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
        "EVAL" => r##"# EVAL - コード評価

## 説明
ベクトル内の要素を連結してコードとして評価・実行します。
操作対象により2つの動作モードがあります。

## 使用法

### STACKTOPモード（デフォルト）
スタックトップのベクトルを評価し、結果を元のスタックに追加します。

[ '7' '8' '+' ] EVAL        # → [ 15 ]
[ '7 8 +' ] EVAL            # → [ 15 ]

# 下の要素は保持される
[ 1 2 3 ] [ 4 5 6 ] [ '7' '8' '+' ] EVAL
# → スタック: [ 1 2 3 ] [ 4 5 6 ] [ 15 ]

### STACKモード
スタック上の全ベクトルを連結してコードとして実行します。

[ '1' ] [ '2' ] [ '+' ] STACK EVAL  # → [ 3 ]
[ '1' '2' ] [ '+' ] STACK EVAL      # → [ 3 ]

## 重要な特性

EVAL内で実行されるコードの演算モード（STACK/STACKTOP）は、
そのコード文字列の内容によって制御されます。

# デフォルトの+はSTACKTOPモード
[ '1' ] [ '2' ] [ '+' ] STACK EVAL
# → "1 2 +" を実行
# → +はSTACKTOPモードで動作
# → 結果: [ 3 ]

# STACKモードを明示的に指定
[ '1' ] [ '2' ] [ '3' ] [ '[ 3 ] STACK +' ] STACK EVAL
# → "1 2 3 [ 3 ] STACK +" を実行
# → 3要素を畳み込み
# → 結果: [ 6 ]

## 応用例

### 動的なコード構築
# カスタムワードを使った計算
[ '+' ] 'ADD' DEF
[ '10' ] [ '20' ] 'ADD' [ 3 ] STACK CONCAT EVAL
# → "10 20 ADD" を実行 → [ 30 ]

### 平均値の計算
[ '10' ] [ '20' ] [ '30' ] 
[ '[ 3 ] STACK +' ]  # 合計
[ '[ 3 ] /' ]        # 3で割る
STACK EVAL
# → "10 20 30 [ 3 ] STACK + [ 3 ] /" を実行
# → 結果: [ 20 ]

## 注意事項
- ベクトル要素は文字列、数値、シンボル、真偽値、またはNilである必要があります
- EVAL内でDEFを使用した場合、辞書に永続的に追加されます
- エラーが発生した場合、スタックは変更されません"##.to_string(),

        "+" => r##"# + - 加算

## 説明
操作対象により2つの動作をします。

1.  **STACKTOP (デフォルト):** スタックトップの2つのベクトル間で、要素ごとの加算を行います。片方がスカラ（単一要素ベクトル）の場合、もう一方のベクトルの全要素に適用されます（ブロードキャスト）。
2.  **STACK:** スタック上のN個の要素をすべて加算（畳み込み）します。

## 使用法
[ vector1 ] [ vector2 ] +
... [ N ] STACK +

## 例
# STACKTOP: ベクトル同士の加算
[ 1 2 3 ] [ 4 5 6 ] +  # → [ [ 5 7 9 ] ]

# STACKTOP: スカラのブロードキャスト
[ 1 2 3 ] [ 10 ] +     # → [ [ 11 12 13 ] ]

# STACK: スタック上の3要素を畳み込み
[ 1 ] [ 2 ] [ 3 ] [ 3 ] STACK +  # → [ [ 6 ] ]"##.to_string(),

        "-" => r##"# - - 減算

## 説明
操作対象により2つの動作をします。

1.  **STACKTOP (デフォルト):** ベクトル間の要素ごとの減算。スカラのブロードキャストに対応。
2.  **STACK:** スタック上のN個の要素を先頭から順に減算（畳み込み）。

## 使用法
[ vector1 ] [ vector2 ] -
... [ N ] STACK -

## 例
# STACKTOP:
[ 5 7 9 ] [ 1 2 3 ] -  # → [ [ 4 5 6 ] ]

# STACK:
[ 10 ] [ 3 ] [ 2 ] [ 3 ] STACK -  # → [ [ 5 ] ] (10-3-2)"##.to_string(),

        "*" => r##"# * - 乗算

## 説明
操作対象により2つの動作をします。

1.  **STACKTOP (デフォルト):** ベクトル間の要素ごとの乗算。スカラのブロードキャストに対応。
2.  **STACK:** スタック上のN個の要素をすべて乗算（畳み込み）。

## 使用法
[ vector1 ] [ vector2 ] *
... [ N ] STACK *

## 例
# STACKTOP:
[ 2 3 4 ] [ 5 6 7 ] * # → [ [ 10 18 28 ] ]

# STACK:
[ 2 ] [ 3 ] [ 4 ] [ 3 ] STACK * # → [ [ 24 ] ]"##.to_string(),

        "/" => r##"# / - 除算

## 説明
操作対象により2つの動作をします。

1.  **STACKTOP (デフォルト):** ベクトル間の要素ごとの除算。スカラのブロードキャストに対応。
2.  **STACK:** スタック上のN個の要素を先頭から順に除算（畳み込み）。

## 使用法
[ vector1 ] [ vector2 ] /
... [ N ] STACK /

## 例
# STACKTOP:
[ 10 20 30 ] [ 2 4 5 ] /  # → [ [ 5 5 6 ] ]

# STACK:
[ 100 ] [ 5 ] [ 2 ] [ 3 ] STACK / # → [ [ 10 ] ] (100/5/2)"##.to_string(),
        
        _ => format!("# {}\n\n組み込みワードです。\n詳細な説明はまだ用意されていません。", name)
    }
}
