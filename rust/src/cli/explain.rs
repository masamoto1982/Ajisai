//! Deterministic plain-language projection of the existing diagnostic
//! structures (`DebugDiagnosis` / `AiDiagnosticPayload`) — the L0 surface of
//! the natural-language design note
//! (`docs/dev/natural-language-surface-design.md` §3).
//!
//! This is a *projection*, not a generator: every sentence is keyed on an
//! existing enum (`CauseClass`) or protocol string (`recoverability`, NIL
//! reason), so the output is deterministic and cannot hallucinate. It defines
//! no language semantics (canonical source: `SPECIFICATION.html`) and adds no
//! diagnostic concept — it only re-renders what the interpreter already
//! decided.
//!
//! Two tiers, per the design note's progressive-disclosure constraint (§2):
//!   - `headline` + `next_step` → L0, plain language, no jargon.
//!   - `details`                → L2, the diagnosis `nextChecks` verbatim.
//!
//! The `headline` distinguishes the three water-model outcomes the design note
//! keeps separate — Stagnation (logical `UNKNOWN`), Bubble (`NIL`), and a
//! Channel error — as different *tones*, never as exposed terminology.

use crate::interpreter::debug_diagnosis::{CauseClass, DebugDiagnosis};

/// Output language for the projected sentences. Because `headline` and
/// `next_step` are keyed on enums, adding a language is a table swap — the
/// design note's "i18n is nearly free" claim made concrete. `details` carry
/// the diagnosis `nextChecks`, which are authored once in the core (currently
/// Japanese) and passed through unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lang {
    Ja,
    En,
}

impl Lang {
    pub(crate) fn parse(s: &str) -> Option<Lang> {
        match s {
            "ja" | "jp" => Some(Lang::Ja),
            "en" => Some(Lang::En),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Lang::Ja => "ja",
            Lang::En => "en",
        }
    }
}

/// Plain-language projection of one diagnosis.
pub(crate) struct Explanation {
    pub lang: Lang,
    /// L0: one sentence describing what happened, in plain language.
    pub headline: String,
    /// L0: one sentence describing the kind of change that resolves it,
    /// keyed on `aiDiagnostic.recoverability`.
    pub next_step: String,
    /// L2: the diagnosis repair checklist as `label: detail`, verbatim.
    pub details: Vec<String>,
}

/// Project a diagnosis to plain language. `recoverability` is the
/// `AiDiagnosticPayload.recoverability` protocol string (the CLI already
/// computes it); `nil_reason` is the absence-reason protocol string when the
/// outcome is a NIL bubble (from the error-flow trace), otherwise `None`.
pub(crate) fn explain(
    diagnosis: &DebugDiagnosis,
    recoverability: Option<&str>,
    nil_reason: Option<&str>,
    lang: Lang,
) -> Explanation {
    Explanation {
        lang,
        headline: headline(diagnosis, nil_reason, lang),
        next_step: next_step(recoverability, lang),
        details: diagnosis
            .next_checks
            .iter()
            .map(|c| format!("{}: {}", c.label, c.detail))
            .collect(),
    }
}

/// The "what happened" sentence. Tone selection order matters: Stagnation
/// (UNKNOWN) is checked first because a comparison that ran out of budget is
/// reported with `why = NilFlow` *and* a non-null `agreed_prefix`, and it must
/// not be described as an absence.
fn headline(diagnosis: &DebugDiagnosis, nil_reason: Option<&str>, lang: Lang) -> String {
    let word = diagnosis.where_.word.as_deref();

    // Stagnation: a value exists but the observation did not settle (SPEC §7.4.1).
    if diagnosis.agreed_prefix.is_some() {
        return match lang {
            Lang::Ja => {
                "まだ決めきれていません（比較が現在の観測範囲内で確定しませんでした）。".to_string()
            }
            Lang::En => "Not decided yet; the comparison did not settle within the current \
                 observation range."
                .to_string(),
        };
    }

    // Bubble: a well-formed operation could not produce a value (SPEC §11.2).
    if nil_reason.is_some() || matches!(diagnosis.why, CauseClass::NilFlow) {
        let reason = nil_reason_phrase(nil_reason, lang);
        return match lang {
            Lang::Ja => with_word_ja(&format!("値が得られませんでした{}", reason), word),
            Lang::En => with_word_en(&format!("no value was produced{}", reason), word),
        };
    }

    // Channel error: malformed use, keyed on the cause class.
    cause_phrase(&diagnosis.why, word, lang)
}

/// A short parenthetical naming the NIL reason, or empty for an unknown /
/// absent reason (consumers must treat unrecognized protocol strings as
/// opaque, per the CLI output contract).
fn nil_reason_phrase(reason: Option<&str>, lang: Lang) -> String {
    let Some(reason) = reason else {
        return String::new();
    };
    let (ja, en) = match reason {
        "divisionByZero" => ("ゼロ除算", "division by zero"),
        "emptySequence" => ("空の列に対する操作", "an operation on an empty sequence"),
        "indexOutOfBounds" => ("範囲外のインデックス", "an out-of-range index"),
        "missingField" => ("見つからない項目", "a missing field"),
        "invalidEncoding" => ("不正なエンコーディング", "invalid encoding"),
        "noData" => ("読み取れるデータがない", "no readable data"),
        "portDisconnected" => ("切断されたポート", "a disconnected port"),
        _ => return String::new(),
    };
    match lang {
        Lang::Ja => format!("（{}）", ja),
        Lang::En => format!(" ({})", en),
    }
}

