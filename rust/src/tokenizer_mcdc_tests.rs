// AQ-VER-002: tokenizer MC/DC tests for QL-B boolean decisions.
//
// Scope: `crate::tokenizer::tokenize` — boolean decisions whose
// independent atomic conditions can each cause an incorrect token
// stream (token-count drift, wrong token kind, mis-classified
// linebreak/comment behavior, etc.).
//
// Tests are black-box through `tokenize()` because the helpers
// (`is_string_close_delimiter`, `parse_number_from_string`, ...) are
// crate-private. For each decision we document the DUT, atomic
// conditions, and the rows of the MC/DC truth table that demonstrate
// each condition independently flipping the outcome.
//
// Trace: docs/quality/TRACEABILITY_MATRIX.md, requirement AQ-REQ-002.

#![cfg(test)]

use crate::tokenizer::tokenize;
use crate::types::Token;

fn sym(s: &str) -> Token {
    Token::Symbol(s.into())
}

fn num(s: &str) -> Token {
    Token::Number(s.into())
}

fn string_tok(s: &str) -> Token {
    Token::String(s.into())
}

// AQ-VER-002-A
// DUT: rust/src/tokenizer.rs:13 inside the '\n' whitespace branch
//
//     if tokens.last() != Some(&Token::LineBreak) { tokens.push(LineBreak); }
//
// One atomic condition C = (tokens.last() != Some(&LineBreak)).
//   row 1: C = T -> emit LineBreak
//   row 2: C = F -> suppress LineBreak (dedup)
//
// To observe row 2 we must place two consecutive newlines so the second
// `\n` sees a LineBreak as the most recent token. The trailing-LineBreak
// pop at the end of `tokenize` does not affect inter-token positions, so
// observing the *count* of LineBreak tokens between two Symbols proves
// dedup behavior.
mod linebreak_dedup {
    use super::*;

    #[test]
    fn aq_ver_002_a_row1_single_newline_emits_linebreak() {
        let tokens = tokenize("a\nb").unwrap();
        assert_eq!(tokens, vec![sym("a"), Token::LineBreak, sym("b")]);
    }

    #[test]
    fn aq_ver_002_a_row2_consecutive_newlines_dedupe() {
        // Three '\n' between a and b must collapse to a single LineBreak
        // token thanks to the C = F path.
        let tokens = tokenize("a\n\n\nb").unwrap();
        assert_eq!(tokens, vec![sym("a"), Token::LineBreak, sym("b")]);
    }

    #[test]
    fn aq_ver_002_a_trailing_linebreak_is_popped() {
        // Documents the post-loop cleanup that complements the dedup rule.
        let tokens = tokenize("a\n").unwrap();
        assert_eq!(tokens, vec![sym("a")]);
    }
}

// AQ-VER-002-B
// DUT: rust/src/tokenizer.rs:23 inside the '#' comment branch
//
//     let had_token_before =
//         !tokens.is_empty() && tokens.last() != Some(&Token::LineBreak);
//
// Conditions:
//   A = !tokens.is_empty()
//   B = tokens.last() != Some(&LineBreak)
//
// Reachable rows (note: A=F implies last()==None, which is != Some(LB),
// so B is forced T whenever A is F):
//   row 1: (A=T, B=T) -> had_token = true  (real preceding token)
//   row 2: (A=T, B=F) -> had_token = false (only a LineBreak preceding)
//   row 3: (A=F, B=T) -> had_token = false (empty stream)
//
// MC/DC pairs:
//   (1,2) holds A=T and flips B -> proves B's independent effect.
//   (1,3) holds B=T and flips A -> proves A's independent effect.
//
// `had_token_before` is observable indirectly via the AQ-VER-002-C
// decision (newline absorption). Here we verify the three reachable
// rows by checking the resulting token stream shape.
mod comment_had_token_before {
    use super::*;

    #[test]
    fn aq_ver_002_b_row1_preceding_real_token_keeps_linebreak() {
        // (A=T, B=T): "a # c\nb" — preceding 'a' is a non-LineBreak token,
        // so had_token=true and the comment's trailing newline is NOT
        // absorbed; a LineBreak appears between 'a' and 'b'.
        let tokens = tokenize("a # c\nb").unwrap();
        assert_eq!(tokens, vec![sym("a"), Token::LineBreak, sym("b")]);
    }

