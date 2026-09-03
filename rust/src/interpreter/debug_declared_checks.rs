//! The half of a diagnosis that reads the Word's own registry entry.
//!
//! `spec/words.json` declares, for every Core Word, the condition under which
//! it projects to NIL, the conditions under which it raises, its stack arity,
//! and how one correct line of it is written. None of that reached a
//! diagnosis: [`super::debug_next_checks`] is a table keyed on the cause
//! class, so a Word with a row in it got good advice — `DIV` names its
//! divisor — and every other Word got the class-level generality, or, when the
//! class could not be decided, "read the message". Two calls that cost a real
//! session a turn each, `\'hello world\' TOKENIZE` and
//! `[ [ 1 2 ] [ 3 ] ] CONCAT`, answered `Stack underflow` and nothing more,
//! while the registry held both the arity and a correct example.
//!
//! These checks derive from the declaration, so a Word earns them by being
//! declared rather than by being tabulated.

use super::debug_diagnosis::{CauseClass, DebugCheck, LocalizedText};
use crate::coreword_registry::get_declared_word;
use crate::error::NilReason;
use crate::kernel::generated::{Arity, GeneratedWord};

fn check(code: &'static str, title: (&str, &str), detail: (&str, &str)) -> DebugCheck {
    DebugCheck {
        code,
        title: LocalizedText::new(title.0, title.1),
        detail: LocalizedText::new(detail.0, detail.1),
    }
}

/// The declared arity written the way a stack effect is written: `( 2 -- 1 )`.
/// A count the specification leaves data-dependent is written as it declares
/// it rather than invented as a number.
fn arity_notation(declared: &GeneratedWord) -> String {
    fn side(arity: Arity) -> String {
        match arity {
            Arity::Fixed(n) => n.to_string(),
            Arity::Variable => "variable".to_string(),
            Arity::Control => "control".to_string(),
        }
    }
    format!(
        "( {} -- {} )",
        side(declared.stack_inputs),
        side(declared.stack_outputs)
    )
}

/// Checks a Word's own registry entry answers, ahead of the class-level ones.
///
/// The registry declares, for every Core Word, the condition under which it
/// projects to NIL, the conditions under which it raises, its arity, and how
/// one line of it is written. None of that reached a diagnosis: the checks
/// were a hand-written table keyed on the cause class, so a Word with an entry
/// in that table got good advice (`DIV` names its divisor) and every other
/// Word got the class-level generality — or, for a condition that fell to
/// `custom`, "read the message". These derive from the declaration instead, so
/// a Word gains them by being declared rather than by being tabulated.
pub(super) fn declared_checks(
    why: &CauseClass,
    word: Option<&str>,
    nil_reason: Option<&NilReason>,
) -> Vec<DebugCheck> {
    let Some(name) = word else {
        return Vec::new();
    };
    let Some(declared) = get_declared_word(name) else {
        return Vec::new();
    };
    let mut out = Vec::new();

    // Stack underflow: "Stack underflow" alone names neither the Word's
    // declared arity nor the shape of a correct call, which is the whole of
    // what the caller got wrong. Both are declared.
    if matches!(why, CauseClass::StackShape) {
        let arity = arity_notation(declared);
        out.push(check(
            "checkDeclaredArity",
            ("Check the declared arity", "宣言されたアリティを確認する"),
            (
                &format!("{} declares {}.", declared.name, arity),
                &format!("{} の宣言は {}。", declared.name, arity),
            ),
        ));
        if let Some(syntax) = declared.syntax {
            out.push(check(
                "checkDeclaredSyntax",
                ("Check the declared syntax", "宣言された構文を確認する"),
                (
                    &format!("One correct call is `{}`.", syntax),
                    &format!("正しい呼び出しの一例は `{}`。", syntax),
                ),
            ));
        }
    }

    // A NIL the Word produced: the registry names the conditions it projects
    // under, so the diagnosis can say which rather than "unknown". A Word may
    // declare several — `MOD` projects for a zero divisor and for an integer
    // projection it cannot decide — and the reason the run actually reported
    // is what tells them apart, so both are put in front of the reader.
    if let (Some(reason), false) = (nil_reason, declared.projection.is_empty()) {
        let when = declared.projection.join(", ");
        out.push(check(
            "checkDeclaredProjection",
            (
                "Check the declared projection",
                "宣言された射影条件を確認する",
            ),
            (
                &format!(
                    "{} declares that it answers NIL when: {}. This one reports reason `{}`.",
                    declared.name,
                    when,
                    reason.as_protocol_str()
                ),
                &format!(
                    "{} が NIL を返すと宣言している条件: {}。今回の reason は `{}`。",
                    declared.name,
                    when,
                    reason.as_protocol_str()
                ),
            ),
        ));
    }

    // A raise the category could not classify. The registry names every
    // condition the Word raises under, which is the shortlist of what to look
    // at — and the alternative here is `checkErrorMessage`, "read the message",
    // which is what the caller had already read. Where the class *is* known
    // (`shapeMismatch` naming the disagreeing axis, say) its own checks are
    // more specific than this list, so it stays out of their way.
    if nil_reason.is_none() && matches!(why, CauseClass::Unknown) && !declared.error_when.is_empty()
    {
        out.push(check(
            "checkDeclaredErrorConditions",
            (
                "Check the declared error conditions",
                "宣言されたエラー条件を確認する",
            ),
            (
                &format!(
                    "{} declares that it raises when: {}.",
                    declared.name,
                    declared.error_when.join(", ")
                ),
                &format!(
                    "{} が raise すると宣言している条件: {}。",
                    declared.name,
                    declared.error_when.join(", ")
                ),
            ),
        ));
    }

    out
}
