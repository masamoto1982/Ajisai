// rust/src/builtins.rs (修正版)

use std::collections::HashMap;
use crate::interpreter::WordDefinition;

pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    let builtin_definitions = get_builtin_definitions();
    
    for (name, description) in builtin_definitions {
        dictionary.insert(name.to_string(), WordDefinition {
            tokens: vec![],
            is_builtin: true,
            description: Some(description.to_string()),
            category: None,
        });
    }
}

pub fn get_builtin_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        // 算術・論理演算妖精
        ("+", "加算を行う妖精です"),
        ("/", "除算を行う妖精です"), 
        ("*", "乗算を行う妖精です"),
        ("-", "減算を行う妖精です"),
        ("=", "等価判定を行う妖精です"),
        (">=", "以上判定を行う妖精です"),
        (">", "大小判定を行う妖精です"),
        ("AND", "論理積を計算する妖精です"),
        ("OR", "論理和を計算する妖精です"),
        ("NOT", "論理否定を計算する妖精です"),
        
        // 位置指定操作妖精（0オリジン）
        ("摘", "0オリジンの位置を指定して要素を摘み取る妖精です"),
        ("挿", "0オリジンの位置を指定して要素を挿し込む妖精です"),
        ("換", "0オリジンの位置を指定して要素を置き換える妖精です"),
        ("削", "0オリジンの位置を指定して要素を削り取る妖精です"),
        
        // 量指定操作妖精（1オリジン）
        ("数", "要素数を数える妖精です"),
        ("取", "指定した個数の要素を取り出す妖精です"),      // 追加！
        ("捨", "指定した個数の要素を捨てる妖精です"),      // 追加！
        ("重", "指定した回数だけ重複させる妖精です"),
        ("分", "指定したサイズで分け隔てる妖精です"),
        
        // Vector操作妖精
        ("結", "複数の部分を結び合わせる妖精です"),
        ("跳", "処理を跳び移す妖精です"),
        
        // 妖精管理妖精
        ("招", "新しい妖精を招き寄せる妖精です"),
        ("払", "妖精を払い除ける妖精です"),
    ]
}
