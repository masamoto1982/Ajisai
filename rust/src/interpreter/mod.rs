fn execute_builtin(&mut self, name: &str) -> Result<()> {
    match name {
        // 算術演算（記号）
        "+" => arithmetic::op_add(self),
        "-" => arithmetic::op_sub(self),
        "*" => arithmetic::op_mul(self),
        "/" => arithmetic::op_div(self),
        ">" => arithmetic::op_gt(self),
        ">=" => arithmetic::op_ge(self),
        "=" => arithmetic::op_eq(self),
        
        // 論理演算（漢字メイン + 英語互換）
        "否" | "NOT" => arithmetic::op_not(self),
        "且" | "AND" => arithmetic::op_and(self),
        "或" | "OR" => arithmetic::op_or(self),
        
        // 存在チェック（漢字メイン + 英語互換）
        "無" | "NIL?" => arithmetic::op_nil_check(self),
        "有" | "SOME?" => arithmetic::op_some_check(self),
        
        // Vector操作（漢字メイン + 英語互換）
        "頭" | "HEAD" => vector_ops::op_head(self),
        "尾" | "TAIL" => vector_ops::op_tail(self),
        "接" | "CONS" => vector_ops::op_cons(self),
        "離" | "UNCONS" => vector_ops::op_uncons(self),
        "追" | "APPEND" => vector_ops::op_append(self),
        "除" | "REMOVE_LAST" => vector_ops::op_remove_last(self),
        "複" | "CLONE" => vector_ops::op_clone(self),
        "選" | "SELECT" => vector_ops::op_select(self),
        "数" | "LENGTH" | "COUNT" => vector_ops::op_count(self),
        "在" | "AT" | "NTH" => vector_ops::op_at(self),
        "行" | "DO" => vector_ops::op_do(self),
        
        // 制御・定義（漢字メイン + 英語互換）
        "定" | "DEF" => {
            Err(error::AjisaiError::from("定 should be handled separately"))
        },
        "削" | "DEL" => control::op_del(self),
        "跳" | "LEAP" => leap::op_leap(self),
        
        // システム（漢字メイン + 英語互換）
        "忘" | "AMNESIA" => op_amnesia(self),
        
        _ => Err(error::AjisaiError::UnknownBuiltin(name.to_string())),
    }
}
