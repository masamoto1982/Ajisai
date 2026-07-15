use crate::error::{AjisaiError, ErrorCategory, NilReason};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPhase {
    Tokenize,
    ParseStructure,
    ResolveWord,
    ExecuteWord,
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DebugCheck {
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AiDiagnosticPayload {
    pub kind: Option<String>,
    pub recoverability: String,
    pub semantic_area: String,
    pub word: Option<String>,
    pub semantic_role: String,
    pub algebraic_family: String,
    pub nil_reason: Option<String>,
    pub truth_value: Option<String>,
    pub effect: Option<String>,
    pub next_checks: Vec<DebugCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugDiagnosis {
    pub when: ErrorPhase,
    pub where_: ErrorLocus,
    pub why: CauseClass,
    pub summary: String,
    pub evidence: Vec<String>,
    pub next_checks: Vec<DebugCheck>,
    /// CF-comparison agreed-prefix length (SPEC §4.5.0 / §7.4.1): the
    /// number of leading partial quotients that matched before the
    /// partial-quotient budget was exhausted on an `Unknown` (U)
    /// comparison result. `None` for diagnoses unrelated to CF
    /// comparison. Machine-readable; surfaced as `diagnosis.agreedPrefix`.
    pub agreed_prefix: Option<usize>,
}

impl ErrorPhase {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorPhase::Tokenize => "tokenize",
            ErrorPhase::ParseStructure => "parseStructure",
            ErrorPhase::ResolveWord => "resolveWord",
            ErrorPhase::ExecuteWord => "executeWord",
            ErrorPhase::NilPropagation => "nilPropagation",
            ErrorPhase::Assertion => "assertion",
            ErrorPhase::HostIo => "hostIo",
            ErrorPhase::OptimizationValidation => "optimizationValidation",
            ErrorPhase::Unknown => "unknown",
        }
    }
}

impl ErrorLocusKind {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorLocusKind::UserWord => "userWord",
            ErrorLocusKind::CoreWord => "coreWord",
            ErrorLocusKind::BuiltinWord => "builtinWord",
            ErrorLocusKind::ModuleWord => "moduleWord",
            ErrorLocusKind::HostEnvironment => "hostEnvironment",
            ErrorLocusKind::Optimizer => "optimizer",
            ErrorLocusKind::Unknown => "unknown",
        }
    }
}

impl CauseClass {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            CauseClass::TypoOrUnknownName => "typoOrUnknownName",
            CauseClass::StackShape => "stackShape",
            CauseClass::ValueShape => "valueShape",
            CauseClass::Domain => "domain",
            CauseClass::Index => "index",
            CauseClass::VectorLength => "vectorLength",
            CauseClass::NilFlow => "nilFlow",
            CauseClass::Environment => "environment",
            CauseClass::Effect => "effect",
            CauseClass::UserLogic => "userLogic",
            CauseClass::ContractViolation => "contractViolation",
            CauseClass::OptimizerMismatch => "optimizerMismatch",
            CauseClass::InternalInvariant => "internalInvariant",
            CauseClass::Unknown => "unknown",
        }
    }
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
            ErrorCategory::RecursionLimitExceeded => CauseClass::UserLogic,
            ErrorCategory::ModeUnsupported => CauseClass::ContractViolation,
            ErrorCategory::BuiltinProtection => CauseClass::ContractViolation,
            ErrorCategory::CondExhausted => CauseClass::UserLogic,
            ErrorCategory::Custom => CauseClass::Unknown,
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
        let next_checks = build_next_checks(&why, word, category);

        DebugDiagnosis {
            when,
            where_,
            why,
            summary,
            evidence,
            next_checks,
            agreed_prefix: None,
        }
    }

    /// Build the diagnostic context for a continued-fraction comparison
    /// that produced the logical `Unknown` (U) because the
    /// partial-quotient budget was exhausted (SPEC §7.4.1). `word` is the
    /// comparison Coreword that produced U (e.g. `"COMPARE-WITHIN"`,
    /// `"LT"`); `agreed_prefix` is the number of leading partial quotients
    /// that matched before the budget ran out, carried machine-readably in
    /// the `agreed_prefix` field (SPEC §4.5.0).
    pub fn comparison_unknown(word: Option<&str>, agreed_prefix: usize) -> Self {
        let where_ = classify_locus(word);
        DebugDiagnosis {
            when: ErrorPhase::ExecuteWord,
            where_,
            why: CauseClass::NilFlow,
            summary: format!(
                "executeWord / {} / comparison undecidable within budget; agreedPrefix={}",
                word.unwrap_or("comparison"),
                agreed_prefix
            ),
            evidence: vec![
                "truthValue=unknown".to_string(),
                format!("agreedPrefix={}", agreed_prefix),
            ],
            next_checks: Vec::new(),
            agreed_prefix: Some(agreed_prefix),
        }
    }
    /// Build the AI-facing structured diagnostic payload used by tests, WASM
    /// adapters, and review tooling. Human-readable `summary` stays separate;
    /// this payload exposes stable protocol fields so agents can distinguish
    /// NIL, UNKNOWN, host-effect violations, portability issues, and input
    /// domain errors without matching display strings.
    pub fn ai_payload(
        &self,
        category: Option<&ErrorCategory>,
        nil_reason: Option<&NilReason>,
        truth_value: Option<&str>,
        effect: Option<&str>,
    ) -> AiDiagnosticPayload {
        let word = self.where_.word.as_deref();
        AiDiagnosticPayload {
            kind: category.map(|c| c.as_protocol_str().to_string()),
            recoverability: recoverability_for(&self.why, category).to_string(),
            semantic_area: semantic_area_for(word, &self.why).to_string(),
            word: self.where_.word.clone(),
            semantic_role: semantic_role_for(word).to_string(),
            algebraic_family: algebraic_family_for(word, &self.why).to_string(),
            nil_reason: nil_reason.map(|r| r.as_protocol_str().to_string()),
            truth_value: truth_value.map(str::to_string),
            effect: effect.map(str::to_string),
            next_checks: self.next_checks.clone(),
        }
    }
}

