//! A `nilProduced` diagnosis built on first read is the one the eager route
//! built at the moment of production, whatever happened in between.

use super::{EventDiagnosis, NilProduction};
use crate::error::NilReason;
use crate::interpreter::debug_diagnosis::{DebugDiagnosis, ErrorPhase, ResourceLimitFacts};
use crate::interpreter::Interpreter;

const REASONS: [NilReason; 5] = [
    NilReason::NotFound,
    NilReason::InvalidEncoding,
    NilReason::IndexOutOfBounds,
    NilReason::DomainMiss,
    NilReason::UserDeclared,
];

/// The diagnosis exactly as `record_nil_produced` used to build it, and as
/// `enclose_nil_productions_since` then extended it.
fn eager(
    word: &str,
    reason: &NilReason,
    vocabulary: &[&str],
    resource_limit: Option<ResourceLimitFacts>,
    enclosing: &[&str],
) -> DebugDiagnosis {
    let message = format!("NIL produced by {word} reason={}", reason.as_protocol_str());
    let mut diagnosis = DebugDiagnosis::from_error_category(
        ErrorPhase::ExecuteWord,
        Some(word),
        None,
        Some(reason),
        2,
        1,
        Some(message),
    );
    diagnosis.with_user_vocabulary(vocabulary.iter().copied());
    diagnosis.resource_limit = resource_limit;
    for frame in enclosing {
        diagnosis.with_enclosing_word(frame);
    }
    diagnosis
}

fn deferred(
    word: &str,
    reason: &NilReason,
    vocabulary: &[&str],
    resource_limit: Option<ResourceLimitFacts>,
) -> EventDiagnosis {
    EventDiagnosis::nil_produced(NilProduction {
        word: word.to_string(),
        reason: *reason,
        stack_len_before: 2,
        stack_len_after: 1,
        message: format!("NIL produced by {word} reason={}", reason.as_protocol_str()),
        user_word: NilProduction::user_word_of(word, |name| vocabulary.contains(&name)),
        resource_limit,
        enclosing: Vec::new(),
    })
}

#[test]
fn a_deferred_diagnosis_is_the_eager_one() {
    let limit = ResourceLimitFacts {
        resource: "materializedElements".to_string(),
        limit: 10,
        observed: Some(11),
        progress: None,
    };
    // A Core Word, a User Word (spelled in another case than it was defined
    // in), and a name the dictionary does not hold.
    let words = ["NUM", "my-word", "ZZ"];
    let vocabulary = ["MY-WORD", "OTHER"];
    for word in words {
        for reason in &REASONS {
            for resource_limit in [None, Some(limit.clone())] {
                for enclosing in [&[][..], &["MAP"][..], &["MAP", "Q"][..]] {
                    let expected =
                        eager(word, reason, &vocabulary, resource_limit.clone(), enclosing);
                    // Enclosed before the first read.
                    let mut lazy = deferred(word, reason, &vocabulary, resource_limit.clone());
                    for frame in enclosing {
                        lazy.with_enclosing_word(frame);
                    }
                    assert_eq!(
                        lazy.to_diagnosis(),
                        expected,
                        "{word} {reason:?} {enclosing:?}"
                    );
                    // Read first, enclosed after.
                    let mut read = deferred(word, reason, &vocabulary, resource_limit.clone());
                    let _ = read.why.clone();
                    for frame in enclosing {
                        read.with_enclosing_word(frame);
                    }
                    assert_eq!(
                        read.to_diagnosis(),
                        expected,
                        "{word} {reason:?} {enclosing:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_user_word_case_is_reached() {
    // The case above would pass vacuously if no reason classified a User
    // Word's locus as one.
    let lazy = deferred("my-word", &NilReason::NotFound, &["MY-WORD"], None);
    assert_eq!(lazy.where_.kind.as_protocol_str(), "userWord");
    let core = deferred("NUM", &NilReason::NotFound, &["MY-WORD"], None);
    assert_eq!(core.where_.kind.as_protocol_str(), "coreWord");
}

/// The dictionary is read when the NIL is produced, not when the trace is: a
/// definition made before the trace is read does not change its record.
#[tokio::test]
async fn the_dictionary_is_read_when_the_nil_is_produced() {
    let mut interp = Interpreter::new();
    interp
        .execute("0 2 RANGE [ 'X' BIND 'x' NUM ] MAP")
        .await
        .expect("computes");
    let before: Vec<_> = interp
        .error_flow_trace_log
        .iter()
        .map(|event| event.diagnosis.as_ref().map(|d| d.to_diagnosis()))
        .collect();
    assert!(!before.is_empty());

    let mut later = Interpreter::new();
    later
        .execute("0 2 RANGE [ 'X' BIND 'x' NUM ] MAP")
        .await
        .expect("computes");
    later
        .execute("[ 1 ] 'NUMX' DEF")
        .await
        .expect("a later definition computes");
    let after: Vec<_> = later
        .error_flow_trace_log
        .iter()
        .take(before.len())
        .map(|event| event.diagnosis.as_ref().map(|d| d.to_diagnosis()))
        .collect();
    assert_eq!(before, after);
}
