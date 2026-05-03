use crate::error::{AjisaiError, ErrorCategory, NilReason};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPhase {
    Tokenize,
    ParseStructure,
    ResolveWord,
    ExecuteWord,
    SafeProjection,
    NilPropagation,
    Assertion,
    HostIo,
    OptimizationValidation,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorLocusKind {
    UserWord,
    CoreWord,
    BuiltinWord,
    ModuleWord,
    HostEnvironment,
    Optimizer,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorLocus {
    pub kind: ErrorLocusKind,
    pub word: Option<String>,
    pub module: Option<String>,
    pub dictionary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CauseClass {
    TypoOrUnknownName,
    StackShape,
    ValueShape,
    Domain,
    Index,
    VectorLength,
    NilFlow,
    Environment,
    Effect,
    UserLogic,
    ContractViolation,
    OptimizerMismatch,
    InternalInvariant,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugCheck {
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugDiagnosis {
    pub when: ErrorPhase,
    pub where_: ErrorLocus,
    pub why: CauseClass,
    pub summary: String,
    pub evidence: Vec<String>,
    pub next_checks: Vec<DebugCheck>,
}

impl CauseClass {
    pub fn from_error_category(category: &ErrorCategory) -> Self {
        match category {
            ErrorCategory::StackUnderflow => CauseClass::StackShape,
            ErrorCategory::StructureError => CauseClass::ValueShape,
            ErrorCategory::UnknownWord => CauseClass::TypoOrUnknownName,
            ErrorCategory::UnknownModule => CauseClass::Environment,
            ErrorCategory::DivisionByZero => CauseClass::Domain,
            ErrorCategory::IndexOutOfBounds => CauseClass::Index,
            ErrorCategory::VectorLengthMismatch => CauseClass::VectorLength,
            ErrorCategory::ExecutionLimitExceeded => CauseClass::UserLogic,
            ErrorCategory::ModeUnsupported => CauseClass::ContractViolation,
            ErrorCategory::BuiltinProtection => CauseClass::ContractViolation,
            ErrorCategory::CondExhausted => CauseClass::UserLogic,
            ErrorCategory::Custom => CauseClass::Unknown,
            ErrorCategory::OverConsumption => CauseClass::Effect,
            ErrorCategory::UnconsumedLeak => CauseClass::Effect,
            ErrorCategory::FlowBreak => CauseClass::Effect,
            ErrorCategory::BifurcationViolation => CauseClass::InternalInvariant,
        }
    }
}

fn classify_locus(word: Option<&str>) -> ErrorLocus {
    let (kind, module) = match word {
        None => (ErrorLocusKind::Unknown, None),
        Some(name) => {
            if let Some(idx) = name.find('@') {
                let (left, right) = name.split_at(idx);
                let _ = right;
                (ErrorLocusKind::ModuleWord, Some(left.to_string()))
            } else if crate::coreword_registry::get_builtin_word_metadata(name).is_some() {
                (ErrorLocusKind::CoreWord, None)
            } else {
                (ErrorLocusKind::Unknown, None)
            }
        }
    };
    ErrorLocus {
        kind,
        word: word.map(|s| s.to_string()),
        module,
        dictionary: None,
    }
}

fn adjust_phase_for_category(phase: ErrorPhase, category: Option<&ErrorCategory>) -> ErrorPhase {
    if !matches!(phase, ErrorPhase::ExecuteWord) {
        return phase;
    }
    match category {
        Some(ErrorCategory::UnknownWord) | Some(ErrorCategory::UnknownModule) => {
            ErrorPhase::ResolveWord
        }
        _ => phase,
    }
}

impl DebugDiagnosis {
    pub fn from_error(
        err: &AjisaiError,
        word: Option<&str>,
        stack_len_before: usize,
        stack_len_after: usize,
    ) -> Self {
        let category = ErrorCategory::from_error(err);
        Self::from_error_category(
            ErrorPhase::ExecuteWord,
            word,
            Some(&category),
            None,
            stack_len_before,
            stack_len_after,
            Some(err.to_string()),
        )
    }

    pub fn from_error_category(
        when: ErrorPhase,
        word: Option<&str>,
        category: Option<&ErrorCategory>,
        nil_reason: Option<&NilReason>,
        stack_len_before: usize,
        stack_len_after: usize,
        message: Option<String>,
    ) -> Self {
        let when = adjust_phase_for_category(when, category);
        let why = category
            .map(CauseClass::from_error_category)
            .unwrap_or(CauseClass::Unknown);
        let where_ = classify_locus(word);

        let summary = build_summary(
            &when,
            &where_,
            &why,
            category,
            nil_reason,
            message.as_deref(),
        );
        let evidence = build_evidence(category, nil_reason, stack_len_before, stack_len_after);
        let next_checks = build_next_checks(&why, word, category, nil_reason);

        DebugDiagnosis {
            when,
            where_,
            why,
            summary,
            evidence,
            next_checks,
        }
    }
}

fn build_summary(
    when: &ErrorPhase,
    locus: &ErrorLocus,
    why: &CauseClass,
    category: Option<&ErrorCategory>,
    nil_reason: Option<&NilReason>,
    message: Option<&str>,
) -> String {
    let where_str = locus
        .word
        .clone()
        .unwrap_or_else(|| format!("{:?}", locus.kind));
    let category_str = category
        .map(|c| format!("{:?}", c))
        .unwrap_or_else(|| "UnknownCategory".to_string());
    let nil_str = nil_reason
        .map(|r| format!(" nil={:?}", r))
        .unwrap_or_default();
    let msg_str = message
        .map(|m| format!(" msg=\"{}\"", m))
        .unwrap_or_default();
    format!(
        "{:?} / {} / {:?} ({}){}{}",
        when, where_str, why, category_str, nil_str, msg_str
    )
}

fn build_evidence(
    category: Option<&ErrorCategory>,
    nil_reason: Option<&NilReason>,
    stack_len_before: usize,
    stack_len_after: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(c) = category {
        out.push(format!("errorCategory={:?}", c));
    }
    if let Some(r) = nil_reason {
        out.push(format!("nilReason={:?}", r));
    }
    out.push(format!("stackLenBefore={}", stack_len_before));
    out.push(format!("stackLenAfter={}", stack_len_after));
    out
}

fn check(label: &str, detail: &str) -> DebugCheck {
    DebugCheck {
        label: label.to_string(),
        detail: detail.to_string(),
    }
}

fn build_next_checks(
    why: &CauseClass,
    word: Option<&str>,
    category: Option<&ErrorCategory>,
    nil_reason: Option<&NilReason>,
) -> Vec<DebugCheck> {
    let mut out = Vec::new();

    let safe_caught = matches!(nil_reason, Some(NilReason::SafeCaught(_)));

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
                "Check imports/definitions",
                "module import 漏れ、または user word 定義漏れを確認する",
            ));
        }
        CauseClass::Environment => {
            if matches!(category, Some(ErrorCategory::UnknownModule)) {
                out.push(check("Check module name", "module 名のスペルを確認する"));
                out.push(check(
                    "Check build inclusion",
                    "module が現在のビルドに含まれているか確認する",
                ));
                out.push(check(
                    "Check import target kind",
                    "import 対象が module word か user word か確認する",
                ));
            } else {
                out.push(check("Check environment", "実行環境の前提条件を確認する"));
            }
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

    if safe_caught {
        out.push(check("Check NIL origin", "NIL が生成された地点を確認する"));
        out.push(check(
            "Check SAFE intent",
            "SAFE で握りつぶしてよいエラーか確認する",
        ));
        out.push(check(
            "Check fallback",
            "fallback が必要なら => の利用を検討する",
        ));
    }

    out
}