fn recoverability_for(why: &CauseClass, category: Option<&ErrorCategory>) -> &'static str {
    match category {
        Some(ErrorCategory::DivisionByZero)
        | Some(ErrorCategory::StructureError)
        | Some(ErrorCategory::IndexOutOfBounds)
        | Some(ErrorCategory::VectorLengthMismatch) => "fixInput",
        Some(ErrorCategory::UnknownWord)
        | Some(ErrorCategory::UnknownModule)
        | Some(ErrorCategory::StackUnderflow)
        | Some(ErrorCategory::ModeUnsupported)
        | Some(ErrorCategory::CondExhausted) => "fixProgram",
        Some(ErrorCategory::BuiltinProtection) => "fixCapabilityOrForce",
        Some(ErrorCategory::ExecutionLimitExceeded)
        | Some(ErrorCategory::RecursionLimitExceeded) => "addBudgetOrFixRecursion",
        Some(ErrorCategory::Custom) | None => match why {
            CauseClass::Environment | CauseClass::Effect => "fixHost",
            CauseClass::NilFlow => "handleUnknownOrNil",
            _ => "inspectContext",
        },
    }
}

fn semantic_role_for(word: Option<&str>) -> &'static str {
    let Some(word) = word else {
        return "Unknown";
    };
    if let Some(meta) = crate::coreword_registry::get_coreword_metadata(word) {
        return match meta.profile {
            crate::coreword_registry::WordProfile::Hosted => "HostedEffect",
            crate::coreword_registry::WordProfile::PlatformSpecific => "Extension",
            crate::coreword_registry::WordProfile::Core => {
                if matches!(word, "COMPARE-WITHIN") {
                    "Primitive"
                } else {
                    "Derived"
                }
            }
        };
    }
    "Unknown"
}

fn semantic_area_for(word: Option<&str>, why: &CauseClass) -> &'static str {
    match word {
        Some("ADD" | "SUB" | "MUL" | "DIV" | "MOD" | "SQRT" | "FLOOR" | "CEIL" | "ROUND") => {
            "exact-real-arithmetic"
        }
        Some("EQ" | "NEQ" | "LT" | "LTE" | "GT" | "GTE" | "COMPARE-WITHIN") => {
            "exact-real-comparison"
        }
        Some("AND" | "OR" | "NOT") => "k3-truth",
        Some(word) if word.contains('@') => "hosted-effect",
        Some("PRINT") => "hosted-effect",
        _ => match why {
            CauseClass::Effect | CauseClass::Environment => "hosted-effect",
            CauseClass::NilFlow => "unknown-or-absence",
            CauseClass::StackShape | CauseClass::ValueShape => "stack-value-shape",
            _ => "unknown",
        },
    }
}

fn algebraic_family_for(word: Option<&str>, why: &CauseClass) -> &'static str {
    match semantic_area_for(word, why) {
        "exact-real-arithmetic" => "exact-arithmetic",
        "exact-real-comparison" => "observation",
        "k3-truth" => "k3-truth",
        "hosted-effect" => "hosted-effect",
        other => other,
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
        .unwrap_or_else(|| locus.kind.as_protocol_str().to_string());
    let category_str = category
        .map(|c| c.as_protocol_str().to_string())
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
        out.push(format!("category={}", c.as_protocol_str()));
    }
    if let Some(r) = nil_reason {
        out.push(format!("absenceReason={}", r.as_protocol_str()));
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
