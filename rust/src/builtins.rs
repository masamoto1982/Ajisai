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
        ("SPLIT", "Split vector by sizes", "Quantity"),
        
        // Vector構造操作
        ("CONCAT", "Concatenate vectors", "Vector"),
        ("REVERSE", "Reverse vector elements", "Vector"),
        ("SLICE", "Slice vector into single-element vectors", "Vector"),
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
        ("EVAL", "Evaluate a vector as code", "Control"),

        // 高階関数
        ("MAP", "Apply word to each element. Usage: [ data ] 'WORD' MAP", "HigherOrder"),
        ("FILTER", "Keep elements that satisfy condition. Usage: [ data ] 'WORD' FILTER", "HigherOrder"),
        ("REDUCE", "Fold elements into single value. Usage: [ data ] [ init ] 'WORD' REDUCE", "HigherOrder"),
        ("EACH", "Execute word for each element (side-effects). Usage: [ data ] 'WORD' EACH", "HigherOrder"),
        
        // 入出力
        ("PRINT", "Print vector value", "IO"),
        
        // オーディオ
        ("AUDIO", "Play audio sequence. Usage: [ notes ] AUDIO", "Audio"),
        
        // システム
        ("DEF", "Define new word. Usage: body 'NAME' DEF or body 'NAME' 'DESCRIPTION' DEF", "System"),
        ("DEL", "Delete word. Usage: 'WORD_NAME' DEL", "System"),
        ("?", "Load word definition into editor. Usage: 'WORD' ?", "System"),
        ("RESET", "Reset all memory and database", "System"),
    ]
}

