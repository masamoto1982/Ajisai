

#[cfg(test)]
mod tokenizer_regression_tests {
    use crate::tokenizer::tokenize;
    use crate::types::Token;


    #[test]
    fn test_comment_basic() {
        let result = tokenize("1 2 # this is a comment").unwrap();
        assert_eq!(
            result,
            vec![Token::Number("1".into()), Token::Number("2".into()),]
        );
    }

    #[test]
    fn test_comment_inline() {
        let result = tokenize("1 2 # comment\n3 4").unwrap();
        assert_eq!(
            result,
            vec![
                Token::Number("1".into()),
                Token::Number("2".into()),
                Token::LineBreak,
                Token::Number("3".into()),
                Token::Number("4".into()),
            ]
        );
    }

    #[test]
    fn test_comment_no_newline() {
        let result = tokenize("1 # comment").unwrap();
        assert_eq!(result, vec![Token::Number("1".into()),]);
    }

    #[test]
    fn test_comment_adjacent_to_number() {
        let result = tokenize("123#comment").unwrap();
        assert_eq!(result, vec![Token::Number("123".into()),]);
    }

    #[test]
    fn test_comment_adjacent_to_fraction() {
        let result = tokenize("1/3#これはコメント").unwrap();
        assert_eq!(result, vec![Token::Number("1/3".into()),]);
    }

    #[test]
    fn test_comment_with_sharp_in_string() {
        let result = tokenize("'#not a comment' 1").unwrap();
        assert_eq!(
            result,
            vec![
                Token::String("#not a comment".into()),
                Token::Number("1".into()),
            ]
        );
    }

    #[test]
    fn test_multiple_comments() {
        let result = tokenize("# line 1\n# line 2\n1 2").unwrap();
        assert_eq!(
            result,
            vec![Token::Number("1".into()), Token::Number("2".into()),]
        );
    }


    #[test]
    fn test_flexible_quotes_single_with_single_inside() {
        let result = tokenize("'He'llo'").unwrap();
        assert_eq!(result, vec![Token::String("He'llo".into()),]);
    }

    #[test]
    fn test_flexible_quotes_single_with_double_inside() {
        let result = tokenize("'He\"llo'").unwrap();
        assert_eq!(result, vec![Token::String("He\"llo".into()),]);
    }

    #[test]
    fn test_flexible_quotes_with_space_delimiter() {
        let result = tokenize("'Hello' 'World'").unwrap();
        assert_eq!(
            result,
            vec![Token::String("Hello".into()), Token::String("World".into()),]
        );
    }

    #[test]
    fn test_flexible_quotes_with_bracket_delimiter() {
        let result = tokenize("['test']").unwrap();
        assert_eq!(
            result,
            vec![
                Token::VectorStart,
                Token::String("test".into()),
                Token::VectorEnd,
            ]
        );
    }


    #[test]
    fn test_japanese_word_with_whitespace() {
        let result = tokenize("2 3 足す").unwrap();
        assert_eq!(
            result,
            vec![
                Token::Number("2".into()),
                Token::Number("3".into()),
                Token::Symbol("足す".into()),
            ]
        );
    }

    #[test]
    fn test_japanese_word_boundary() {
        let result = tokenize("足す").unwrap();
        assert_eq!(result, vec![Token::Symbol("足す".into()),]);

        let result2 = tokenize("2 足す 3 掛ける 4").unwrap();
        assert_eq!(
            result2,
            vec![
                Token::Number("2".into()),
                Token::Symbol("足す".into()),
                Token::Number("3".into()),
                Token::Symbol("掛ける".into()),
                Token::Number("4".into()),
            ]
        );
    }

    #[test]
    fn test_mixed_japanese_english() {
        let result = tokenize("'Hello' 出力する").unwrap();
        assert_eq!(
            result,
            vec![
                Token::String("Hello".into()),
                Token::Symbol("出力する".into()),
            ]
        );
    }

