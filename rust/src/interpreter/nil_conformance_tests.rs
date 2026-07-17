//! Contract-driven NIL / Bubble-Rule behavioral conformance (phase B).
//!
//! Earlier coverage asserted the *metadata* of the Coreword registry
//! (every word has a contract, fields are internally consistent). This
//! suite closes the complementary gap: it drives the interpreter and
//! asserts the *runtime* honors each word's declared `nil_policy` and the
//! Bubble Rule (SPEC §4.5.1, §7.12, §11.2).
//!
//! The completeness tests are registry-driven: they enumerate the live
//! registry and fail if a newly added Core passthrough / projecting word
//! is not covered by a behavioral probe here. That coupling is what lets
//! coverage grow automatically with the language instead of drifting.
//!
//! Trace: docs/quality/TRACEABILITY_MATRIX.md (NIL/Bubble conformance).

use crate::coreword_registry::{get_builtin_word_registry, CanonicalHome, NilPolicy};
use crate::error::NilReason;
use crate::interpreter::Interpreter;
use crate::types::Value;

async fn run(code: &str) -> Result<Vec<Value>, String> {
    let mut interp = Interpreter::new();
    interp.execute(code).await.map_err(|e| e.to_string())?;
    Ok(interp.get_stack().to_vec())
}

async fn run_ok(code: &str) -> Vec<Value> {
    run(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"))
}

fn is_nil(v: &Value) -> bool {
    v.is_nil()
}

fn is_true(v: &Value) -> bool {
    v.as_truth() == Some(true)
}

fn is_false(v: &Value) -> bool {
    v.as_truth() == Some(false)
}

fn reason_of(v: &Value) -> Option<NilReason> {
    v.nil_reason().cloned()
}

// --- Core passthrough classification (SPEC §7.12) -------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
enum NilClass {
    /// Any NIL operand collapses the result to NIL (arithmetic, comparison).
    BinaryBlanket,
    /// Unary word: NIL operand yields NIL.
    UnaryNil,
    /// Ternary comparison `[ a ] [ b ] [ budget ] -> ...` whose a/b operands
    /// are NIL-passthrough (COMPARE-WITHIN, SPEC §7.4.2). A NIL in either
    /// value operand yields NIL; the budget operand is a plain integer.
    TernaryValueNil,
    /// Three-valued AND: NIL with definite-false => false, else NIL.
    ThreeValAnd,
    /// Three-valued OR: NIL with definite-true => true, else NIL.
    ThreeValOr,
}

/// The Core (canonical-home == Core) passthrough words and their NIL
/// behavior class. `core_passthrough_completeness` keeps this in lockstep
/// with the registry: adding a Core arithmetic/comparison/logic passthrough
/// word without classifying it here fails the build.
const CORE_PASSTHROUGH: &[(&str, NilClass)] = &[
    ("ADD", NilClass::BinaryBlanket),
    ("SUB", NilClass::BinaryBlanket),
    ("MUL", NilClass::BinaryBlanket),
    // MOD/FLOOR/CEIL/ROUND are Projecting/CreatesNil: on ExactScalar (CF)
    // operands the partial-quotient budget can exhaust, yielding an
    // Undecidable NIL (SPEC §7.4.1). They are covered by
    // projecting_word_set_matches_registry and the arithmetic NIL-input
    // probes below.
    // Comparison words (EQ/NEQ/LT/LTE/GT/GTE) are Projecting/Passthrough
    // per SPEC §7.14 (revised): budget exhaustion now yields the logical
    // truth value Unknown (U), not a reasoned NIL, so they no longer create
    // NIL — but they still pass NIL operands through (§7.12), which is the
    // BinaryBlanket behavior verified here.
    ("EQ", NilClass::BinaryBlanket),
    ("NEQ", NilClass::BinaryBlanket),
    ("LT", NilClass::BinaryBlanket),
    ("LTE", NilClass::BinaryBlanket),
    ("GT", NilClass::BinaryBlanket),
    ("GTE", NilClass::BinaryBlanket),
    // COMPARE-WITHIN (SPEC §7.4.2) is Projecting/Passthrough like the six
    // relations, but ternary: its a/b value operands pass NIL through while
    // the trailing budget operand is a plain positive integer.
    ("COMPARE-WITHIN", NilClass::TernaryValueNil),
    ("NOT", NilClass::UnaryNil),
    ("AND", NilClass::ThreeValAnd),
    ("OR", NilClass::ThreeValOr),
];

/// Categories whose Core passthrough words this suite is responsible for.
/// Module-canonical passthrough words (MUSIC/MATH/...) have their own
/// operand shapes and import needs and are covered by module suites.
const COVERED_CATEGORIES: &[&str] = &["arithmetic", "comparison", "logic"];

fn lookup_class(name: &str) -> Option<NilClass> {
    CORE_PASSTHROUGH
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| *c)
}

