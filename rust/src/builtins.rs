use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // スタック操作
    register_builtin(dictionary, "DUP", "スタックトップを複製 ( a -- a a )");
    register_builtin(dictionary, "DROP", "スタックトップを削除 ( a -- )");
    register_builtin(dictionary, "SWAP", "上位2つを交換 ( a b -- b a )");
    register_builtin(dictionary, "OVER", "2番目をコピー ( a b -- a b a )");
    register_builtin(dictionary, "ROT", "3番目を最上位へ ( a b c -- b c a )");
    register_builtin(dictionary, "NIP", "2番目を削除 ( a b -- b )");
    
    // レジスタ操作
    register_builtin(dictionary, ">R", "スタックからレジスタへ移動 ( a -- )");
    register_builtin(dictionary, "R>", "レジスタからスタックへ移動 ( -- a )");
    register_builtin(dictionary, "R@", "レジスタの値をコピー ( -- a )");
    
    // ベクトル操作
    register_builtin(dictionary, "LENGTH", "ベクトルの長さ ( vec -- n )");
    register_builtin(dictionary, "HEAD", "最初の要素 ( vec -- elem )");
    register_builtin(dictionary, "TAIL", "最初以外の要素 ( vec -- vec' )");
    register_builtin(dictionary, "CONS", "要素を先頭に追加 ( elem vec -- vec' )");
    register_builtin(dictionary, "APPEND", "要素をベクトルの末尾に追加 ( vec elem -- vec' )");
    register_builtin(dictionary, "REVERSE", "ベクトルを逆順に ( vec -- vec' )");
    register_builtin(dictionary, "NTH", "N番目の要素を取得（負数は末尾から） ( n vec -- elem )");
    
    // スタックベース反復サポート（再帰の構成要素）
    register_builtin(dictionary, "UNCONS", "ベクトルを先頭要素と残りに分解 ( vec -- elem vec' )");
    register_builtin(dictionary, "EMPTY?", "ベクトルが空かチェック ( vec -- bool )");
    
    // 制御構造
    register_builtin(dictionary, "DEF", "新しいワードを定義 ( vec str -- )");
    register_builtin(dictionary, "IF", "条件分岐 ( bool vec vec -- ... )");
    
    // 辞書操作
    register_builtin(dictionary, "DEL", "カスタムワードを削除 ( str -- )");
    
    // 算術演算子（暗黙の反復対応）
    register_builtin(dictionary, "+", "加算 - 暗黙の反復対応 ( a b -- a+b )");
    register_builtin(dictionary, "-", "減算 - 暗黙の反復対応 ( a b -- a-b )");
    register_builtin(dictionary, "*", "乗算 - 暗黙の反復対応 ( a b -- a*b )");
    register_builtin(dictionary, "/", "除算 - 暗黙の反復対応 ( a b -- a/b )");
    
    // 比較演算子（暗黙の反復対応）
    register_builtin(dictionary, ">", "より大きい - 暗黙の反復対応 ( a b -- bool )");
    register_builtin(dictionary, ">=", "以上 - 暗黙の反復対応 ( a b -- bool )");
    register_builtin(dictionary, "=", "等しい ( a b -- bool )");
    register_builtin(dictionary, "<", "より小さい - 暗黙の反復対応 ( a b -- bool )");
    register_builtin(dictionary, "<=", "以下 - 暗黙の反復対応 ( a b -- bool )");
