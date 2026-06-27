//! Tests for modifier inference (`super::modifier`). Pins the cue → lattice
//! classification, the defaults, the sugar, and the ambiguity signal that
//! approach 4 turns into a clarifying question. Design note:
//! `docs/dev/natural-language-surface-design.md` §5.

use super::explain::Lang;
use super::modifier::{infer, Consume, Target};

#[test]
fn keep_and_whole_maps_to_stak_keep() {
    let inference = infer("残したまま全体に変換して", Lang::Ja);
    assert_eq!(inference.target, Target::Stack);
    assert_eq!(inference.consume, Consume::Keep);
    assert!(inference.sugar.contains(".."));
    assert!(inference.sugar.contains(",,"));
    assert!(!inference.ambiguous);
}

#[test]
fn english_keep_defaults_target_to_top() {
    let inference = infer("keep the original", Lang::En);
    assert_eq!(inference.consume, Consume::Keep);
    assert_eq!(inference.target, Target::Top); // no target cue → default
    assert!(inference.consume_explicit);
    assert!(!inference.target_explicit);
    assert_eq!(inference.sugar, ",,");
    assert!(inference.rationale.is_ascii());
}

#[test]
fn fallback_phrase_requests_vent() {
    let inference = infer("失敗したら既定値で補って", Lang::Ja);
    assert!(inference.fallback);
    assert!(inference.sugar.contains('^'));
}

#[test]
fn conflicting_cues_are_ambiguous() {
    // "残し" (keep) and "消費" (consume) on the same axis.
    let inference = infer("元を残しつつ消費して", Lang::Ja);
    assert!(inference.ambiguous);
}

#[test]
fn no_cue_takes_defaults_without_ambiguity() {
    let inference = infer("各要素を2倍する", Lang::Ja);
    assert_eq!(inference.target, Target::Top);
    assert_eq!(inference.consume, Consume::Eat);
    assert!(inference.sugar.is_empty());
    assert!(!inference.ambiguous);
    assert!(!inference.target_explicit);
    assert!(!inference.consume_explicit);
}
