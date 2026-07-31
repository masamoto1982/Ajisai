//! Property-based string / text laws (Phase 9, SPEC §7.6).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-octies I.2 (Phase 9):
//! a string literal `'abc'` is a **codepoint sequence** (a `Text`-hinted vector,
//! empty → NIL). The text words (`STR`/`NUM`/`CHR`/`CHARS`/`JOIN`/
//! `TRIM`/`TOKENIZE`/`SUBSTITUTE`/`STARTS-WITH?`/`ENDS-WITH?`) are Core Words.
//!
//! Observation is firewall-clean: text is read through the pure `render` (a
//! `Text`-hinted value renders `'…'`); predicates through `render`
//! (`TRUE`/`FALSE`). Every law was probe-confirmed first (roadmap §1.2-(T)).

mod test_support;

use proptest::prelude::*;
use test_support::generators::ascii_word;
use test_support::observe::{observe_program, render, run};

/// Render the single result value.
fn obs1(src: &str) -> String {
    let stack = run(src);
    assert_eq!(
        stack.len(),
        1,
        "{src:?} must leave one value, got {}",
        stack.len()
    );
    render(&stack[0], stack[0].hint)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// **A string literal renders back to itself** (Text role): `'w'` ⟼ `'w'`.
    #[test]
    fn literal_renders_itself(w in ascii_word()) {
        prop_assert_eq!(obs1(&format!("'{w}'")), format!("'{w}'"));
    }

    /// **`CHARS` then `JOIN` is the identity on text** (the codepoint sequence
    /// is split into single-char strings and re-concatenated): `w CHARS JOIN = w`.
    #[test]
    fn chars_join_round_trip(w in ascii_word()) {
        prop_assert_eq!(obs1(&format!("'{w}' CHARS JOIN")), format!("'{w}'"));
    }

    /// **`TRIM` is idempotent** (it strips to a fixed point): `TRIM ∘ TRIM = TRIM`.
    /// Checked on a word padded with spaces on both sides.
    #[test]
    fn trim_is_idempotent(w in ascii_word()) {
        let once = obs1(&format!("'  {w}  ' TRIM"));
        let twice = obs1(&format!("'  {w}  ' TRIM TRIM"));
        prop_assert_eq!(&once, &twice);
        prop_assert_eq!(once, format!("'{w}'"));
    }
    /// **`STR`∘`NUM` round-trips an integer through text** (value-preserving):
    /// `n STR NUM = n`. (`STR` renders the canonical integer form `'n'`; `NUM`
    /// parses it back to the rational `n/1`.)
    #[test]
    fn str_num_round_trip(n in -1000i64..=1000) {
        prop_assert_eq!(obs1(&format!("{n} STR NUM")), format!("{n}/1"));
    }

    /// **`STR` of an integer is its canonical decimal text**: `n STR = 'n'`.
    #[test]
    fn str_of_integer_is_decimal(n in -1000i64..=1000) {
        prop_assert_eq!(obs1(&format!("{n} STR")), format!("'{n}'"));
    }

    /// **`SUBSTITUTE` of a token by itself is the identity**, and a fresh token
    /// not present leaves the text unchanged (no-op replacements).
    #[test]
    fn substitute_identity(w in ascii_word()) {
        // replacing every 'a' with 'a' changes nothing.
        prop_assert_eq!(obs1(&format!("'{w}' 'a' 'a' SUBSTITUTE")), format!("'{w}'"));
    }

    /// **A string starts with, and ends with, itself** (reflexive prefix /
    /// suffix): `w 'w' STARTS-WITH? = TRUE`, `w 'w' ENDS-WITH? = TRUE`.
    #[test]
    fn starts_and_ends_with_self(w in ascii_word()) {
        prop_assert_eq!(obs1(&format!("'{w}' '{w}' STARTS-WITH?")), "TRUE");
        prop_assert_eq!(obs1(&format!("'{w}' '{w}' ENDS-WITH?")), "TRUE");
    }

    /// **`CHARS` of a word has one element per codepoint, and `JOIN` of two
    /// char-vectors concatenates** (free monoid on codepoints):
    /// `(u CHARS) (v CHARS) CONCAT JOIN = uv`. The word lengths start at 1: a
    /// one-character word makes `CHARS` yield a one-element vector, which
    /// `CONCAT` used to mistake for an operand count (finding I2, resolved —
    /// see `finding_i2_concat_joins_a_singleton_top_operand`).
    #[test]
    fn join_concat_is_concatenation(
        u in "[a-h]{1,6}",
        v in "[a-h]{1,6}",
    ) {
        prop_assert_eq!(
            obs1(&format!("'{u}' CHARS '{v}' CHARS CONCAT JOIN")),
            format!("'{u}{v}'")
        );
    }

    /// **`TOKENIZE` then `JOIN` with no separator reconstructs the joined
    /// pieces**: splitting `a,b,c` on `,` yields three pieces whose `JOIN` is
    /// `abc`.
    #[test]
    fn tokenize_pieces_join_back(x in ascii_word(), y in ascii_word(), z in ascii_word()) {
        prop_assert_eq!(
            obs1(&format!("'{x},{y},{z}' ',' TOKENIZE JOIN")),
            format!("'{x}{y}{z}'")
        );
    }
}

