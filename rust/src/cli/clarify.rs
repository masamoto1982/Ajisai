//! Clarification — the "approach 4" dialogue layer of the natural-language
//! surface design note (`docs/dev/natural-language-surface-design.md` §6):
//! turn an *undecided* signal into a plain-language clarifying question with
//! concrete choices, instead of guessing. Each choice carries the Ajisai sugar
//! it resolves to, so an answer maps straight back to code.
//!
//! This binds the signals the earlier approaches already produce and the CLI
//! already surfaces:
//!   - modifier ambiguity (approach 3, `modifier::ModifierInference`): a
//!     per-axis conflict becomes one question for that axis;
//!   - an unguarded NIL source (approach 2 light, `plan_check::PlanCheck`):
//!     a "supply a fallback?" question.
//!
//! Minimization (design note §6): a question is emitted only for an axis that
//! is actually undecided, and the NIL question is suppressed when a fallback is
//! already present (that gating lives in `plan_check`). The comparison-UNKNOWN
//! clarification (`agreedPrefix`, SPEC §7.4.1) is **deferred**: the runtime U
//! value's `agreedPrefix` is not yet surfaced to the CLI, so there is nothing
//! to drive it here without a separate value-protocol change.
//!
//! It executes nothing and defines no semantics (canonical: `SPECIFICATION.html`).

use super::explain::Lang;
use super::modifier::ModifierInference;
use super::plan_check::PlanCheck;

/// Which undecided signal a clarification came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClarKind {
    TargetAxis,
    ConsumeAxis,
    UnguardedNil,
}

impl ClarKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ClarKind::TargetAxis => "targetAxis",
            ClarKind::ConsumeAxis => "consumeAxis",
            ClarKind::UnguardedNil => "unguardedNil",
        }
    }
}

/// One answer to a clarifying question. `apply` is the Ajisai modifier sugar /
/// word the choice resolves to, or `None` when the choice is "leave it as is"
/// (no code change).
pub(crate) struct Choice {
    pub label: String,
    pub apply: Option<String>,
}

/// A plain-language clarifying question (L0/L1; design note §2, §6).
pub(crate) struct Clarification {
    pub kind: ClarKind,
    pub question: String,
    pub choices: Vec<Choice>,
}

fn choice(label: &str, apply: Option<&str>) -> Choice {
    Choice {
        label: label.to_string(),
        apply: apply.map(str::to_string),
    }
}

/// Clarifications implied by a modifier inference: one question per ambiguous
/// axis. Empty when the inference is unambiguous (the defaults are taken
/// silently, design note §2 — no question for a merely-defaulted axis).
pub(crate) fn from_modifier(inference: &ModifierInference, lang: Lang) -> Vec<Clarification> {
    let mut out = Vec::new();
    if inference.target_ambiguous {
        out.push(Clarification {
            kind: ClarKind::TargetAxis,
            question: match lang {
                Lang::Ja => "先頭の値だけに作用しますか、スタック全体に作用しますか？".to_string(),
                Lang::En => "Act on just the top value, or on the whole stack?".to_string(),
            },
            choices: match lang {
                Lang::Ja => vec![choice("先頭だけ", Some(".")), choice("全体に", Some(".."))],
                Lang::En => vec![
                    choice("just the top", Some(".")),
                    choice("the whole stack", Some("..")),
                ],
            },
        });
    }
    if inference.consume_ambiguous {
        out.push(Clarification {
            kind: ClarKind::ConsumeAxis,
            question: match lang {
                Lang::Ja => "元の値を残しますか、消費しますか？".to_string(),
                Lang::En => "Keep the original value, or consume it?".to_string(),
            },
            choices: match lang {
                Lang::Ja => vec![
                    choice("残す（分岐）", Some(",,")),
                    choice("消費する", Some(",")),
                ],
                Lang::En => vec![
                    choice("keep it (branch)", Some(",,")),
                    choice("consume it", Some(",")),
                ],
            },
        });
    }
    out
}

/// Clarifications implied by a plan check: an unguarded NIL source becomes a
/// "supply a fallback?" question. Empty when there is no NIL source or a
/// fallback is already present (the gating in `PlanCheck`).
pub(crate) fn from_plan_check(check: &PlanCheck, lang: Lang) -> Vec<Clarification> {
    if check.may_bubble.is_empty() || check.has_fallback {
        return Vec::new();
    }
    let words = check.may_bubble.join(", ");
    vec![Clarification {
        kind: ClarKind::UnguardedNil,
        question: match lang {
            Lang::Ja => format!(
                "{} は値が得られないこと（NIL）があります。どうしますか？",
                words
            ),
            Lang::En => format!(
                "{} can fail to produce a value (NIL). How should that be handled?",
                words
            ),
        },
        choices: match lang {
            Lang::Ja => vec![
                choice("既定値で補う", Some("^")),
                choice("NIL のまま流す", None),
            ],
            Lang::En => vec![
                choice("supply a fallback value", Some("^")),
                choice("let the NIL flow on", None),
            ],
        },
    }]
}
