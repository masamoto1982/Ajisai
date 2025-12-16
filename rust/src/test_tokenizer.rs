// test_tokenizer.rs - 空白区切りトークナイザーのテスト

#[cfg(test)]
mod test_tokenizer {
    use crate::tokenizer::tokenize_with_custom_words;
    use crate::types::Token;
    use std::collections::HashSet;

    // === コメント処理のテスト ===

    #[test]
    fn test_comment_basic() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("1 2 # this is a comment", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
        ]);
    }

    #[test]
    fn test_comment_inline() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("1 2 # comment\n3 4", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::LineBreak,
            Token::Number("3".to_string()),
            Token::Number("4".to_string()),
        ]);
    }

    #[test]
    fn test_comment_no_newline() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("1 # comment", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
        ]);
    }

    #[test]
    fn test_comment_with_sharp_in_string() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("'#not a comment' 1", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::String("#not a comment".to_string()),
            Token::Number("1".to_string()),
        ]);
    }

    #[test]
    fn test_multiple_comments() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("# line 1\n# line 2\n1 2", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
        ]);
    }

    // === 引用文字列のテスト ===

    #[test]
    fn test_flexible_quotes_single_with_single_inside() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("'He'llo'", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::String("He'llo".to_string()),
        ]);
    }

    #[test]
    fn test_flexible_quotes_double_with_double_inside() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("\"He\"llo\"", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::String("He\"llo".to_string()),
        ]);
    }

    #[test]
    fn test_flexible_quotes_single_with_double_inside() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("'He\"llo'", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::String("He\"llo".to_string()),
        ]);
    }

    #[test]
    fn test_flexible_quotes_double_with_single_inside() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("\"He'llo\"", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::String("He'llo".to_string()),
        ]);
    }

    #[test]
    fn test_flexible_quotes_with_space_delimiter() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("'Hello' 'World'", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::String("Hello".to_string()),
            Token::String("World".to_string()),
        ]);
    }

    #[test]
    fn test_flexible_quotes_with_bracket_delimiter() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("['test']", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::String("test".to_string()),
            Token::VectorEnd,
        ]);
    }

    // === 日本語ワードサポートのテスト（空白区切り） ===

    #[test]
    fn test_japanese_word_with_whitespace() {
        let custom_words = HashSet::new();

        // 空白で区切られた日本語ワード
        let result = tokenize_with_custom_words("2 3 足す", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::Symbol("足す".to_string()),
        ]);
    }

    #[test]
    fn test_japanese_word_boundary() {
        let custom_words = HashSet::new();

        // 日本語ワードだけの入力
        let result = tokenize_with_custom_words("足す", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Symbol("足す".to_string()),
        ]);

        // 複数の日本語ワード（空白区切り）
        let result2 = tokenize_with_custom_words("2 足す 3 掛ける 4", &custom_words).unwrap();
        assert_eq!(result2, vec![
            Token::Number("2".to_string()),
            Token::Symbol("足す".to_string()),
            Token::Number("3".to_string()),
            Token::Symbol("掛ける".to_string()),
            Token::Number("4".to_string()),
        ]);
    }

    #[test]
    fn test_mixed_japanese_english() {
        let custom_words = HashSet::new();

        let result = tokenize_with_custom_words("'Hello' 出力する", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::String("Hello".to_string()),
            Token::Symbol("出力する".to_string()),
        ]);
    }

    #[test]
    fn test_hiragana_katakana_kanji() {
        let custom_words = HashSet::new();

        // ひらがな
        let result = tokenize_with_custom_words("あいうえお", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Symbol("あいうえお".to_string()),
        ]);

        // カタカナ
        let result2 = tokenize_with_custom_words("アイウエオ", &custom_words).unwrap();
        assert_eq!(result2, vec![
            Token::Symbol("アイウエオ".to_string()),
        ]);

        // 漢字
        let result3 = tokenize_with_custom_words("合計", &custom_words).unwrap();
        assert_eq!(result3, vec![
            Token::Symbol("合計".to_string()),
        ]);

        // 混在
        let result4 = tokenize_with_custom_words("ひらがなカタカナ漢字", &custom_words).unwrap();
        assert_eq!(result4, vec![
            Token::Symbol("ひらがなカタカナ漢字".to_string()),
        ]);
    }

    #[test]
    fn test_japanese_with_operators() {
        let custom_words = HashSet::new();

        // 演算子と日本語（空白で区切る）
        let result = tokenize_with_custom_words("1 + 2 結果", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("2".to_string()),
            Token::Symbol("結果".to_string()),
        ]);
    }

    // === 数値のテスト ===

    #[test]
    fn test_number_parsing() {
        let custom_words = HashSet::new();

        // 整数
        let result = tokenize_with_custom_words("123", &custom_words).unwrap();
        assert_eq!(result, vec![Token::Number("123".to_string())]);

        // 小数
        let result2 = tokenize_with_custom_words("123.456", &custom_words).unwrap();
        assert_eq!(result2, vec![Token::Number("123.456".to_string())]);

        // 負の数
        let result3 = tokenize_with_custom_words("-123", &custom_words).unwrap();
        assert_eq!(result3, vec![Token::Number("-123".to_string())]);

        // 科学的記数法
        let result4 = tokenize_with_custom_words("1.5e10", &custom_words).unwrap();
        assert_eq!(result4, vec![Token::Number("1.5e10".to_string())]);
    }

    #[test]
    fn test_operator_symbols() {
        let custom_words = HashSet::new();

        // + と - は単独で演算子
        let result = tokenize_with_custom_words("+ -", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Symbol("+".to_string()),
            Token::Symbol("-".to_string()),
        ]);

        // 数値と区別
        let result2 = tokenize_with_custom_words("1 + 2 - 3", &custom_words).unwrap();
        assert_eq!(result2, vec![
            Token::Number("1".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("2".to_string()),
            Token::Symbol("-".to_string()),
            Token::Number("3".to_string()),
        ]);
    }

    // === キーワードのテスト ===

    #[test]
    fn test_keywords() {
        let custom_words = HashSet::new();

        let result = tokenize_with_custom_words("TRUE FALSE NIL", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Boolean(true),
            Token::Boolean(false),
            Token::Nil,
        ]);

        // 大文字小文字を区別しない
        let result2 = tokenize_with_custom_words("true false nil", &custom_words).unwrap();
        assert_eq!(result2, vec![
            Token::Boolean(true),
            Token::Boolean(false),
            Token::Nil,
        ]);
    }

    // === ブラケットのテスト ===

    #[test]
    fn test_brackets() {
        let custom_words = HashSet::new();

        // Phase 2: [] のみサポート
        let result = tokenize_with_custom_words("[ 1 2 3 ]", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::VectorEnd,
        ]);

        // {} と () は削除されたため、エラーとして扱われる
        let result2 = tokenize_with_custom_words("{ a b c }", &custom_words);
        assert!(result2.is_err(), "Curly brackets should cause an error");
    }

    // === 複雑なパターンのテスト ===

    #[test]
    fn test_complex_expression() {
        let custom_words = HashSet::new();

        let result = tokenize_with_custom_words(
            "[ 1 2 3 ] LENGTH '結果' PRINT",
            &custom_words
        ).unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::VectorEnd,
            Token::Symbol("LENGTH".to_string()),
            Token::String("結果".to_string()),
            Token::Symbol("PRINT".to_string()),
        ]);
    }

    #[test]
    fn test_multiline_expression() {
        let custom_words = HashSet::new();

        let result = tokenize_with_custom_words(
            "1 2 +\n3 4 *\n",
            &custom_words
        ).unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Symbol("+".to_string()),
            Token::LineBreak,
            Token::Number("3".to_string()),
            Token::Number("4".to_string()),
            Token::Symbol("*".to_string()),
        ]);
    }

    // === 空白のテスト ===

    #[test]
    fn test_whitespace_handling() {
        let custom_words = HashSet::new();

        // タブやスペースは同じように扱われる
        let result = tokenize_with_custom_words("1\t2  3   4", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::Number("4".to_string()),
        ]);
    }

    // === エッジケースのテスト ===

    #[test]
    fn test_empty_input() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("", &custom_words).unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_only_whitespace() {
        let custom_words = HashSet::new();
        let result = tokenize_with_custom_words("   \n  \t  ", &custom_words).unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_symbol_with_special_chars() {
        let custom_words = HashSet::new();

        // FORTHスタイルのワード（?や!を含む）
        let result = tokenize_with_custom_words("PRINT? SET!", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Symbol("PRINT?".to_string()),
            Token::Symbol("SET!".to_string()),
        ]);
    }

    // === 分数リテラルのテスト ===

    #[test]
    fn test_fraction_literal() {
        let custom_words = HashSet::new();

        // 基本的な分数
        let result = tokenize_with_custom_words("1/3", &custom_words).unwrap();
        assert_eq!(result, vec![Token::Number("1/3".to_string())]);

        // 負の分数
        let result2 = tokenize_with_custom_words("-1/3", &custom_words).unwrap();
        assert_eq!(result2, vec![Token::Number("-1/3".to_string())]);

        // 分数と他のトークン
        let result3 = tokenize_with_custom_words("1/3 + 2/5", &custom_words).unwrap();
        assert_eq!(result3, vec![
            Token::Number("1/3".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("2/5".to_string()),
        ]);
    }

    #[test]
    fn test_fraction_in_vector() {
        let custom_words = HashSet::new();

        let result = tokenize_with_custom_words("[ 1/2 3/4 ]", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1/2".to_string()),
            Token::Number("3/4".to_string()),
            Token::VectorEnd,
        ]);
    }

    #[test]
    fn test_invalid_fraction() {
        let custom_words = HashSet::new();

        // "1/" は不完全なのでシンボルとして扱われる
        let result = tokenize_with_custom_words("1/", &custom_words).unwrap();
        assert_eq!(result, vec![Token::Symbol("1/".to_string())]);

        // "1/a" もシンボル
        let result2 = tokenize_with_custom_words("1/a", &custom_words).unwrap();
        assert_eq!(result2, vec![Token::Symbol("1/a".to_string())]);
    }

    // === 閉じられていない文字列のテスト ===

    #[test]
    fn test_unclosed_string_error() {
        let custom_words = HashSet::new();

        let result = tokenize_with_custom_words("'unclosed string", &custom_words);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unclosed string"));
    }

    #[test]
    fn test_unclosed_double_quote_error() {
        let custom_words = HashSet::new();

        let result = tokenize_with_custom_words("\"unclosed", &custom_words);
        assert!(result.is_err());
    }

    // === Dot operator テスト ===

    #[test]
    fn test_dot_operator() {
        let custom_words = HashSet::new();

        let result = tokenize_with_custom_words(". + 3", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::Symbol(".".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("3".to_string()),
        ]);

        let result2 = tokenize_with_custom_words(".. + 3", &custom_words).unwrap();
        assert_eq!(result2, vec![
            Token::Symbol("..".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("3".to_string()),
        ]);
    }

    #[test]
    fn test_dot_operator_with_vector() {
        let custom_words = HashSet::new();

        let result = tokenize_with_custom_words("[ 1 2 3 ] . LENGTH", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::VectorEnd,
            Token::Symbol(".".to_string()),
            Token::Symbol("LENGTH".to_string()),
        ]);

        let result2 = tokenize_with_custom_words("a b c [ 1 ] .. GET", &custom_words).unwrap();
        assert_eq!(result2, vec![
            Token::Symbol("a".to_string()),
            Token::Symbol("b".to_string()),
            Token::Symbol("c".to_string()),
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::VectorEnd,
            Token::Symbol("..".to_string()),
            Token::Symbol("GET".to_string()),
        ]);
    }

    // === 空白なしブラケットのテスト ===

    #[test]
    fn test_bracket_without_space() {
        let custom_words = HashSet::new();

        // 空白なしの基本ケース
        let result = tokenize_with_custom_words("[1]", &custom_words).unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::VectorEnd,
        ]);

        // 複数要素
        let result2 = tokenize_with_custom_words("[1 2 3]", &custom_words).unwrap();
        assert_eq!(result2, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::VectorEnd,
        ]);

        // ネストされた構造
        let result3 = tokenize_with_custom_words("[[1][2]]", &custom_words).unwrap();
        assert_eq!(result3, vec![
            Token::VectorStart,
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::VectorEnd,
            Token::VectorStart,
            Token::Number("2".to_string()),
            Token::VectorEnd,
            Token::VectorEnd,
        ]);

        // ワードとの組み合わせ
        let result4 = tokenize_with_custom_words("[1 2]+[3 4]", &custom_words).unwrap();
        assert_eq!(result4, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::VectorEnd,
            Token::Symbol("+".to_string()),
            Token::VectorStart,
            Token::Number("3".to_string()),
            Token::Number("4".to_string()),
            Token::VectorEnd,
        ]);
    }
}
