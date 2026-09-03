use super::debug_next_checks::build_next_checks;
use super::word_candidates::suggest_words;
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
    HostEnvironment,
    Optimizer,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorLocus {
    pub kind: ErrorLocusKind,
    pub word: Option<String>,
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
    ShapeMismatch,
    NilFlow,
    Environment,
    Effect,
    UserLogic,
    ResourceLimit,
    SourceForm,
    ContractViolation,
    OptimizerMismatch,
    InternalInvariant,
    Unknown,
}

/// One piece of display text in every locale the diagnosis vocabulary is
/// translated into.
///
/// Diagnostics used to carry an English label beside a Japanese sentence in
/// one string pair, which read as a mixed-language message to a human and as
/// an unstable, unlocalizable key to a machine. The stable identity now lives
/// in [`DebugCheck::code`]; this type carries only what is shown.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LocalizedText {
    pub en: String,
    pub ja: String,
}

impl LocalizedText {
    pub fn new(en: impl Into<String>, ja: impl Into<String>) -> Self {
        LocalizedText {
            en: en.into(),
            ja: ja.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DebugCheck {
    /// Stable machine-readable identifier, e.g. `checkSpelling`. A consumer
    /// keys off this and never off the display text, which is free to change
    /// wording or gain a locale without breaking anyone.
    pub code: &'static str,
    /// Short heading for the check.
    pub title: LocalizedText,
    /// What to actually look at.
    pub detail: LocalizedText,
}

/// The named ceiling a resource-limit failure crossed, its configured value
/// and the size that crossed it — the machine-readable half of "the program
/// is too big", indexed by the same identifier the host publishes in its
/// declared limit table.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ResourceLimitFacts {
    pub resource: String,
    pub limit: u64,
    pub observed: Option<u64>,
    /// How far an incrementally charged operation got before it was refused.
    /// `None` for every ceiling whose `observed` is a real measurement of a
    /// real size; present exactly where `observed` cannot say how far over the
    /// request was. See `error::ResourceProgress`.
    pub progress: Option<crate::error::ResourceProgress>,
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
    /// Known Words within a small edit distance of an unrecognized name,
    /// best match first. Empty for every other cause class.
    pub candidates: Vec<String>,
    pub resource_limit: Option<ResourceLimitFacts>,
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
    /// Known Words within a small edit distance of an unrecognized name, best
    /// match first. "Check the spelling" without saying what the spelling
    /// might have been is the one repair hint an agent cannot act on, and the
    /// vocabulary needed to answer it is already compiled in.
    pub candidates: Vec<String>,
    /// Which declared ceiling a resource-limit failure crossed. `None` for
    /// every other cause class.
    pub resource_limit: Option<ResourceLimitFacts>,
}

impl DebugDiagnosis {
    /// Record where in the source the failure happened, as two machine-readable
    /// evidence entries.
    ///
    /// Evidence is the established place for a `key=value` fact a reader may
    /// want and a consumer may parse (`stackLenBefore=5` is already there), so
    /// the position needs no new protocol field and reaches every host that
    /// already renders a diagnosis. Adding it twice is a no-op: the position of
    /// a failure does not change as the error unwinds.
    pub fn with_source_position(mut self, span: Option<crate::tokenizer::SourceSpan>) -> Self {
        let Some(span) = span else { return self };
        if self.evidence.iter().any(|e| e.starts_with("sourceLine=")) {
            return self;
        }
        self.evidence.push(format!("sourceLine={}", span.line));
        self.evidence.push(format!("sourceColumn={}", span.column));
        self
    }

    /// Record `word` as a Word the failure happened *inside* — the higher-order
    /// Word whose block raised it, or the User Word whose body did.
    ///
    /// The locus stays where the failure was raised. A block applied by `MAP`
    /// is not `MAP`'s contract: when `[ 1 2 ] { 'x' 1 ADD } MAP` failed, the
    /// diagnosis named `MAP` and every next-check line asked about `MAP`'s
    /// expected shape, while the Word that could not do the work was `ADD`. So
    /// the enclosing Words are context, kept innermost-first in one evidence
    /// entry (`insideWords=MAP,FOLD`) rather than overwriting the answer to
    /// "which Word failed".
    pub fn with_enclosing_word(&mut self, word: &str) {
        let entry = self
            .evidence
            .iter_mut()
            .find(|e| e.starts_with("insideWords="));
        match entry {
            Some(existing) => {
                existing.push(',');
                existing.push_str(word);
            }
            None => self.evidence.push(format!("insideWords={}", word)),
        }
    }

    /// Re-rank the candidate list against names this interpreter knows on top
    /// of the compiled-in vocabulary — user Words and live bindings.
    ///
    /// The static registry answers a misspelled Coreword on its own, but a
    /// misspelled *user* Word is only knowable at the failure site, which is
    /// the one place that holds the dictionary.
    pub fn with_user_vocabulary<'a>(&mut self, names: impl Iterator<Item = &'a str>) {
        if !matches!(self.why, CauseClass::TypoOrUnknownName) {
            return;
        }
        let Some(word) = self.where_.word.as_deref() else {
            return;
        };
        self.candidates = suggest_words(word, names);
    }
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
            CauseClass::ShapeMismatch => "shapeMismatch",
            CauseClass::NilFlow => "nilFlow",
            CauseClass::Environment => "environment",
            CauseClass::Effect => "effect",
            CauseClass::UserLogic => "userLogic",
            CauseClass::ResourceLimit => "resourceLimit",
            CauseClass::SourceForm => "sourceForm",
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
            ErrorCategory::DivisionByZero => CauseClass::Domain,
            ErrorCategory::IndexOutOfBounds => CauseClass::Index,
            ErrorCategory::VectorLengthMismatch => CauseClass::VectorLength,
            ErrorCategory::ShapeMismatch => CauseClass::ShapeMismatch,
            ErrorCategory::MalformedSource => CauseClass::SourceForm,
            ErrorCategory::NameConflict => CauseClass::ContractViolation,
            // LANG.MACHINE.LIMITS calls the step and recursion budgets host
            // safety controls rather than language semantics, and the two
            // answers differ: "the program is wrong" is fixed by rewriting it,
            // "the program is too big" by raising the budget or by finding a
            // cheaper shape for the same computation. Filing both under
            // `userLogic` sent every reader down the first road.
            ErrorCategory::ExecutionLimitExceeded => CauseClass::ResourceLimit,
            ErrorCategory::ResourceLimitExceeded => CauseClass::ResourceLimit,
            ErrorCategory::RecursionLimitExceeded => CauseClass::ResourceLimit,
            ErrorCategory::ModeUnsupported => CauseClass::ContractViolation,
            ErrorCategory::BuiltinProtection => CauseClass::ContractViolation,
            ErrorCategory::CondExhausted => CauseClass::UserLogic,
            // A cyclic DEF is a static shape rejected before anything runs —
            // the same kind of fault as `NameConflict`, not a runtime resource
            // question.
            ErrorCategory::SelfReferentialDefinition => CauseClass::ContractViolation,
            // The registry named the condition at the raise site, so the class
            // follows from the spec's own vocabulary rather than from `Custom`.
            ErrorCategory::Declared(condition) => {
                super::debug_declared_checks::cause_class_for_declared_condition(condition)
            }
            ErrorCategory::Custom => CauseClass::Unknown,
        }
    }
}

