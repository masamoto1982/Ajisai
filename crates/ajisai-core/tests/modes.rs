//! `TOP`/`STAK` × `EAT`/`KEEP` — all four combinations, in both notations,
//! plus the scope, nesting, and boundary rules.

mod support;
use support::{failure, line};

use ajisai_core::mode::Mode;
use ajisai_core::Error;

#[test]
fn all_four_combinations() {
    // TOP EAT — the default: the surface two, consumed.
    assert_eq!(line("1 2 3 TOP EAT ADD"), "1 5");
    assert_eq!(line("1 2 3 ADD"), "1 5");
    // TOP KEEP — the surface two stay standing, the result branches above.
    assert_eq!(line("1 2 3 TOP KEEP ADD"), "1 2 3 5");
    // STAK EAT — the whole flow, folded, consumed.
    assert_eq!(line("1 2 3 STAK EAT ADD"), "6");
    // STAK KEEP — the whole flow stays standing, the fold branches above.
    assert_eq!(line("1 2 3 STAK KEEP ADD"), "1 2 3 6");
}

#[test]
fn every_combination_has_a_symbol_form_that_agrees() {
    let pairs = [
        ("1 2 3 TOP EAT ADD", "1 2 3 . ! +"),
        ("1 2 3 TOP KEEP ADD", "1 2 3 . & +"),
        ("1 2 3 STAK EAT ADD", "1 2 3 : ! +"),
        ("1 2 3 STAK KEEP ADD", "1 2 3 : & +"),
    ];
    for (canonical, symbolic) in pairs {
        assert_eq!(
            line(canonical),
            line(symbolic),
            "`{canonical}` and `{symbolic}` must agree"
        );
    }
}

#[test]
fn the_two_axes_are_independent() {
    // Order of arming does not matter, and each axis can be armed alone.
    assert_eq!(line("1 2 3 STAK KEEP ADD"), line("1 2 3 KEEP STAK ADD"));
    assert_eq!(line("1 2 3 KEEP ADD"), "1 2 3 5");
    assert_eq!(line("1 2 3 STAK ADD"), "6");
    assert_eq!(Mode::ALL.len(), 4);
}

/// `STAK` on a one-in word reaches every cell of the standing flow.
#[test]
fn stak_maps_one_in_words_across_the_flow() {
    assert_eq!(line("1 -2 3 STAK NEG"), "-1 2 -3");
    assert_eq!(line("1 -2 3 STAK ABS"), "1 2 3");
    assert_eq!(line("1 2 STAK DUP"), "1 1 2 2");
    assert_eq!(line("1 2 3 STAK DROP"), "");
    assert_eq!(line("1 2 STAK KEEP NEG"), "1 2 -1 -2");
}

/// `STAK` on a two-in one-out word folds left across the standing flow.
#[test]
fn stak_folds_two_in_words_across_the_flow() {
    assert_eq!(line("1 2 3 4 STAK ADD"), "10");
    assert_eq!(line("2 3 4 STAK MUL"), "24");
    assert_eq!(line("5 2 9 STAK MAX"), "9");
    // A flow of one folds to itself.
    assert_eq!(line("7 STAK ADD"), "7");
}

/// Anything else has no defensible reading across a whole flow, so it is
/// refused rather than given an invented one.
#[test]
fn stak_refuses_shapes_it_cannot_read() {
    for source in ["1 2 STAK SWAP", "STAK TRUE", "1 2 STAK MAP"] {
        assert!(
            matches!(failure(source), Error::ModeUnsupported { .. }),
            "`{source}` should be ModeUnsupported"
        );
    }
}

/// A mode applies to exactly one word and then the default returns.
#[test]
fn a_mode_governs_one_word_and_then_resets() {
    assert_eq!(line("1 2 KEEP ADD 10 ADD"), "1 2 13");
    assert_eq!(line("1 2 3 STAK ADD 4 ADD"), "10");
}

/// Literals, basins, and quotes do not consume an armed mode: a mode is a
/// statement about the next *word*.
#[test]
fn intervening_values_do_not_swallow_a_mode() {
    // KEEP reaches ADD across the literal: both operands stay standing and
    // the sum branches above them. Were the literal to swallow the mode, the
    // flow would read `3`.
    assert_eq!(line("1 KEEP 2 ADD"), "1 2 3");
    assert_eq!(line("1 2 ADD"), "3");
    assert_eq!(line("KEEP [ 1 2 ] LENGTH"), "[ 1 2 ] 2");
}

/// A mode that no word consumes is reported where it was written.
#[test]
fn a_dangling_mode_is_an_error() {
    for source in ["KEEP", "1 2 ADD STAK", "{ 1 KEEP } EXEC"] {
        assert!(
            matches!(failure(source), Error::DanglingMode { .. }),
            "`{source}` should be DanglingMode"
        );
    }
}

/// A quote boundary saves the surrounding mode, starts the body at the
/// default, and restores it on the way out.
#[test]
fn quote_boundaries_scope_the_mode() {
    // The inner body starts at the default even though the outer flow is
    // mid-program, and the outer mode is untouched by the quote.
    assert_eq!(line("1 2 { 3 4 ADD } EXEC"), "1 2 7");
    assert_eq!(line("[ 1 2 3 ] { KEEP NEG ADD } MAP"), "[ 0 0 0 ]");
    // A basin is a body too.
    assert_eq!(line("[ 1 2 3 STAK ADD ]"), "[ 6 ]");
}

