use std::fmt;

pub type Result<T> = std::result::Result<T, AjisaiError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NilReason {
    DivisionByZero,
    NotFound,
    InvalidEncoding,
    IndexOutOfBounds,
    // `LogicallyUnknown` was retired: no `NilReason` value represents the
    // logical truth value UNKNOWN (LANG.VALUES.TRUTH): UNKNOWN is a NIL read
    // in truth position, carrying whatever reason that NIL has.
    /// A well-formed generative operation (`RANGE`, `FILL`) whose materialized
    /// result would exceed the space water level (`max_materialized_elements`).
    /// The NIL Projection Rule projects this to NIL with `absence.origin = spaceBudget`
    /// (LANG.FAILURE.PROJECT) rather than aborting the process, so a pipeline can
    /// recover it with a chosen fallback. Malformed inputs (an infinite `RANGE`, a
    /// non-conforming `RESHAPE`) remain ordinary errors.
    SpaceExhausted,
    /// A well-formed operation applied to an input outside its domain — the
    /// canonical case being `SQRT` of a negative rational, which
    /// LANG.FAILURE.PROJECT calls a "well-formed domain miss". The NIL
    /// Projection Rule projects it to NIL with `absence.origin = domainMiss`
    /// (LANG.FAILURE.PROJECT).
    ///
    /// Deliberately named for the classification, not for the operation: a
    /// domain miss is recoverable by supplying a different input, which is what
    /// distinguishes it from an execution failure and what makes the same
    /// variant right for future domain misses in other words.
    DomainMiss,
    /// A NIL the program *wrote* rather than computed — the `NIL` Word and the
    /// `NIL` symbol inside a vector literal.
    ///
    /// `LANG.VALUES.NIL` makes the reason a NIL's entire observable content, so
    /// a written absence needs one too; without it `NIL NIL-REASON` answered
    /// NIL and the value had nothing to observe (audit finding D24). It names
    /// the only thing true of it: nothing failed, it was written down.
    Literal,
    /// An absence the program declared itself with `ABSENT`. The reason the
    /// registry closes over is this one id; the text the program gave lives
    /// beside it as the value's `detail` (`AbsenceMetadata::detail`), is what
    /// `NIL-REASON` answers, and is part of the value (LANG.VALUES.NIL) — the
    /// first parameterized reason, recorded as such in `spec/outcomes.json`.
    UserDeclared,
}

/// One named internal-computation ceiling from
/// [`crate::interpreter::RuntimeLimits`].
///
/// A host declares each ceiling separately — the MCP adapter publishes all of
/// them in `mcp.limits` — so an agent that hits one has to be able to tell
/// *which* one fired without reading an English sentence. The protocol
/// spelling is deliberately the same identifier the host profile publishes,
/// so `resourceLimit.resource` indexes straight into the declared limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLimit {
    /// Byte length of one source program (`max_source_bytes`).
    SourceBytes,
    /// Digit count of one numeric literal (`max_numeric_literal_digits`).
    NumericLiteralDigits,
    /// Accumulated internal numeric work units (`max_numeric_work`).
    NumericWork,
    /// Accumulated collection work units (`max_collection_work`) — the element
    /// copies, comparisons and equality probes a collection Word performs
    /// inside a single execution step.
    ///
    /// Separate from `NumericWork` because the two count different things and
    /// prescribe different fixes. `numericWork` counts limb multiplies and says
    /// "compute less"; this counts element operations and says "work over a
    /// smaller collection". Folding them into one number would leave an agent
    /// unable to tell a wide arithmetic chain from a large `UNIQUE`, and would
    /// send it to shrink the wrong thing.
    CollectionWork,
    /// Bit length of a BigInt arithmetic result (`max_bigint_bits`).
    BigintBits,
    /// Algebraic term count of one exact value (`max_algebraic_terms`).
    AlgebraicTerms,
    /// Execution-step budget (`Interpreter::max_execution_steps`). Kept in the
    /// same vocabulary even though it lives outside `RuntimeLimits`, because a
    /// host publishes it as one more entry in the same limit table.
    ExecutionSteps,
    /// Materialized element count of one generated collection
    /// (`max_materialized_elements`).
    ///
    /// The only ceiling in this vocabulary that is never *raised*: crossing it
    /// is a well-formed operation that cannot produce a value within budget, so
    /// the NIL Projection Rule projects it (LANG.FAILURE.PROJECT). It is named here all
    /// the same, because a projection and a raise refuse for the same kind of
    /// reason and a caller plans against the same published entry — and
    /// without a name the projection could only say *that* a ceiling fired,
    /// never which one or how much would have fitted.
    MaterializedElements,
}

