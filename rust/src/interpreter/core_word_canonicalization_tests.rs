//! Test suite for `crate::core_word_aliases` canonicalization.

use crate::interpreter::Interpreter;

async fn assert_same_stack(left_code: &str, right_code: &str) {
    let mut left = Interpreter::new();
    left.execute("").await.unwrap();
    left.execute(left_code).await.unwrap();

    let mut right = Interpreter::new();
    right.execute("").await.unwrap();
    right.execute(right_code).await.unwrap();

    assert_eq!(left.get_stack(), right.get_stack());
}

#[tokio::test]
async fn symbol_aliases_execute_same_as_canonical_words() {
    assert_same_stack("1 2 +", "1 2 ADD").await;
    assert_same_stack("5 3 -", "5 3 SUB").await;
    assert_same_stack("2 4 *", "2 4 MUL").await;
    assert_same_stack("8 2 /", "8 2 DIV").await;
    assert_same_stack("1 1 =", "1 1 EQ").await;
    assert_same_stack("1 2 <", "1 2 LT").await;
    assert_same_stack("2 1 >", "2 1 GT").await;
    assert_same_stack("7 2 %", "7 2 MOD").await;
}
#[tokio::test]
async fn symbol_alias_canonicalizes_to_english_word() {
    use crate::core_word_aliases::canonicalize_core_word_name;
    assert_eq!(canonicalize_core_word_name("+"), "ADD");
}

/// `OR-NIL` has no symbol or legacy-name sugar: `^` and `VENT` (the former
/// canonical spelling, retired with the water-metaphor decoupling) are
/// ordinary, unrecognized names now — canonicalization leaves them
/// unchanged rather than folding them onto `OR-NIL`.
#[tokio::test]
async fn caret_and_legacy_vent_spelling_are_no_longer_aliases() {
    use crate::core_word_aliases::canonicalize_core_word_name;
    assert_eq!(canonicalize_core_word_name("^"), "^");
    assert_eq!(canonicalize_core_word_name("VENT"), "VENT");
}

/// `?` is the host's spelling of a lookup, not a Word alias, so canonicalization
/// leaves it alone. It used to fold to `LOOKUP`; if it still did, a User Word
/// named `?` would resolve to a Word that no longer exists.
#[tokio::test]
async fn the_lookup_mark_is_not_a_word_alias() {
    use crate::core_word_aliases::canonicalize_core_word_name;
    assert_eq!(canonicalize_core_word_name("?"), "?");
}

/// Lexical / structural surface forms are documented as named concepts but are
/// never runtime words: `canonicalize_core_word_name` must not return their
/// concept names. (See `crate::surface_forms`.)
#[tokio::test]
async fn surface_form_concepts_are_not_runtime_canonicalizations() {
    use crate::core_word_aliases::canonicalize_core_word_name;
    use crate::surface_forms::lookup_surface_form;

    assert_eq!(lookup_surface_form("#").unwrap().concept, "COMMENT-LINE");
    assert_eq!(lookup_surface_form("[").unwrap().concept, "BEGIN-VECTOR");

    assert_ne!(canonicalize_core_word_name("#"), "COMMENT-LINE");
    assert_ne!(canonicalize_core_word_name("["), "BEGIN-VECTOR");
    assert_ne!(canonicalize_core_word_name("{"), "BEGIN-BLOCK");
    assert_ne!(canonicalize_core_word_name("'"), "STRING-QUOTE");
}

/// 手3 (dispatch de-allocation): canonicalization must not allocate on the two
/// dominant dispatch cases — a symbol alias (borrows the `&'static` canonical
/// name) and an already-uppercase word (borrows the input slice). Only a name
/// that genuinely needs case folding takes the owned path.
#[test]
fn canonicalize_borrows_without_allocating_on_hot_paths() {
    use crate::core_word_aliases::canonicalize_core_word_name;
    use std::borrow::Cow;

    // Symbol alias → borrowed &'static canonical, value still correct.
    let add = canonicalize_core_word_name("+");
    assert!(matches!(add, Cow::Borrowed(_)), "alias must borrow");
    assert_eq!(add, "ADD");

    // Already-uppercase non-alias word → input borrowed unchanged.
    for word in ["MAP", "LENGTH", "TIME@NOW", "USER-WORD"] {
        let canon = canonicalize_core_word_name(word);
        assert!(
            matches!(canon, Cow::Borrowed(_)),
            "uppercase word {word} must borrow"
        );
        assert_eq!(canon, word);
    }

    // Mixed/lowercase requires folding → owned, and folds correctly.
    let folded = canonicalize_core_word_name("map");
    assert!(matches!(folded, Cow::Owned(_)), "lowercase must fold owned");
    assert_eq!(folded, "MAP");
}
