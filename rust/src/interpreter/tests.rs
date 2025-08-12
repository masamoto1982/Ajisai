// rust/src/interpreter/tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Value, ValueType, Fraction};

    fn create_test_interpreter() -> Interpreter {
        Interpreter::new()
    }

    fn assert_stack_top_number(interp: &Interpreter, expected_num: i64, expected_den: i64) {
        assert!(!interp.stack.is_empty(), "Stack is empty");
        let top = interp.stack.last().unwrap();
        match &top.val_type {
            ValueType::Number(frac) => {
                let expected = Fraction::new(expected_num, expected_den);
                assert_eq!(*frac, expected, "Expected {}/{}, got {}/{}", 
                          expected.numerator, expected.denominator,
                          frac.numerator, frac.denominator);
            }
            _ => panic!("Expected number, got {:?}", top.val_type),
        }
    }

    fn assert_stack_top_boolean(interp: &Interpreter, expected: bool) {
        assert!(!interp.stack.is_empty(), "Stack is empty");
        let top = interp.stack.last().unwrap();
        match &top.val_type {
            ValueType::Boolean(b) => assert_eq!(*b, expected),
            _ => panic!("Expected boolean, got {:?}", top.val_type),
        }
    }

    fn assert_stack_top_vector_length(interp: &Interpreter, expected_len: usize) {
        assert!(!interp.stack.is_empty(), "Stack is empty");
        let top = interp.stack.last().unwrap();
        match &top.val_type {
            ValueType::Vector(v) => assert_eq!(v.len(), expected_len),
            _ => panic!("Expected vector, got {:?}", top.val_type),
        }
    }

    // === 基本的な算術演算のテスト ===

    #[test]
    fn test_arithmetic_infix_notation() {
        let mut interp = create_test_interpreter();
        
        // 中置記法: 3 + 4
        interp.execute("3 + 4").unwrap();
        assert_stack_top_number(&interp, 7, 1);
        
        interp.stack.clear();
        
        // 中置記法: 10 - 3
        interp.execute("10 - 3").unwrap();
        assert_stack_top_number(&interp, 7, 1);
        
        interp.stack.clear();
        
        // 中置記法: 6 * 7
        interp.execute("6 * 7").unwrap();
        assert_stack_top_number(&interp, 42, 1);
        
        interp.stack.clear();
        
        // 中置記法: 15 / 3
        interp.execute("15 / 3").unwrap();
        assert_stack_top_number(&interp, 5, 1);
    }

    #[test]
    fn test_arithmetic_postfix_notation() {
        let mut interp = create_test_interpreter();
        
        // 後置記法（RPN）: 3 4 +
        interp.execute("3 4 +").unwrap();
        assert_stack_top_number(&interp, 7, 1);
        
        interp.stack.clear();
        
        // 後置記法: 10 3 -
        interp.execute("10 3 -").unwrap();
        assert_stack_top_number(&interp, 7, 1);
    }

    #[test]
    fn test_arithmetic_prefix_notation() {
        let mut interp = create_test_interpreter();
        
        // 前置記法: + 3 4
        interp.execute("+ 3 4").unwrap();
        assert_stack_top_number(&interp, 7, 1);
        
        interp.stack.clear();
        
        // 前置記法: * 6 7
        interp.execute("* 6 7").unwrap();
        assert_stack_top_number(&interp, 42, 1);
    }

    // === 分数のテスト ===

    #[test]
    fn test_fractions() {
        let mut interp = create_test_interpreter();
        
        // 分数の加算: 1/2 + 1/3 = 5/6
        interp.execute("1/2 1/3 +").unwrap();
        assert_stack_top_number(&interp, 5, 6);
        
        interp.stack.clear();
        
        // 小数点記法: 0.5 + 0.25 = 0.75 = 3/4
        interp.execute("0.5 0.25 +").unwrap();
        assert_stack_top_number(&interp, 3, 4);
    }

    // === ベクトル操作のテスト ===

    #[test]
    fn test_vector_literals() {
        let mut interp = create_test_interpreter();
        
        // ベクトルリテラル
        interp.execute("[ 1 2 3 ]").unwrap();
        assert_stack_top_vector_length(&interp, 3);
    }

    #[test]
    fn test_vector_implicit_iteration() {
        let mut interp = create_test_interpreter();
        
        // ベクトルに対する暗黙の反復: [ 1 2 3 ] DUP
        interp.execute("[ 1 2 3 ] DUP").unwrap();
        assert_eq!(interp.stack.len(), 2);
        assert_stack_top_vector_length(&interp, 3);
        
        interp.stack.clear();
        
        // ベクトルと数値の演算: [ 1 2 3 ] 2 *
        interp.execute("[ 1 2 3 ] 2 *").unwrap();
        assert_stack_top_vector_length(&interp, 3);
        
        // 結果の各要素をチェック
        let top = interp.stack.last().unwrap();
        if let ValueType::Vector(v) = &top.val_type {
            for (i, expected) in [2, 4, 6].iter().enumerate() {
                if let ValueType::Number(frac) = &v[i].val_type {
                    assert_eq!(frac.numerator, *expected);
                    assert_eq!(frac.denominator, 1);
                }
            }
        }
    }

    // === スタック操作のテスト ===

    #[test]
    fn test_stack_operations() {
        let mut interp = create_test_interpreter();
        
        // DUP
        interp.execute("42 DUP").unwrap();
        assert_eq!(interp.stack.len(), 2);
        assert_stack_top_number(&interp, 42, 1);
        
        // DROP
        interp.execute("DROP").unwrap();
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_number(&interp, 42, 1);
        
        // SWAP
        interp.execute("100 SWAP").unwrap();
        assert_eq!(interp.stack.len(), 2);
        assert_stack_top_number(&interp, 42, 1);
    }

    // === レジスタ操作のテスト ===

    #[test]
    fn test_register_operations() {
        let mut interp = create_test_interpreter();
        
        // >R (to register)
        interp.execute("42 >R").unwrap();
        assert!(interp.stack.is_empty());
        assert!(interp.register.is_some());
        
        // R@ (fetch register)
        interp.execute("R@").unwrap();
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_number(&interp, 42, 1);
        assert!(interp.register.is_some());
        
        // R> (from register)
        interp.execute("R>").unwrap();
        assert_eq!(interp.stack.len(), 2);
        assert!(interp.register.is_none());
    }

    // === 条件演算のテスト ===

    #[test]
    fn test_conditional_operations() {
        let mut interp = create_test_interpreter();
        
        // 条件選択: true 10 20 ?
        interp.execute("true 10 20 ?").unwrap();
        assert_stack_top_number(&interp, 10, 1);
        
        interp.stack.clear();
        
        // 条件選択: false 10 20 ?
        interp.execute("false 10 20 ?").unwrap();
        assert_stack_top_number(&interp, 20, 1);
    }

    // === 比較演算のテスト ===

    #[test]
    fn test_comparison_operations() {
        let mut interp = create_test_interpreter();
        
        // 5 > 3
        interp.execute("5 3 >").unwrap();
        assert_stack_top_boolean(&interp, true);
        
        interp.stack.clear();
        
        // 3 > 5
        interp.execute("3 5 >").unwrap();
        assert_stack_top_boolean(&interp, false);
        
        interp.stack.clear();
        
        // 5 = 5
        interp.execute("5 5 =").unwrap();
        assert_stack_top_boolean(&interp, true);
    }

    // === カスタムワード定義のテスト ===

    #[test]
    fn test_custom_word_definition() {
        let mut interp = create_test_interpreter();
        
        // 明示的定義: 3 4 + "SEVEN" DEF
        interp.execute("3 4 + \"SEVEN\" DEF").unwrap();
        assert!(interp.dictionary.contains_key("SEVEN"));
        
        // カスタムワードの実行
        interp.execute("SEVEN").unwrap();
        assert_stack_top_number(&interp, 7, 1);
    }

    #[test]
    fn test_auto_word_generation() {
        let mut interp = create_test_interpreter();
        
        // 自動定義: 3 4 +
        let result = interp.execute("3 4 +").unwrap();
        
        // 自動命名されたワードが実行できることを確認
        assert!(interp.auto_named);
        
        if let Some(word_name) = &interp.last_auto_named_word {
            // 生成されたワードが辞書に存在することを確認
            assert!(interp.dictionary.contains_key(word_name));
        }
    }

    // === ベクトル操作関数のテスト ===

    #[test]
    fn test_vector_operations() {
        let mut interp = create_test_interpreter();
        
        // LENGTH
        interp.execute("[ 1 2 3 4 5 ] LENGTH").unwrap();
        assert_stack_top_number(&interp, 5, 1);
        
        interp.stack.clear();
        
        // HEAD
        interp.execute("[ 10 20 30 ] HEAD").unwrap();
        assert_stack_top_number(&interp, 10, 1);
        
        interp.stack.clear();
        
        // TAIL
        interp.execute("[ 10 20 30 ] TAIL").unwrap();
        assert_stack_top_vector_length(&interp, 2);
        
        interp.stack.clear();
        
        // CONS
        interp.execute("5 [ 10 20 ] CONS").unwrap();
        assert_stack_top_vector_length(&interp, 3);
        
        interp.stack.clear();
        
        // APPEND
        interp.execute("[ 10 20 ] 30 APPEND").unwrap();
        assert_stack_top_vector_length(&interp, 3);
    }

    // === エラーケースのテスト ===

    #[test]
    fn test_division_by_zero() {
        let mut interp = create_test_interpreter();
        
        let result = interp.execute("5 0 /");
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::interpreter::error::AjisaiError::DivisionByZero => {},
            _ => panic!("Expected DivisionByZero error"),
        }
    }

    #[test]
    fn test_stack_underflow() {
        let mut interp = create_test_interpreter();
        
        let result = interp.execute("+");
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::interpreter::error::AjisaiError::StackUnderflow => {},
            _ => panic!("Expected StackUnderflow error"),
        }
    }

    #[test]
    fn test_unknown_word() {
        let mut interp = create_test_interpreter();
        
        let result = interp.execute("UNKNOWN_WORD");
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::interpreter::error::AjisaiError::UnknownWord(_) => {},
            _ => panic!("Expected UnknownWord error"),
        }
    }

    // === 複雑な式のテスト ===

    #[test]
    fn test_complex_expressions() {
        let mut interp = create_test_interpreter();
        
        // ネストしたベクトル
        interp.execute("[ [ 1 2 ] [ 3 4 ] ]").unwrap();
        assert_stack_top_vector_length(&interp, 2);
        
        interp.stack.clear();
        
        // 複雑な算術式
        interp.execute("2 3 + 4 *").unwrap(); // (2 + 3) * 4 = 20
        assert_stack_top_number(&interp, 20, 1);
    }

    // === Nil関連のテスト ===

    #[test]
    fn test_nil_operations() {
        let mut interp = create_test_interpreter();
        
        // NIL?
        interp.execute("nil NIL?").unwrap();
        assert_stack_top_boolean(&interp, true);
        
        interp.stack.clear();
        
        interp.execute("42 NIL?").unwrap();
        assert_stack_top_boolean(&interp, false);
        
        interp.stack.clear();
        
        // DEFAULT
        interp.execute("nil 42 DEFAULT").unwrap();
        assert_stack_top_number(&interp, 42, 1);
        
        interp.stack.clear();
        
        interp.execute("10 42 DEFAULT").unwrap();
        assert_stack_top_number(&interp, 10, 1);
    }

    // === 論理演算のテスト ===

    #[test]
    fn test_logical_operations() {
        let mut interp = create_test_interpreter();
        
        // AND
        interp.execute("true true AND").unwrap();
        assert_stack_top_boolean(&interp, true);
        
        interp.stack.clear();
        
        interp.execute("true false AND").unwrap();
        assert_stack_top_boolean(&interp, false);
        
        interp.stack.clear();
        
        // OR
        interp.execute("true false OR").unwrap();
        assert_stack_top_boolean(&interp, true);
        
        interp.stack.clear();
        
        interp.execute("false false OR").unwrap();
        assert_stack_top_boolean(&interp, false);
        
        interp.stack.clear();
        
        // NOT
        interp.execute("true NOT").unwrap();
        assert_stack_top_boolean(&interp, false);
    }

    // === ステップ実行のテスト ===

    #[test]
    fn test_step_execution() {
        let mut interp = create_test_interpreter();
        
        // ステップ実行の初期化
        interp.init_step_execution("3 4 +\n5 6 *").unwrap();
        
        // 最初のステップ
        let has_more = interp.execute_step().unwrap();
        assert!(has_more);
        assert_stack_top_number(&interp, 7, 1);
        
        // 2番目のステップ
        let has_more = interp.execute_step().unwrap();
        assert!(!has_more);
        assert_stack_top_number(&interp, 30, 1);
    }
}
