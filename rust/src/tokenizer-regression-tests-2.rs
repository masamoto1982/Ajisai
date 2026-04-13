

#[cfg(test)]
mod tokenizer_regression_tests_2 {
    use crate::tokenizer::tokenize;
    use crate::types::Token;


    #[test]
    fn test_whitespace_handling() {

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

        let result = tokenize("PRINT? SET!").unwrap();
        assert_eq!(
            result,
            vec![Token::Symbol("PRINT?".into()), Token::Symbol("SET!".into()),]
        );
    }


    #[test]
    fn test_fraction_literal() {

        let result = tokenize("1/3").unwrap();
        assert_eq!(result, vec![Token::Number("1/3".into())]);


        let result2 = tokenize("-1/3").unwrap();
        assert_eq!(result2, vec![Token::Number("-1/3".into())]);


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

        let result = tokenize("1/").unwrap();
        assert_eq!(result, vec![Token::Symbol("1/".into())]);


        let result2 = tokenize("1/a").unwrap();
        assert_eq!(result2, vec![Token::Symbol("1/a".into())]);
    }


    #[test]
    fn test_unclosed_string_error() {
        let result = tokenize("'unclosed string");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unclosed literal"));
    }


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


    #[test]
    fn test_bracket_without_space() {

        let result = tokenize("[1]").unwrap();
        assert_eq!(
            result,
            vec![
                Token::VectorStart,
                Token::Number("1".into()),
                Token::VectorEnd,
            ]
        );


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


    #[test]
    fn test_string_with_double_quote() {

        let result = tokenize("'He said \"Hello\"'").unwrap();
        assert_eq!(result, vec![Token::String("He said \"Hello\"".into()),]);
    }

    #[test]
    fn test_string_with_single_quote() {

        let result = tokenize("'It's fine'").unwrap();
        assert_eq!(result, vec![Token::String("It's fine".into()),]);
    }


    #[test]
    fn test_vector_as_code_syntax() {

        let result = tokenize("[ [ 1 ] + ]").unwrap();

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

        let result = tokenize("[ [ 2 ] * ] 'DOUBLE' DEF").unwrap();

        assert_eq!(result.len(), 8);
        assert!(matches!(&result[6], Token::String(s) if s.as_ref() == "DOUBLE"));
        assert!(matches!(&result[7], Token::Symbol(s) if s.as_ref() == "DEF"));
    }


    #[test]
    fn test_chevron_branch_token_removed() {

        let result = tokenize(">> [ 5 ] [ 3 ] <");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("removed"));
    }

    #[test]
    fn test_chevron_default_token_removed() {

        let result = tokenize(">>> [ 0 ]");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("removed"));
    }


    #[test]
    fn test_code_block_tokens() {
        let result = tokenize("{ [ 2 ] * }").unwrap();
        assert_eq!(result[0], Token::BlockStart);
        assert_eq!(result[result.len() - 1], Token::BlockEnd);
    }

    #[test]
    fn test_code_block_def_syntax() {
        let result = tokenize("{ [ 2 ] * } 'DOUBLE' DEF").unwrap();
        assert_eq!(result[0], Token::BlockStart);
        assert_eq!(result[5], Token::BlockEnd);
        assert!(matches!(&result[6], Token::String(s) if s.as_ref() == "DOUBLE"));
        assert!(matches!(&result[7], Token::Symbol(s) if s.as_ref() == "DEF"));
    }

    #[test]
    fn test_colon_and_semicolon_removed() {
        let colon_result = tokenize(": [ 2 ] *");
        assert!(colon_result.is_err());
        assert!(colon_result.unwrap_err().contains("removed"));

        let semicolon_result = tokenize("[ 2 ] * ;");
        assert!(semicolon_result.is_err());
        assert!(semicolon_result.unwrap_err().contains("removed"));
    }


    #[test]
    fn test_greater_than_error() {

        let result = tokenize("[ 5 ] [ 3 ] >");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("removed"));
    }

    #[test]
    fn test_greater_than_equal_error() {

        let result = tokenize("[ 5 ] [ 3 ] >=");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("removed"));
    }

    #[test]
    fn test_multiline_code_block_error() {
        let input = "{ ,, [ 1 ] =\n[ 10 ] } 'CHECK_ONE' DEF";
        let result = tokenize(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Code block must be on a single line"));
    }

    #[test]
    fn test_multiple_dollar_clauses_in_single_line_error() {
        let result = tokenize("{ [ 0 ] < $ 'negative' } { IDLE $ 'positive' }");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("COND: $ clauses must be written one clause per line"));
    }


    #[test]
    fn test_mismatched_brace_paren() {

        let result = tokenize("{ [ 2 ] * )");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Mismatched brackets"));
    }

    #[test]
    fn test_mismatched_paren_brace() {

        let result = tokenize("( [ 2 ] * }");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Mismatched brackets"));
    }

    #[test]
    fn test_mismatched_bracket_brace() {

        let result = tokenize("[ 1 2 3 }");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Mismatched brackets"));
    }

    #[test]
    fn test_mismatched_bracket_paren() {

        let result = tokenize("[ 1 2 3 )");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Mismatched brackets"));
    }

    #[test]
    fn test_mismatched_brace_bracket() {

        let result = tokenize("{ [ 2 ] * ]");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Mismatched brackets"));
    }

    #[test]
    fn test_matched_braces_ok() {

        let result = tokenize("{ [ 2 ] * }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_matched_parens_ok() {

        let result = tokenize("( [ 2 ] * )");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_mixed_brackets_ok() {

        let result = tokenize("{ ( [ 1 ] + ) }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mismatched_nested_brackets() {

        let result = tokenize("{ ( [ 1 ] + } )");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Mismatched brackets"));
    }

    #[test]
    fn test_brackets_in_string_ignored() {

        let result = tokenize("'{ ( [' [ 1 ]");
        assert!(result.is_ok());
    }

    #[test]
    fn test_brackets_in_comment_ignored() {

        let result = tokenize("[ 1 ] # { ( [");
        assert!(result.is_ok());
    }
}