/// **The empty string is NIL** (`EmptySequence`, §4.5): `''` is an absence, not
/// a zero-length vector — pinned as a guarded oracle.
#[test]
fn empty_string_is_nil() {
    assert_eq!(obs1("''"), "NIL");
}

/// **A non-numeric `NUM` projects NIL** (`NUM` is total-by-projection, Bubble
/// Rule §11.2): parsing `'abc'` as a number yields an absence, not an error.
#[test]
fn num_of_non_numeric_projects_nil() {
    assert_eq!(obs1("'abc' NUM"), "NIL");
}

/// **`CHR` maps a codepoint to its single-character string** — a concrete anchor.
#[test]
fn chr_anchor() {
    assert_eq!(obs1("65 CHR"), "'A'");
}

/// **Finding I2 (resolved): `CONCAT`'s arity does not depend on its operands'
/// values.**
///
/// `CONCAT` used to accept an undeclared count-prefixed form (`a b c 3 CONCAT`)
/// that it recognized by sniffing the stack top. The sniff first read a
/// one-element vector as its element, so `[ 1 ] [ 2 ] CONCAT` meant "join the
/// top 2 after popping `[ 2 ]`" and raised `StackUnderflow`; restricting the
/// sniff to bare scalars fixed those shapes but kept the form. The form itself
/// is gone now — the specification declares `2 -> 1` over vectors and nothing
/// else — so the arity is fixed for every operand shape.
#[test]
fn finding_i2_concat_joins_a_singleton_top_operand() {
    // The shapes that used to underflow.
    assert_eq!(obs1("[ 1 ] [ 2 ] CONCAT"), "[ 1/1 2/1 ]");
    assert_eq!(obs1("[ 1 2 ] [ 3 ] CONCAT"), "[ 1/1 2/1 3/1 ]");
    assert_eq!(obs1("'ab' 'c' CONCAT JOIN"), "'abc'");

    // The shape that always worked, unchanged.
    assert_eq!(obs1("[ 1 ] [ 2 3 ] CONCAT"), "[ 1/1 2/1 3/1 ]");
}

/// **A bare scalar is an operand, and `CONCAT` refuses it** — the declared
/// `errorWhen: [nonVector]`.
///
/// While the count form existed this error was unreachable: a scalar on top was
/// eaten as a count, so `2 3 CONCAT` reported a short stack rather than a
/// non-vector operand, and `[ 1 2 ] TRUE CONCAT` silently lifted `TRUE` into
/// the result instead of refusing it. Both now raise the declared error.
#[test]
fn concat_refuses_a_non_vector_operand() {
    for src in [
        "2 3 CONCAT",
        "[ 1 2 ] TRUE CONCAT",
        "TRUE [ 1 2 ] CONCAT",
        "{ 1 } [ 2 ] CONCAT",
        // The three spellings of the retired count form. A trailing scalar is
        // now just a non-vector operand.
        "[ 1 2 ] [ 3 4 ] 2 CONCAT",
        "[ 1 ] [ 2 ] [ 3 ] 3 CONCAT",
        "[ 1 2 ] [ 3 4 ] -2 CONCAT",
    ] {
        assert_eq!(
            observe_program(src).error_category,
            Some("structureError"),
            "{src:?} must raise the declared nonVector error"
        );
    }
}