/// How far an incrementally charged operation had got when its budget ran out.
///
/// For a *size* ceiling `observed` is the whole story: `bigintBits` observing
/// 272,133 against a limit of 262,144 is a real measurement of a real value,
/// visibly 4% over. A *cumulative work* ceiling charged as it goes cannot say
/// that — it stops the instant the budget is crossed, so `observed` reads a
/// hair over `limit` whether the caller asked for one element too many or
/// fifteen times too many.
///
/// Not a harmless imprecision: measured against a real model (`eval/traces/`),
/// reading it proportionally is exactly what happens. Refused at 0.02% over
/// while deduplicating 100,000 integers, the model retried with 99,999 and then
/// 99,979 and failed both times; it needed a 94% cut and nothing in the refusal
/// pointed there. Reporting where the scan stopped is the answer rather than a
/// hint at it — the budget bought exactly `completed` elements of this data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ResourceProgress {
    /// Units of `unit` fully processed before the budget ran out.
    pub completed: u64,
    /// Units of `unit` the operation was asked for.
    pub total: u64,
    /// What is being counted. `"elements"` for the collection scans, which are
    /// the only operations charged incrementally today.
    pub unit: &'static str,
}

/// The unit the collection scans count in.
pub const PROGRESS_UNIT_ELEMENTS: &str = "elements";

impl ResourceLimit {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ResourceLimit::SourceBytes => "sourceBytes",
            ResourceLimit::NumericLiteralDigits => "numericLiteralDigits",
            ResourceLimit::NumericWork => "numericWork",
            ResourceLimit::CollectionWork => "collectionWork",
            ResourceLimit::BigintBits => "bigintBits",
            ResourceLimit::AlgebraicTerms => "algebraicTerms",
            ResourceLimit::ExecutionSteps => "executionSteps",
            ResourceLimit::MaterializedElements => "materializedElements",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    StackUnderflow,
    UnknownWord,
    DivisionByZero,
    MalformedSource,
    ExecutionLimitExceeded,
    /// A named `RuntimeLimits` ceiling other than the step budget. Separate
    /// from `ExecutionLimitExceeded` so "the program never terminated" and
    /// "one value grew past the declared size ceiling" stop sharing an answer.
    ResourceLimitExceeded,
    RecursionLimitExceeded,
    /// The condition the failing Word's `errorWhen` declares for this state.
    /// Its protocol spelling *is* the declared condition name, so a reader who
    /// asked `word_contract` for the Word gets back the same vocabulary the
    /// failure answers in.
    Declared(&'static str),
}

impl ErrorCategory {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorCategory::StackUnderflow => "stackUnderflow",
            ErrorCategory::UnknownWord => "unknownWord",
            ErrorCategory::DivisionByZero => "divisionByZero",
            ErrorCategory::MalformedSource => "malformedSource",
            ErrorCategory::ExecutionLimitExceeded => "executionLimitExceeded",
            ErrorCategory::ResourceLimitExceeded => "resourceLimitExceeded",
            ErrorCategory::RecursionLimitExceeded => "recursionLimitExceeded",
            ErrorCategory::Declared(condition) => condition,
        }
    }

    pub fn from_error(err: &AjisaiError) -> Self {
        match err {
            AjisaiError::StackUnderflow { .. } => ErrorCategory::StackUnderflow,
            AjisaiError::UnknownWord(_) => ErrorCategory::UnknownWord,
            AjisaiError::DivisionByZero => ErrorCategory::DivisionByZero,
            AjisaiError::MalformedSource(_) => ErrorCategory::MalformedSource,
            AjisaiError::ExecutionLimitExceeded { .. } => ErrorCategory::ExecutionLimitExceeded,
            AjisaiError::ResourceLimitExceeded { .. } => ErrorCategory::ResourceLimitExceeded,
            AjisaiError::RecursionLimitExceeded { .. } => ErrorCategory::RecursionLimitExceeded,
            AjisaiError::DeclaredCondition { condition, .. } => ErrorCategory::Declared(condition),
        }
    }
}

