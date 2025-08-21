// tests/kanji_builtin_tests.rs

#[cfg(test)]
mod tests {
    use ajisai_core::AjisaiInterpreter;

    fn create_interpreter() -> AjisaiInterpreter {
        AjisaiInterpreter::new()
    }

    #[test]
    fn test_kanji_arithmetic() {
        let mut interp = create_interpreter();
        
        // 基本算術
        let result = interp.execute("3 4 +");
        assert!(result.status == "OK");
        let workspace = interp.get_workspace();
        assert_eq!(workspace.len(), 1);
        
        // 論理演算（漢字）
        interp.execute("true false 且");  // AND
        interp.execute("true false 或");  // OR  
        interp.execute("true 否");        // NOT
    }

    #[test]
    fn test_symmetric_pairs() {
        let mut interp = create_interpreter();
        
        // 接/離 対称性テスト
        interp.execute("5 [ 1 2 3 ] 接");  // [5 1 2 3]
        let result = interp.execute("離");  // 5 [1 2 3]
        assert!(result.status == "OK");
        
        // 追/除 対称性テスト
        interp.execute("[ 1 2 ] 3 追");    // [1 2 3]  
        let result = interp.execute("除");  // [1 2] 3
        assert!(result.status == "OK");
        
        // 有/無 対称性テスト
        interp.execute("nil 無");          // true
        interp.execute("5 有");            // true
    }

    #[test]
    fn test_unified_operations() {
        let mut interp = create_interpreter();
        
        // 数（COUNT）- 要素数統一
        interp.execute("[ 1 2 3 4 5 ] 数");  // 5
        
        // 在（AT）- 位置アクセス統一
        interp.execute("1 [ 10 20 30 ] 在"); // 20
        
        // 行（DO）- 実行統一
        interp.execute("42 行");             // 42を出力
        interp.execute("[ 1 2 + ] 行");      // 3を計算
    }

    #[test]
    fn test_new_operations() {
        let mut interp = create_interpreter();
        
        // 複（CLONE）- 複製
        interp.execute("5 複 *");  // 5を複製して掛ける（自乗）
        
        // 選（SELECT）- 条件選択
        interp.execute("true 10 20 選");   // true なら 10
        interp.execute("false 10 20 選");  // false なら 20
    }

    #[test]
    fn test_word_definition() {
        let mut interp = create_interpreter();
        
        // 漢字での定義・削除
        interp.execute("[ 複 * ] \"平方\" 定");  // 平方ワード定義
        interp.execute("5 平方");                // 25
        interp.execute("\"平方\" 削");           // 平方ワード削除
        
        let result = interp.execute("平方");     // エラーになるはず
        assert!(result.status == "ERROR");
    }

    #[test]
    fn test_error_messages() {
        let mut interp = create_interpreter();
        
        // 空ベクトルエラー
        let result = interp.execute("[ ] 頭");
        assert!(result.status == "ERROR");
        assert!(result.message.unwrap().contains("空のベクトル"));
        
        // ワークスペース不足エラー
        let result = interp.execute("+");
        assert!(result.status == "ERROR");
        assert!(result.message.unwrap().contains("Workspace underflow"));
    }

    #[test]
    fn test_beautiful_programs() {
        let mut interp = create_interpreter();
        
        // 漢詩のような美しいプログラム
        interp.execute("[ 1 2 3 4 5 ] 複 数 * 頭 + 行");
        // ベクトル複製、要素数掛け、先頭足し、実行
        
        // 対称性を活用した処理
        interp.execute("[ 1 2 3 ] 複 3 追 離 尾 頭 行");
        // ベクトル複製、3追加、分離、末尾取得、先頭取得、実行
    }
}
