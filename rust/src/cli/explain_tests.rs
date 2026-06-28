//! Tests for the deterministic plain-language projection (`super::explain`).
//! Each test pins the *tone* selection (Stagnation / Bubble / Channel error)
//! and the recoverability-keyed next step, in both languages. Design note:
//! `docs/dev/natural-language-surface-design.md` §3.

use super::explain::{explain, Lang};
use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{DebugDiagnosis, ErrorPhase};

fn unknown_word_diagnosis() -> DebugDiagnosis {
    DebugDiagnosis::from_error_category(
        ErrorPhase::ResolveWord,
        Some("FROBNICATE"),
        Some(&ErrorCategory::UnknownWord),
        None,
        1,
        1,
        Some("Unknown word: FROBNICATE".to_string()),
    )
}

fn division_diagnosis() -> DebugDiagnosis {
    DebugDiagnosis::from_error_category(
        ErrorPhase::ExecuteWord,
        Some("DIV"),
        Some(&ErrorCategory::DivisionByZero),
        None,
        2,
        1,
        Some("division by zero".to_string()),
    )
}

#[test]
fn unknown_word_projects_channel_error_tone_ja() {
    let diagnosis = unknown_word_diagnosis();
    let ai = diagnosis.ai_payload(Some(&ErrorCategory::UnknownWord), None, None, None);
    let explanation = explain(&diagnosis, Some(&ai.recoverability), None, Lang::Ja);

    // The offending word is the subject, not a location.
    assert!(
        explanation.headline.contains("知らない語『FROBNICATE』"),
        "headline was: {}",
        explanation.headline
    );
    // recoverability=fixProgram → "手順（プログラム）を見直して".
    assert!(
        explanation.next_step.contains("手順"),
        "next_step was: {}",
        explanation.next_step
    );
    // L2 details are the diagnosis nextChecks, verbatim and non-empty.
    assert!(!explanation.details.is_empty());
}

#[test]
fn unknown_word_projects_english_utf8_plain_text() {
    let diagnosis = unknown_word_diagnosis();
    let ai = diagnosis.ai_payload(Some(&ErrorCategory::UnknownWord), None, None, None);
    let explanation = explain(&diagnosis, Some(&ai.recoverability), None, Lang::En);

    assert_eq!(explanation.lang, Lang::En);
    assert!(
        explanation
            .headline
            .contains("An unknown word \"FROBNICATE\""),
        "headline was: {}",
        explanation.headline
    );
    // The L0 sentences are UTF-8 plain text in English mode.
    assert!(!explanation.headline.chars().any(char::is_control));
    assert!(!explanation.next_step.chars().any(char::is_control));
}

#[test]
fn division_by_zero_nil_projects_bubble_tone() {
    let diagnosis = division_diagnosis();
    // On a successful run the value bubbles to NIL; the CLI projects
    // handleUnknownOrNil with the absence reason.
    let explanation = explain(
        &diagnosis,
        Some("handleUnknownOrNil"),
        Some("divisionByZero"),
        Lang::Ja,
    );

    assert!(
        explanation.headline.contains("値が得られませんでした")
            && explanation.headline.contains("ゼロ除算"),
        "headline was: {}",
        explanation.headline
    );
    assert!(
        explanation.headline.contains("DIV"),
        "headline should name the word: {}",
        explanation.headline
    );
    // handleUnknownOrNil → fallback advice.
    assert!(
        explanation.next_step.contains("既定値") || explanation.next_step.contains("分岐"),
        "next_step was: {}",
        explanation.next_step
    );
}

#[test]
fn comparison_unknown_projects_stagnation_tone() {
    // A continued-fraction comparison that did not settle within budget
    // (SPEC §7.4.1): why is NilFlow but agreed_prefix is set, so the tone is
    // Stagnation, never absence.
    let diagnosis = DebugDiagnosis::comparison_unknown(Some("LT"), 40);
    let explanation = explain(&diagnosis, None, None, Lang::Ja);

    assert!(
        explanation.headline.contains("決めきれていません"),
        "headline was: {}",
        explanation.headline
    );
    // Must not be described as an absence.
    assert!(
        !explanation.headline.contains("値が得られませんでした"),
        "stagnation must not read as a bubble: {}",
        explanation.headline
    );
}

#[test]
fn unrecognized_recoverability_falls_back_to_inspect_context() {
    let diagnosis = unknown_word_diagnosis();
    let explanation = explain(&diagnosis, Some("someFutureValue"), None, Lang::En);
    assert!(
        explanation
            .next_step
            .contains("Inspect the surrounding context"),
        "next_step was: {}",
        explanation.next_step
    );
}
