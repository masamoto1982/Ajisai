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
        ("+", "Vector addition", "Arithmetic"),
        ("-", "Vector subtraction", "Arithmetic"),
        ("*", "Vector multiplication", "Arithmetic"),
        ("/", "Vector division", "Arithmetic"),
        
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
    ]
}

pub fn get_builtin_detail(name: &str) -> String {
    match name {
        // === 位置指定操作（0オリジン） ===
        "GET" => r#"# GET - 要素の取得（0オリジン）

## 説明
ベクトルまたはスタック全体から指定位置の要素を取得します。
デフォルトの操作対象はスタックトップのベクトルです。
`STACK`ワードを使うと、スタック全体を一つのベクトルと見なして操作できます。

## 使用法
[ vector ] [ index ] GET
... STACK [ index ] GET

## 例
# スタックトップのベクトルを操作
[ 10 20 30 ] [ 1 ] GET  # → [ 20 ]

# スタック全体を操作
[ 1 2 ] [ 3 4 ] STACK [ 0 ] GET  # → [ 1 2 ] [ 3 4 ] [ 1 2 ]"#.to_string(),

        "INSERT" => r#"# INSERT - 要素の挿入（0オリジン）

## 説明
ベクトルまたはスタック全体に、指定位置で新しい要素を挿入します。

## 使用法
[ vector ] [ index ] [ element ] INSERT
... STACK [ index ] [ element ] INSERT

## 例
# スタックトップに挿入
[ 2 3 ] [ 0 ] [ 1 ] INSERT  # → [ 1 2 3 ]

# スタックの途中に挿入
[ 1 ] [ 3 ] STACK [ 1 ] [ 2 ] INSERT  # → [ 1 ] [ 2 ] [ 3 ]"#.to_string(),

        "REPLACE" => r#"# REPLACE - 要素の置換（0オリジン）

## 説明
ベクトルまたはスタックの指定位置の要素を新しい値に置き換えます。

## 使用法
[ vector ] [ index ] [ new_element ] REPLACE
... STACK [ index ] [ new_element ] REPLACE

## 例
[ 1 2 3 ] [ 1 ] [ 5 ] REPLACE  # → [ 1 5 3 ]
[ 1 ] [ 2 ] STACK [ 0 ] [ 9 ] REPLACE # → [ 9 ] [ 2 ]"#.to_string(),

        "REMOVE" => r#"# REMOVE - 要素の削除（0オリジン）

## 説明
ベクトルまたはスタックから指定位置の要素を削除します。

## 使用法
[ vector ] [ index ] REMOVE
... STACK [ index ] REMOVE

## 例
[ 1 2 3 ] [ 1 ] REMOVE  # → [ 1 3 ]
[ 1 ] [ 2 ] [ 3 ] STACK [ 1 ] REMOVE # → [ 1 ] [ 3 ]"#.to_string(),

        // === 量指定操作（1オリジン） ===
        "LENGTH" => r#"# LENGTH - 長さ取得

## 説明
ベクトルまたはスタックの要素数を返します。

## 使用法
[ vector ] LENGTH
... STACK LENGTH

## 例
[ 1 2 3 4 5 ] LENGTH  # → [ 5 ]
[ 1 ] [ 2 ] STACK LENGTH # → [ 1 ] [ 2 ] [ 2 ]"#.to_string(),

        "TAKE" => r#"# TAKE - 先頭からN個取得（1オリジン）

## 説明
ベクトルまたはスタックの先頭からN個の要素を取得します。

## 使用法
[ vector ] [ count ] TAKE
... STACK [ count ] TAKE

## 例
[ 1 2 3 4 5 ] [ 3 ] TAKE  # → [ 1 2 3 ]
[ 1 ] [ 2 ] [ 3 ] STACK [ 2 ] TAKE # → [ 1 ] [ 2 ]"#.to_string(),

        // === Vector構造操作 ===
        "SPLIT" => r#"# SPLIT - 分割・分解

## 説明
ベクトルまたはスタックを指定したサイズで分割します。

## 使用法
[ vector ] [ size1 ] ... SPLIT
... STACK [ size1 ] ... SPLIT

## 例
[ 1 2 3 4 5 6 ] [ 2 ] [ 3 ] [ 1 ] SPLIT # → [ 1 2 ] [ 3 4 5 ] [ 6 ]
[ 1 ] [ 2 ] [ 3 ] STACK [ 1 ] [ 2 ] SPLIT # → [ 1 ] [ 2 3 ]"#.to_string(),

        "CONCAT" => r#"# CONCAT - 連結

## 説明
ベクトルまたはスタック上の複数のベクトルを連結します。

## 使用法
[ vector1 ] [ vector2 ] ... [ N ] CONCAT
... STACK [ N ] CONCAT

## 例
[ 1 2 ] [ 3 4 ] CONCAT  # → [ 1 2 3 4 ]
[ 1 ] [ 2 ] STACK [ 2 ] CONCAT # → [ 1 2 ]"#.to_string(),

        "REVERSE" => r#"# REVERSE - 反転

## 説明
ベクトルまたはスタックの要素順序を逆にします。

## 使用法
[ vector ] REVERSE
... STACK REVERSE

## 例
[ 1 2 3 4 ] REVERSE  # → [ 4 3 2 1 ]
[ 1 ] [ 2 ] [ 3 ] STACK REVERSE # → [ 3 ] [ 2 ] [ 1 ]"#.to_string(),

        "LEVEL" => r#"# LEVEL - フラット化

## 説明
ネストされたベクトルやスタックを平坦なベクトルに変換します。

## 使用法
[ nested_vector ] LEVEL
... STACK LEVEL

## 例
[ [ 1 2 ] [ 3 [ 4 ] ] ] LEVEL # → [ 1 2 3 4 ]
[ [ 1 ] [ 2 ] ] [ 3 ] STACK LEVEL # → [ 1 2 3 ]"#.to_string(),

        // === 算術演算 ===
        "+" => r#"# + - ベクトルの加算

## 説明
2つのベクトルの要素を加算します。

## 使用法
[ vector1 ] [ vector2 ] +

## 例
[ 1 2 3 ] [ 4 5 6 ] +  # → [ 5 7 9 ]"#.to_string(),

        "-" => r#"# - - ベクトルの減算

## 説明
2つのベクトルの要素を減算します。

## 使用法
[ vector1 ] [ vector2 ] -

## 例
[ 5 7 9 ] [ 1 2 3 ] -  # → [ 4 5 6 ]"#.to_string(),

        "*" => r#"# * - ベクトルの乗算

## 説明
2つのベクトルの要素を乗算します。

## 使用法
[ vector1 ] [ vector2 ] *

## 例
[ 2 3 4 ] [ 5 6 7 ] * # → [ 10 18 28 ]"#.to_string(),

        "/" => r#"# / - ベクトルの除算

## 説明
2つのベクトルの要素を除算します。

## 使用法
[ vector1 ] [ vector2 ] /

## 例
[ 10 20 30 ] [ 2 4 5 ] /  # → [ 5 5 6 ]"#.to_string(),

        // === 比較演算 ===
        "=" => r#"# = - 等価比較

## 説明
2つのベクトルの要素が等しいかを比較します。

## 使用法
[ vector1 ] [ vector2 ] =

## 例
[ 5 ] [ 5 ] =  # → [ TRUE ]
[ 3 ] [ 5 ] =  # → [ FALSE ]"#.to_string(),

        "<" => r#"# < - 小なり比較

## 説明
ベクトルの要素を比較します。

## 使用法
[ vector1 ] [ vector2 ] <

## 例
[ 3 ] [ 5 ] <  # → [ TRUE ]"#.to_string(),

        "<=" => r#"# <= - 小なりイコール比較

## 説明
ベクトルの要素を比較します。

## 使用法
[ vector1 ] [ vector2 ] <=

## 例
[ 5 ] [ 5 ] <=  # → [ TRUE ]"#.to_string(),

        ">" => r#"# > - 大なり比較

## 説明
ベクトルの要素を比較します。

## 使用法
[ vector1 ] [ vector2 ] >

## 例
[ 7 ] [ 5 ] >  # → [ TRUE ]"#.to_string(),

        ">=" => r#"# >= - 大なりイコール比較

## 説明
ベクトルの要素を比較します。

## 使用法
[ vector1 ] [ vector2 ] >=

## 例
[ 5 ] [ 5 ] >=  # → [ TRUE ]"#.to_string(),

        // === 論理演算 ===
        "AND" => r#"# AND - 論理積

## 説明
2つのベクトルの論理積を取ります。

## 使用法
[ vector1 ] [ vector2 ] AND

## 例
[ TRUE ] [ TRUE ] AND  # → [ TRUE ]
[ TRUE ] [ FALSE ] AND  # → [ FALSE ]"#.to_string(),

        "OR" => r#"# OR - 論理和

## 説明
2つのベクトルの論理和を取ります。

## 使用法
[ vector1 ] [ vector2 ] OR

## 例
[ TRUE ] [ FALSE ] OR  # → [ TRUE ]"#.to_string(),

        "NOT" => r#"# NOT - 論理否定

## 説明
ベクトルの各要素の論理否定を取ります。

## 使用法
[ vector ] NOT

## 例
[ TRUE ] NOT  # → [ FALSE ]"#.to_string(),

        // === 制御構造 ===
        ":" => r#"# : - 条件実行（ゲート）

## 説明
条件が真の場合のみ、後続の処理を実行します。

## 使用法
condition : action

## 例
[ 5 ] [ 5 ] = : [ 10 ] [ 5 ] +"#.to_string(),

        ";" => r#"# ; - 条件実行の代替記法

## 説明
':'と同じ機能を持つ条件実行演算子です。

## 使用法
condition ; action

## 例
[ 5 ] [ 5 ] = ; [ 'Equal' ] PRINT"#.to_string(),

        "TIMES" => r#"# TIMES - カスタムワードの繰り返し実行

## 説明
指定したカスタムワードを指定回数だけ実行します。

## 使用法
'WORD_NAME' [ count ] TIMES

## 例
[ 'Hello' ] PRINT 'GREET' DEF
'GREET' [ 3 ] TIMES"#.to_string(),

        "WAIT" => r#"# WAIT - カスタムワードの遅延実行

## 説明
指定したカスタムワードを指定時間待機してから実行します。

## 使用法
'WORD_NAME' [ milliseconds ] WAIT

## 例
[ 'Delayed' ] PRINT 'MSG' DEF
'MSG' [ 2000 ] WAIT"#.to_string(),

        // === 入出力 ===
        "PRINT" => r#"# PRINT - 値の出力

## 説明
スタックトップの値をOutputエリアに出力します。

## 使用法
[ value ] PRINT

## 例
[ 42 ] PRINT"#.to_string(),

        // === システム ===
        "DEF" => r#"# DEF - カスタムワードの定義

## 説明
新しいカスタムワードを定義します。

## 使用法
body 'NAME' DEF
body 'NAME' 'DESCRIPTION' DEF

## 例
[ 1 ] [ 2 ] + 'ADD12' DEF"#.to_string(),

        "DEL" => r#"# DEL - カスタムワードの削除

## 説明
定義済みのカスタムワードを削除します。

## 使用法
'WORD_NAME' DEL"#.to_string(),

        "?" => r#"# ? - ワード定義の表示

## 説明
ワードの定義や詳細情報をエディタに表示します。

## 使用法
'WORD_NAME' ?"#.to_string(),

        "RESET" => r#"# RESET - システムのリセット

## 説明
すべてのカスタムワード定義とデータベースをクリアし、
システムを初期状態に戻します。

## 使用法
RESET"#.to_string(),

        "STACK" => r#"# STACK - スタック操作モード

## 説明
後続のVector操作ワードの対象を、スタック全体に変更します。
この効果は一度きりです。

## 使用法
... STACK [op]

## 例
[1] [2] STACK REVERSE # → [2] [1]"#.to_string(),

        "STACKTOP" => r#"# STACKTOP - スタックトップ操作モード

## 説明
後続のVector操作ワードの対象を、スタックのトップ要素（デフォルト）に明示的に設定します。

## 使用法
... STACKTOP [op]"#.to_string(),
        
        // === オーディオ ===
        "AUDIO" => r#"# AUDIO - オーディオ再生

## 説明
数値や分数を音として再生します。

## 使用法
[ notes ] AUDIO

## 例
[ 440 523 659 ] AUDIO"#.to_string(),

        _ => format!("# {}\n\n組み込みワードです。\n詳細な説明はまだ用意されていません。", name)
    }
}
