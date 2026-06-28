//! Tests for the clarification layer (`super::clarify`). Pins which undecided
//! signals raise a question, the per-axis minimization, the fallback gating,
//! and that each choice carries the Ajisai sugar it resolves to. Design note:
//! `docs/dev/natural-language-surface-design.md` §6.

use super::clarify::{from_modifier, from_plan_check, ClarKind};
use super::explain::Lang;
use super::modifier::infer;
use super::plan_check::check_plan;
use crate::interpreter::Interpreter;

fn plan(src: &str) -> super::plan_check::PlanCheck {
    let interp = Interpreter::new();
    check_plan(&interp, src).expect("well-formed source must tokenize")
}

#[test]
fn unambiguous_modifier_asks_nothing() {
    // A merely-defaulted axis is taken silently (design note §2), not asked.
    let inference = infer("各要素を2倍する", Lang::Ja);
    assert!(from_modifier(&inference, Lang::Ja).is_empty());
}

#[test]
fn conflicting_consume_cues_ask_one_consume_question() {
    let inference = infer("元を残しつつ消費して", Lang::Ja);
    let clarifications = from_modifier(&inference, Lang::Ja);
    assert_eq!(clarifications.len(), 1);
    assert_eq!(clarifications[0].kind, ClarKind::ConsumeAxis);
    // The two choices resolve to the KEEP / EAT sugar.
    let applies: Vec<Option<&str>> = clarifications[0]
        .choices
        .iter()
        .map(|choice| choice.apply.as_deref())
        .collect();
    assert!(applies.contains(&Some(",,")));
    assert!(applies.contains(&Some(",")));
}

#[test]
fn conflicts_on_both_axes_ask_two_questions() {
    let inference = infer("先頭だけ全体に、残しつつ消費して", Lang::Ja);
    let clarifications = from_modifier(&inference, Lang::Ja);
    let kinds: Vec<ClarKind> = clarifications.iter().map(|c| c.kind).collect();
    assert!(kinds.contains(&ClarKind::TargetAxis));
    assert!(kinds.contains(&ClarKind::ConsumeAxis));
    assert_eq!(kinds.len(), 2);
}

#[test]
fn unguarded_nil_asks_with_a_vent_choice() {
    let clarifications = from_plan_check(&plan("[ 1 ] [ 0 ] DIV"), Lang::Ja);
    assert_eq!(clarifications.len(), 1);
    assert_eq!(clarifications[0].kind, ClarKind::UnguardedNil);
    // One choice maps to `^`, the other to "no change" (None).
    let applies: Vec<Option<&str>> = clarifications[0]
        .choices
        .iter()
        .map(|choice| choice.apply.as_deref())
        .collect();
    assert!(applies.contains(&Some("^")));
    assert!(applies.contains(&None));
}

#[test]
fn present_fallback_suppresses_the_nil_question() {
    // Minimization: a `^` already present means there is nothing to ask.
    let clarifications = from_plan_check(&plan("[ 1 ] [ 0 ] DIV ^ [ 99 ]"), Lang::Ja);
    assert!(clarifications.is_empty());
}

#[test]
fn english_clarifications_are_utf8_plain_text() {
    let inference = infer("keep it but also consume it", Lang::En);
    for clarification in from_modifier(&inference, Lang::En) {
        assert!(!clarification.question.chars().any(char::is_control));
        for choice in &clarification.choices {
            assert!(!choice.label.chars().any(char::is_control));
        }
    }
}