/// The cause class a *reasoned absence* names on its own.
///
/// [`ErrorCategory`] cannot answer this. Every `NilReason` without a matching
/// `AjisaiError` variant behind it lands on `ErrorCategory::Custom`, which
/// maps to `Unknown`, so a projection the registry declares — `SQRT`'s
/// negative radicand, `RANGE`'s materialization ceiling — reached the caller
/// as `why: "unknown"` with "read the message" as its only next check. The
/// reason had named the condition all along; this reads it.
fn cause_class_for_nil_reason(reason: &NilReason) -> CauseClass {
    match reason {
        // A well-formed operand outside the operation's domain: a negative
        // radicand, a zero divisor.
        NilReason::DomainMiss | NilReason::DivisionByZero => CauseClass::Domain,
        // Both are budgets rather than mistakes: the materialization ceiling
        // and the comparison budget answer to "the request is too big", not
        // "the program is wrong" — the distinction `ResourceLimit` exists for.
        NilReason::SpaceExhausted | NilReason::Undecidable => CauseClass::ResourceLimit,
        NilReason::IndexOutOfBounds => CauseClass::Index,
        NilReason::MissingField | NilReason::InvalidEncoding | NilReason::InvalidLens => {
            CauseClass::ValueShape
        }
        NilReason::StackUnderflow => CauseClass::StackShape,
        NilReason::UnknownWord => CauseClass::TypoOrUnknownName,
        NilReason::NotAvailable => CauseClass::Environment,
        NilReason::ExecutionFailure => CauseClass::UserLogic,
        // Absence that no operation produced — a `NIL` in source, or one that
        // has passed through a dense lane, which carries presence but no
        // reason. Nothing is wrong; a NIL is simply flowing.
        NilReason::EmptySequence | NilReason::Literal => CauseClass::NilFlow,
    }
}