/// A word whose stack effect is not statically known has no operand region for
/// the mode layer to select, so it refuses the mode instead of ignoring it.
///
/// This is about the *stack effect*, not about how the word is implemented.
/// `MAP` needs the interpreter to run a quote and still takes exactly two
/// values and leaves one, so `KEEP MAP` works; `EXEC` and every user
/// definition leave whatever their body leaves, so they do not.
#[test]
fn genuinely_dynamic_words_refuse_modes() {
    assert!(matches!(
        failure("1 { 1 ADD } KEEP EXEC"),
        Error::ModeUnsupported { .. }
    ));
    assert!(matches!(
        failure("{ 1 ADD } \"BUMP\" DEF 1 KEEP BUMP"),
        Error::ModeUnsupported { .. }
    ));
    // ...and a fixed effect is enough, whatever the dispatch.
    assert_eq!(line("[ 1 ] { 1 ADD } KEEP MAP"), "[ 1 ] { 1 ADD } [ 2 ]");
}

/// After an error the next fragment starts at the default mode; nothing is
/// inherited across the failure.
#[test]
fn an_error_leaves_no_armed_mode_behind() {
    let mut interpreter = ajisai_core::Interpreter::new();
    assert!(interpreter.execute("1 2 KEEP NOSUCHWORD").is_err());
    interpreter.execute("10 20 ADD").expect("recovers cleanly");
    assert_eq!(
        ajisai_core::render_stack(&interpreter).last().unwrap(),
        "30"
    );
}

/// Roles travel with values through the flow-shaping words for free, because
/// a value's reading lives on the value and nowhere else.
#[test]
fn modes_carry_roles_with_values() {
    assert_eq!(line("[ 1 3 ] >INTERVAL DUP"), "1..3 1..3");
    assert_eq!(line("\"hi\" 1 SWAP"), "1 \"hi\"");
    assert_eq!(line("[ 1 3 ] >INTERVAL KEEP ROLE"), "1..3 \"INTERVAL\"");
}

/// What `STAK` means for a word is declared by the word, not derived from how
/// many operands it takes.
///
/// The derived rule — "two in, one out, therefore foldable" — was the same
/// mistake as Flow Mass Conservation: a count of operands is not a meaning.
/// It made these three programs answer nonsense.
#[test]
fn stak_refuses_words_that_have_no_fold() {
    // `1 1 1 STAK EQ` computed EQ(EQ(1, 1), 1) — EQ(TRUE, 1) — and answered
    // FALSE about three equal values.
    for source in [
        "1 1 1 STAK EQ",
        "1 2 3 STAK LT",
        "1 2 3 STAK GE",
        "[ 1 2 ] 0 3 STAK NTH",
        "[ 1 ] 2 3 STAK APPEND",
        "1 2 3 STAK RANGE",
    ] {
        assert!(
            matches!(failure(source), Error::ModeUnsupported { .. }),
            "`{source}` should be refused, not folded"
        );
    }
    // A flow of one used to return without running the word at all, so
    // `7 STAK EQ` left a number where the contract promises a truth value.
    assert!(matches!(
        failure("7 STAK EQ"),
        Error::ModeUnsupported { .. }
    ));
}

/// The words that do fold are closed: the result of one step is a legitimate
/// operand for the next, whatever the types involved.
#[test]
fn stak_folds_only_closed_operations() {
    assert_eq!(line("1 2 3 4 STAK ADD"), "10");
    assert_eq!(line("2 3 4 STAK MUL"), "24");
    assert_eq!(line("5 2 9 STAK MAX"), "9");
    assert_eq!(line("TRUE FALSE UNKNOWN STAK AND"), "FALSE");
    assert_eq!(line("FALSE UNKNOWN STAK OR"), "UNKNOWN");
    assert_eq!(line("[ 1 ] [ 2 ] [ 3 ] STAK CONCAT"), "[ 1 2 3 ]");
    // A flow of one folds to itself, and that is type-correct precisely
    // because the operation is closed.
    assert_eq!(line("7 STAK ADD"), "7");
}

/// `KEEP` works for any word that declares a fixed stack effect, whatever the
/// implementation needs to do internally. Refusing it for the higher-order
/// words was an implementation detail leaking into the language.
#[test]
fn keep_reaches_words_that_draw_their_own_operands() {
    assert_eq!(
        line("[ 1 2 ] { 2 MUL } KEEP MAP"),
        "[ 1 2 ] { 2 MUL } [ 2 4 ]"
    );
    assert_eq!(
        line("[ 1 2 3 ] { 1 GT } KEEP FILTER"),
        "[ 1 2 3 ] { 1 GT } [ 2 3 ]"
    );
    assert_eq!(line("1 2 KEEP DEPTH"), "1 2 2");
    // `EXEC` alone is genuinely dynamic — what it leaves depends on the quote
    // — so there is no operand region to keep.
    assert!(matches!(
        failure("1 { 1 ADD } KEEP EXEC"),
        Error::ModeUnsupported { .. }
    ));
}
