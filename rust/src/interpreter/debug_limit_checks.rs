//! The checks for a refusal by a host safety control, kept apart from the
//! language-fault table.
//!
//! `LANG.MACHINE.LIMITS` calls the step, recursion and size budgets host
//! safety controls rather than language semantics, and the two prescribe
//! opposite fixes: "the program is wrong" is answered by rewriting it, "the
//! program is too big" by raising the budget or finding a cheaper shape for
//! the same computation. That is the same boundary this file draws, and the
//! reason the arm is the largest in the table — each ceiling has its own
//! question to ask.

use super::debug_diagnosis::{DebugCheck, LocalizedText};
use crate::error::ErrorCategory;

fn check(code: &'static str, title: (&str, &str), detail: (&str, &str)) -> DebugCheck {
    DebugCheck {
        code,
        title: LocalizedText::new(title.0, title.1),
        detail: LocalizedText::new(detail.0, detail.1),
    }
}

pub(super) fn resource_limit_checks(category: Option<&ErrorCategory>) -> Vec<DebugCheck> {
    let mut out = Vec::new();
    // Not "the program is wrong" — LANG.MACHINE.LIMITS makes these
    // host safety controls. The first question is whether the work is
    // genuinely that large, and only then whether it fails to
    // terminate.
    if matches!(category, Some(ErrorCategory::RecursionLimitExceeded)) {
        out.push(check(
                "checkDepthVsData",
                ("Check depth vs data", "深さと入力量を比べる"),
                (
                    "Separate \"this depth is reasonable for the input\" from \"the base case is missing\".",
                    "入力の大きさに対して妥当な深さか、それとも停止条件漏れかを切り分ける",
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
    } else if matches!(category, Some(ErrorCategory::ResourceLimitExceeded)) {
        // A size ceiling, not a time budget: the program may be
        // perfectly terminating and still produce one value too large
        // to represent under this host's profile.
        out.push(check(
                "checkWhichLimit",
                ("Check which limit fired", "どの上限かを確認する"),
                (
                    "diagnosis.resourceLimit names the ceiling, its configured value and the observed size.",
                    "diagnosis.resourceLimit に、超過した上限の名前・設定値・実測値が入っている",
                ),
            ));
        // The one check carrying a number to act on. A meter charged as
        // it goes stops the instant the budget is crossed, so
        // `observed` sits a hair over `limit` however far over the
        // request was; reading it proportionally is what made a real
        // model retry 100,000 elements as 99,999 and fail again.
        out.push(check(
                "checkHowFarItGot",
                ("Check how far it got", "どこまで進んだかを確認する"),
                (
                    "When diagnosis.resourceLimit.progress is present, the budget bought exactly `completed` of `total` units: retry with that many, not slightly fewer. `observed` cannot tell you how much to cut, because a meter charged as it goes stops the moment it crosses.",
                    "diagnosis.resourceLimit.progress があるとき、予算で処理できたのは total のうち completed 単位ちょうど。少し減らすのではなく、その数で再試行する。逐次課金のメーターは超えた瞬間に止まるので、observed からは削る量が分からない",
                ),
            ));
        out.push(check(
                "checkValueGrowth",
                ("Check value growth", "値の増大を確認する"),
                (
                    "Check what makes one value grow that large — repeated squaring, or a product of distinct radicals.",
                    "1 つの値がその大きさになる原因 (繰り返し二乗、相異なる根号の積など) を確認する",
                ),
            ));
        out.push(check(
                "checkHostProfile",
                ("Check host profile", "ホストプロファイルを確認する"),
                (
                    "Limits are per host profile: the same source can succeed on a host that declares a larger ceiling.",
                    "上限はホストプロファイルごとに異なる。より大きい上限を宣言するホストでは同じソースが成功しうる",
                ),
            ));
    } else {
        out.push(check(
                "checkBudgetVsWork",
                ("Check budget vs work", "予算と処理量を比べる"),
                (
                    "Budget exhaustion is a host safety control, not language semantics; first confirm the work is genuinely that large.",
                    "予算超過は言語の意味論ではなくホストの安全制御。処理量が本当にその規模かをまず確認する",
                ),
            ));
        out.push(check(
            "checkAlgorithmicCost",
            ("Check algorithmic cost", "計算量を確認する"),
            (
                "Check whether O(n^2) index work can become a bulk operation such as SORT or GET.",
                "O(n^2) の添字操作を SORT / GET などの一括操作に置き換えられないか確認する",
            ),
        ));
        out.push(check(
                "checkTermination",
                ("Check termination", "停止性を確認する"),
                (
                    "If the scale is reasonable, look for an infinite loop or a missing termination condition.",
                    "規模が妥当なら、無限ループまたは終了条件漏れを確認する",
                ),
            ));
    }
    out
}
