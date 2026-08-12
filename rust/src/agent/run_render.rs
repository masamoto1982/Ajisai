//! I/O-free report assembly for a completed execution.
//!
//! Factored out so the typed agent API and terminal CLI observe the same stack,
//! NIL flow, diagnostics, output and runtime metrics. This module adds no
//! behavior of its own.

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::interpreter::error_flow_trace::ErrorFlowEvent;
use crate::interpreter::Interpreter;

use super::report::{stack_json, Report};
use super::{error_report, stack_display, Opts};

/// Assemble the report for a completed execution without performing host I/O.
/// The typed agent API and the terminal renderer share this boundary.
pub(crate) fn completed_run_report(
    interp: &Interpreter,
    result: crate::error::Result<()>,
    trace: Vec<ErrorFlowEvent>,
    output: Vec<String>,
    opts: &Opts,
) -> Report {
    match result {
        Ok(()) => Report {
            status: "ok",
            stack: stack_json(interp),
            stack_display: stack_display(interp),
            output,
            message: None,
            diagnosis: None,
            ai_diagnostic: None,
            error_flow_trace: trace,
            runtime_metrics: interp.runtime_metrics(),
            contract_decls: None,
            stack_elided: None,
        },
        Err(err) => {
            let message = err.to_string();
            let stack_len = interp.get_stack().len();
            let diagnosis = trace
                .iter()
                .rev()
                .find_map(|event| event.diagnosis.clone())
                .unwrap_or_else(|| DebugDiagnosis::from_error(&err, None, stack_len, stack_len));
            let category = ErrorCategory::from_error(&err);
            error_report(
                interp,
                &diagnosis,
                Some(&category),
                message,
                output,
                trace,
                opts,
            )
        }
    }
}
