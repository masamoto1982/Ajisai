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

use crate::coreword_registry::{get_builtin_word_registry, NilPolicy};
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
    /// Boolean AND: a NIL operand passes through.
    ThreeValAnd,
    /// Boolean OR: a NIL operand passes through.
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
    // MOD / FLOOR / ROUND create NIL on a domain miss and are covered
    // by projecting_word_set_matches_registry.
    // The comparison words pass NIL operands through. Comparison itself is
    // total (LANG.VALUES.EXACT), so they never create NIL.
    ("EQ", NilClass::BinaryBlanket),
    ("NEQ", NilClass::BinaryBlanket),
    ("LT", NilClass::BinaryBlanket),
    ("LTE", NilClass::BinaryBlanket),
    ("GT", NilClass::BinaryBlanket),
    ("GTE", NilClass::BinaryBlanket),
    ("NOT", NilClass::UnaryNil),
    ("AND", NilClass::ThreeValAnd),
    ("OR", NilClass::ThreeValOr),
];

/// Categories whose Core passthrough words this suite is responsible for.
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
        let covered = meta.nil_policy == NilPolicy::Passthrough
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
            // AND / OR pass NIL through rather than absorbing it into a
            // definite operand; covered by the corpus's logic cases.
            NilClass::ThreeValAnd | NilClass::ThreeValOr => {}
        }
    }
}

// --- Bubble creation: Projecting / CreatesNil words (SPEC §11.2) ----------

/// Projecting words: a well-formed domain miss yields Bubble/NIL with a
/// reason; malformed use raises an ordinary error.
const PROJECTING_WORDS: &[&str] = &[
    "DIV",
    "FILL",
    "FLOOR",
    "GET",
    "INDEX-OF",
    "MOD",
    "NIL-REASON",
    "NUM",
    "QUANTIZE",
    "RANGE",
    "ROUND",
    "SQRT",
    "STR",
];

/// Declaring a projection condition is a claim that the Word can hand back a
/// NIL it produced, so **every** Word that declares one carries a behavioral
/// probe. Declaring a NIL *policy* is not that claim: ABS/NEG/SIGN/MIN/MAX
/// declare `passthroughThenProject` with a projection of `never`, so nothing
/// about them can produce a NIL and they need no probe. The condition is what
/// this set is keyed on.
///
/// It used to be keyed on the policy as well — `createsNil` or
/// `passthroughThenProject`, *and* a declared condition. That conjunction let a
/// Word declare a projection and still escape every probe just by having some
/// other policy, and four Words did. Three of them were declaring something
/// untrue: `SORT` (`passthrough`) and `UNIQUE` (`rejectNil`) declared
/// `emptyVector`, a condition no program can reach because an empty vector is
/// inexpressible, and `NIL?` (`consumeNil`) declared the same condition as
/// `NIL-REASON` but answers `FALSE`, never NIL. All
/// three are now `never`. The fourth, `NIL-REASON`, genuinely projects under
/// `consumeNil`, so keying on the condition alone brings it into the probed set
/// instead of leaving it unchecked.
#[test]
fn projecting_word_set_matches_registry() {
    let mut registry: Vec<&str> = crate::kernel::generated::GENERATED_WORDS
        .iter()
        .filter(|word| word.projection.is_some())
        .map(|word| word.name)
        .collect();
    registry.sort_unstable();
    let mut expected: Vec<&str> = PROJECTING_WORDS.to_vec();
    expected.sort_unstable();
    assert_eq!(
        registry, expected,
        "projecting word set drifted from PROJECTING_WORDS; add a Bubble \
         behavioral probe for any new word"
    );
}

/// Run `code`, require it to land on a NIL, and answer the reason that NIL
/// carries. Shared by every Bubble-creation probe below: a projection is
/// "NIL with the reason the contract registers" (LANG.FAILURE.PROJECT), so
/// every probe asks the same two questions and only the reason differs.
async fn projected_reason(code: &str) -> Option<String> {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
    let answer = interp.stack.last().expect("a projecting Word pushes an answer");
    assert!(answer.is_nil(), "`{code}` must project NIL, got {answer:?}");
    answer
        .absence_metadata()
        .and_then(|absence| absence.reason.as_ref())
        .map(|reason| reason.as_protocol_str().to_string())
}

/// Run `code` and answer the Text it leaves on top, for the non-projecting
/// half of a probe.
async fn text_answer(code: &str) -> Option<String> {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
    crate::interpreter::value_extraction_helpers::value_as_string(interp.stack.last().unwrap())
}