    #[test]
    fn aq_ver_002_b_row2_preceding_linebreak_absorbs_newline() {
        // (A=T, B=F): "a\n# c\nb" — last token before the comment is the
        // LineBreak emitted by the first '\n', so had_token=false. The
        // comment line absorbs its trailing newline, leaving a single
        // LineBreak between 'a' and 'b'.
        let tokens = tokenize("a\n# c\nb").unwrap();
        assert_eq!(tokens, vec![sym("a"), Token::LineBreak, sym("b")]);
    }

    #[test]
    fn aq_ver_002_b_row3_empty_stream_absorbs_newline() {
        // (A=F, B=T trivially): "# c\nb" — empty before the comment. The
        // comment swallows its newline, so the result starts directly
        // with 'b' (no leading LineBreak).
        let tokens = tokenize("# c\nb").unwrap();
        assert_eq!(tokens, vec![sym("b")]);
    }
}

// AQ-VER-002-C
// DUT: rust/src/tokenizer.rs:29 inside the '#' comment branch
//
//     if !had_token_before && i < chars.len() && chars[i] == '\n' {
//         i += 1;
//     }
//
// Conditions:
//   C = !had_token_before
//   D = i < chars.len()
//   E = chars[i] == '\n'
//
// Reachability constraint: the inner `while i < chars.len() && chars[i]
// != '\n'` exits only when D = F (EOF) or chars[i] == '\n' (E = T given
// D = T). Hence E cannot be F when D is T — the (C, D=T, E=F) rows are
// structurally unreachable. This is documented but not asserted.
//
// Reachable rows:
//   row 1: (C=T, D=T, E=T) -> consume newline (no LineBreak inserted)
//   row 2: (C=F, D=T, E=T) -> leave newline (LineBreak will be emitted)
//   row 3: (C=T, D=F, _ )  -> nothing to consume (EOF)
//   row 4: (C=F, D=F, _ )  -> nothing to consume (EOF)
//
// MC/DC pairs:
//   (1,2) holds D=T,E=T and flips C -> proves C's independent effect.
//   (1,3) holds C=T,E=*  and flips D -> proves D's independent effect.
//   E is short-circuit-masked by D and cannot be exercised independently.
mod comment_newline_absorption {
    use super::*;

    #[test]
    fn aq_ver_002_c_row1_lone_comment_absorbs_trailing_newline() {
        // (C=T, D=T, E=T): see AQ-VER-002-B row 3 for the full chain.
        let tokens = tokenize("# c\nb").unwrap();
        assert_eq!(tokens, vec![sym("b")]);
    }

    #[test]
    fn aq_ver_002_c_row2_inline_comment_keeps_trailing_newline() {
        // (C=F, D=T, E=T): preceding token forces had_token=true so the
        // newline is not absorbed and surfaces as a LineBreak token.
        let tokens = tokenize("a # c\nb").unwrap();
        assert_eq!(tokens, vec![sym("a"), Token::LineBreak, sym("b")]);
    }

    #[test]
    fn aq_ver_002_c_row3_lone_comment_at_eof_no_absorption() {
        // (C=T, D=F): comment runs to EOF, nothing to absorb.
        let tokens = tokenize("# c").unwrap();
        assert_eq!(tokens, Vec::<Token>::new());
    }

    #[test]
    fn aq_ver_002_c_row4_inline_comment_at_eof_no_absorption() {
        // (C=F, D=F): same EOF condition, but with preceding token.
        let tokens = tokenize("a # c").unwrap();
        assert_eq!(tokens, vec![sym("a")]);
    }
}

// AQ-VER-002-D
// DUT: rust/src/tokenizer.rs:50, 56 inside the '=' branch
//
//     if i + 1 < chars.len() && chars[i + 1] == '=' { Pipeline; }
//     if i + 1 < chars.len() && chars[i + 1] == '>' { NilCoalesce; }
//
// Per multi-char operator we have two short-circuit conditions:
//   A = (i + 1 < chars.len())
//   B = (chars[i + 1] == X)  for X in {'=','>'}
//
// MC/DC for `A && B` per operator:
//   (T,T) -> match
//   (T,F) -> no match (B's independent effect, A held T)
//   (F,_) -> no match (A's independent effect, B masked)
//
// We cover both `==` -> Pipeline and `=>` -> NilCoalesce, plus the
// fall-through to bare `=` Symbol when neither lookahead succeeds.
mod equals_lookahead {
    use super::*;

