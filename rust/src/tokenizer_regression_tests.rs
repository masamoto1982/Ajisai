//! Regression test suite for `crate::tokenizer`.

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
    fn test_adjacent_quotes_are_literal() {
        let result = tokenize("'hel''lo'").unwrap();
        assert_eq!(result, vec![Token::String("hel''lo".into()),]);
    }

    #[test]
    fn test_multiple_inner_quotes() {
        let result = tokenize("'a'b'c'").unwrap();
        assert_eq!(result, vec![Token::String("a'b'c".into()),]);
    }

    #[test]
    fn test_adjacent_quotes_followed_by_space() {
        let result = tokenize("'hel''lo' 'world'").unwrap();
        assert_eq!(
            result,
            vec![
                Token::String("hel''lo".into()),
                Token::String("world".into()),
            ]
        );
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

    /// A decimal literal carries digits on both sides of the point. The point is
    /// therefore never a number's first or last character, so `.` on its own is
    /// unambiguously a name -- which is what keeps the character allocatable as a
    /// symbol later without changing the numeric language a second time.
    #[test]
    fn test_decimal_point_needs_digits_on_both_sides() {
        assert_eq!(tokenize("0.5").unwrap(), vec![Token::Number("0.5".into())]);
        assert_eq!(
            tokenize("-0.5").unwrap(),
            vec![Token::Number("-0.5".into())]
        );
        assert_eq!(tokenize("5.0").unwrap(), vec![Token::Number("5.0".into())]);

        // No integer part, no fractional part, and a point followed only by an
        // exponent: none of these is a number, so each reaches the dictionary as
        // a name and fails there.
        for lexeme in [".5", "-.5", "+.5", "5.", "5.e3"] {
            assert_eq!(
                tokenize(lexeme).unwrap(),
                vec![Token::Symbol(lexeme.into())],
                "`{lexeme}` must not be a Number"
            );
        }

        // A bare point or a run of points is a name, not a number.
        assert_eq!(tokenize(".").unwrap(), vec![Token::Symbol(".".into())]);
        assert_eq!(tokenize("..").unwrap(), vec![Token::Symbol("..".into())]);
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

        let result3 = tokenize("( x y z )");
        assert!(result3.is_err());
        assert!(result3
            .unwrap_err()
            .contains("not a valid Ajisai source character"));
    }

    #[test]
    fn test_paren_rejected_in_block_position() {
        let result = tokenize("{ ( [ 1 ] ) }");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not a valid Ajisai source character"));

        let result2 = tokenize("{ ( X ) ( Y ) }");
        assert!(result2.is_err());
        assert!(result2
            .unwrap_err()
            .contains("not a valid Ajisai source character"));
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
    fn test_ampersand_lexes_as_a_name() {
        let result = tokenize("&").unwrap();
        assert_eq!(result, vec![Token::Symbol("&".into())]);
    }

    #[test]
    fn test_ampersand_lexes_as_a_name_after_vectors() {
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
}

// ── Source positions ──────────────────────────────────────────────────────
// An Ajisai error used to say what went wrong and nothing about where, so
// finding the fault in a six-line program meant running it a line at a time.
// The tokenizer now records a 1-based line and column per token, index-aligned
// with the tokens themselves.
#[cfg(test)]
mod source_span_tests {
    use crate::tokenizer::{tokenize, tokenize_with_spans};

    #[test]
    fn spans_are_index_aligned_with_tokens() {
        let source = "1 2 ADD\n[ 3 ] LENGTH";
        let (tokens, spans) = tokenize_with_spans(source).unwrap();
        assert_eq!(tokens.len(), spans.len());
        assert_eq!(tokens, tokenize(source).unwrap(), "the wrapper agrees");
    }

    #[test]
    fn a_token_carries_the_line_and_column_it_was_written_at() {
        let (tokens, spans) = tokenize_with_spans("1 2 ADD\n  BADWORD").unwrap();
        let index = tokens
            .iter()
            .position(|t| matches!(t, crate::types::Token::Symbol(s) if s.as_ref() == "BADWORD"))
            .expect("BADWORD is a token");
        assert_eq!(spans[index].line, 2);
        assert_eq!(spans[index].column, 3, "columns are 1-based characters");
    }

    #[test]
    fn a_comment_does_not_shift_the_positions_after_it() {
        let (tokens, spans) = tokenize_with_spans("# note\n42 ADD").unwrap();
        let index = tokens
            .iter()
            .position(|t| matches!(t, crate::types::Token::Number(n) if n.as_ref() == "42"))
            .expect("42 is a token");
        assert_eq!((spans[index].line, spans[index].column), (2, 1));
    }

    #[test]
    fn a_string_literal_is_positioned_at_its_opening_quote() {
        let (tokens, spans) = tokenize_with_spans("PRINT 'hi there'").unwrap();
        let index = tokens
            .iter()
            .position(|t| matches!(t, crate::types::Token::String(_)))
            .expect("the string is a token");
        assert_eq!((spans[index].line, spans[index].column), (1, 7));
    }
}