/// Plain-language sentence for a Channel-error cause class. `TypoOrUnknownName`
/// is special-cased because the offending word is the *subject* of the
/// sentence, not a location within it.
fn cause_phrase(why: &CauseClass, word: Option<&str>, lang: Lang) -> String {
    if matches!(why, CauseClass::TypoOrUnknownName) {
        return match (lang, word) {
            (Lang::Ja, Some(w)) => format!("知らない語『{}』が使われています。", w),
            (Lang::Ja, None) => "知らない語が使われています。".to_string(),
            (Lang::En, Some(w)) => format!("An unknown word \"{}\" was used.", w),
            (Lang::En, None) => "An unknown word was used.".to_string(),
        };
    }
    let (ja, en) = match why {
        CauseClass::TypoOrUnknownName => unreachable!("handled above"),
        CauseClass::StackShape => (
            "値の数が合っていません",
            "the number of values does not match",
        ),
        CauseClass::ValueShape => ("値の形が合っていません", "the value shape does not match"),
        CauseClass::Domain => (
            "入力値が扱える範囲の外です",
            "the input is outside the defined domain",
        ),
        CauseClass::Index => ("範囲外のインデックスです", "the index is out of range"),
        CauseClass::VectorLength => (
            "ベクトルの長さが合っていません",
            "the vector lengths do not match",
        ),
        CauseClass::NilFlow => ("値が得られませんでした", "no value was produced"),
        CauseClass::Environment => (
            "実行環境側の問題です",
            "this is an execution-environment issue",
        ),
        CauseClass::Effect => (
            "副作用の実行で問題が起きました",
            "an effect could not be performed",
        ),
        CauseClass::UserLogic => (
            "プログラム自身の論理で停止しました",
            "the program's own logic stopped execution",
        ),
        CauseClass::ContractViolation => (
            "語の使い方が契約に反しています",
            "the word was used against its contract",
        ),
        CauseClass::OptimizerMismatch => (
            "最適化の検証で不一致が見つかりました",
            "an optimization check found a mismatch",
        ),
        CauseClass::InternalInvariant => (
            "内部の不変条件が破れました",
            "an internal invariant was violated",
        ),
        CauseClass::Unknown => (
            "原因を特定できませんでした",
            "the cause could not be determined",
        ),
    };
    match lang {
        Lang::Ja => with_word_ja(ja, word),
        Lang::En => with_word_en(en, word),
    }
}

/// The "what kind of change resolves it" sentence, keyed on the
/// `recoverability` protocol string. An absent or unrecognized value falls
/// back to `inspectContext`.
fn next_step(recoverability: Option<&str>, lang: Lang) -> String {
    let (ja, en) = match recoverability.unwrap_or("inspectContext") {
        "fixInput" => (
            "入力した値を見直してください。",
            "Check the input values you passed.",
        ),
        "fixProgram" => (
            "手順（プログラム）を見直してください。",
            "Check the program: the words used and their order.",
        ),
        "fixHost" => (
            "実行環境（入出力など）側の問題です。その機能を持つホストで試してください。",
            "This is an execution-environment issue; try a host that provides the capability.",
        ),
        "fixCapabilityOrForce" => (
            "この操作には許可が必要です。",
            "This operation needs a capability, or an explicit force.",
        ),
        "addBudgetOrFixRecursion" => (
            "処理の上限に達しました。範囲を絞るか、上限を増やしてください。",
            "A processing limit was reached; narrow the work or raise the limit.",
        ),
        "handleUnknownOrNil" => (
            "答えが得られませんでした。既定値を決めるか、分岐を足してください。",
            "No answer was produced; supply a fallback value or add a branch.",
        ),
        // "inspectContext" and any unrecognized value.
        _ => (
            "周辺の文脈を確認してください。",
            "Inspect the surrounding context.",
        ),
    };
    match lang {
        Lang::Ja => ja.to_string(),
        Lang::En => en.to_string(),
    }
}

/// `"WORD で<phrase>。"`, or just `"<phrase>。"` when no word is known. The
/// locative reads naturally for the non-subject cause classes.
fn with_word_ja(phrase: &str, word: Option<&str>) -> String {
    match word {
        Some(w) => format!("{} で{}。", w, phrase),
        None => format!("{}。", phrase),
    }
}

/// `"WORD: <phrase>."`, or just `"<Phrase>."` (sentence-cased) when no word is
/// known.
fn with_word_en(phrase: &str, word: Option<&str>) -> String {
    match word {
        Some(w) => format!("{}: {}.", w, capitalize(phrase)),
        None => format!("{}.", capitalize(phrase)),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