    #[test]
    fn test_hiragana_katakana_kanji() {
        let result = tokenize("あいうえお").unwrap();
        assert_eq!(result, vec![Token::Symbol("あいうえお".into()),]);

        let result2 = tokenize("アイウエオ").unwrap();
        assert_eq!(result2, vec![Token::Symbol("アイウエオ".into()),]);

        let result3 = tokenize("合計").unwrap();
        assert_eq!(result3, vec![Token::Symbol("合計".into()),]);

        let result4 = tokenize("ひらがなカタカナ漢字").unwrap();
        assert_eq!(result4, vec![Token::Symbol("ひらがなカタカナ漢字".into()),]);
    }

    #[test]
    fn test_japanese_with_operators() {
        let result = tokenize("1 + 2 結果").unwrap();
        assert_eq!(
            result,
            vec![
                Token::Number("1".into()),
                Token::Symbol("+".into()),
                Token::Number("2".into()),
                Token::Symbol("結果".into()),
            ]
        );
    }


    #[test]
    fn test_number_parsing() {
        let result = tokenize("123").unwrap();
        assert_eq!(result, vec![Token::Number("123".into())]);

        let result2 = tokenize("123.456").unwrap();
        assert_eq!(result2, vec![Token::Number("123.456".into())]);

        let result3 = tokenize("-123").unwrap();
        assert_eq!(result3, vec![Token::Number("-123".into())]);

        let result4 = tokenize("1.5e10").unwrap();
        assert_eq!(result4, vec![Token::Number("1.5e10".into())]);
    }


    #[test]
    fn test_percent_symbol_token() {
        let result = tokenize("%").unwrap();
        assert_eq!(result, vec![Token::Symbol("%".into())]);
    }

    #[test]
    fn test_percent_symbol_in_mod_context() {
        let result = tokenize("[ 7 ] [ 3 ] %").unwrap();
        assert_eq!(
            result,
            vec![
                Token::VectorStart,
                Token::Number("7".into()),
                Token::VectorEnd,
                Token::VectorStart,
                Token::Number("3".into()),
                Token::VectorEnd,
                Token::Symbol("%".into()),
            ]
        );
    }

    #[test]
    fn test_operator_symbols() {
        let result = tokenize("+ -").unwrap();
        assert_eq!(
            result,
            vec![Token::Symbol("+".into()), Token::Symbol("-".into()),]
        );

        let result2 = tokenize("1 + 2 - 3").unwrap();
        assert_eq!(
            result2,
            vec![
                Token::Number("1".into()),
                Token::Symbol("+".into()),
                Token::Number("2".into()),
                Token::Symbol("-".into()),
                Token::Number("3".into()),
            ]
        );
    }


    #[test]
    fn test_keywords() {
        let result = tokenize("TRUE FALSE NIL").unwrap();
        assert_eq!(
            result,
            vec![
                Token::Symbol("TRUE".into()),
                Token::Symbol("FALSE".into()),
                Token::Symbol("NIL".into()),
            ]
        );

        let result2 = tokenize("true false nil").unwrap();
        assert_eq!(
            result2,
            vec![
                Token::Symbol("true".into()),
                Token::Symbol("false".into()),
                Token::Symbol("nil".into()),
            ]
        );
    }


    #[test]
    fn test_brackets() {
        let result = tokenize("[ 1 2 3 ]").unwrap();
        assert_eq!(
            result,
            vec![
                Token::VectorStart,
                Token::Number("1".into()),
                Token::Number("2".into()),
                Token::Number("3".into()),
                Token::VectorEnd,
            ]
        );

        let result2 = tokenize("{ a b c }").unwrap();
        assert_eq!(
            result2,
            vec![
                Token::BlockStart,
                Token::Symbol("a".into()),
                Token::Symbol("b".into()),
                Token::Symbol("c".into()),
                Token::BlockEnd,
            ]
        );

        let result3 = tokenize("( x y z )").unwrap();
        assert_eq!(
            result3,
            vec![
                Token::BlockStart,
                Token::Symbol("x".into()),
                Token::Symbol("y".into()),
                Token::Symbol("z".into()),
                Token::BlockEnd,
            ]
        );
    }

