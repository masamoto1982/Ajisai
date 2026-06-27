//! Modifier inference — the "approach 3" classifier of the natural-language
//! surface design note (`docs/dev/natural-language-surface-design.md` §5):
//! map an operation-intent phrase onto the *finite* modifier lattice rather
//! than generating free code. The target space is small and closed —
//! `TOP`/`STAK` × `EAT`/`KEEP`, plus the `VENT` (`^`) fallback — so this is a
//! classification, not generation, and it is deterministic.
//!
//! It recognizes a controlled vocabulary of cues (Japanese and English). An
//! axis with no cue takes its language default (`TOP` / `EAT`, SPEC §6.1/§6.2);
//! conflicting cues on one axis are reported as `ambiguous`, which is the
//! signal approach 4 turns into a plain-language clarifying question rather
//! than guessing. It defines no language semantics
//! (canonical source: `SPECIFICATION.html`).

use super::explain::Lang;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Top,
    Stack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Consume {
    Eat,
    Keep,
}

/// The inferred modifier choice plus how confident the inference is.
pub(crate) struct ModifierInference {
    pub target: Target,
    pub consume: Consume,
    /// A NIL fallback (`VENT` / `^`) was requested by the phrase.
    pub fallback: bool,
    /// `true` when a cue was found for the target axis (otherwise it defaulted).
    pub target_explicit: bool,
    /// `true` when a cue was found for the consumption axis.
    pub consume_explicit: bool,
    /// An axis received conflicting cues (e.g. both "keep" and "consume"); the
    /// design note routes this to approach 4 as a clarifying question.
    pub ambiguous: bool,
    /// The Ajisai modifier sugar for the non-default choices, e.g. `,, ` for
    /// `KEEP` or `..` for `STAK`; empty when both axes are at their default.
    pub sugar: String,
    /// Plain-language explanation of the inference (L0; design note §2).
    pub rationale: String,
}

/// Infer a modifier from an intent `phrase`. Matching is case-insensitive
/// substring containment over a controlled cue vocabulary.
pub(crate) fn infer(phrase: &str, lang: Lang) -> ModifierInference {
    let haystack = phrase.to_lowercase();
    let has = |cues: &[&str]| cues.iter().any(|cue| haystack.contains(cue));

    let wants_stack = has(&["全体", "スタック", "all", "entire", "whole", "everything"]);
    let wants_top = has(&[
        "直前",
        "先頭",
        "トップ",
        "top",
        "last",
        "previous",
        "just the",
    ]);
    let wants_keep = has(&["残", "コピー", "複製", "keep", "retain", "copy", "preserve"]);
    let wants_eat = has(&["消費", "使い切", "consume", "use up", "eat", "discard"]);
    let fallback = has(&[
        "失敗",
        "ダメ",
        "既定",
        "代わり",
        "fallback",
        "default",
        "if it fails",
        "on failure",
        "otherwise",
    ]);

    let target_conflict = wants_stack && wants_top;
    let consume_conflict = wants_keep && wants_eat;

    let target = if wants_stack && !wants_top {
        Target::Stack
    } else {
        Target::Top
    };
    let consume = if wants_keep && !wants_eat {
        Consume::Keep
    } else {
        Consume::Eat
    };

    let target_explicit = wants_stack || wants_top;
    let consume_explicit = wants_keep || wants_eat;
    let ambiguous = target_conflict || consume_conflict;

    let mut sugar_parts: Vec<&str> = Vec::new();
    if target == Target::Stack {
        sugar_parts.push("..");
    }
    if consume == Consume::Keep {
        sugar_parts.push(",,");
    }
    if fallback {
        sugar_parts.push("^");
    }
    let sugar = sugar_parts.join(" ");

    let rationale = rationale(target, consume, fallback, ambiguous, lang);

    ModifierInference {
        target,
        consume,
        fallback,
        target_explicit,
        consume_explicit,
        ambiguous,
        sugar,
        rationale,
    }
}

fn rationale(
    target: Target,
    consume: Consume,
    fallback: bool,
    ambiguous: bool,
    lang: Lang,
) -> String {
    let target_word = match (target, lang) {
        (Target::Top, Lang::Ja) => "先頭の値に作用",
        (Target::Stack, Lang::Ja) => "スタック全体に作用",
        (Target::Top, Lang::En) => "act on the top value",
        (Target::Stack, Lang::En) => "act on the whole stack",
    };
    let consume_word = match (consume, lang) {
        (Consume::Eat, Lang::Ja) => "入力を消費",
        (Consume::Keep, Lang::Ja) => "入力を残す（分岐）",
        (Consume::Eat, Lang::En) => "consume the input",
        (Consume::Keep, Lang::En) => "keep the input (branch)",
    };
    let fallback_word = match (fallback, lang) {
        (true, Lang::Ja) => "、失敗時は既定値で補う",
        (false, Lang::Ja) => "",
        (true, Lang::En) => ", with a fallback on failure",
        (false, Lang::En) => "",
    };

    let base = match lang {
        Lang::Ja => format!(
            "{}し、{}します{}。",
            target_word, consume_word, fallback_word
        ),
        Lang::En => format!("{} and {}{}.", target_word, consume_word, fallback_word),
    };

    if ambiguous {
        match lang {
            Lang::Ja => format!(
                "{} ただし指定が一意に定まりません。意図を確認してください。",
                base
            ),
            Lang::En => format!(
                "{} However the intent is not unambiguous; please confirm.",
                base
            ),
        }
    } else {
        base
    }
}

impl Target {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Target::Top => "TOP",
            Target::Stack => "STAK",
        }
    }
}

impl Consume {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Consume::Eat => "EAT",
            Consume::Keep => "KEEP",
        }
    }
}
