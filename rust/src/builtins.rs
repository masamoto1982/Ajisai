// rust/src/builtins.rs (わかりやすい機能説明版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // 算術演算（記号・7個）
    register_builtin(dictionary, "+", "2つの数値またはベクトルを加算します。例: 3 4 + → 7", "Arithmetic");
    register_builtin(dictionary, "-", "2つの数値またはベクトルを減算します。例: 10 3 - → 7", "Arithmetic");
    register_builtin(dictionary, "*", "2つの数値またはベクトルを乗算します。例: 6 7 * → 42", "Arithmetic");
    register_builtin(dictionary, "/", "2つの数値またはベクトルを除算します。例: 15 3 / → 5", "Arithmetic");
    register_builtin(dictionary, ">", "左辺が右辺より大きいかを判定します。例: 5 3 > → true", "Comparison");
    register_builtin(dictionary, ">=", "左辺が右辺以上かを判定します。例: 5 5 >= → true", "Comparison");
    register_builtin(dictionary, "=", "左辺と右辺が等しいかを判定します。例: 5 5 = → true", "Comparison");
    
    // 論理・存在（漢字・5個）
    register_builtin(dictionary, "否", "真偽値を反転します。例: true 否 → false", "Logic");
    register_builtin(dictionary, "且", "2つの真偽値の論理積を取ります。例: true false 且 → false", "Logic");
    register_builtin(dictionary, "或", "2つの真偽値の論理和を取ります。例: true false 或 → true", "Logic");
    register_builtin(dictionary, "無", "値がnilかどうかをチェックします。例: nil 無 → true", "Logic");
    register_builtin(dictionary, "有", "値がnilでないかをチェックします。例: 5 有 → true", "Logic");

    // Vector操作（基本・11個）
    register_builtin(dictionary, "頭", "ベクトルの先頭要素を取得します。例: [ 10 20 30 ] 頭 → 10", "Vector");
    register_builtin(dictionary, "尾", "ベクトルの先頭を除いた残り部分を取得します。例: [ 10 20 30 ] 尾 → [ 20 30 ]", "Vector");
    register_builtin(dictionary, "接", "要素をベクトルの先頭に追加します。例: 5 [ 1 2 3 ] 接 → [ 5 1 2 3 ]", "Vector");
    register_builtin(dictionary, "離", "ベクトルを先頭要素と残りに分離します。例: [ 5 1 2 3 ] 離 → 5 [ 1 2 3 ]", "Vector");
    register_builtin(dictionary, "追", "要素をベクトルの末尾に追加します。例: [ 1 2 ] 3 追 → [ 1 2 3 ]", "Vector");
    register_builtin(dictionary, "除", "ベクトルから末尾要素を除去します。例: [ 1 2 3 ] 除 → [ 1 2 ] 3", "Vector");
    register_builtin(dictionary, "複", "値を複製します。例: 5 複 → 5 5", "Vector");
    register_builtin(dictionary, "選", "条件に基づいて値を選択します。例: 10 20 true 選 → 10", "Vector");
    register_builtin(dictionary, "数", "ベクトルの要素数を取得します。例: [ 1 2 3 4 5 ] 数 → 5", "Vector");
    register_builtin(dictionary, "在", "ベクトルの指定位置の要素を取得します。例: [ 10 20 30 ] 1 在 → 20", "Vector");
    register_builtin(dictionary, "行", "値の出力またはベクトルコードの実行を行います。例: 42 行 → 42を出力", "Vector");
    
    // Vector操作（新機能・13個）
    register_builtin(dictionary, "結", "2つのベクトルを結合します。例: [ 1 2 ] [ 3 4 ] 結 → [ 1 2 3 4 ]", "Vector");
    register_builtin(dictionary, "切", "ベクトルを指定位置で分割します。例: [ 1 2 3 4 ] 2 切 → [ 1 2 ] [ 3 4 ]", "Vector");
    register_builtin(dictionary, "反", "ベクトルの順序を反転します。例: [ 1 2 3 ] 反 → [ 3 2 1 ]", "Vector");
    register_builtin(dictionary, "挿", "ベクトルの指定位置に要素を挿入します。例: [ 1 3 ] 1 2 挿 → [ 1 2 3 ]", "Vector");
    register_builtin(dictionary, "消", "ベクトルの指定位置の要素を削除します。例: [ 1 2 3 ] 1 消 → [ 1 3 ] 2", "Vector");
    register_builtin(dictionary, "探", "ベクトル内の要素を検索しインデックスを返します。例: [ 10 20 30 ] 20 探 → 1", "Vector");
    register_builtin(dictionary, "含", "ベクトルが指定要素を含むかをチェックします。例: [ 1 2 3 ] 2 含 → true", "Vector");
    register_builtin(dictionary, "換", "ベクトルの指定位置の要素を置換します。例: [ 1 2 3 ] 1 9 換 → [ 1 9 3 ] 2", "Vector");
    register_builtin(dictionary, "抽", "条件に合う要素のみを抽出します。例: [ 1 2 3 4 ] [ 2 > ] 抽 → [ 3 4 ]", "Vector");
    register_builtin(dictionary, "変", "各要素に変換処理を適用します。例: [ 1 2 3 ] [ 2 * ] 変 → [ 2 4 6 ]", "Vector");
    register_builtin(dictionary, "畳", "ベクトルを畳み込み処理で1つの値にします。例: [ 1 2 3 4 ] [ + ] 畳 → 10", "Vector");
    register_builtin(dictionary, "並", "ベクトルの要素をソートします。例: [ 3 1 2 ] 並 → [ 1 2 3 ]", "Vector");
    register_builtin(dictionary, "空", "ベクトルが空かどうかをチェックします。例: [ ] 空 → true", "Vector");
    
    // 制御・システム（漢字・4個）
    register_builtin(dictionary, "定", "新しいワードを定義します。例: [ 複 * ] \"平方\" 定", "Control");
    register_builtin(dictionary, "削", "カスタムワードを削除します。例: \"平方\" 削", "Control");
    register_builtin(dictionary, "成", "条件が真の時に処理を実行します。例: error \"修復処理\" 成", "Control");
    register_builtin(dictionary, "忘", "全データを消去してリセットします。例: 忘", "System");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str, category: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
        category: Some(category.to_string()),
    });
}