    #[test]
    fn test_mixed_bracket_styles() {
        let result = tokenize("{ ( [ 1 ] ) }").unwrap();
        assert_eq!(
            result,
            vec![
                Token::BlockStart,
                Token::BlockStart,
                Token::VectorStart,
                Token::Number("1".into()),
                Token::VectorEnd,
                Token::BlockEnd,
                Token::BlockEnd,
            ]
        );

        let result2 = tokenize("{ ( X ) ( Y ) }").unwrap();
        assert_eq!(
            result2,
            vec![
                Token::BlockStart,
                Token::BlockStart,
                Token::Symbol("X".into()),
                Token::BlockEnd,
                Token::BlockStart,
                Token::Symbol("Y".into()),
                Token::BlockEnd,
                Token::BlockEnd,
            ]
        );
    }

    #[test]
    fn test_frame_output_format() {
        let frame_output = "[ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ]";
        let result = tokenize(frame_output).unwrap();

        assert!(result
            .iter()
            .all(|t| matches!(t, Token::VectorStart | Token::VectorEnd)));

        let starts = result
            .iter()
            .filter(|t| matches!(t, Token::VectorStart))
            .count();
        let ends = result
            .iter()
            .filter(|t| matches!(t, Token::VectorEnd))
            .count();
        assert_eq!(starts, ends);
    }


    #[test]
    fn test_complex_expression() {
        let result = tokenize("[ 1 2 3 ] LENGTH '結果' PRINT").unwrap();
        assert_eq!(
            result,
            vec![
                Token::VectorStart,
                Token::Number("1".into()),
                Token::Number("2".into()),
                Token::Number("3".into()),
                Token::VectorEnd,
                Token::Symbol("LENGTH".into()),
                Token::String("結果".into()),
                Token::Symbol("PRINT".into()),
            ]
        );
    }

    #[test]
    fn test_multiline_expression() {
        let result = tokenize("1 2 +\n3 4 *\n").unwrap();
        assert_eq!(
            result,
            vec![
                Token::Number("1".into()),
                Token::Number("2".into()),
                Token::Symbol("+".into()),
                Token::LineBreak,
                Token::Number("3".into()),
                Token::Number("4".into()),
                Token::Symbol("*".into()),
            ]
        );
    }


    #[test]
    fn test_dollar_tokenized_as_cond_clause_separator() {
        let result = tokenize("{ IDLE $ 'ok' }").unwrap();
        assert_eq!(
            result,
            vec![
                Token::BlockStart,
                Token::Symbol("IDLE".into()),
                Token::CondClauseSep,
                Token::String("ok".into()),
                Token::BlockEnd,
            ]
        );
    }

    #[test]
    fn test_ampersand_is_treated_as_symbol() {
        let result = tokenize("&").unwrap();
        assert_eq!(result, vec![Token::Symbol("&".into())]);
    }

    #[test]
    fn test_ampersand_symbol_in_and_context() {
        let result = tokenize("[ TRUE ] [ FALSE ] &").unwrap();
        assert_eq!(
            result,
            vec![
                Token::VectorStart,
                Token::Symbol("TRUE".into()),
                Token::VectorEnd,
                Token::VectorStart,
                Token::Symbol("FALSE".into()),
                Token::VectorEnd,
                Token::Symbol("&".into()),
            ]
        );
    }

    #[test]
    fn test_removed_colon_code_block() {
        let result = tokenize(": DUP ;");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("':'"));
    }

    #[test]
    fn test_removed_chevron_branch() {
        let result = tokenize(">> { TRUE }");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("'>>'"));
    }
}