/// `QUANTIZE` projects on the condition it declares: a denominator that is not
/// a positive integer is a well-formed operand outside the Word's domain, so it
/// yields a reasoned NIL rather than an error — the `SQRT` of a negative rule.
/// Malformed use (an operand that is not a single number at all) still raises.
#[tokio::test]
async fn bubble_creation_quantize_projects_on_a_denominator_outside_its_domain() {
    for code in ["7 0 QUANTIZE", "7 -4 QUANTIZE", "7 3/2 QUANTIZE"] {
        assert_eq!(projected_reason(code).await.as_deref(), Some("domainMiss"));
    }

    let mut malformed = Interpreter::new();
    assert!(
        malformed.execute("7 'ten' QUANTIZE").await.is_err(),
        "a denominator that is not a number at all is malformed use, not a domain miss"
    );
}

/// `STR` projects on the condition it declares: the sealed numeric grammar
/// writes integers and ratios, so an exact irrational has no lexeme and there
/// is no text to answer with.
///
/// It used to answer anyway, with a continued-fraction convergent — `2 SQRT STR`
/// gave `'665857/470832'`, so `2 SQRT STR NUM` was a *different number* than
/// `2 SQRT` with nothing on the stack to say so. That silence was worst inside
/// `REFLECT`-based partial application, where `STR` is how a runtime value
/// becomes a token: building a closure over an exact irrational quietly
/// substituted a rational look-alike for it.
#[tokio::test]
async fn bubble_creation_str_projects_on_a_number_with_no_lexeme() {
    for code in ["2 SQRT STR", "2 SQRT 3 SQRT ADD STR", "[ 1 2 ] 2 SQRT MUL STR"] {
        assert_eq!(
            projected_reason(code).await.as_deref(),
            Some("invalidEncoding")
        );
    }

    // An exact real that collapses to a rational *does* have a lexeme, and a
    // rational is spelled exactly. Only the irrational case projects.
    assert_eq!(text_answer("4 SQRT STR").await.as_deref(), Some("2"));

    // The escape hatch is a Word, not a silent default: the caller names the
    // resolution, so the approximation is visible in the source.
    assert_eq!(
        text_answer("2 SQRT 10000 QUANTIZE STR").await.as_deref(),
        Some("7071/5000")
    );
}

/// `NIL-REASON` projects on the condition it declares: a value that is not an
/// operational NIL carrying a reason has no reason to read, so the answer is
/// NIL rather than an error. A reasoned NIL answers with its reason string.
///
/// The projected NIL carries the reason the contract registers, `notAvailable`.
/// It used to be a bare literal NIL, which made
/// `projection.reason: "notAvailable"` the one declared projection reason no
/// program could observe — `5 NIL-REASON NIL-REASON` answered NIL instead of
/// naming why. `LANG.FAILURE.PROJECT` requires a projection to produce "NIL
/// with the reason its contract registers", and `LANG.VALUES.NIL` makes the
/// reason a NIL's entire observable content.
#[tokio::test]
async fn bubble_creation_nil_reason_projects_on_a_reasonless_value() {
    for code in ["5 NIL-REASON", "[ 1 2 ] NIL-REASON", "'ab' NIL-REASON"] {
        assert_eq!(projected_reason(code).await.as_deref(), Some("notAvailable"));
    }

    // Reading the projected NIL is what makes the registered reason
    // observable from inside the language.
    assert_eq!(
        text_answer("5 NIL-REASON NIL-REASON").await.as_deref(),
        Some("notAvailable"),
        "the projected NIL must name its own reason"
    );

    // The retained operand keeps its place under the answer, and a NIL that
    // does carry a reason reads back as that reason rather than projecting.
    let mut interp = Interpreter::new();
    interp.execute("1 0 / NIL-REASON").await.unwrap();
    let answer = interp.stack.last().expect("NIL-REASON pushes an answer");
    assert!(
        !answer.is_nil(),
        "a reasoned NIL must read back its reason, got NIL"
    );
}

/// `NIL?` answers a question; it never projects. It declared
/// `valueIsNotOperationalNilOrFieldAbsent` → `notAvailable`, the same condition
/// as `NIL-REASON`, but a predicate that returned NIL for "not a NIL" could not
/// be asked its question. Pinned so the declaration cannot drift back.
#[tokio::test]
async fn nil_check_answers_rather_than_projecting() {
    for (code, expected_nil) in [
        ("5 NIL?", false),
        ("[ 1 2 ] NIL?", false),
        ("'ab' NIL?", false),
        ("NIL NIL?", true),
        ("1 0 / NIL?", true),
    ] {
        let mut interp = Interpreter::new();
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
        let answer = interp.stack.last().expect("NIL? pushes an answer");
        assert!(
            !answer.is_nil(),
            "`{code}` must answer TRUE or FALSE, never NIL"
        );
        assert_eq!(
            answer.as_truth(),
            Some(expected_nil),
            "`{code}` answered the wrong way"
        );
    }
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
    for name in &["FLOOR", "ROUND"] {
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
