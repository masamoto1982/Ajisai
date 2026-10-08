use super::debug_next_checks::{build_next_checks, spelling_check};
use super::word_candidates::suggest_words;
use crate::error::{AjisaiError, ErrorCategory, NilReason};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPhase {
    /// Every source error, the grammar's structural phase included: an
    /// unbalanced bracket is a `Tokenize` failure like any other.
    Tokenize,
    ResolveWord,
    ExecuteWord,
    /// The pre-execution check of `#:contract` declarations
    /// (LANG.CONTRACT.CHECK): decided before any Word runs, by `check` and by
    /// `compute` alike.
    CheckContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorLocusKind {
    UserWord,
    CoreWord,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorLocus {
    pub kind: ErrorLocusKind,
    pub word: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CauseClass {
    TypoOrUnknownName,
    StackShape,
    ValueShape,
    Domain,
    Index,
    ShapeMismatch,
    NilFlow,
    UserLogic,
    ResourceLimit,
    SourceForm,
    ContractViolation,
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
    /// The spec/outcomes.json error category, the id `error:<category>` names.
    pub category: Option<String>,
    /// `Some("program")` when the registry marks the category `repair:
    /// program`; absent otherwise, as in the registry.
    pub repair: Option<&'static str>,
    pub word: Option<String>,
    /// The Word's semantic family as `spec/words.json` declares it, or
    /// `None` when no Core Word is at fault. The one classification of a
    /// Word the diagnosis reports is the registry's own.
    pub family: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugDiagnosis {
    pub when: ErrorPhase,
    pub where_: ErrorLocus,
    pub why: CauseClass,
    pub summary: String,
    pub evidence: Vec<String>,
    pub next_checks: Vec<DebugCheck>,
    /// Known Words within a small edit distance of an unrecognized name, best
    /// match first. "Check the spelling" without saying what the spelling
    /// might have been is the one repair hint an agent cannot act on, and the
    /// vocabulary needed to answer it is already compiled in.
    pub candidates: Vec<String>,
    /// Which declared ceiling a resource-limit failure crossed. `None` for
    /// every other cause class.
    pub resource_limit: Option<ResourceLimitFacts>,
}

impl ErrorPhase {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorPhase::Tokenize => "tokenize",
            ErrorPhase::ResolveWord => "resolveWord",
            ErrorPhase::ExecuteWord => "executeWord",
            ErrorPhase::CheckContract => "checkContract",
        }
    }
}

impl ErrorLocusKind {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorLocusKind::UserWord => "userWord",
            ErrorLocusKind::CoreWord => "coreWord",
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
            CauseClass::ShapeMismatch => "shapeMismatch",
            CauseClass::NilFlow => "nilFlow",
            CauseClass::UserLogic => "userLogic",
            CauseClass::ResourceLimit => "resourceLimit",
            CauseClass::SourceForm => "sourceForm",
            CauseClass::ContractViolation => "contractViolation",
            CauseClass::Unknown => "unknown",
        }
    }
}

