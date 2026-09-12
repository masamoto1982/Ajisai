//! What a numeric literal must still denote, and still refuse, now that the
//! parse happens where the lexeme is read rather than where it is reached.
//!
//! Two properties carry the whole change. The `i64` fast path must never
//! disagree with the full parse — otherwise a literal would denote one number in
//! a block and another in straight-line code. And a lexeme that denotes no
//! rational must still be refused *at the point it is reached*, because work
//! before it is work a program is entitled to: `1 PRINT 1/0` prints `1/1` and
//! then fails, so refusing at tokenize time would erase a host effect.

#[cfg(test)]
mod number_literal_tests {
    use crate::types::fraction::Fraction;
    use crate::types::{NumberLiteral, Token};

    fn literal(lexeme: &str) -> NumberLiteral {
        match Token::number(lexeme) {
            Token::Number(literal) => literal,
            other => panic!("Token::number must build a Number, got {other:?}"),
        }
    }

    /// The fast path agrees with the full parse wherever it fires. Stated over
    /// the spellings `i64` accepts and the ones it does not, so both the taken
    /// and the declined branch are covered.
    #[test]
    fn the_integer_fast_path_never_disagrees_with_the_full_parse() {
        for lexeme in [
            // `i64` accepts these: the fast path fires.
            "0",
            "-0",
            "+0",
            "1",
            "-1",
            "+1",
            "007",
            "-007",
            "0000000000000000000001",
            "9223372036854775807",
            "-9223372036854775808",
            // `i64` declines these: the full parse governs, as before.
            "1/2",
            "-1/2",
            "1.5",
            "0.5",
            ".5",
            "1e5",
            "1_000",
            "9223372036854775808",
            "-9223372036854775809",
            "99999999999999999999999999",
        ] {
            let expected = Fraction::from_str(lexeme);
            let actual = literal(lexeme).parsed();
            match (&expected, &actual) {
                (Ok(want), Ok(got)) => assert_eq!(
                    want, got,
                    "`{lexeme}` denotes {want} but the literal answered {got}"
                ),
                (Err(want), Err(got)) => assert_eq!(
                    want, got,
                    "`{lexeme}` must be refused with the same message"
                ),
                _ => panic!("`{lexeme}`: expected {expected:?}, got {actual:?}"),
            }
        }
    }

    /// Densely, over every integer in a range and both spellings of it, because
    /// a disagreement that only showed up at one magnitude or one sign would be
    /// a value silently changing meaning.
    #[test]
    fn the_integer_fast_path_agrees_across_a_dense_range() {
        for n in -5000i64..=5000 {
            for lexeme in [n.to_string(), format!("{n:+}")] {
                let expected = Fraction::from_str(&lexeme).expect("an integer parses");
                let actual = literal(&lexeme).parsed().expect("an integer parses");
                assert_eq!(expected, actual, "`{lexeme}` changed meaning");
            }
        }
    }

    /// The lexeme survives, because three consumers read the spelling rather
    /// than the value: the digit ceiling counts digits as written,
    /// `format_token_to_string` echoes source back, and the tokenizer's
    /// round-trip check compares a lexeme against itself. A literal that
    /// normalized its own spelling would break all three.
    #[test]
    fn the_lexeme_is_preserved_exactly_as_written() {
        for lexeme in ["007", "1.50", "0.5", "+1", "-0", "1e5", "1_000", "1/0"] {
            assert_eq!(
                literal(lexeme).lexeme(),
                lexeme,
                "the spelling must survive unchanged"
            );
        }
    }

    /// A lexeme denoting no rational carries no value and reports the parse's own
    /// message when reached — not at construction, which is what keeps the
    /// refusal where it has always been.
    #[test]
    fn a_lexeme_that_denotes_no_rational_defers_its_refusal() {
        for lexeme in ["1/0", "0/0", "-1/0"] {
            let literal = literal(lexeme);
            // Construction succeeded; only reading the value refuses.
            assert_eq!(literal.lexeme(), lexeme);
            let error = literal
                .parsed()
                .expect_err("a zero denominator denotes no rational");
            assert_eq!(
                error,
                Fraction::from_str(lexeme).expect_err("same lexeme, same refusal"),
                "the message must be the one the parse itself reports"
            );
        }
    }

    /// A literal built from a value the caller already holds must denote that
    /// value — this is the constructor that removed `value_as_code`'s
    /// parsed-value → string → parsed-value round trip.
    #[test]
    fn a_literal_built_from_a_value_denotes_that_value() {
        for fraction in [
            Fraction::from(0),
            Fraction::from(7),
            Fraction::from(-3),
            Fraction::new(1.into(), 2.into()),
            Fraction::new((-3).into(), 4.into()),
        ] {
            let token = Token::number_from_value(fraction.clone());
            let Token::Number(literal) = token else {
                panic!("must be a Number");
            };
            assert_eq!(
                literal.parsed().expect("a built value parses back"),
                fraction,
                "a literal built from {fraction} must denote it"
            );
        }
    }
}