pub fn get_builtin_detail(name: &str) -> String {
    match name {
        // === 位置指定操作（0オリジン） ===
        "GET" => r#"# GET - 要素の取得（0オリジン）

## 説明
ベクトルから指定位置の要素を取得します。
インデックスは0から始まります（0オリジン）。
負のインデックスで末尾からアクセスできます。

## 使用法
[ vector ] [ index ] GET

## 例
# 正のインデックス
[ 10 20 30 ] [ 1 ] GET  # → [ 20 ]

# 負のインデックス（末尾から）
[ 10 20 30 ] [ -1 ] GET  # → [ 30 ]

# ネストしたベクトル
[ [ 1 2 ] [ 3 4 ] ] [ 0 ] GET  # → [ [ 1 2 ] ]"#.to_string(),

        "INSERT" => r#"# INSERT - 要素の挿入（0オリジン）

## 説明
ベクトルの指定位置に新しい要素を挿入します。
インデックスは0から始まります。

## 使用法
[ vector ] [ index ] [ element ] INSERT

## 例
# 先頭に挿入
[ 2 3 ] [ 0 ] [ 1 ] INSERT  # → [ 1 2 3 ]

# 途中に挿入
[ 1 3 ] [ 1 ] [ 2 ] INSERT  # → [ 1 2 3 ]

# 末尾に挿入（負のインデックス）
[ 1 2 ] [ -1 ] [ 3 ] INSERT  # → [ 1 2 3 ]"#.to_string(),

        "REPLACE" => r#"# REPLACE - 要素の置換（0オリジン）

## 説明
ベクトルの指定位置の要素を新しい値に置き換えます。

## 使用法
[ vector ] [ index ] [ new_element ] REPLACE

## 例
[ 1 2 3 ] [ 1 ] [ 5 ] REPLACE  # → [ 1 5 3 ]

# 負のインデックス
[ 1 2 3 ] [ -1 ] [ 9 ] REPLACE  # → [ 1 2 9 ]"#.to_string(),

        "REMOVE" => r#"# REMOVE - 要素の削除（0オリジン）

## 説明
ベクトルから指定位置の要素を削除します。

## 使用法
[ vector ] [ index ] REMOVE

## 例
[ 1 2 3 ] [ 1 ] REMOVE  # → [ 1 3 ]

# 負のインデックス
[ 1 2 3 ] [ -1 ] REMOVE  # → [ 1 2 ]"#.to_string(),

        // === 量指定操作（1オリジン） ===
        "LENGTH" => r#"# LENGTH - ベクトルの長さ取得

## 説明
ベクトルの要素数を返します。

## 使用法
[ vector ] LENGTH

## 例
[ 1 2 3 4 5 ] LENGTH  # → [ 5 ]

[ ] LENGTH  # → [ 0 ]"#.to_string(),

        "TAKE" => r#"# TAKE - 先頭からN個取得（1オリジン）

## 説明
ベクトルの先頭からN個の要素を取得します。
量を指定するため、1オリジンです。

## 使用法
[ vector ] [ count ] TAKE

## 例
# 先頭から3個
[ 1 2 3 4 5 ] [ 3 ] TAKE  # → [ 1 2 3 ]

# 負の数で末尾からN個
[ 1 2 3 4 5 ] [ -2 ] TAKE  # → [ 4 5 ]"#.to_string(),

        "SPLIT" => r#"# SPLIT - ベクトルの分割（1オリジン）

## 説明
ベクトルを指定したサイズで分割します。

## 使用法
[ vector ] [ size1 ] [ size2 ] ... [ sizeN ] SPLIT

## 例
[ 1 2 3 4 5 6 ] [ 2 ] [ 3 ] [ 1 ] SPLIT
# → [ 1 2 ] [ 3 4 5 ] [ 6 ]"#.to_string(),

        // === Vector構造操作 ===
        "CONCAT" => r#"# CONCAT - ベクトルの連結

## 説明
2つのベクトルを連結して1つのベクトルにします。

## 使用法
[ vector1 ] [ vector2 ] CONCAT

## 例
[ 1 2 ] [ 3 4 ] CONCAT  # → [ 1 2 3 4 ]"#.to_string(),

        "REVERSE" => r#"# REVERSE - ベクトルの反転

## 説明
ベクトルの要素順序を逆にします。

## 使用法
[ vector ] REVERSE

## 例
[ 1 2 3 4 ] REVERSE  # → [ 4 3 2 1 ]"#.to_string(),

        "SLICE" => r#"# SLICE - ベクトルの分解

## 説明
ベクトルを要素ごとに分解し、それぞれを単一要素のベクトルとしてスタックに積みます。

## 使用法
[ vector ] SLICE

## 例
[ 1 2 3 ] SLICE
# スタックの状態 (上から):
# [ 3 ]
# [ 2 ]
# [ 1 ]

[ [ 1 2 ] 'A' ] SLICE
# スタックの状態 (上から):
# [ 'A' ]
# [ [ 1 2 ] ]"#.to_string(),

        "LEVEL" => r#"# LEVEL - ベクトルのフラット化

## 説明
ネスト（入れ子）されたベクトルを、ネストのない平坦なベクトルに変換します。

## 使用法
[ nested_vector ] LEVEL

## 例
[ [ 1 2 ] 'A' [ 3 [ 4 ] ] ] LEVEL
# → [ 1 2 'A' 3 4 ]"#.to_string(),

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
[ vector1 ] [ vector2 ] 

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
条件が偽の場合、条件値自体を返します。

## 使用法
condition : action
または
condition : true-action : false-action

## 例
# 単純な条件分岐
[ 5 ] [ 5 ] = : [ 10 ] [ 5 ] +
# 結果: [ 15 ]（5 = 5 が真なので実行される）

# 条件分岐の連鎖
[ 5 ] [ 3 ] = : [ 10 ] [ 5 ] +
# 結果: [ FALSE ]（5 = 3 が偽なので条件値を返す）

# 複数条件
[ 0 ] = : [ 0 ] : [ 0 ] > : [ 1 ] : [ -1 ] 'SIGN' DEF
[ 5 ] SIGN  # → [ 1 ]"#.to_string(),

        ";" => r#"# ; - 条件実行の代替記法

## 説明
':'と同じ機能を持つ条件実行演算子です。
見た目の好みで使い分けられます。

## 使用法
condition ; action

## 例
[ 5 ] [ 5 ] = ; [ 'Equal' ] PRINT"#.to_string(),

        "TIMES" => r#"# TIMES - カスタムワードの繰り返し実行

## 説明
指定したカスタムワードを指定回数だけ実行します。
組み込みワードには使用できません。

## 使用法
'WORD_NAME' [ count ] TIMES

## 例
# カスタムワードを定義
[ 'Hello' ] PRINT
'GREET' DEF

# 3回実行
'GREET' [ 3 ] TIMES
# Output: ['Hello'] ['Hello'] ['Hello']

# WAITと組み合わせる
'GREET' [ 3 ] TIMES [ 1000 ] WAIT  # 3回実行後、1秒待つ

## 注意
- カスタムワードのみが対象です
- ワード名は文字列（'または"）で囲みます
- 回数は [ ] で囲んだ整数です"#.to_string(),

        "WAIT" => r#"# WAIT - カスタムワードの遅延実行

## 説明
指定したカスタムワードを指定時間待機してから実行します。
組み込みワードには使用できません。

## 使用法
'WORD_NAME' [ milliseconds ] WAIT

## 例
# カスタムワードを定義
[ 'Delayed message' ] PRINT
'MSG' DEF

# 2秒後に実行
'MSG' [ 2000 ] WAIT

# TIMESと組み合わせる
'MSG' [ 1000 ] WAIT [ 3 ] TIMES  # 1秒待機後、3回実行

## 注意
- カスタムワードのみが対象です
- ワード名は文字列（'または"）で囲みます
- 待機時間はミリ秒単位です
- 1000ms = 1秒"#.to_string(),

        "EVAL" => r#"# EVAL - ベクトルの評価実行

## 説明
ベクトルをコード片とみなし、その内容を実行します。
動的にコードを生成して実行したい場合に使用します。

## 使用法
[ code_vector ] EVAL

## 例
# [ 1 2 + ] を実行する
[ [ 1 ] [ 2 ] + ] EVAL
# 結果: [ 3 ]

# 文字列からワードを組み立てて実行
[ 'PRINT' ] EVAL
[ 'Hello' ] SWAP
# → 'Hello' と表示される"#.to_string(),

        // === 高階関数 ===
        "MAP" => r#"# MAP - 各要素への関数適用

## 説明
ベクトルの各要素に指定したワードを適用し、
結果を新しいベクトルとして返します。

## 使用法
[ data ] 'WORD' MAP

## 例
# 各要素を2倍にする
[ 2 ] * 'DOUBLE' DEF
[ 1 2 3 4 5 ] 'DOUBLE' MAP
# 結果: [ 2 4 6 8 10 ]

# 各要素を二乗する
: [ 1 ] GET DUP * 'SQUARE' DEF
[ 3 4 5 ] 'SQUARE' MAP
# 結果: [ 9 16 25 ]

## 注意
- ワードは1要素（ベクトル）を受け取り、1要素（ベクトル）を返す必要があります
- カスタムワード、組み込みワードのどちらも使用可能です"#.to_string(),

        "FILTER" => r#"# FILTER - 条件による絞り込み

## 説明
ベクトルの各要素に指定したワードを適用し、
結果が真（TRUE）の要素だけを残します。

## 使用法
[ data ] 'WORD' FILTER

## 例
# 5より大きい要素だけを残す
[ 5 ] > 'IS-BIG' DEF
[ 3 7 2 8 1 9 ] 'IS-BIG' FILTER
# 結果: [ 7 8 9 ]

# 偶数だけを残す
[ 2 ] % [ 0 ] = 'IS-EVEN' DEF
[ 1 2 3 4 5 6 ] 'IS-EVEN' FILTER
# 結果: [ 2 4 6 ]

## 注意
- ワードは1要素を受け取り、真偽値を返す必要があります
- カスタムワード、組み込みワードのどちらも使用可能です"#.to_string(),

        "REDUCE" => r#"# REDUCE - 畳み込み演算

## 説明
ベクトルの要素を順次処理し、1つの値に集約します。
アキュムレータと各要素を指定したワードに渡して処理します。

## 使用法
[ data ] [ initial_value ] 'WORD' REDUCE

## 例
# 合計を計算
+ 'ADD' DEF
[ 1 2 3 4 5 ] [ 0 ] 'ADD' REDUCE
# 結果: [ 15 ]

# 積を計算
* 'MUL' DEF
[ 1 2 3 4 ] [ 1 ] 'MUL' REDUCE
# 結果: [ 24 ]

# 最大値を求める
[ 2 ] GET [ 1 ] GET > : [ 2 ] GET : [ 1 ] GET 'MAX2' DEF
[ 3 7 2 9 1 ] [ 0 ] 'MAX2' REDUCE
# 結果: [ 9 ]

## 注意
- ワードは2要素（アキュムレータと現在値）を受け取り、1要素を返す必要があります
- 初期値は必須です
- カスタムワード、組み込みワードのどちらも使用可能です"#.to_string(),

        "EACH" => r#"# EACH - 各要素への副作用実行

## 説明
ベクトルの各要素に指定したワードを適用します。
MAPと違い、結果は返さず副作用（出力など）のみを実行します。

## 使用法
[ data ] 'WORD' EACH

## 例
# 各要素を出力
[ 1 2 3 ] 'PRINT' EACH
# Output: [1] [2] [3]
# スタック: 空

# カスタムワードで処理
'Value: ' CONCAT PRINT 'SHOW' DEF
[ 'A' 'B' 'C' ] 'SHOW' EACH
# Output: Value: A Value: B Value: C

## 注意
- ワードは1要素を受け取ります
- 戻り値は破棄されます
- スタックには何も残りません
- カスタムワード、組み込みワードのどちらも使用可能です"#.to_string(),

        // === 入出力 ===
        "PRINT" => r#"# PRINT - 値の出力

## 説明
スタックトップの値をOutputエリアに出力します。

## 使用法
[ value ] PRINT

## 例
[ 42 ] PRINT  # Output: [42]

[ 'Hello' ] PRINT  # Output: ['Hello']

[ 1 2 3 ] PRINT  # Output: [1 2 3]"#.to_string(),

        // === システム ===
        "DEF" => r#"# DEF - カスタムワードの定義

## 説明
新しいカスタムワードを定義します。
スタックからワード名と定義を取得します。

## 使用法（スタック経由）
: body ; 'NAME' DEF
または
: body ; 'NAME' 'DESCRIPTION' DEF

## 使用法（複数行記法）
body-line1
body-line2
'NAME' DEF

または

body-line1
body-line2
'NAME' 'DESCRIPTION' DEF

## 例
# 基本的な定義
[ 'Hello' ] PRINT
'GREET' DEF

# 説明付きの定義
[ 1 ] [ 2 ] +
'ADD_ONE_TWO' '1と2を足す' DEF

# 条件分岐を含む定義
[ 10 ] > : [ 'Large' ] PRINT : [ 'Small' ] PRINT
'SIZE_CHECK' DEF"#.to_string(),

        "DEL" => r#"# DEL - カスタムワードの削除

## 説明
定義済みのカスタムワードを削除します。
組み込みワードは削除できません。

## 使用法
'WORD_NAME' DEL
または
"WORD_NAME" DEL

## 例
'MY_WORD' DEL
"ANOTHER_WORD" DEL"#.to_string(),

        "?" => r#"# ? - ワード定義の表示

## 説明
ワードの定義や詳細情報をエディタに表示します。

カスタムワードの場合：定義時のソースコードを表示
組み込みワードの場合：詳細説明と使用例を表示

## 使用法
'WORD_NAME' ?
または
"WORD_NAME" ?

## 例
# カスタムワードの定義を確認
'MY_WORD' ?

# 組み込みワードの使い方を確認
'GET' ?
'+' ?
'MAP' ?
'FILTER' ?
'?' ?  # このヘルプを表示"#.to_string(),

        "RESET" => r#"# RESET - システムのリセット

## 説明
すべてのカスタムワード定義とデータベースをクリアし、
システムを初期状態に戻します。

⚠️ 警告：この操作は取り消せません！

## 使用法
RESET

## 例
# すべてをクリア
RESET"#.to_string(),

        // === オーディオ ===
        "AUDIO" => r#"# AUDIO - オーディオ再生

## 説明
数値や分数を音として再生します。

## 使用法
[ notes ] AUDIO

## 例
# 単音の再生
[ 440 ] AUDIO  # A4 (ラ)

# 和音の再生（分数）
[ 3/2 ] AUDIO  # 完全五度

# シーケンス
[ 440 523 659 ] AUDIO  # C-E-A"#.to_string(),

        _ => format!("# {}\n\n組み込みワードです。\n詳細な説明はまだ用意されていません。", name)
    }
}
