//! I/O-free report assembly for a completed execution.
//!
//! Factored out so the typed agent API and terminal CLI observe the same stack,
//! NIL flow, diagnostics, output and runtime metrics. This module adds no
//! behavior of its own.

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::interpreter::error_flow_trace::ErrorFlowEvent;
use crate::interpreter::upstream_nil_link::link_upstream_nil;
use crate::interpreter::Interpreter;

use super::execution_receipt::build_receipt;
use super::observation_digest::{observation_digest, ObservationDigestInput};
use super::report::{stack_json, Report};
use super::{error_report, stack_display, stack_values, user_word_identities};

/// Assemble the report for a completed execution without performing host I/O.
/// The typed agent API and the terminal renderer share this boundary.
///
/// `source` is the exact program text that was run — carried through only to
/// name it in the execution receipt (`Report::receipt`); nothing here
/// re-parses or re-executes it.
pub(crate) fn completed_run_report(
    interp: &Interpreter,
    result: crate::error::Result<()>,
    trace: Vec<ErrorFlowEvent>,
    output: Vec<String>,
    source: &str,
) -> Report {
    match result {
        Ok(()) => {
            let digest = observation_digest(ObservationDigestInput {
                status: "ok",
                stack: &stack_values(interp),
                output: &output,
                user_words: &user_word_identities(interp),
                error_category: None,
            });
            let resource_usage = interp.resource_usage();
            let receipt = build_receipt(
                source,
                interp.runtime_limits(),
                interp.max_execution_steps(),
                "ok",
                &resource_usage,
                digest.as_deref(),
            );
            Report {
                status: "ok",
                stack: stack_json(interp),
                stack_display: stack_display(interp),
                output,
                message: None,
                diagnosis: None,
                ai_diagnostic: None,
                error_flow_trace: trace,
                runtime_metrics: interp.runtime_metrics(),
                resource_usage,
                contract_decls: None,
                stack_elided: None,
                observation_digest: digest,
                receipt,
            }
        }
        Err(err) => {
            let message = err.to_string();
            let stack_len = interp.get_stack().len();
            let mut diagnosis = trace
                .iter()
                .rev()
                .find_map(|event| event.diagnosis.clone())
                .unwrap_or_else(|| DebugDiagnosis::from_error(&err, None, stack_len, stack_len));
            // A NIL that flowed downstream fails at the Word that *received* it,
            // so the top-level diagnosis names that Word and not the cause. Give
            // the top level a link back to the producing node rather than
            // leaving the cause reachable only by walking `errorFlowTrace`.
            link_upstream_nil(&mut diagnosis, &trace);
            let category = ErrorCategory::from_error(&err);
            error_report(
                interp,
                &diagnosis,
                Some(&category),
                message,
                output,
                trace,
                Some(source),
            )
        }
    }
}
