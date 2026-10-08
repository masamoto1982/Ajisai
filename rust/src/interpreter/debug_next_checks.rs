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

use super::debug_declared_checks::{declared_checks, resource_limit_checks};
use super::debug_diagnosis::{CauseClass, DebugCheck, LocalizedText};
use crate::error::{ErrorCategory, NilReason};

/// Whether an unresolved name is a double-quoted string someone wrote with the
/// wrong delimiter. Both quotes are required: a name that merely contains one
/// is a different mistake, and a bare leading quote is more likely a truncated
/// token than a string.
fn looks_double_quoted(word: &str) -> bool {
    word.len() >= 2 && word.starts_with('"') && word.ends_with('"')
}

/// Forth-style stack shufflers, which Ajisai does not have: every Word
/// consumes the operands it reads, and a value used twice is named with
/// `BIND`. `DROP` is not here — it is an Ajisai collection Word.
const STACK_SHUFFLERS: &[&str] = &["DUP", "SWAP", "OVER", "ROT", "NIP", "TUCK", "PICK", "2DUP"];

/// Structured-programming keywords, which Ajisai does not have: a branch is
/// `SELECT` over two values, and iteration is `MAP`/`FILTER`/`FOLD`/`SCAN`
/// over a finite Vector.
const CONTROL_KEYWORDS: &[&str] = &[
    "IF", "ELSE", "THEN", "ENDIF", "WHILE", "FOR", "LOOP", "REPEAT", "UNTIL", "BEGIN", "DO",
    "BREAK", "RETURN",
];

/// Whether an unresolved name contains a full-width form (U+FF01–U+FF5E) of an
/// ASCII character: `ＡＤＤ` for `ADD`, `１` for `1`. A Japanese input method
/// produces these with one key, and the dictionary never holds them.
fn has_full_width_ascii(word: &str) -> bool {
    word.chars().any(|c| ('\u{FF01}'..='\u{FF5E}').contains(&c))
}

fn check(code: &'static str, title: (&str, &str), detail: (&str, &str)) -> DebugCheck {
    DebugCheck {
        code,
        title: LocalizedText::new(title.0, title.1),
        detail: LocalizedText::new(detail.0, detail.1),
    }
}

/// The spelling check for a name that did not resolve, written against the
/// suggestions the same diagnosis carries.
///
/// It used to say the closest known names were in `diagnosis.candidates`
/// whether or not any were: `^`, `FOO` and `=` each produce an empty list (a
/// symbol is never a typo of an alphabetic name, and the distance ceiling is
/// deliberately tight), so the reader was sent to a list that was not there
/// under a field name no host prints. Both halves are fixed here — the
/// sentence names what the host actually renders, and says something true
/// when there is nothing to name.
pub(super) fn spelling_check(candidates: &[String]) -> DebugCheck {
    if candidates.is_empty() {
        return check(
            "checkSpelling",
            ("Check spelling", "スペルを確認する"),
            (
                "Check the spelling of the Word name. No known name is close enough to be a likely \
                 misspelling of it, so this is more likely a Word that was never defined.",
                "word 名のスペルを確認する。綴り間違いと見なせるほど近い既知の名前はないので、\
                 未定義の word である可能性が高い",
            ),
        );
    }
    check(
        "checkSpelling",
        ("Check spelling", "スペルを確認する"),
        (
            &format!(
                "Check the spelling of the Word name; the closest known names are {} \
                 (also shown on the \"did you mean\" line).",
                candidates.join(", "),
            ),
            &format!(
                "word 名のスペルを確認する。最も近い既知の名前は {}（\"did you mean\" の行にも出る）",
                candidates.join(", "),
            ),
        ),
    )
}

