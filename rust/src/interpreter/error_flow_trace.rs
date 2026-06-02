use super::debug_diagnosis::DebugDiagnosis;
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

impl ErrorFlowEventKind {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorFlowEventKind::WordError => "wordError",
            ErrorFlowEventKind::NilProduced => "nilProduced",
        }
    }
}
