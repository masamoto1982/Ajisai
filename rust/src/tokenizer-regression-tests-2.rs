// rust/src/tokenizer-regression-tests-2.rs - トークナイザー追加テスト

#[cfg(test)]
mod tokenizer_regression_tests_2 {
    use crate::tokenizer::tokenize;
    use crate::types::Token;

    // === 空白のテスト ===

    #[test]
    fn test_whitespace_handling() {
        // タブやスペースは同じように扱われる
        let result = tokenize("1\t2  3   4").unwrap();
        assert_eq!(
            result,
            vec![
                Token::Number("1".into()),
                Token::Number("2".into()),
                Token::Number("3".into()),
                Token::Number("4".into()),
            ]
        );
    }

    // === エッジケースのテスト ===

    #[test]
    fn test_empty_input() {
        let result = tokenize("").unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_only_whitespace() {
        let result = tokenize("   \n  \t  ").unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_symbol_with_special_chars() {
        // FORTHスタイルのワード（?や!を含む）
        let result = tokenize("PRINT? SET!").unwrap();
        assert_eq!(
            result,
            vec![Token::Symbol("PRINT?".into()), Token::Symbol("SET!".into()),]
        );
    }

    // === 分数リテラルのテスト ===

    #[test]
    fn test_fraction_literal() {
        // 基本的な分数
        let result = tokenize("1/3").unwrap();
        assert_eq!(result, vec![Token::Number("1/3".into())]);

        // 負の分数
        let result2 = tokenize("-1/3").unwrap();
        assert_eq!(result2, vec![Token::Number("-1/3".into())]);

        // 分数と他のトークン
        let result3 = tokenize("1/3 + 2/5").unwrap();
        assert_eq!(
            result3,
            vec![
                Token::Number("1/3".into()),
                Token::Symbol("+".into()),
                Token::Number("2/5".into()),
            ]
        );
    }

    #[test]
    fn test_fraction_in_vector() {
        let result = tokenize("[ 1/2 3/4 ]").unwrap();
        assert_eq!(
            result,
            vec![
                Token::VectorStart,
                Token::Number("1/2".into()),
                Token::Number("3/4".into()),
                Token::VectorEnd,
            ]
        );
    }

    #[test]
    fn test_invalid_fraction() {
        // "1/" は不完全なのでシンボルとして扱われる
        let result = tokenize("1/").unwrap();
        assert_eq!(result, vec![Token::Symbol("1/".into())]);

        // "1/a" もシンボル
        let result2 = tokenize("1/a").unwrap();
        assert_eq!(result2, vec![Token::Symbol("1/a".into())]);
    }

    // === 閉じられていない文字列のテスト ===

    #[test]
    fn test_unclosed_string_error() {
        let result = tokenize("'unclosed string");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unclosed literal"));
    }

    // === Dot operator テスト ===

    #[test]
    fn test_dot_operator() {
        let result = tokenize(". + 3").unwrap();
        assert_eq!(
            result,
            vec![
                Token::Symbol(".".into()),
                Token::Symbol("+".into()),
                Token::Number("3".into()),
            ]
        );

        let result2 = tokenize(".. + 3").unwrap();
        assert_eq!(
            result2,
            vec![
                Token::Symbol("..".into()),
                Token::Symbol("+".into()),
                Token::Number("3".into()),
            ]
        );
    }

    #[test]
    fn test_dot_operator_with_vector() {
        let result = tokenize("[ 1 2 3 ] . LENGTH").unwrap();
        assert_eq!(
            result,
            vec![
                Token::VectorStart,
                Token::Number("1".into()),
                Token::Number("2".into()),
                Token::Number("3".into()),
                Token::VectorEnd,
                Token::Symbol(".".into()),
                Token::Symbol("LENGTH".into()),
            ]
        );

        let result2 = tokenize("a b c [ 1 ] .. GET").unwrap();
        assert_eq!(
            result2,
            vec![
                Token::Symbol("a".into()),
                Token::Symbol("b".into()),
                Token::Symbol("c".into()),
                Token::VectorStart,
                Token::Number("1".into()),
                Token::VectorEnd,
                Token::Symbol("..".into()),
                Token::Symbol("GET".into()),
            ]
        );
    }

    // === 空白なしブラケットのテスト ===

    #[test]
    fn test_bracket_without_space() {
        // 空白なしの基本ケース
        let result = tokenize("[1]").unwrap();
        assert_eq!(
            result,
            vec![
                Token::VectorStart,
                Token::Number("1".into()),
                Token::VectorEnd,
            ]
        );

        // 複数要素
        let result2 = tokenize("[1 2 3]").unwrap();
        assert_eq!(
            result2,
            vec![
                Token::VectorStart,
                Token::Number("1".into()),
                Token::Number("2".into()),
                Token::Number("3".into()),
                Token::VectorEnd,
            ]
        );

        // ネストされた構造
        let result3 = tokenize("[[1][2]]").unwrap();
        assert_eq!(
            result3,
            vec![
                Token::VectorStart,
                Token::VectorStart,
                Token::Number("1".into()),
                Token::VectorEnd,
                Token::VectorStart,
                Token::Number("2".into()),
                Token::VectorEnd,
                Token::VectorEnd,
            ]
        );

        // ワードとの組み合わせ
        let result4 = tokenize("[1 2]+[3 4]").unwrap();
        assert_eq!(
            result4,
            vec![
                Token::VectorStart,
                Token::Number("1".into()),
                Token::Number("2".into()),
                Token::VectorEnd,
                Token::Symbol("+".into()),
                Token::VectorStart,
                Token::Number("3".into()),
                Token::Number("4".into()),
                Token::VectorEnd,
            ]
        );
    }

    // === 文字列リテラルのテスト ===

    #[test]
    fn test_string_with_double_quote() {
        // 文字列内にダブルクォートを含む
        let result = tokenize("'He said \"Hello\"'").unwrap();
        assert_eq!(result, vec![Token::String("He said \"Hello\"".into()),]);
    }

    #[test]
    fn test_string_with_single_quote() {
        // 文字列内にシングルクォートを含む
        let result = tokenize("'It's fine'").unwrap();
        assert_eq!(result, vec![Token::String("It's fine".into()),]);
    }

    // === Vector Duality - Vectorをコードとして使用するテスト ===

    #[test]
    fn test_vector_as_code_syntax() {
        // Vectorをコードとして記述（新構文）
        let result = tokenize("[ [ 1 ] + ]").unwrap();
        // VectorStart, VectorStart, Number, VectorEnd, Symbol, VectorEnd
        assert_eq!(result.len(), 6);
        assert!(matches!(&result[0], Token::VectorStart));
        assert!(matches!(&result[1], Token::VectorStart));
        assert!(matches!(&result[2], Token::Number(n) if n.as_ref() == "1"));
        assert!(matches!(&result[3], Token::VectorEnd));
        assert!(matches!(&result[4], Token::Symbol(s) if s.as_ref() == "+"));
        assert!(matches!(&result[5], Token::VectorEnd));
    }

    #[test]
    fn test_def_with_vector_code() {
        // DEFでVectorをコードとして使用
        let result = tokenize("[ [ 2 ] * ] 'DOUBLE' DEF").unwrap();
        // VectorStart, VectorStart, Number, VectorEnd, Symbol, VectorEnd, String, Symbol
        assert_eq!(result.len(), 8);
        assert!(matches!(&result[6], Token::String(s) if s.as_ref() == "DOUBLE"));
        assert!(matches!(&result[7], Token::Symbol(s) if s.as_ref() == "DEF"));
    }

    // === シェブロン分岐トークンのテスト ===

    #[test]
    fn test_chevron_branch_token() {
        // >> トークン
        let result = tokenize(">> [ 5 ] [ 3 ] <").unwrap();
        assert_eq!(result[0], Token::ChevronBranch);
    }

    #[test]
    fn test_chevron_default_token() {
        // >>> トークン
        let result = tokenize(">>> [ 0 ]").unwrap();
        assert_eq!(result[0], Token::ChevronDefault);
    }

    #[test]
    fn test_chevron_structure() {
        let result = tokenize(">> [ 5 ] [ 3 ] <\n>> [ 100 ]\n>>> [ 0 ]").unwrap();
        assert!(matches!(&result[0], Token::ChevronBranch));
        assert!(matches!(&result[8], Token::LineBreak));
        assert!(matches!(&result[9], Token::ChevronBranch));
        assert!(matches!(&result[13], Token::LineBreak));
        assert!(matches!(&result[14], Token::ChevronDefault));
    }

    // === コードブロックトークンのテスト ===

    #[test]
    fn test_code_block_tokens() {
        // : と ; トークン
        let result = tokenize(": [ 2 ] * ;").unwrap();
        assert_eq!(result[0], Token::CodeBlockStart);
        assert_eq!(result[result.len() - 1], Token::CodeBlockEnd);
    }

    #[test]
    fn test_code_block_def_syntax() {
        // 新しいDEF構文
        let result = tokenize(": [ 2 ] * ; 'DOUBLE' DEF").unwrap();
        assert_eq!(result[0], Token::CodeBlockStart);
        assert_eq!(result[5], Token::CodeBlockEnd);
        assert!(matches!(&result[6], Token::String(s) if s.as_ref() == "DOUBLE"));
        assert!(matches!(&result[7], Token::Symbol(s) if s.as_ref() == "DEF"));
    }

    // === 廃止された演算子のエラーテスト ===

    #[test]
    fn test_greater_than_error() {
        // 単独の > はエラー
        let result = tokenize("[ 5 ] [ 3 ] >");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("removed"));
    }

    #[test]
    fn test_greater_than_equal_error() {
        // >= はエラー
        let result = tokenize("[ 5 ] [ 3 ] >=");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("removed"));
    }

    #[test]
    fn test_multiline_code_block_with_chevrons() {
        let input = r#":
>> [ 1 ] =
>> [ 10 ]
>>> [ 20 ]
; 'CHECK_ONE' DEF"#;

        let result = tokenize(input).unwrap();

        // Verify key tokens
        assert_eq!(result[0], Token::CodeBlockStart); // :

        // Find CodeBlockEnd
        let code_block_end_index = result.iter().position(|t| matches!(t, Token::CodeBlockEnd));
        assert!(
            code_block_end_index.is_some(),
            "CodeBlockEnd should exist in tokens"
        );
    }
}
