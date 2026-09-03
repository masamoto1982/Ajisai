//! The "what to look at next" half of a diagnosis.
//!
//! Split out of `debug_diagnosis` when the resource-limit, shape-mismatch and
//! source-form classes were added: the checks are a table that grows with the
//! error vocabulary, while the diagnosis type around it does not, so keeping
//! them in one file made the file's size a function of the wrong thing.
//!
//! Every entry carries a stable `code` plus display text per locale. The two
//! are separate on purpose: the code is what a consumer matches on and what a
//! repair-rate scorer counts, and it must survive a reworded sentence or a
//! newly translated locale untouched.

use super::debug_declared_checks::declared_checks;
use super::debug_diagnosis::{CauseClass, DebugCheck, LocalizedText};
use super::debug_limit_checks::resource_limit_checks;
use crate::error::{ErrorCategory, NilReason};

fn check(code: &'static str, title: (&str, &str), detail: (&str, &str)) -> DebugCheck {
    DebugCheck {
        code,
        title: LocalizedText::new(title.0, title.1),
        detail: LocalizedText::new(detail.0, detail.1),
    }
}

pub(crate) fn build_next_checks(
    why: &CauseClass,
    word: Option<&str>,
    category: Option<&ErrorCategory>,
    nil_reason: Option<&NilReason>,
) -> Vec<DebugCheck> {
    let fired_condition = match category {
        Some(ErrorCategory::Declared(condition)) => Some(*condition),
        _ => None,
    };
    let mut out = declared_checks(why, word, nil_reason, fired_condition);

    match why {
        CauseClass::Domain => {
            if matches!(category, Some(ErrorCategory::DivisionByZero)) {
                // Name the Word that actually met the zero. This check was
                // written for `DIV` and hard-coded its spelling, so `MOD` —
                // which declares the same `divisorEqualsZero` condition — sent
                // the reader to look at an operand of a Word it had not called.
                let word_label = word.unwrap_or("the word");
                out.push(check(
                    "checkDivisor",
                    ("Check divisor", "除数を確認する"),
                    (
                        &format!("Inspect the right operand of {}.", word_label),
                        &format!("{} の右オペランドを確認する", word_label),
                    ),
                ));
                out.push(check(
                    "checkZeroIsExpected",
                    ("Check zero is expected", "0 が正常値かを確認する"),
                    (
                        "If 0 is a legitimate value here, handle it with SAFE or a fallback.",
                        "0 が正常値としてあり得るなら SAFE / fallback を検討する",
                    ),
                ));
                out.push(check(
                    "checkDivisorOrigin",
                    ("Check divisor origin", "除数の生成元を確認する"),
                    (
                        "If 0 is anomalous, inspect the Word that produced the right operand.",
                        "0 が異常値なら、右オペランドを生成した直前の word を確認する",
                    ),
                ));
            } else {
                out.push(check(
                    "checkOperandDomain",
                    ("Check operand domain", "オペランドの値域を確認する"),
                    (
                        "Check whether the operand left the domain the operation admits.",
                        "演算が許す値域の外に入っていないか確認する",
                    ),
                ));
            }
        }
        CauseClass::StackShape => {
            let word_label = word.unwrap_or("the word");
            out.push(check(
                "checkArity",
                ("Check arity", "アリティを確認する"),
                (
                    &format!("Check how many inputs {} requires.", word_label),
                    &format!("{} が必要とする入力個数を確認する", word_label),
                ),
            ));
            out.push(check(
                "checkStackLength",
                ("Check stack length", "スタック長を確認する"),
                (
                    "Check the stack length immediately before the call.",
                    "実行直前のスタック長を確認する",
                ),
            ));
            out.push(check(
                "checkUpstreamConsumers",
                ("Check upstream consumers", "上流の消費を確認する"),
                (
                    "Check whether an earlier Word consumed more values than intended.",
                    "直前の word が値を消費しすぎていないか確認する",
                ),
            ));
        }
        CauseClass::TypoOrUnknownName => {
            out.push(check(
                "checkSpelling",
                ("Check spelling", "スペルを確認する"),
                (
                    "Check the spelling of the Word name; diagnosis.candidates lists the closest known names.",
                    "word 名のスペルを確認する。diagnosis.candidates に近い既知の名前が並ぶ",
                ),
            ));
            out.push(check(
                "checkAliasCanonicalization",
                ("Check alias canonicalization", "別名の正規化を確認する"),
                (
                    "Check the canonical Word name the alias expands to.",
                    "alias 展開後の canonical word 名を確認する",
                ),
            ));
            out.push(check(
                "checkUserDefinitions",
                ("Check user definitions", "ユーザー定義を確認する"),
                (
                    "Check the user Word's definition and the dictionary it belongs to.",
                    "user word の定義と所属 dictionary を確認する",
                ),
            ));
        }
        CauseClass::Environment => {
            out.push(check(
                "checkEnvironment",
                ("Check environment", "実行環境を確認する"),
                (
                    "Check the host environment's preconditions.",
                    "実行環境の前提条件を確認する",
                ),
            ));
        }
        CauseClass::ValueShape => {
            let word_label = word.unwrap_or("the word");
            out.push(check(
                "checkExpectedShape",
                ("Check expected shape", "期待される形を確認する"),
                (
                    &format!("Check the value shape {} expects.", word_label),
                    &format!("{} が期待する値の形を確認する", word_label),
                ),
            ));
            out.push(check(
                "checkTypeConfusion",
                ("Check type confusion", "型の取り違えを確認する"),
                (
                    "Check for a Vector / Scalar / CodeBlock / Nil mix-up.",
                    "Vector / Scalar / CodeBlock / Nil の取り違えを確認する",
                ),
            ));
            out.push(check(
                "checkProducer",
                ("Check producer", "生成元を確認する"),
                (
                    "Check that the preceding Word produced the expected type.",
                    "直前の word が想定した型の値を生成しているか確認する",
                ),
            ));
        }
        CauseClass::Index => {
            out.push(check(
                "checkIndexAndLength",
                ("Check index and length", "index と長さを確認する"),
                (
                    "Check the index against the vector length.",
                    "index と vector 長を確認する",
                ),
            ));
            out.push(check(
                "checkOriginConvention",
                ("Check origin convention", "原点規約を確認する"),
                (
                    "Check for a 0-origin / 1-origin mix-up.",
                    "0-origin / 1-origin の取り違えを確認する",
                ),
            ));
            out.push(check(
                "checkEmptyVector",
                ("Check empty vector", "空 vector を確認する"),
                (
                    "Check whether the input vector was empty.",
                    "空 vector が入力されていないか確認する",
                ),
            ));
        }
        CauseClass::VectorLength => {
            out.push(check(
                "checkOperandLengths",
                ("Check operand lengths", "オペランド長を確認する"),
                (
                    "Check the lengths of the two vectors involved.",
                    "対象の 2 つの vector 長を確認する",
                ),
            ));
            out.push(check(
                "checkElementWiseContract",
                ("Check element-wise contract", "要素ごとの前提を確認する"),
                (
                    "Check the assumptions of the zip / map / element-wise operation.",
                    "zip / map / element-wise 演算の前提を確認する",
                ),
            ));
            out.push(check(
                "checkSelectiveOps",
                ("Check selective ops", "選択的操作を確認する"),
                (
                    "Check whether a filter or drop was applied to only one side.",
                    "片方だけ filter や drop が適用されていないか確認する",
                ),
            ));
        }
        CauseClass::ShapeMismatch => {
            out.push(check(
                "checkDisagreeingAxis",
                ("Check the disagreeing axis", "食い違う軸を確認する"),
                (
                    "On the axis the message names, check which operand has the unexpected extent.",
                    "メッセージが示す軸で、左右のオペランドのどちらが想定外の長さかを確認する",
                ),
            ));
            out.push(check(
                "checkBroadcastability",
                ("Check broadcastability", "broadcast 可能性を確認する"),
                (
                    "Each axis must match, or one side must be 1 to broadcast.",
                    "軸ごとに長さが一致するか、片方が 1 であれば broadcast できる",
                ),
            ));
            out.push(check(
                "checkRank",
                ("Check rank", "ランクを確認する"),
                (
                    "Check whether the rank itself is off in a matrix product, transpose or one-hot.",
                    "行列積・転置・One-hot などで次元数そのものがずれていないか確認する",
                ),
            ));
        }
        CauseClass::SourceForm => {
            out.push(check(
                "checkDelimiters",
                ("Check delimiters", "区切り記号を確認する"),
                (
                    "Check that every [ and ] pair up and no vector was left unclosed.",
                    "[ ] の対応と、閉じ忘れた vector がないか確認する",
                ),
            ));
            out.push(check(
                "checkClauseForm",
                ("Check clause form", "節の形を確認する"),
                (
                    "'|' is only legal directly inside a vector, and needs both a guard and a body.",
                    "'|' は vector の直下にのみ書ける。guard と body の両方が必要",
                ),
            ));
        }
        CauseClass::ResourceLimit => out.extend(resource_limit_checks(category)),
        CauseClass::UserLogic => {
            if matches!(category, Some(ErrorCategory::ExecutionLimitExceeded)) {
                out.push(check(
                    "checkTermination",
                    ("Check termination", "停止性を確認する"),
                    (
                        "Look for an infinite loop or a missing termination condition.",
                        "無限ループまたは終了条件漏れを確認する",
                    ),
                ));
                out.push(check(
                    "checkRecursionBase",
                    ("Check recursion base", "再帰の基底を確認する"),
                    (
                        "Check the recursive call's stopping condition.",
                        "再帰呼び出しの停止条件を確認する",
                    ),
                ));
                out.push(check(
                    "checkInputSize",
                    ("Check input size", "入力サイズを確認する"),
                    (
                        "Check whether an oversized input caused unintended iteration.",
                        "大きすぎる入力に対して想定外の反復が発生していないか確認する",
                    ),
                ));
            } else if matches!(category, Some(ErrorCategory::RecursionLimitExceeded)) {
                out.push(check(
                    "checkRecursionBase",
                    ("Check recursion base", "再帰の基底を確認する"),
                    (
                        "Check the recursive call's stopping condition.",
                        "再帰呼び出しの停止条件を確認する",
                    ),
                ));
                out.push(check(
                    "checkTailPosition",
                    ("Check tail position", "末尾位置を確認する"),
                    (
                        "Guarded tail recursion at the end of a COND clause (SPEC 8.4) is not depth-limited.",
                        "COND 節末尾のガード付き末尾再帰 (SPEC 8.4) に書き換えると深度制限を受けない",
                    ),
                ));
            } else if matches!(category, Some(ErrorCategory::CondExhausted)) {
                out.push(check(
                    "checkGuardCoverage",
                    ("Check guard coverage", "ガードの網羅を確認する"),
                    (
                        "Check every COND branch condition and the else clause.",
                        "COND の全ての分岐条件と else 句を確認する",
                    ),
                ));
            } else {
                out.push(check(
                    "checkUserLogic",
                    ("Check user logic", "ユーザーロジックを確認する"),
                    (
                        "Check the assumptions the user logic makes.",
                        "ユーザーロジックの前提を確認する",
                    ),
                ));
            }
        }
        CauseClass::ContractViolation => {
            if matches!(category, Some(ErrorCategory::ModeUnsupported)) {
                out.push(check(
                    "checkSupportedModes",
                    ("Check supported modes", "対応 mode を確認する"),
                    (
                        "Check whether the Word supports the current mode.",
                        "対象 word が現在の mode をサポートしているか確認する",
                    ),
                ));
                out.push(check(
                    "checkModeConfusion",
                    ("Check mode confusion", "mode の取り違えを確認する"),
                    (
                        "Check for a Stack mode / Vector mode / Code block mode mix-up.",
                        "Stack mode / Vector mode / Code block mode の取り違えを確認する",
                    ),
                ));
            } else if matches!(category, Some(ErrorCategory::BuiltinProtection)) {
                out.push(check(
                    "checkProtection",
                    ("Check protection", "保護を確認する"),
                    (
                        "A mutating operation was requested against a built-in Word.",
                        "built-in word に対する不可変操作が要求されている",
                    ),
                ));
            } else {
                out.push(check(
                    "checkContract",
                    ("Check contract", "契約を確認する"),
                    (
                        "Check the Word's preconditions and postconditions.",
                        "word の事前条件・事後条件を確認する",
                    ),
                ));
            }
        }
        CauseClass::Effect => {
            out.push(check(
                "checkEffectBookkeeping",
                ("Check effect bookkeeping", "効果の収支を確認する"),
                (
                    "Check conservation of mass between consume and produce.",
                    "consume / produce の質量保存を確認する",
                ),
            ));
        }
        CauseClass::NilFlow => {
            out.push(check(
                "checkNilPropagation",
                ("Check NIL propagation", "NIL の伝播を確認する"),
                (
                    "Check whether NIL flowed somewhere unintended.",
                    "NIL が想定外に流れていないか確認する",
                ),
            ));
        }
        CauseClass::OptimizerMismatch => {
            out.push(check(
                "checkOptimizerAssumptions",
                ("Check optimizer assumptions", "最適化の前提を確認する"),
                (
                    "Check that meaning is preserved across the optimization.",
                    "最適化前後の意味が一致しているか確認する",
                ),
            ));
        }
        CauseClass::InternalInvariant => {
            out.push(check(
                "checkInternalInvariant",
                ("Check internal invariant", "内部不変条件を確認する"),
                (
                    "An internal invariant was violated. Save the reproduction steps and report it.",
                    "内部不変条件違反が発生している。再現手順を保存し報告する",
                ),
            ));
        }
        CauseClass::Unknown => {
            out.push(check(
                "checkErrorMessage",
                ("Check error message", "エラーメッセージを確認する"),
                (
                    "For a Custom error, read the message directly.",
                    "Custom エラーの場合は message を直接確認する",
                ),
            ));
        }
    }

    out
}