impl NilReason {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            NilReason::DivisionByZero => "divisionByZero",
            NilReason::NotFound => "notFound",
            NilReason::InvalidEncoding => "invalidEncoding",
            NilReason::IndexOutOfBounds => "indexOutOfBounds",
            NilReason::SpaceExhausted => "spaceExhausted",
            NilReason::DomainMiss => "domainMiss",
            NilReason::Literal => "literal",
            NilReason::UserDeclared => "userDeclared",
        }
    }

    /// Every `NilReason`, so a boundary that must round-trip one can search
    /// this list instead of restating the mapping. `as_protocol_str` stays the
    /// single spelling authority: a new reason is added here and named there,
    /// and `from_protocol_str` follows without another table to update.
    pub const ALL: &'static [NilReason] = &[
        NilReason::DivisionByZero,
        NilReason::NotFound,
        NilReason::InvalidEncoding,
        NilReason::IndexOutOfBounds,
        NilReason::SpaceExhausted,
        NilReason::DomainMiss,
        NilReason::Literal,
        NilReason::UserDeclared,
    ];

    /// The reason a protocol string names, or `None` when it names none.
    ///
    /// LANG.VALUES.NIL makes the reason the entire observable content of a
    /// NIL, so a boundary that persists a value and reads it back has to carry
    /// the reason across or it changes the value. The persistence codec and
    /// the value arena both decode through here.
    pub fn from_protocol_str(s: &str) -> Option<NilReason> {
        NilReason::ALL
            .iter()
            .find(|reason| reason.as_protocol_str() == s)
            .copied()
    }
}

#[derive(Debug, Clone)]
pub enum AjisaiError {
    /// A Word was called with fewer operands than its declared arity. `word`
    /// is filled in at dispatch (`attributed_to`), like a declared condition's,
    /// so the message names the Word that was short.
    StackUnderflow {
        word: Option<&'static str>,
    },
    UnknownWord(String),
    DivisionByZero,
    /// Program text that does not parse: an unclosed or crossed delimiter, a
    /// delimiter glued to a name, an unclosed string. The fault is in the
    /// writing, not in any value, so it belongs to neither the value-shape nor
    /// the user-logic families.
    MalformedSource(String),
    ExecutionLimitExceeded {
        limit: usize,
    },
    /// A named internal-computation ceiling was crossed: one value grew past
    /// `max_bigint_bits` / `max_algebraic_terms`, a literal past
    /// `max_numeric_literal_digits`, a program past `max_source_bytes`, or the
    /// accumulated work meter past `max_numeric_work`.
    ///
    /// Carries the ceiling's own name, its configured value and — when the
    /// guard measured one — the observed size, so a host can report which of
    /// the limits it declared actually fired and by how much it was missed.
    ResourceLimitExceeded {
        resource: ResourceLimit,
        limit: u64,
        observed: Option<u64>,
        /// How far the refused operation had got, when the operation is one
        /// that is charged as it goes. `None` everywhere else — see
        /// [`ResourceProgress`] for why the distinction is the whole point.
        progress: Option<ResourceProgress>,
    },
    /// Native call-depth guard (LANG.DICTIONARY.ACYCLIC): `word` reached `limit` nested
    /// activations — a pathologically long acyclic call chain, since the
    /// DEF-time acyclicity check (LANG.DICTIONARY.ACYCLIC) rules out recursion.
    RecursionLimitExceeded {
        limit: usize,
        word: String,
    },
    /// A raise the failing Word's own registry entry already names: `condition`
    /// is one of the conditions its `errorWhen` declares, spelled the way
    /// `spec/words.json` spells it.
    ///
    /// Naming the condition at the raise site is what lets the classification
    /// be derived rather than guessed. Every raise site names one — there is
    /// no catch-all variant left to fall back on
    /// (docs/dev/outcome-space-bijection-work-order-2026-09.md Phase 2): an
    /// undeclared outcome is a compile error, not a `custom`/`unknown` at
    /// runtime.
    DeclaredCondition {
        condition: &'static str,
        message: String,
        /// The Word the failure belongs to, attached once by the dispatcher
        /// as the error leaves that Word (`execute_generated_word`). The
        /// message itself never spells the Word's name: every declared
        /// message is written "expected …, got …", and `Display` prefixes the
        /// name, so every one reads `WORD: expected …` and no raise site can
        /// forget, misspell, or borrow another Word's name.
        word: Option<&'static str>,
    },
}

