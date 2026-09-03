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

/// The cause class a *declared condition* names on its own.
///
/// The conditions are `spec/words.json`'s own vocabulary — the names
/// `errorWhen` is written in — so this is a mapping over that vocabulary rather
/// than a per-Word table: a Word earns the classification by declaring a
/// condition, and a new Word that declares `notExecutable` is classified the
/// day it is written. `declared_condition_vocabulary_is_classified` fails if
/// the spec grows a condition this does not answer for.
pub(crate) fn cause_class_for_declared_condition(condition: &str) -> CauseClass {
    match condition {
        // The operand is not of the kind the Word takes. Whatever the kind
        // named, the repair is the same one: pass the kind it asked for.
        "nonVector"
        | "nonNumeric"
        | "nonText"
        | "nonTextVector"
        | "nonTextElement"
        | "nonTextSeparator"
        | "nonTruthValue"
        | "nonTruthGuard"
        | "nonInteger"
        | "nonComparableElement"
        | "notExecutable"
        | "unsupportedComparison"
        | "invalidShape"
        | "invalidClauseShape"
        | "invalidCount"
        | "negativeCount"
        | "invalidRange"
        | "invalidName" => CauseClass::ValueShape,
        // A position outside the operand.
        "indexOutOfBounds" | "invalidIndex" => CauseClass::Index,
        "shapeMismatch" => CauseClass::ShapeMismatch,
        "vectorLengthMismatch" => CauseClass::VectorLength,
        "stackUnderflow" => CauseClass::StackShape,
        // A rule about names, definitions, or what a block promised to leave
        // behind — broken by the program rather than by any one value.
        "blockContractViolation"
        | "protectedWord"
        | "definitionConflict"
        | "selfReferentialDefinition"
        | "nameIsAWord" => CauseClass::ContractViolation,
        "wordNotFound" => CauseClass::TypoOrUnknownName,
        // The block ran and raised. The fault is inside it, not in the Word
        // that applied it.
        "nestedExecutionError" => CauseClass::UserLogic,
        "missingFollowingSourceUnit" => CauseClass::SourceForm,
        _ => CauseClass::Unknown,
    }
}

/// Where a declared condition is repaired: in the operand it names, or in the
/// program that broke the rule it names.
pub(crate) fn repair_for_declared_condition(why: &CauseClass) -> &'static str {
    match why {
        CauseClass::ValueShape
        | CauseClass::Index
        | CauseClass::ShapeMismatch
        | CauseClass::VectorLength
        | CauseClass::Domain => "fixInput",
        _ => "fixProgram",
    }
}

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
    fired_condition: Option<&str>,
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
                    "{} declares the condition it answers NIL under: `{}`. \
                     The NIL it answered carries the outcome reason `{}`. \
                     A condition and a reason are a declared pair, not a disagreement.",
                    declared.name,
                    when,
                    reason.as_protocol_str()
                ),
                &format!(
                    "{} が NIL を返す条件（when）の宣言: `{}`。\
                     返された NIL が持つ結果側の理由（reason）: `{}`。\
                     when と reason は対で宣言された別々の項目であり、食い違いではない。",
                    declared.name,
                    when,
                    reason.as_protocol_str()
                ),
            ),
        ));
    }

    // The raise named its own declared condition, so the reader is told which
    // one fired rather than handed the Word's whole shortlist to guess from.
    if let (None, Some(condition)) = (nil_reason, fired_condition) {
        out.push(check(
            "checkFiredCondition",
            ("Check the condition that fired", "発火した条件を確認する"),
            (
                &format!(
                    "{} raised the condition `{}`, one of the conditions its contract declares: {}.",
                    declared.name,
                    condition,
                    declared.error_when.join(", ")
                ),
                &format!(
                    "{} が raise した条件（errorWhen）は `{}`。契約が宣言している条件は: {}。",
                    declared.name,
                    condition,
                    declared.error_when.join(", ")
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
