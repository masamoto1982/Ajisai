//! MC/DC test suite for `crate::tokenizer`.

// AQ-VER-002: tokenizer MC/DC tests for QL-B boolean decisions.
//
// Scope: `crate::tokenizer::tokenize` — boolean decisions whose
// independent atomic conditions can each cause an incorrect token
// stream (token-count drift, wrong token kind, mis-classified
// comment behavior, etc.).
//
// Tests are black-box through `tokenize()` because the helpers
// (`is_string_close_delimiter`, `parse_number_from_string`, ...) are
// crate-private. For each decision we document the DUT, atomic
// conditions, and the rows of the MC/DC truth table that demonstrate
// each condition independently flipping the outcome.
//
// Trace: docs/quality/TRACEABILITY_MATRIX.md, requirement AQ-REQ-002.

use crate::tokenizer::tokenize;
use crate::types::Token;

fn sym(s: &str) -> Token {
    Token::Symbol(s.into())
}

fn num(s: &str) -> Token {
    Token::number(s)
}

fn string_tok(s: &str) -> Token {
    Token::String(s.into())
}

// AQ-VER-002-A
// DUT: rust/src/tokenizer.rs, the whitespace branch and the '#' comment branch
//
// Whitespace is the sole token delimiter and carries nothing else
// (LANG.SOURCE.TEXT): a line break is whitespace like a space, so no token
// ever records one. A comment is the one construct a line break ends: `#` at
// a fresh word position runs to the end of its line.
//
// One decision in the comment loop, `i < chars.len() && chars[i] != '\n'`:
//   row 1: a newline ends the comment      -> the next line's tokens follow
//   row 2: end of input ends the comment   -> nothing follows
mod whitespace_and_comments {
    use super::*;

    #[test]
    fn aq_ver_002_a_a_line_break_is_whitespace() {
        assert_eq!(tokenize("a\nb").unwrap(), vec![sym("a"), sym("b")]);
        assert_eq!(tokenize("a\n\n\nb").unwrap(), vec![sym("a"), sym("b")]);
        assert_eq!(tokenize("a\n").unwrap(), vec![sym("a")]);
        assert_eq!(tokenize("a\nb").unwrap(), tokenize("a b").unwrap());
    }

    #[test]
    fn aq_ver_002_a_row1_a_newline_ends_a_comment() {
        assert_eq!(tokenize("a # c\nb").unwrap(), vec![sym("a"), sym("b")]);
        assert_eq!(tokenize("# c\nb").unwrap(), vec![sym("b")]);
        assert_eq!(tokenize("a\n# c\nb").unwrap(), vec![sym("a"), sym("b")]);
    }

    #[test]
    fn aq_ver_002_a_row2_end_of_input_ends_a_comment() {
        assert_eq!(tokenize("# c").unwrap(), Vec::<Token>::new());
        assert_eq!(tokenize("a # c").unwrap(), vec![sym("a")]);
    }
}

// AQ-VER-002-D
// DUT: rust/src/tokenizer.rs `parse_control_directive_word`
//
// `OR-NIL` has no symbol sugar: it is a spelled-out control directive
// recognized only by its bare word, case-folded (LANG.FAILURE.RECOVERY). `^` and `~`
// carry no meaning of their own, so both are ordinary Symbols the dictionary
// does not have. `=` is an unconditional single-char `EQ` Symbol with no
// lookahead. We cover that boundary plus the bare `=` Symbol.
mod single_char_aliases {
    use super::*;

    /// `^` used to be `OR-NIL`'s sugar; it now carries no meaning of its own,
    /// so it is an ordinary Symbol the dictionary does not have — not a token
    /// of its own, and not a word that silently does nothing.
    #[test]
    fn aq_ver_002_d_caret_is_an_ordinary_symbol() {
        let tokens = tokenize("a ^ b").unwrap();
        assert_eq!(tokens, vec![sym("a"), sym("^"), sym("b")]);
    }

