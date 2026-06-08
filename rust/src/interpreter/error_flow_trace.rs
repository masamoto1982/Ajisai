use super::debug_diagnosis::{AiDiagnosticPayload, DebugDiagnosis};
use crate::error::ErrorCategory;
use crate::semantic::AbsenceMetadata;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorFlowEventKind {
    WordError,
    NilProduced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorFlowEvent {
    pub kind: ErrorFlowEventKind,
    pub word: Option<String>,
    pub error_category: Option<ErrorCategory>,
    pub absence: Option<AbsenceMetadata>,
    pub stack_len_before: usize,
    pub stack_len_after: usize,
    pub message: String,
    pub diagnosis: Option<DebugDiagnosis>,
}

impl ErrorFlowEvent {
    pub fn ai_diagnostic_payload(&self) -> Option<AiDiagnosticPayload> {
        self.diagnosis.as_ref().map(|diagnosis| {
            diagnosis.ai_payload(
                self.error_category.as_ref(),
                self.absence
                    .as_ref()
                    .and_then(|absence| absence.reason.as_ref()),
                None,
                None,
            )
        })
    }
}

impl ErrorFlowEventKind {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorFlowEventKind::WordError => "wordError",
            ErrorFlowEventKind::NilProduced => "nilProduced",
        }
    }
}