#[test]
fn core_passthrough_completeness() {
    // Every Core passthrough word in a covered category must be classified,
    // and every classified word must still be a Core passthrough word.
    for meta in get_builtin_word_registry() {
        let covered = meta.canonical_home == CanonicalHome::Core
            && meta.nil_policy == NilPolicy::Passthrough
            && COVERED_CATEGORIES.contains(&meta.category.as_str());
        if covered {
            assert!(
                lookup_class(&meta.name).is_some(),
                "Core passthrough word `{}` (category {}) is not classified in \
                 CORE_PASSTHROUGH; add its NIL behavior class",
                meta.name,
                meta.category
            );
        }
    }

    for (name, _) in CORE_PASSTHROUGH {
        let meta = get_builtin_word_registry()
            .iter()
            .find(|m| &m.name == name)
            .unwrap_or_else(|| panic!("classified word `{name}` is not registered"));
        assert_eq!(
            meta.nil_policy,
            NilPolicy::Passthrough,
            "`{name}` is classified as passthrough but registry says {:?}",
            meta.nil_policy
        );
        assert_eq!(
            meta.canonical_home,
            CanonicalHome::Core,
            "`{name}` is classified as a Core passthrough word but is not Core"
        );
    }
}

#[tokio::test]
async fn passthrough_blanket_and_unary_collapse_to_nil() {
    for (name, class) in CORE_PASSTHROUGH {
        match class {
            NilClass::BinaryBlanket => {
                // §4.5.1: any NIL operand => single NIL result.
                for code in [
                    format!("NIL NIL {name}"),
                    format!("1 NIL {name}"),
                    format!("NIL 1 {name}"),
                ] {
                    let stack = run_ok(&code).await;
                    assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
                    assert!(is_nil(&stack[0]), "`{code}` must produce NIL");
                }
            }
            NilClass::UnaryNil => {
                let code = format!("NIL {name}");
                let stack = run_ok(&code).await;
                assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
                assert!(is_nil(&stack[0]), "`{code}` must produce NIL");
            }
            NilClass::TernaryValueNil => {
                // §7.12 / §7.4.2: a NIL in either value operand (with a
                // valid budget) passes through to a single NIL result.
                for code in [
                    format!("NIL 1 8 {name}"),
                    format!("1 NIL 8 {name}"),
                    format!("NIL NIL 8 {name}"),
                ] {
                    let stack = run_ok(&code).await;
                    assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
                    assert!(is_nil(&stack[0]), "`{code}` must produce NIL");
                }
            }
            // Three-valued words are verified by their dedicated truth-table
            // tests below; the completeness test guarantees they are present.
            NilClass::ThreeValAnd | NilClass::ThreeValOr => {}
        }
    }
}

#[tokio::test]
async fn three_valued_and() {
    // SPEC §7.12: NIL with a definite false collapses to false; otherwise NIL.
    let nil_cases = ["TRUE NIL AND", "NIL TRUE AND", "NIL NIL AND"];
    for code in nil_cases {
        let stack = run_ok(code).await;
        assert!(is_nil(&stack[0]), "`{code}` must be NIL");
    }
    for code in ["FALSE NIL AND", "NIL FALSE AND"] {
        let stack = run_ok(code).await;
        assert!(is_false(&stack[0]), "`{code}` must collapse to FALSE");
    }
}

#[tokio::test]
async fn three_valued_or() {
    // SPEC §7.12: NIL with a definite true collapses to true; otherwise NIL.
    for code in ["FALSE NIL OR", "NIL FALSE OR", "NIL NIL OR"] {
        let stack = run_ok(code).await;
        assert!(is_nil(&stack[0]), "`{code}` must be NIL");
    }
    for code in ["TRUE NIL OR", "NIL TRUE OR"] {
        let stack = run_ok(code).await;
        assert!(is_true(&stack[0]), "`{code}` must collapse to TRUE");
    }
}