    /// A symbol obeys the same boundary rule as every other word: only
    /// whitespace ends a token, so `^` glued to a name is part of that name
    /// rather than a symbol of its own. `[` and `]` are no exception — they
    /// must stand alone too, so `^` glued to a closing bracket does not split
    /// off either; it makes the whole `]^` an invalid token (LANG.SOURCE.TEXT).
    #[test]
    fn aq_ver_002_d_caret_needs_surrounding_whitespace() {
        assert_eq!(tokenize("a^b").unwrap(), vec![sym("a^b")]);
        let err = tokenize("[ a ]^").unwrap_err();
        assert!(err.contains("must stand alone"), "got: {err}");
        assert_eq!(tokenize("[ a ] ^").unwrap()[3], sym("^"));
    }

    /// `~` carries no meaning, so it is an ordinary Symbol the dictionary does
    /// not have — not a token of its own, and not a word that silently does
    /// nothing.
    #[test]
    fn aq_ver_002_d_tilde_is_an_ordinary_symbol() {
        let tokens = tokenize("a ~ b").unwrap();
        assert_eq!(tokens, vec![sym("a"), sym("~"), sym("b")]);
    }

    #[test]
    fn aq_ver_002_d_equals_is_bare_symbol() {
        // `=` has no lookahead: it is always the bare EQ Symbol.
        let tokens = tokenize("= a").unwrap();
        assert_eq!(tokens, vec![sym("="), sym("a")]);
    }

    /// Every symbol is one character and nothing looks ahead, so `==` is a single
    /// name token rather than two `EQ`s.
    #[test]
    fn aq_ver_002_d_double_equals_is_one_name() {
        let tokens = tokenize("a == b").unwrap();
        assert_eq!(tokens, vec![sym("a"), sym("=="), sym("b")]);
    }

    #[test]
    fn aq_ver_002_d_equals_at_eof_is_bare_symbol() {
        let tokens = tokenize("=").unwrap();
        assert_eq!(tokens, vec![sym("=")]);
    }
}

// AQ-VER-002-E
// DUT: rust/src/tokenizer.rs in `is_string_close_delimiter`
//
//     fn is_string_close_delimiter(c: char) -> bool {
//         c.is_whitespace()
//     }
//
// Whitespace is the sole token delimiter in Ajisai (LANG.SOURCE.TEXT), so
// this decision has collapsed to the one atomic condition it always should
// have been: A = c.is_whitespace(). `[`, `]`, `#`, `(`, `)`, `{`, `}` no
// longer get a lookahead exemption of their own — a string glued to any of
// them (`'foo'[1]`, `'foo'#c`) does not close there either; it needs the
// same surrounding space every other token boundary does.
//
// Reachable rows:
//   row 1: A=T (whitespace or EOF, checked by the caller) -> close.
//   row 2: A=F, and the lookahead char is itself `'` -> not a close (the
//          current quote is content; SPEC's "no escape character" rule).
//   row 3: A=F, ordinary character -> not a close; the parser keeps
//          scanning for a real close, surfacing "Unclosed literal" if none
//          exists before EOF.
//
// Observed via `tokenize()` of `'foo'<X>` strings.
mod string_close_delimiter {
    use super::*;

    #[test]
    fn aq_ver_002_e_row1_whitespace_after_quote_closes_string() {
        // A=T
        let tokens = tokenize("'foo' BAR").unwrap();
        assert_eq!(tokens, vec![string_tok("foo"), sym("BAR")]);
    }

    /// A bracket (or any other formerly-"special" character) no longer gets
    /// its own lookahead exemption: with whitespace as the sole delimiter,
    /// a string glued directly to `[1]` never finds a real close before EOF.
    #[test]
    fn aq_ver_002_e_row1b_bracket_after_quote_no_longer_closes_string() {
        let err = tokenize("'foo'[1]").unwrap_err();
        assert!(err.contains("Unclosed literal"), "got: {err}");
        // The space every other token boundary needs works here too.
        let tokens = tokenize("'foo' [ 1 ]").unwrap();
        assert_eq!(
            tokens,
            vec![
                string_tok("foo"),
                Token::VectorStart,
                num("1"),
                Token::VectorEnd,
            ]
        );
    }

