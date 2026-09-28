//! The NIL a generative Word projects when its result will not fit.
//!
//! Separate from `runtime_limits`, which decides *whether* a ceiling was
//! crossed and raises when it was. This is the other answer to the same
//! question: a well-formed operation whose result cannot be materialized
//! within budget is projected onto a diagnosable NIL under the NIL Projection
//! Rule (LANG.FAILURE.PROJECT) rather than raised, because the program is not wrong —
//! it is too big, and a pipeline can recover it with a chosen fallback.

use crate::error::{ErrorCategory, NilReason, ResourceLimit};
use crate::interpreter::debug_diagnosis::{DebugDiagnosis, ErrorPhase, ResourceLimitFacts};
use crate::semantic::{AbsenceMetadata, AbsenceOrigin, Recoverability};
use crate::types::Value;

/// The NIL a generative Word projects when its result will not fit, carrying
/// the ceiling that refused it.
///
/// `absence.reason = spaceExhausted` says *that* a ceiling fired. It does not
/// say which one, what it is set to, or how much would have fitted — and those
/// three are what a caller needs in order to retry. `tools/mcp-server/README.md`
/// promises `diagnosis.resourceLimit` (`{ resource, limit, observed }`) for a
/// resource-limit failure, naming "the very entry in `mcp.limits` that fired";
/// a projection is one, and it now carries the same facts a raise does.
///
/// `progress` is `None` on purpose. It exists for a *cumulative* meter, which
/// stops the instant the budget is crossed and so cannot say how far over the
/// request was. This is a size ceiling: `observed` is the whole requested
/// count, measured before anything was built, and `limit` is what fits.
pub(crate) fn space_exhausted_nil(word: &str, limit: usize, observed: Option<u128>) -> Value {
    let message = match observed {
        Some(count) => format!(
            "{} would materialize {} elements; materializedElements is {}",
            word, count, limit
        ),
        // A shape whose element product overflows `usize` has no count to
        // report: the size is past what the machine can express, let alone
        // allocate. The ceiling and its name still are.
        None => format!(
            "{} names a shape whose element count overflows; materializedElements is {}",
            word, limit
        ),
    };
    exhausted_nil(
        word,
        ResourceLimit::MaterializedElements,
        limit,
        observed.and_then(|count| u64::try_from(count).ok()),
        message,
    )
}

/// The same projection for a result that would nest deeper than the nesting
/// ceiling (LANG.MACHINE.LIMITS): a shape of too many axes, or JSON text
/// nested too deep. Depth is a dimension of the size of what a generative
/// Word was asked to build, so it declines the same way a count does; a value
/// that grows too deep through Words that build nothing of their own size is
/// refused with an ERROR instead (`Interpreter::check_fresh_nesting`).
pub(crate) fn nesting_exhausted_nil(word: &str, limit: usize, observed: usize) -> Value {
    exhausted_nil(
        word,
        ResourceLimit::NestingDepth,
        limit,
        Some(observed as u64),
        format!(
            "{} would build a value nested {} deep; nestingDepth is {}",
            word, observed, limit
        ),
    )
}

/// The same projection for text that spells a number too large to build: more
/// digits, counting the exponent's magnitude, than the numeric-literal ceiling
/// allows a source literal (`tokenizer::denoted_digit_count`). A Word that reads
/// the numeric grammar out of data declines it, where the same spelling in
/// source is refused before the program runs.
pub(crate) fn numeric_literal_exhausted_nil(word: &str, limit: usize, observed: u64) -> Value {
    exhausted_nil(
        word,
        ResourceLimit::NumericLiteralDigits,
        limit,
        Some(observed),
        format!(
            "{} would build a number of {} digits; numericLiteralDigits is {}",
            word, observed, limit
        ),
    )
}

fn exhausted_nil(
    word: &str,
    resource: ResourceLimit,
    limit: usize,
    observed: Option<u64>,
    message: String,
) -> Value {
    let mut diagnosis = DebugDiagnosis::from_error_category(
        ErrorPhase::ExecuteWord,
        Some(word),
        Some(&ErrorCategory::ResourceLimitExceeded),
        Some(&NilReason::SpaceExhausted),
        0,
        0,
        Some(message),
    );
    diagnosis.resource_limit = Some(ResourceLimitFacts {
        resource: resource.as_protocol_str().to_string(),
        limit: limit as u64,
        observed,
        progress: None,
    });
    // Minted through the one constructor every projected absence goes through,
    // so the ceiling's NIL is counted as produced like any other; only the
    // diagnosis is this projection's own.
    let mut absence = AbsenceMetadata::with_reason(
        NilReason::SpaceExhausted,
        AbsenceOrigin::SpaceBudget,
        Recoverability::Unknown,
    );
    absence.diagnosis = Some(Box::new(diagnosis));
    Value::nil_with_absence(absence)
}
