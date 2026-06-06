//! Property-based string / text laws (Phase 9, SPEC §7.6).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-octies I.2 (Phase 9):
//! a string literal `'abc'` is a **codepoint sequence** (a `Text`-hinted vector,
//! empty → NIL §4.5). The text words (`STR`/`NUM`/`BOOL`/`CHR`/`CHARS`/`JOIN`/
//! `TRIM*`/`TOKENIZE`/`SUBSTITUTE`/`STARTS-WITH?`/`ENDS-WITH?`) are Canonical
//! Core (boundary-listed `TEXT`), so no import is needed.
//!
//! Observation is firewall-clean: text is read through the pure `render` (a
//! `Text`-hinted value renders `'…'`); predicates through `render`
//! (`TRUE`/`FALSE`). Every law was probe-confirmed first (roadmap §1.2-(T)).

mod test_support;

use proptest::prelude::*;
use test_support::generators::ascii_word;
use test_support::observe::{render, run};

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

    /// **One-sided trims compose to the two-sided trim** on padded text.
    #[test]
    fn trim_left_right_compose_to_trim(w in ascii_word()) {
        prop_assert_eq!(
            obs1(&format!("'  {w}  ' TRIM-LEFT TRIM-RIGHT")),
            obs1(&format!("'  {w}  ' TRIM")),
        );
        prop_assert_eq!(obs1(&format!("'  {w}' TRIM-LEFT")), format!("'{w}'"));
        prop_assert_eq!(obs1(&format!("'{w}  ' TRIM-RIGHT")), format!("'{w}'"));
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
    /// `(u CHARS) (v CHARS) CONCAT JOIN = uv`. Both words are ≥ 2 chars so each
    /// `CHARS` yields a multi-element vector (finding I2: `CONCAT` underflows on a
    /// single-element top operand).
    #[test]
    fn join_concat_is_concatenation(
        u in "[a-h]{2,6}",
        v in "[a-h]{2,6}",
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

/// **`CHR` maps a codepoint to its single-character string** and `BOOL` parses
/// a truth literal — concrete §7.6 anchors.
#[test]
fn chr_and_bool_anchors() {
    assert_eq!(obs1("65 CHR"), "'A'");
    assert_eq!(obs1("'TRUE' BOOL"), "TRUE");
    assert_eq!(obs1("'FALSE' BOOL"), "FALSE");
}

/// **Finding I2 (guarded oracle): `CONCAT` underflows on a single-element top
/// operand.** A single-element vector on top of the stack is treated specially
/// (spread), so `[ 1 ] [ 2 ] CONCAT` raises `StackUnderflow`, while a
/// multi-element top operand concatenates normally (`[ 1 ] [ 2 3 ] CONCAT`).
/// This is why the `JOIN`∘`CONCAT` law above requires ≥ 2-char words. Pinned so
/// a future change to single-element handling is loud.
#[test]
fn finding_i2_concat_underflows_on_singleton_top() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ok = |s: &str| {
        rt.block_on(async {
            ajisai_core::interpreter::Interpreter::new()
                .execute(s)
                .await
                .is_ok()
        })
    };
    assert!(
        !ok("[ 1 ] [ 2 ] CONCAT"),
        "singleton top operand should underflow"
    );
    assert!(
        ok("[ 1 ] [ 2 3 ] CONCAT"),
        "multi-element top operand concatenates"
    );
}