pub(crate) fn build_next_checks(
    why: &CauseClass,
    word: Option<&str>,
    category: Option<&ErrorCategory>,
    nil_reason: Option<&NilReason>,
    candidates: &[String],
    stack_len_before: usize,
) -> Vec<DebugCheck> {
    let fired_condition = match category {
        Some(ErrorCategory::Declared(condition)) => Some(*condition),
        _ => None,
    };
    let mut out = declared_checks(why, word, nil_reason, fired_condition, stack_len_before);

    match why {
        CauseClass::Domain => {
            if matches!(nil_reason, Some(NilReason::DivisionByZero)) {
                // Name the Word that actually met the zero rather than
                // hard-coding `DIV`: any Word that declares the same
                // `divisorEqualsZero` condition must send the reader to an
                // operand of the Word it actually called.
                let word_label = word.unwrap_or("the word");
                out.push(check(
                    "checkDivisor",
                    ("Check divisor", "除数を確認する"),
                    (
                        &format!("Inspect the right operand of {}.", word_label),
                        &format!("{} の右オペランドを確認する", word_label),
                    ),
                ));
                // Name the Words that actually recover an absence. This
                // check has twice named a Word the dictionary would reject —
                // first `SAFE`, which never existed, then `OR-NIL`, which was
                // retired — and a diagnosis is the one surface an agent is
                // told to follow literally, so a stale spelling here costs
                // more than saying nothing. `word_recovery_tests` holds every
                // Word a check names to the dictionary, which is what stops
                // that class of staleness coming back silently.
                out.push(check(
                    "checkZeroIsExpected",
                    ("Check zero is expected", "0 が正常値かを確認する"),
                    (
                        "If 0 is a legitimate value here, name the quotient with BIND and choose a fallback with NIL? and SELECT, or guard the divisor.",
                        "0 が正常値としてあり得るなら商を BIND で名付け、NIL? と SELECT で代替値を選ぶか、除数を事前に確認する",
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
            // A double-quoted token is a specific, recognizable mistake, not a
            // misspelling: `'` is Ajisai's only string delimiter, so `"hi"`
            // reaches the dictionary as the *name* `"HI"` — quotes included —
            // and fails as an unknown word. Left to the generic checks the
            // reader was sent to look for a spelling error in a token that has
            // no spelling error, with nothing naming the real rule. SKILL.md
            // already lists this among the common mistakes, which is the
            // clearest sign it is worth diagnosing rather than documenting.
            // The names a Forth or a structured language would have here are
            // a recognizable mistake of their own, not a misspelling: the
            // repair is a different phrase, which `checkSpelling` cannot
            // say. SKILL.md §8 lists both; a diagnosis is where an agent
            // meets them at run time.
            let upper = word.map(|w| w.trim().to_uppercase());
            if upper
                .as_deref()
                .is_some_and(|name| STACK_SHUFFLERS.contains(&name))
            {
                out.push(check(
                    "checkNoStackShufflers",
                    ("No stack shufflers", "スタック操作語はない"),
                    (
                        "Ajisai has no DUP / SWAP / OVER / ROT. Every Word consumes the operands it \
                         reads; to use a value more than once, name it with BIND and read the name: \
                         `5 'N' BIND N N MUL`.",
                        "Ajisai に DUP / SWAP / OVER / ROT はない。どの word も読んだオペランドを消費する。\
                         値を二度使うなら BIND で名前を付けて名前を読む: `5 'N' BIND N N MUL`",
                    ),
                ));
            }
            if upper
                .as_deref()
                .is_some_and(|name| CONTROL_KEYWORDS.contains(&name))
            {
                out.push(check(
                    "checkNoControlKeywords",
                    ("No control keywords", "制御構文のキーワードはない"),
                    (
                        "Ajisai has no IF / ELSE / WHILE / FOR. Branch by choosing between two values \
                         already built — `[ whenTrue ] [ whenFalse ] test SELECT` — and iterate with \
                         MAP / FILTER / FOLD / SCAN over a finite Vector.",
                        "Ajisai に IF / ELSE / WHILE / FOR はない。分岐は作り終えた二つの値から選ぶ \
                         `[ whenTrue ] [ whenFalse ] test SELECT`、反復は有限の Vector に対する \
                         MAP / FILTER / FOLD / SCAN",
                    ),
                ));
            }
            if word.is_some_and(has_full_width_ascii) {
                out.push(check(
                    "checkCharacterWidth",
                    ("Check character width", "全角文字を確認する"),
                    (
                        "This name contains full-width characters. Word names and numbers are \
                         written in ASCII: `ＡＤＤ` is not `ADD`, and `１` is not `1`. Switch the \
                         input method to half-width and retype the token.",
                        "この名前に全角文字が含まれている。word 名と数値は半角 (ASCII) で書く: \
                         `ＡＤＤ` は `ADD` ではなく、`１` は `1` ではない。半角に切り替えて打ち直す",
                    ),
                ));
            }
            if word.is_some_and(looks_double_quoted) {
                out.push(check(
                    "checkStringQuoting",
                    ("Check string quoting", "文字列の引用符を確認する"),
                    (
                        "This name is wrapped in double quotes. Ajisai has one string \
                         delimiter, the single quote: write 'text', not \"text\". A \
                         double-quoted token is read as a Word name, quotes and all.",
                        "この名前は二重引用符で囲まれている。Ajisai の文字列区切りは単一引用符だけなので \
                         'text' と書く。二重引用符付きのトークンは引用符ごと word 名として読まれる",
                    ),
                ));
            }
            out.push(spelling_check(candidates));
            out.push(check(
                "checkUserDefinitions",
                ("Check user definitions", "ユーザー定義を確認する"),
                (
                    "Check that the User Word is defined (DEF) and spelled as defined.",
                    "その User Word が DEF で定義済みで、定義どおりの綴りかを確認する",
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
            out.push(check(
                "checkSelectiveOps",
                ("Check selective ops", "選択的操作を確認する"),
                (
                    "Check whether a filter or drop was applied to only one side.",
                    "片方だけ filter や drop が適用されていないか確認する",
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
                        "Nesting deepens with each block a value is built inside; flatten the phrase so the work happens at one level.",
                        "値を組み立てるブロックを入れ子にするほど深くなる。ひとつの段で済むように式を平らにする",
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
            if matches!(category, Some(ErrorCategory::ContractViolation)) {
                out.push(check(
                    "checkDeclaredContract",
                    ("Check the declared contract", "宣言した契約を確認する"),
                    (
                        "A `#:contract` line declares something the Word's body does not do; \
                         the message says which key, as declared and as inferred. Fix the body \
                         or the declaration. infer_contracts (`ajisai agent infer-contracts`) \
                         answers a paste-ready `suggested` line for the Word as written.",
                        "`#:contract` 行の宣言が word 本体の振る舞いと食い違う。どのキーが宣言と推論で \
                         異なるかはメッセージにある。本体か宣言を直す。infer_contracts \
                         (`ajisai agent infer-contracts`) が今の本体に合う `suggested` 行を返す",
                    ),
                ));
            } else if matches!(category, Some(ErrorCategory::Declared("protectedWord"))) {
                out.push(check(
                    "checkProtection",
                    ("Check protection", "保護を確認する"),
                    (
                        "A dictionary change was asked of a Core Word, which the dictionary seals.",
                        "Core Word に対する辞書の変更が求められたが、Core は封印されている",
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