    #[test]
    fn aq_ver_002_d_pipeline_match_when_both_conditions_true() {
        // (A=T, B=T) for the '==' branch.
        let tokens = tokenize("a == b").unwrap();
        assert_eq!(tokens, vec![sym("a"), Token::Pipeline, sym("b")]);
    }

    #[test]
    fn aq_ver_002_d_nilcoalesce_match_when_both_conditions_true() {
        // (A=T, B=T) for the '=>' branch.
        let tokens = tokenize("a => b").unwrap();
        assert_eq!(tokens, vec![sym("a"), Token::NilCoalesce, sym("b")]);
    }

    #[test]
    fn aq_ver_002_d_falls_through_when_lookahead_char_mismatches() {
        // (A=T, B=F) for both lookaheads: '=' followed by space. The two
        // checks both see B=F (chars[i+1] is not '=' nor '>'), so we fall
        // through to the bare '=' Symbol.
        let tokens = tokenize("= a").unwrap();
        assert_eq!(tokens, vec![sym("="), sym("a")]);
    }

    #[test]
    fn aq_ver_002_d_falls_through_when_at_eof() {
        // (A=F): bare '=' at end of input. Both lookahead conditions fail
        // on A's short-circuit, so we emit the bare Symbol.
        let tokens = tokenize("=").unwrap();
        assert_eq!(tokens, vec![sym("=")]);
    }
}

// AQ-VER-002-E
// DUT: rust/src/tokenizer.rs:410 in `is_string_close_delimiter`
//
//     fn is_string_close_delimiter(c: char) -> bool {
//         c.is_whitespace() || (is_special_char(c) && c != '\'')
//     }
//
// Conditions:
//   A = c.is_whitespace()
//   B = is_special_char(c)
//   C = c != '\''
//
// MC/DC pairs (see analysis below):
//   row 1 (T, F, T): whitespace -> close. Together with row 4 proves A.
//   row 2 (F, T, T): special non-quote -> close. Together with row 3
//                     proves C, together with row 4 proves B.
//   row 3 (F, T, F): a literal quote in lookahead position -> not close
//                     (treated as escaped quote, pushed as literal).
//   row 4 (F, F, T): regular alpha char -> not close, parser keeps
//                     scanning until it hits an actual close or EOF.
//
// Observed via `tokenize()` of `'foo'<X>` strings.
mod string_close_delimiter {
    use super::*;

    #[test]
    fn aq_ver_002_e_row1_whitespace_after_quote_closes_string() {
        // (A=T, B=F, C=T)
        let tokens = tokenize("'foo' BAR").unwrap();
        assert_eq!(tokens, vec![string_tok("foo"), sym("BAR")]);
    }

    #[test]
    fn aq_ver_002_e_row2_special_nonquote_after_quote_closes_string() {
        // (A=F, B=T, C=T): `[` is a special char and not a quote.
        let tokens = tokenize("'foo'[1]").unwrap();
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
    fn aq_ver_002_e_row3_quote_after_quote_is_literal() {
        // (A=F, B=T, C=F): when a `'` is followed by another `'`, the
        // close-delimiter check returns false (special, but C=F), so the
        // current quote is pushed as a literal and the scan continues.
        // The next `'` likewise sees a non-close delimiter ahead ('b')
        // and is pushed too, so the resulting string contains BOTH
        // literal quotes — there is no escape collapsing.
        let tokens = tokenize("'foo''bar' END").unwrap();
        assert_eq!(tokens, vec![string_tok("foo''bar"), sym("END")]);
    }

    #[test]
    fn aq_ver_002_e_row4_regular_alpha_after_quote_does_not_close() {
        // (A=F, B=F, C=T): alphabetic char is neither whitespace nor
        // special. The parser treats the quote as a literal and keeps
        // scanning for a real close; with no close available it surfaces
        // as an Unclosed-literal error.
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