impl AjisaiError {
    /// Raise the named condition from the failing Word's `errorWhen`.
    ///
    /// `condition` has to be a condition that Word declares — the diagnosis
    /// reads the declaration back and says which of them fired, and
    /// `a_named_condition_is_one_the_word_declares`
    /// (`interpreter::declared_condition_tests`) pins a sample of call sites
    /// here against `spec/words.json`. That test enumerates call sites by
    /// hand rather than scanning the source, so it only catches a condition
    /// this file names for a program its own list exercises — not every call
    /// site automatically.
    /// A stack short of operands, not yet attributed to a Word.
    pub const fn stack_underflow() -> Self {
        AjisaiError::StackUnderflow { word: None }
    }

    pub fn declared(condition: &'static str, message: impl Into<String>) -> Self {
        AjisaiError::DeclaredCondition {
            condition,
            message: message.into(),
            word: None,
        }
    }

    /// Two operands whose shapes do not align on `axis` (LANG.COLLECTIONS.LIFT).
    /// One condition for every alignment a Word performs — lifting, pairing
    /// keys with values, zipping rows — so a length that differs is
    /// `shapeMismatch` whichever Word noticed it.
    pub fn shape_mismatch(left: &[usize], right: &[usize], axis: usize) -> Self {
        AjisaiError::declared(
            "shapeMismatch",
            format!(
                "expected shapes that align, got {:?} and {:?} (axis {} is {} and {}, and neither is 1)",
                left,
                right,
                axis,
                left.get(axis).copied().unwrap_or(1),
                right.get(axis).copied().unwrap_or(1)
            ),
        )
    }

    /// Two sequences that must pair position by position and do not.
    pub fn length_mismatch(left: usize, right: usize) -> Self {
        AjisaiError::declared(
            "shapeMismatch",
            format!(
                "expected Vectors of the same length, got {} and {}",
                left, right
            ),
        )
    }

    /// Attach the Word a declared failure belongs to, if none is attached
    /// yet. The innermost Word wins: an error raised by `ADD` inside a `MAP`
    /// block is `ADD`'s, and `MAP` passing it on does not relabel it.
    ///
    /// `declaredFailure` is the program's own text (`FAIL`), so it is left as
    /// the program wrote it.
    pub fn attributed_to(self, name: &'static str) -> Self {
        match self {
            AjisaiError::DeclaredCondition {
                condition,
                message,
                word: None,
            } if condition != "declaredFailure" => AjisaiError::DeclaredCondition {
                condition,
                message,
                word: Some(name),
            },
            AjisaiError::StackUnderflow { word: None } => {
                AjisaiError::StackUnderflow { word: Some(name) }
            }
            other => other,
        }
    }
}

impl fmt::Display for AjisaiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AjisaiError::StackUnderflow { word: Some(word) } => {
                write!(f, "{}: stack underflow", word)
            }
            AjisaiError::StackUnderflow { word: None } => write!(f, "stack underflow"),
            AjisaiError::UnknownWord(name) => write!(f, "Unknown word: {}", name),
            AjisaiError::DivisionByZero => write!(f, "Division by zero"),
            AjisaiError::MalformedSource(msg) => write!(f, "{}", msg),
            AjisaiError::ExecutionLimitExceeded { limit } => {
                write!(f, "Execution step limit ({}) exceeded", limit)
            }
            AjisaiError::ResourceLimitExceeded {
                resource,
                limit,
                observed,
                progress,
            } => match observed {
                // The progress clause carries the repair. Without it the
                // sentence reads as a near miss on every incrementally charged
                // ceiling, because that is the only thing `observed` can say
                // there — see `ResourceProgress`.
                Some(observed) => match progress {
                    Some(progress) => write!(
                        f,
                        "{} of {} exceeds the limit of {} after {} of {} {}",
                        resource.as_protocol_str(),
                        observed,
                        limit,
                        progress.completed,
                        progress.total,
                        progress.unit
                    ),
                    None => write!(
                        f,
                        "{} of {} exceeds the limit of {}",
                        resource.as_protocol_str(),
                        observed,
                        limit
                    ),
                },
                None => write!(
                    f,
                    "{} limit ({}) exceeded",
                    resource.as_protocol_str(),
                    limit
                ),
            },
            AjisaiError::RecursionLimitExceeded { limit, word } => {
                write!(f, "recursion limit exceeded ({}) in '{}'", limit, word)
            }
            AjisaiError::DeclaredCondition {
                message,
                word: Some(word),
                ..
            } => write!(f, "{}: {}", word, message),
            AjisaiError::DeclaredCondition { message, .. } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for AjisaiError {}
