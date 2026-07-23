//! Rendering of a completed execution, shared by `run` and `build`.
//!
//! Factored out of `mod.rs` so a single-file `run` and a project `build`
//! (`project.rs`) render identically: the same JSON/text envelope, the same NIL
//! explanation on a successful bubble, and the same missing-capability
//! diagnosis path on failure. This module adds no behavior of its own — it only
//! assembles the existing `Report` and delegates to the shared `emit`.

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::interpreter::error_flow_trace::ErrorFlowEvent;
use crate::interpreter::Interpreter;

use super::report::{stack_json, Report};
use super::{
    emit, error_report, missing_capability_diagnosis, nil_explanation, stack_display, Opts,
};

/// Emit the report for a completed execution and return the process exit code.
/// `receipt` is prebuilt by the caller (only `run --receipt` on a successful
/// run supplies one).
pub(crate) fn render_completed_run(
    interp: &Interpreter,
    result: crate::error::Result<()>,
    trace: Vec<ErrorFlowEvent>,
    output: Vec<String>,
    receipt: Option<serde_json::Value>,
    opts: &Opts,
) -> i32 {
    match result {
        Ok(()) => {
            let explanation = nil_explanation(&trace, opts);
            let report = Report {
                status: "ok",
                stack: stack_json(interp),
                stack_display: stack_display(interp),
                output,
                message: None,
                diagnosis: None,
                ai_diagnostic: None,
                error_flow_trace: trace,
                runtime_metrics: interp.runtime_metrics(),
                explanation,
                plan_check: None,
                contract_decls: None,
                receipt,
                lang: opts.lang,
            };
            emit(&report, opts);
            0
        }
        Err(err) => {
            let message = err.to_string();
            let stack_len = interp.get_stack().len();
            let diagnosis = missing_capability_diagnosis(interp, &message)
                .or_else(|| trace.iter().rev().find_map(|event| event.diagnosis.clone()))
                .unwrap_or_else(|| DebugDiagnosis::from_error(&err, None, stack_len, stack_len));
            let category = ErrorCategory::from_error(&err);
            emit(
                &error_report(
                    interp,
                    &diagnosis,
                    Some(&category),
                    message,
                    output,
                    trace,
                    opts,
                ),
                opts,
            );
            1
        }
    }
}