    #[test]
    fn aq_ver_002_e_row2_quote_after_quote_is_literal() {
        // A=F, lookahead is `'`: the close-delimiter check is false, so the
        // current quote is pushed as a literal and the scan continues. The
        // next `'` likewise sees a non-close delimiter ahead ('b') and is
        // pushed too, so the resulting string contains BOTH literal quotes —
        // there is no escape collapsing.
        let tokens = tokenize("'foo''bar' END").unwrap();
        assert_eq!(tokens, vec![string_tok("foo''bar"), sym("END")]);
    }

    #[test]
    fn aq_ver_002_e_row3_regular_alpha_after_quote_does_not_close() {
        // A=F, ordinary character. The parser treats the quote as a literal
        // and keeps scanning for a real close; with no close available it
        // surfaces as an Unclosed-literal error.
        let err = tokenize("'foo'bar").unwrap_err();
        assert!(
            err.contains("Unclosed literal"),
            "expected Unclosed literal error, got: {err}",
        );
    }
}

// AQ-VER-002-F
// DUT: rust/src/tokenizer.rs:464-474 in `parse_number_from_string`
//
//     if chars[i] == '-' || chars[i] == '+' {
//         if chars.len() == 1 { return None; }
//         if !chars[i + 1].is_ascii_digit() { return None; }
//         i += 1;
//     }
//
// We treat the sign-handling preamble as three sequential decisions:
//   D-F1: SIGN  = (chars[i] == '-' || chars[i] == '+')
//   D-F2: SOLE  = (chars.len() == 1)               -- only when SIGN=T
//   D-F3: NDIG  = !chars[i + 1].is_ascii_digit()   -- only when SOLE=F
//
// Reachable rows for the combined decision:
//   row 1: SIGN=F                  -> proceed to digit scan (e.g. "5")
//   row 2: SIGN=T, SOLE=T          -> not a number (e.g. "-")
//   row 3: SIGN=T, SOLE=F, NDIG=T  -> not a number (e.g. "-x")
//   row 4: SIGN=T, SOLE=F, NDIG=F  -> proceed to digit scan (e.g. "-5")
//
// Each row is observable via the kind of `Token` produced by
// `tokenize()`: Number for rows 1/4, Symbol for rows 2/3.
//
// MC/DC pairs:
//   (1,4) holds the digit scan path constant and flips SIGN's effect on
//         entry into the sign branch.
//   (2,3) holds SIGN=T and flips SOLE while keeping the path leading to
//         a None outcome (still Symbol, but exercises a different
//         internal exit).
//   (3,4) holds SIGN=T,SOLE=F and flips NDIG -> proves NDIG.
mod number_sign_guards {
    use super::*;

    #[test]
    fn aq_ver_002_f_row1_unsigned_digit_is_number() {
        let tokens = tokenize("5").unwrap();
        assert_eq!(tokens, vec![num("5")]);
    }

    #[test]
    fn aq_ver_002_f_row2_lone_minus_is_symbol() {
        let tokens = tokenize("-").unwrap();
        assert_eq!(tokens, vec![sym("-")]);
    }

    #[test]
    fn aq_ver_002_f_row2_lone_plus_is_symbol() {
        let tokens = tokenize("+").unwrap();
        assert_eq!(tokens, vec![sym("+")]);
    }

    #[test]
    fn aq_ver_002_f_row3_sign_then_nondigit_is_symbol() {
        // "-x" forms a single token because '-' and 'x' are neither
        // whitespace nor special. parse_number rejects it, leaving the
        // raw symbol "-x".
        let tokens = tokenize("-x").unwrap();
        assert_eq!(tokens, vec![sym("-x")]);
    }

    #[test]
    fn aq_ver_002_f_row4_sign_then_digit_is_negative_number() {
        let tokens = tokenize("-5").unwrap();
        assert_eq!(tokens, vec![num("-5")]);
    }

    #[test]
    fn aq_ver_002_f_row4_sign_then_digit_is_positive_number() {
        let tokens = tokenize("+5").unwrap();
        assert_eq!(tokens, vec![num("+5")]);
    }
}