// --- Three-valued logic with the logical Unknown (SPEC §7.5, §4.5.2) ------

/// Run `rest` with the logical Unknown (U) already on the stack.
///
/// Comparison is total over Tier ≤ 1 (everything the current vocabulary
/// constructs, SPEC §7.4), so U is produced authentically through a
/// `COMPARE-WITHIN` against a **Tier 2** observation — a type-level
/// starvation witness no word can build yet — exhausting the explicit
/// 8-step water budget.
async fn run_ok_with_u(rest: &str) -> Vec<Value> {
    use crate::types::exact::{Computable, ExactReal};
    let mut interp = Interpreter::new();
    interp
        .stack
        .push(Value::from_exact_real(ExactReal::Computable(
            Computable::vanishing(),
        )));
    let code = format!("0 8 COMPARE-WITHIN {rest}");
    interp
        .execute(&code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"));
    interp.get_stack().to_vec()
}

fn is_unknown(v: &Value) -> bool {
    v.is_unknown()
}

#[tokio::test]
async fn unknown_is_produced_by_undecidable_comparison() {
    let stack = run_ok_with_u("").await;
    assert_eq!(stack.len(), 1);
    assert!(
        is_unknown(&stack[0]),
        "a starved Tier 2 comparison must yield Unknown"
    );
    assert_eq!(stack[0].truth_value(), Some("unknown"));
}

#[tokio::test]
async fn k3_and_or_not_with_unknown() {
    // U AND TRUE = U ; U AND FALSE = FALSE (F absorbs).
    let s = run_ok_with_u("TRUE AND").await;
    assert!(is_unknown(&s[0]), "U AND TRUE must be Unknown");
    let s = run_ok_with_u("FALSE AND").await;
    assert!(is_false(&s[0]), "U AND FALSE must be FALSE");

    // U OR FALSE = U ; U OR TRUE = TRUE (T absorbs).
    let s = run_ok_with_u("FALSE OR").await;
    assert!(is_unknown(&s[0]), "U OR FALSE must be Unknown");
    let s = run_ok_with_u("TRUE OR").await;
    assert!(is_true(&s[0]), "U OR TRUE must be TRUE");

    // NOT U = U.
    let s = run_ok_with_u("NOT").await;
    assert!(is_unknown(&s[0]), "NOT U must be Unknown");
}

#[tokio::test]
async fn nil_takes_priority_over_unknown_in_logic() {
    // SPEC §4.5.2: when NIL and U meet with no absorbing definite, NIL wins
    // (it carries a diagnostic reason that must be preserved). The result is
    // an operational NIL, not U.
    let s = run_ok_with_u("NIL AND").await;
    assert!(
        is_nil(&s[0]) && !is_unknown(&s[0]),
        "U AND NIL must be NIL, not Unknown"
    );
    let s = run_ok_with_u("NIL OR").await;
    assert!(
        is_nil(&s[0]) && !is_unknown(&s[0]),
        "U OR NIL must be NIL, not Unknown"
    );

    // But an absorbing definite still decides over both.
    let s = run_ok_with_u("FALSE AND").await;
    assert!(
        is_false(&s[0]),
        "U AND FALSE must be FALSE even though U is present"
    );
}

#[tokio::test]
async fn unknown_passes_through_pipeline_distinct_from_nil() {
    // U flowing through a logic pipeline stays U (not collapsed to NIL) until
    // an absorbing element or a real NIL intervenes.
    let s = run_ok_with_u("NOT NOT").await;
    assert!(is_unknown(&s[0]), "NOT NOT U must remain Unknown");
}

// --- Bubble creation: Projecting / CreatesNil words (SPEC §11.2) ----------

/// Projecting words: a well-formed domain miss yields Bubble/NIL with a
/// reason; malformed use raises an ordinary error. `READ` is host/serial
/// dependent (needs a port) so only its registry presence is asserted.
const PROJECTING_WORDS: &[&str] = &[
    "CEIL",
    "CHR",
    "DIV",
    "FLOOR",
    "GET",
    "INDEX-OF",
    "MOD",
    "NUM",
    "PARSE-ISO",
    "POW",
    "QUANTIZE",
    "QUANTIZE-CEIL",
    "QUANTIZE-FLOOR",
    "QUANTIZE-HALF-AWAY",
    "QUANTIZE-TRUNC",
    "READ",
    "ROUND",
];

#[test]
fn projecting_word_set_matches_registry() {
    let mut registry: Vec<&str> = get_builtin_word_registry()
        .iter()
        .filter(|m| m.nil_policy == NilPolicy::CreatesNil)
        .map(|m| m.name.as_str())
        .collect();
    registry.sort_unstable();
    let mut expected: Vec<&str> = PROJECTING_WORDS.to_vec();
    expected.sort_unstable();
    assert_eq!(
        registry, expected,
        "CreatesNil word set drifted from PROJECTING_WORDS; add a Bubble \
         behavioral probe for any new word"
    );
}

#[tokio::test]
async fn bubble_creation_well_formed_domain_miss() {
    // divisor reduces to zero
    let stack = run_ok("1 0 DIV").await;
    assert!(is_nil(&stack[0]));
    assert_eq!(reason_of(&stack[0]), Some(NilReason::DivisionByZero));

    // valid vector, out-of-range index: source kept, NIL pushed
    let stack = run_ok("[ 1 2 3 ] [ 10 ] GET").await;
    assert_eq!(stack.len(), 2, "GET keeps its source vector");
    assert!(is_nil(stack.last().unwrap()));
    assert_eq!(
        reason_of(stack.last().unwrap()),
        Some(NilReason::IndexOutOfBounds)
    );

    // unparseable text
    let stack = run_ok("'abc' NUM").await;
    assert!(is_nil(&stack[0]));
    assert_eq!(reason_of(&stack[0]), Some(NilReason::InvalidEncoding));

    // code point above the Unicode scalar range (0x10FFFF == 1114111)
    let stack = run_ok("1114112 CHR").await;
    assert!(is_nil(&stack[0]));
    assert_eq!(reason_of(&stack[0]), Some(NilReason::InvalidEncoding));

    // zero raised to a negative exponent: well-formed domain miss
    let stack = run_ok("'math' IMPORT 0 -1 POW").await;
    assert!(is_nil(stack.last().unwrap()));
    assert_eq!(
        reason_of(stack.last().unwrap()),
        Some(NilReason::DivisionByZero)
    );

    // value absent from a valid vector: well-formed search miss
    let stack = run_ok("'algo' IMPORT [ 1 2 3 ] 9 INDEX-OF").await;
    assert!(is_nil(stack.last().unwrap()));
    assert_eq!(
        reason_of(stack.last().unwrap()),
        Some(NilReason::MissingField)
    );

    // well-formed text that is not a valid ISO-8601 civil value
    let stack = run_ok("'time' IMPORT 'not-a-date' PARSE-ISO").await;
    assert!(is_nil(stack.last().unwrap()));
    assert_eq!(
        reason_of(stack.last().unwrap()),
        Some(NilReason::InvalidEncoding)
    );
}

#[tokio::test]
async fn bubble_creation_comparison_nil_input() {
    // Comparison words are Projecting/Passthrough (SPEC §7.14, revised). A
    // NIL operand propagates as NIL output via the passthrough rule
    // (SPEC §4.5.1, §7.12). (Budget exhaustion instead yields Unknown, not
    // a NIL — covered by the comparison Unknown tests.)
    for name in &["EQ", "NEQ", "LT", "LTE", "GT", "GTE"] {
        for code in [
            format!("NIL 1 {name}"),
            format!("1 NIL {name}"),
            format!("NIL NIL {name}"),
        ] {
            let stack = run_ok(&code).await;
            assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
            assert!(
                is_nil(&stack[0]),
                "`{code}` must produce NIL, got {:?}",
                stack[0]
            );
        }
    }
}

#[tokio::test]
async fn projecting_arithmetic_nil_input_passes_through() {
    // MOD/FLOOR/CEIL/ROUND are Projecting/CreatesNil, but a NIL operand
    // still propagates as NIL via the universal Bubble Rule (SPEC §4.5.1)
    // — the CreatesNil policy is about CF-budget exhaustion on irrational
    // operands, not about rejecting NIL inputs.
    for name in &["FLOOR", "CEIL", "ROUND"] {
        let code = format!("NIL {name}");
        let stack = run_ok(&code).await;
        assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
        assert!(is_nil(&stack[0]), "`{code}` must produce NIL");
    }
    for code in ["NIL 1 MOD", "1 NIL MOD", "NIL NIL MOD"] {
        let stack = run_ok(code).await;
        assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
        assert!(is_nil(&stack[0]), "`{code}` must produce NIL");
    }
}

#[tokio::test]
async fn malformed_use_raises_error_not_bubble() {
    // SPEC §11.2: "そもそも使い方が違う -> エラー". A non-numeric DIV operand
    // and a malformed GET index are structural misuse, not domain misses.
    assert!(
        run("'x' 1 DIV").await.is_err(),
        "non-numeric DIV operand must raise an error, not Bubble/NIL"
    );
    assert!(
        run("1 [ 1 2 ] GET").await.is_err(),
        "malformed GET index must raise an error, not Bubble/NIL"
    );
}

// --- VENT (^) replaces Bubble/NIL with a fallback (SPEC §11.2) ---------

#[tokio::test]
async fn or_nil_supplies_fallback_and_clears_reason() {
    // bare NIL replaced by the fallback
    let stack = run_ok("NIL ^ [ 0 ]").await;
    assert_eq!(format!("{}", stack[0]), "[ 0/1 ]");

    // non-NIL value passes through unchanged
    let stack = run_ok("[ 42 ] ^ [ 0 ]").await;
    assert_eq!(format!("{}", stack[0]), "[ 42/1 ]");

    // a reasoned Bubble (division by zero) is replaced; no NIL survives
    let stack = run_ok("1 0 DIV ^ [ 7 ]").await;
    assert!(!is_nil(&stack[0]), "VENT must consume the Bubble");
    assert_eq!(format!("{}", stack[0]), "[ 7/1 ]");
}

// --- A raised error propagates; a well-formed Bubble keeps its reason -----

#[tokio::test]
async fn raised_errors_propagate_instead_of_projecting_to_nil() {
    // stack underflow propagates as an error
    let mut interp = Interpreter::new();
    assert!(
        interp.execute("ADD").await.is_err(),
        "stack underflow must propagate, not project to NIL"
    );

    // unknown word propagates as an error
    let mut interp = Interpreter::new();
    assert!(
        interp.execute("__NO_SUCH_WORD__").await.is_err(),
        "unknown word must propagate, not project to NIL"
    );
}

#[tokio::test]
async fn direct_bubble_preserves_its_own_reason() {
    // A well-formed Bubble (division by zero) keeps its own reason.
    let stack = run_ok("1 0 DIV").await;
    assert!(is_nil(&stack[0]));
    assert_eq!(
        reason_of(&stack[0]),
        Some(NilReason::DivisionByZero),
        "a direct Bubble keeps its DivisionByZero reason"
    );
}

// --- Property: NIL passthrough is total over the numeric domain -----------

#[cfg(test)]
mod properties {
    use super::run;
    use crate::error::NilReason;
    use proptest::prelude::*;

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(f)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        // For any scalar, a NIL co-operand makes a blanket-passthrough binary
        // op total and NIL — never an error, never a definite value.
        #[test]
        fn binary_passthrough_with_nil_is_nil(a in -50i64..50) {
            for op in ["ADD", "SUB", "MUL", "MOD", "LT", "LTE", "GT", "GTE", "EQ", "NEQ"] {
                let stack = block_on(run(&format!("{a} NIL {op}")))
                    .unwrap_or_else(|e| panic!("`{a} NIL {op}` errored: {e}"));
                prop_assert_eq!(stack.len(), 1);
                prop_assert!(stack[0].is_nil(), "`{} NIL {}` must be NIL", a, op);
            }
        }

        // Division by a zero divisor is a total projection to a reasoned Bubble.
        #[test]
        fn division_by_zero_is_reasoned_bubble(a in -50i64..50) {
            let stack = block_on(run(&format!("{a} 0 DIV")))
                .unwrap_or_else(|e| panic!("`{a} 0 DIV` errored: {e}"));
            prop_assert!(stack[0].is_nil());
            prop_assert_eq!(stack[0].nil_reason().cloned(), Some(NilReason::DivisionByZero));
        }
    }
}
