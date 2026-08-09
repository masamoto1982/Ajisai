//! The "what to look at next" half of a diagnosis.
//!
//! Split out of `debug_diagnosis` when the resource-limit, shape-mismatch and
//! source-form classes were added: the checks are a table that grows with the
//! error vocabulary, while the diagnosis type around it does not, so keeping
//! them in one file made the file's size a function of the wrong thing.

use super::debug_diagnosis::{CauseClass, DebugCheck};
use crate::error::ErrorCategory;

fn check(label: &str, detail: &str) -> DebugCheck {
    DebugCheck {
        label: label.to_string(),
        detail: detail.to_string(),
    }
}

pub(crate) fn build_next_checks(
    why: &CauseClass,
    word: Option<&str>,
    category: Option<&ErrorCategory>,
) -> Vec<DebugCheck> {
    let mut out = Vec::new();

    match why {
        CauseClass::Domain => {
            if matches!(category, Some(ErrorCategory::DivisionByZero)) {
                out.push(check(
                    "Check divisor",
                    "\"/\" または DIV の右オペランドを確認する",
                ));
                out.push(check(
                    "Check zero is expected",
                    "0 が正常値としてあり得るなら SAFE / fallback を検討する",
                ));
                out.push(check(
                    "Check divisor origin",
                    "0 が異常値なら、右オペランドを生成した直前の word を確認する",
                ));
            } else {
                out.push(check(
                    "Check operand domain",
                    "演算が許す値域の外に入っていないか確認する",
                ));
            }
        }
        CauseClass::StackShape => {
            let word_label = word.unwrap_or("the word");
            out.push(check(
                "Check arity",
                &format!("{} が必要とする入力個数を確認する", word_label),
            ));
            out.push(check(
                "Check stack length",
                "実行直前のスタック長を確認する",
            ));
            out.push(check(
                "Check upstream consumers",
                "直前の word が値を消費しすぎていないか確認する",
            ));
        }
        CauseClass::TypoOrUnknownName => {
            out.push(check("Check spelling", "word 名のスペルを確認する"));
            out.push(check(
                "Check alias canonicalization",
                "alias 展開後の canonical word 名を確認する",
            ));
            out.push(check(
                "Check user definitions",
                "user word の定義と所属 dictionary を確認する",
            ));
        }
        CauseClass::Environment => {
            out.push(check("Check environment", "実行環境の前提条件を確認する"));
        }
        CauseClass::ValueShape => {
            let word_label = word.unwrap_or("the word");
            out.push(check(
                "Check expected shape",
                &format!("{} が期待する値の形を確認する", word_label),
            ));
            out.push(check(
                "Check type confusion",
                "Vector / Scalar / CodeBlock / Nil の取り違えを確認する",
            ));
            out.push(check(
                "Check producer",
                "直前の word が想定した型の値を生成しているか確認する",
            ));
        }
        CauseClass::Index => {
            out.push(check(
                "Check index and length",
                "index と vector 長を確認する",
            ));
            out.push(check(
                "Check origin convention",
                "0-origin / 1-origin の取り違えを確認する",
            ));
            out.push(check(
                "Check empty vector",
                "空 vector が入力されていないか確認する",
            ));
        }
        CauseClass::VectorLength => {
            out.push(check(
                "Check operand lengths",
                "対象の 2 つの vector 長を確認する",
            ));
            out.push(check(
                "Check element-wise contract",
                "zip / map / element-wise 演算の前提を確認する",
            ));
            out.push(check(
                "Check selective ops",
                "片方だけ filter や drop が適用されていないか確認する",
            ));
        }
        CauseClass::ShapeMismatch => {
            out.push(check(
                "Check the disagreeing axis",
                "メッセージが示す軸で、左右のオペランドのどちらが想定外の長さかを確認する",
            ));
            out.push(check(
                "Check broadcastability",
                "軸ごとに長さが一致するか、片方が 1 であれば broadcast できる",
            ));
            out.push(check(
                "Check rank",
                "行列積・転置・One-hot などで次元数そのものがずれていないか確認する",
            ));
        }
        CauseClass::SourceForm => {
            out.push(check(
                "Check delimiters",
                "{ } [ ] の対応と、閉じ忘れた block がないか確認する",
            ));
            out.push(check(
                "Check clause form",
                "'|' は code block の直下にのみ書ける。guard と body の両方が必要",
            ));
        }
        CauseClass::ResourceLimit => {
            // Not "the program is wrong" — LANG.MACHINE.LIMITS makes these
            // host safety controls. The first question is whether the work is
            // genuinely that large, and only then whether it fails to
            // terminate.
            if matches!(category, Some(ErrorCategory::RecursionLimitExceeded)) {
                out.push(check(
                    "Check depth vs data",
                    "入力の大きさに対して妥当な深さか、それとも停止条件漏れかを切り分ける",
                ));
                out.push(check(
                    "Check tail position",
                    "COND 節末尾のガード付き末尾再帰 (SPEC 8.4) に書き換えると深度制限を受けない",
                ));
            } else {
                out.push(check(
                    "Check budget vs work",
                    "予算超過は言語の意味論ではなくホストの安全制御。処理量が本当にその規模かをまず確認する",
                ));
                out.push(check(
                    "Check algorithmic cost",
                    "O(n^2) の添字操作を SORT / GET などの一括操作に置き換えられないか確認する",
                ));
                out.push(check(
                    "Check termination",
                    "規模が妥当なら、無限ループまたは終了条件漏れを確認する",
                ));
            }
        }
        CauseClass::UserLogic => {
            if matches!(category, Some(ErrorCategory::ExecutionLimitExceeded)) {
                out.push(check(
                    "Check termination",
                    "無限ループまたは終了条件漏れを確認する",
                ));
                out.push(check(
                    "Check recursion base",
                    "再帰呼び出しの停止条件を確認する",
                ));
                out.push(check(
                    "Check input size",
                    "大きすぎる入力に対して想定外の反復が発生していないか確認する",
                ));
            } else if matches!(category, Some(ErrorCategory::RecursionLimitExceeded)) {
                out.push(check(
                    "Check recursion base",
                    "再帰呼び出しの停止条件を確認する",
                ));
                out.push(check(
                    "Check tail position",
                    "COND 節末尾のガード付き末尾再帰 (SPEC 8.4) に書き換えると深度制限を受けない",
                ));
            } else if matches!(category, Some(ErrorCategory::CondExhausted)) {
                out.push(check(
                    "Check guard coverage",
                    "COND の全ての分岐条件と else 句を確認する",
                ));
            } else {
                out.push(check(
                    "Check user logic",
                    "ユーザーロジックの前提を確認する",
                ));
            }
        }
        CauseClass::ContractViolation => {
            if matches!(category, Some(ErrorCategory::ModeUnsupported)) {
                out.push(check(
                    "Check supported modes",
                    "対象 word が現在の mode をサポートしているか確認する",
                ));
                out.push(check(
                    "Check mode confusion",
                    "Stack mode / Vector mode / Code block mode の取り違えを確認する",
                ));
            } else if matches!(category, Some(ErrorCategory::BuiltinProtection)) {
                out.push(check(
                    "Check protection",
                    "built-in word に対する不可変操作が要求されている",
                ));
            } else {
                out.push(check(
                    "Check contract",
                    "word の事前条件・事後条件を確認する",
                ));
            }
        }
        CauseClass::Effect => {
            out.push(check(
                "Check effect bookkeeping",
                "consume / produce の質量保存を確認する",
            ));
        }
        CauseClass::NilFlow => {
            out.push(check(
                "Check NIL propagation",
                "NIL が想定外に流れていないか確認する",
            ));
        }
        CauseClass::OptimizerMismatch => {
            out.push(check(
                "Check optimizer assumptions",
                "最適化前後の意味が一致しているか確認する",
            ));
        }
        CauseClass::InternalInvariant => {
            out.push(check(
                "Check internal invariant",
                "内部不変条件違反が発生している。再現手順を保存し報告する",
            ));
        }
        CauseClass::Unknown => {
            out.push(check(
                "Check error message",
                "Custom エラーの場合は message を直接確認する",
            ));
        }
    }

    out
}