fn classify_locus(word: Option<&str>) -> ErrorLocus {
    let (kind, dictionary) = match word {
        None => (ErrorLocusKind::Unknown, None),
        Some(name) => {
            if let Some(idx) = name.find('@') {
                let (dictionary, _) = name.split_at(idx);
                (ErrorLocusKind::UserWord, Some(dictionary.to_string()))
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
        dictionary,
    }
}

fn adjust_phase_for_category(phase: ErrorPhase, category: Option<&ErrorCategory>) -> ErrorPhase {
    if !matches!(phase, ErrorPhase::ExecuteWord) {
        return phase;
    }
    match category {
        Some(ErrorCategory::UnknownWord) => ErrorPhase::ResolveWord,
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
        let mut diagnosis = Self::from_error_category(
            ErrorPhase::ExecuteWord,
            word,
            Some(&category),
            None,
            stack_len_before,
            stack_len_after,
            Some(err.to_string()),
        );
        diagnosis.resource_limit = resource_limit_facts(err);
        diagnosis
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
        // A reasoned absence classifies itself; `ErrorCategory` is consulted
        // only where there is no reason to read, because `Custom` absorbs
        // every reason without an `AjisaiError` variant behind it and would
        // answer `Unknown` for a condition the registry declares.
        let why = match (nil_reason, category) {
            (Some(reason), _) => cause_class_for_nil_reason(reason),
            (None, Some(category)) => CauseClass::from_error_category(category),
            (None, None) => CauseClass::Unknown,
        };
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
        let candidates = match (&why, word) {
            (CauseClass::TypoOrUnknownName, Some(name)) => suggest_words(name, std::iter::empty()),
            _ => Vec::new(),
        };

        DebugDiagnosis {
            when,
            where_,
            why,
            summary,
            evidence,
            next_checks,
            agreed_prefix: None,
            candidates,
            resource_limit: None,
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
            candidates: self.candidates.clone(),
            resource_limit: self.resource_limit.clone(),
        }
    }
}

/// The machine-readable facts behind a resource-limit failure, or `None` when
/// the error is not one.
fn resource_limit_facts(err: &AjisaiError) -> Option<ResourceLimitFacts> {
    match err {
        AjisaiError::ResourceLimitExceeded {
            resource,
            limit,
            observed,
            progress,
        } => Some(ResourceLimitFacts {
            resource: resource.as_protocol_str().to_string(),
            limit: *limit,
            observed: *observed,
            progress: *progress,
        }),
        // The step budget lives outside `RuntimeLimits` but is published in
        // the same limit table, so it answers "which ceiling" the same way.
        AjisaiError::ExecutionLimitExceeded { limit } => Some(ResourceLimitFacts {
            resource: crate::error::ResourceLimit::ExecutionSteps
                .as_protocol_str()
                .to_string(),
            limit: *limit as u64,
            observed: None,
            progress: None,
        }),
        _ => None,
    }
}

fn recoverability_for(why: &CauseClass, category: Option<&ErrorCategory>) -> &'static str {
    match category {
        Some(ErrorCategory::DivisionByZero)
        | Some(ErrorCategory::StructureError)
        | Some(ErrorCategory::IndexOutOfBounds)
        | Some(ErrorCategory::ShapeMismatch)
        | Some(ErrorCategory::VectorLengthMismatch) => "fixInput",
        Some(ErrorCategory::UnknownWord)
        | Some(ErrorCategory::StackUnderflow)
        | Some(ErrorCategory::ModeUnsupported)
        | Some(ErrorCategory::MalformedSource)
        | Some(ErrorCategory::NameConflict)
        | Some(ErrorCategory::CondExhausted)
        | Some(ErrorCategory::SelfReferentialDefinition) => "fixProgram",
        Some(ErrorCategory::BuiltinProtection) => "fixCapabilityOrForce",
        Some(ErrorCategory::ExecutionLimitExceeded)
        | Some(ErrorCategory::RecursionLimitExceeded) => "addBudgetOrFixRecursion",
        // A size ceiling is not fixed by letting the program run longer: the
        // work itself has to get smaller, or the host has to declare a larger
        // ceiling.
        Some(ErrorCategory::ResourceLimitExceeded) => "reduceWorkOrRaiseLimit",
        // A declared condition answers by what it names: a wrong operand is
        // repaired in the input, a broken rule in the program.
        Some(ErrorCategory::Declared(_)) => {
            super::debug_declared_checks::repair_for_declared_condition(why)
        }
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

#[cfg(test)]
mod tests {
    use super::{classify_locus, ErrorLocusKind};

    #[test]
    fn qualified_word_is_classified_as_a_user_dictionary_word() {
        let locus = classify_locus(Some("EXAMPLE@DOUBLE"));
        assert_eq!(locus.kind, ErrorLocusKind::UserWord);
        assert_eq!(locus.dictionary.as_deref(), Some("EXAMPLE"));
    }
}
