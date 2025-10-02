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
ベクトルを指定されたサイズに分割します。
各サイズの合計が元のベクトルの長さと一致する必要があります。

## 使用法
[ vector ] [ size1 ] [ size2 ] ... SPLIT

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
[ 1 2 ] [ 3 4 ] CONCAT  # → [ 1 2 3 4 ]

# 空ベクトルとの連結
[ 1 2 ] [ ] CONCAT  # → [ 1 2 ]"#.to_string(),

        "REVERSE" => r#"# REVERSE - ベクトルの反転

## 説明
ベクトルの要素の順序を逆転させます。

## 使用法
[ vector ] REVERSE

## 例
[ 1 2 3 4 ] REVERSE  # → [ 4 3 2 1 ]

[ 'a' 'b' 'c' ] REVERSE  # → [ 'c' 'b' 'a' ]"#.to_string(),

        // === 算術演算 ===
        "+" => r#"# + - 加算

## 説明
2つの数値を加算します。すべての数値は分数として扱われます。

## 使用法
[ number1 ] [ number2 ] +

## 例
[ 5 ] [ 3 ] +  # → [ 8 ]

# 分数の計算
[ 1/2 ] [ 1/3 ] +  # → [ 5/6 ]

# 小数も正確に計算
[ 0.1 ] [ 0.2 ] +  # → [ 3/10 ]"#.to_string(),

        "-" => r#"# - - 減算

## 説明
2つの数値を減算します。

## 使用法
[ number1 ] [ number2 ] -

## 例
[ 10 ] [ 3 ] -  # → [ 7 ]

[ 3/4 ] [ 1/4 ] -  # → [ 1/2 ]"#.to_string(),

        "*" => r#"# * - 乗算

## 説明
2つの数値を乗算します。

## 使用法
[ number1 ] [ number2 ] *

## 例
[ 4 ] [ 7 ] *  # → [ 28 ]

[ 2/3 ] [ 3/4 ] *  # → [ 1/2 ]"#.to_string(),

        "/" => r#"# / - 除算

## 説明
2つの数値を除算します。ゼロ除算はエラーになります。

## 使用法
[ number1 ] [ number2 ] /

## 例
[ 15 ] [ 3 ] /  # → [ 5 ]

[ 2/3 ] [ 1/2 ] /  # → [ 4/3 ]"#.to_string(),

        // === 比較演算 ===
        "=" => r#"# = - 等価判定

## 説明
2つの値が等しいかどうかを判定します。

## 使用法
[ value1 ] [ value2 ] =

## 例
[ 5 ] [ 5 ] =  # → [ TRUE ]

[ 5 ] [ 3 ] =  # → [ FALSE ]

[ 'hello' ] [ 'hello' ] =  # → [ TRUE ]"#.to_string(),

        "<" => r#"# < - より小さい

## 説明
最初の数値が2番目の数値より小さいかを判定します。

## 使用法
[ number1 ] [ number2 ] 

## 例
[ 3 ] [ 5 ] <  # → [ TRUE ]

[ 5 ] [ 3 ] <  # → [ FALSE ]"#.to_string(),

        "<=" => r#"# <= - 以下

## 説明
最初の数値が2番目の数値以下かを判定します。

## 使用法
[ number1 ] [ number2 ] <=

## 例
[ 3 ] [ 5 ] <=  # → [ TRUE ]

[ 5 ] [ 5 ] <=  # → [ TRUE ]"#.to_string(),

        ">" => r#"# > - より大きい

## 説明
最初の数値が2番目の数値より大きいかを判定します。

## 使用法
[ number1 ] [ number2 ] >

## 例
[ 7 ] [ 3 ] >  # → [ TRUE ]

[ 3 ] [ 7 ] >  # → [ FALSE ]"#.to_string(),

        ">=" => r#"# >= - 以上

## 説明
最初の数値が2番目の数値以上かを判定します。

## 使用法
[ number1 ] [ number2 ] >=

## 例
[ 7 ] [ 3 ] >=  # → [ TRUE ]

[ 5 ] [ 5 ] >=  # → [ TRUE ]"#.to_string(),

        // === 論理演算 ===
        "AND" => r#"# AND - 論理積

## 説明
2つの真偽値の論理積を計算します。

## 使用法
[ boolean1 ] [ boolean2 ] AND

## 例
[ TRUE ] [ TRUE ] AND  # → [ TRUE ]

[ TRUE ] [ FALSE ] AND  # → [ FALSE ]

[ FALSE ] [ FALSE ] AND  # → [ FALSE ]"#.to_string(),

        "OR" => r#"# OR - 論理和

## 説明
2つの真偽値の論理和を計算します。

## 使用法
[ boolean1 ] [ boolean2 ] OR

## 例
[ TRUE ] [ FALSE ] OR  # → [ TRUE ]

[ FALSE ] [ FALSE ] OR  # → [ FALSE ]"#.to_string(),

        "NOT" => r#"# NOT - 論理否定

## 説明
真偽値を反転させます。

## 使用法
[ boolean ] NOT

## 例
[ TRUE ] NOT  # → [ FALSE ]

[ FALSE ] NOT  # → [ TRUE ]"#.to_string(),

        // === 制御構造 ===
        ":" | ";" => r#"# : または ; - 条件分岐ゲート

## 説明
条件が真の場合のみ、後続の処理を実行する「ゲート」です。
`:` と `;` は同じ意味で使えます。

一行の中で複数のゲートを連鎖させることで、
ケース式のような分岐が実現できます。

## 使用法
condition : action
または
condition ; action

## 複数ゲートの連鎖
cond1 : action1 : cond2 : action2 : default-action

## 例
# 単純な条件分岐
[ 5 ] [ 5 ] = : [ 'Equal' ] PRINT

# 複数条件の連鎖（ケース式）
[ 0 ] = : [ 'Zero' ] PRINT : [ 0 ] > : [ 'Positive' ] PRINT : [ 'Negative' ] PRINT

# カスタムワード定義内で使用
[ 0 ] = : [ 'Zero' ] PRINT
[ 0 ] > : [ 'Positive' ] PRINT
: [ 'Negative' ] PRINT
'CHECK-NUM' DEF"#.to_string(),

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
'TIMES' ?
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