impl CauseClass {
    pub fn from_error_category(category: &ErrorCategory) -> Self {
        match category {
            ErrorCategory::StackUnderflow => CauseClass::StackShape,
            ErrorCategory::UnknownWord => CauseClass::TypoOrUnknownName,
            ErrorCategory::MalformedSource => CauseClass::SourceForm,
            // LANG.MACHINE.LIMITS calls the step and recursion budgets host
            // safety controls rather than language semantics, and the two
            // answers differ: "the program is wrong" is fixed by rewriting it,
            // "the program is too big" by raising the budget or by finding a
            // cheaper shape for the same computation. Filing both under
            // `userLogic` sent every reader down the first road.
            ErrorCategory::ExecutionLimitExceeded => CauseClass::ResourceLimit,
            ErrorCategory::ResourceLimitExceeded => CauseClass::ResourceLimit,
            ErrorCategory::RecursionLimitExceeded => CauseClass::ResourceLimit,
            // A declaration the program made about itself that inference
            // disproved: a rule broken by the program, not by any value.
            ErrorCategory::ContractViolation => CauseClass::ContractViolation,
            // The registry named the condition at the raise site, so the class
            // follows from the spec's own vocabulary.
            ErrorCategory::Declared(condition) => {
                super::debug_declared_checks::cause_class_for_declared_condition(condition)
            }
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
        // A budget rather than a mistake: the materialization ceiling answers
        // to "the request is too big", not "the program is wrong" — the
        // distinction `ResourceLimit` exists for.
        NilReason::SpaceExhausted => CauseClass::ResourceLimit,
        NilReason::IndexOutOfBounds => CauseClass::Index,
        NilReason::NotFound | NilReason::InvalidEncoding => CauseClass::ValueShape,
        // Absence that no operation produced — a `NIL` in source, or one that
        // has passed through a dense lane, which carries presence but no
        // reason. Nothing is wrong; a NIL is simply flowing.
        NilReason::Literal => CauseClass::NilFlow,
        // The program said so itself: the cause is in its own logic.
        NilReason::UserDeclared => CauseClass::UserLogic,
    }
}

/// The locus as far as the compiled-in registry can tell: a Core Word, or not
/// known yet. A User Word is only knowable where the live dictionary is — see
/// `DebugDiagnosis::with_user_vocabulary`, which completes it there. This used
/// to recognise a User Word by a `DICT@NAME` prefix, which no name has carried
/// since the dictionary became two tiers; every failing User Word reported
/// `kind: unknown`.
pub(super) fn classify_locus(word: Option<&str>) -> ErrorLocus {
    let kind = match word {
        Some(name) if crate::coreword_registry::get_coreword_metadata(name).is_some() => {
            ErrorLocusKind::CoreWord
        }
        _ => ErrorLocusKind::Unknown,
    };
    ErrorLocus {
        kind,
        word: word.map(|s| s.to_string()),
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
            category.as_ref(),
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
        // Candidates first: the spelling check is written against them, and a
        // check that promises a list there is none is the failure this order
        // prevents. Only `unknownWord` puts the misspelled name in the locus;
        // `wordNotFound` is raised by `DEL` about its operand, and spelling
        // `DEL` against the vocabulary offered "DEF".
        let candidates = match (&why, word, category) {
            (CauseClass::TypoOrUnknownName, Some(name), Some(ErrorCategory::UnknownWord)) => {
                suggest_words(name, std::iter::empty())
            }
            _ => Vec::new(),
        };
        let next_checks = build_next_checks(
            &why,
            word,
            category,
            nil_reason,
            &candidates,
            stack_len_before,
        );

        DebugDiagnosis {
            when,
            where_,
            why,
            summary,
            evidence,
            next_checks,
            candidates,
            resource_limit: None,
        }
    }

    /// Build the AI-facing structured diagnostic payload. Human-readable
    /// `summary` stays separate; this payload exposes stable protocol fields
    /// so an agent can branch on the failure without matching display
    /// strings. It describes an ERROR: a NIL's reason, a truth value and an
    /// effect are observed on the stack and in the output, not here — the
    /// three fields that once carried them here were always null.
    ///
    /// It classifies and nothing more. `nextChecks`, `candidates` and
    /// `resourceLimit` are the diagnosis's, and used to be copied here too, so
    /// every error report carried them twice (three times, with the trace).
    pub fn ai_payload(&self, category: Option<&ErrorCategory>) -> AiDiagnosticPayload {
        let word = self.where_.word.as_deref();
        let category = category.map(ErrorCategory::as_protocol_str);
        AiDiagnosticPayload {
            category: category.map(str::to_string),
            repair: category.and_then(super::word_outcome_vocabulary::repair_for_category),
            word: self.where_.word.clone(),
            family: word
                .and_then(crate::kernel::generated::generated_word)
                .map(|w| w.family.as_spec_str().to_string()),
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
        // The step meter refuses on the step that crosses the budget, so
        // what it observed is the budget plus that one step. It used to
        // report `null` here, the one ceiling whose `observed` said nothing.
        AjisaiError::ExecutionLimitExceeded { limit } => Some(ResourceLimitFacts {
            resource: crate::error::ResourceLimit::ExecutionSteps
                .as_protocol_str()
                .to_string(),
            limit: *limit as u64,
            observed: Some(*limit as u64 + 1),
            progress: None,
        }),
        _ => None,
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
    // The outcome in the ids spec/outcomes.json and `outcomes` use, not the
    // engine's own type names: this line used to read
    // `ExecuteWord / DIV / Domain (divisionByZero) nil=DivisionByZero`, four
    // spellings for one fact, two of them Rust `Debug` output.
    let outcome = match (nil_reason, category) {
        (Some(reason), _) => format!("nil:{}", reason.as_protocol_str()),
        (None, Some(category)) => format!("error:{}", category.as_protocol_str()),
        (None, None) => "unknown".to_string(),
    };
    let msg_str = message
        .map(|m| format!(" msg=\"{}\"", m))
        .unwrap_or_default();
    format!(
        "{} / {} / {} ({}){}",
        when.as_protocol_str(),
        where_str,
        why.as_protocol_str(),
        outcome,
        msg_str
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

// The facts a diagnosis can only learn at the failure site.
//
// [`DebugDiagnosis`] is built from what the error itself carries: the phase,
// the Word, the cause class. Everything here is a fact the surrounding frame
// holds and the error does not — where in the source the token was written,
// how it was spelled, which Words it was written inside, what the live
// dictionary contains — recorded after the fact by the frame that has it.
//
// Each one writes into the same `key=value` evidence list rather than adding
// a protocol field, so a fact reaches every host that already renders a
// diagnosis, and a host that does not read the key is unaffected.
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
        let names: Vec<&str> = names.collect();
        let Some(word) = self.where_.word.clone() else {
            return;
        };
        // The locus a registry lookup could not name: a Word the live
        // dictionary holds is a User Word.
        if self.where_.kind == ErrorLocusKind::Unknown {
            let canonical = crate::word_name::canonical_word_name(&word);
            if names.iter().any(|name| *name == canonical.as_ref()) {
                self.where_.kind = ErrorLocusKind::UserWord;
            }
        }
        if !matches!(self.why, CauseClass::TypoOrUnknownName) {
            return;
        }
        // Only a locus that resolved to nothing is a misspelling to correct.
        // A Core or User Word in the locus is spelled right — it resolved —
        // and the unresolved name is inside it, or is its operand
        // (`wordNotFound` is `DEL`'s condition, and spelling `DEL` against
        // the vocabulary offered "DEF").
        self.candidates = if self.where_.kind == ErrorLocusKind::Unknown {
            suggest_words(&word, names.into_iter())
        } else {
            Vec::new()
        };
        // The spelling check names the candidates, so it has to be rebuilt
        // against the list that won: a user Word found here can turn an empty
        // list into a suggestion, and the check would otherwise still say
        // nothing was close.
        if let Some(existing) = self
            .next_checks
            .iter_mut()
            .find(|check| check.code == "checkSpelling")
        {
            *existing = spelling_check(&self.candidates);
        }
    }
}
