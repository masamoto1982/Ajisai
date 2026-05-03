use super::debug_diagnosis::DebugDiagnosis;
use crate::error::{ErrorCategory, NilReason};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorFlowEventKind {
    WordError,
    SafeEnter,
    SafeSuccess,
    SafeCaught,
    NilProduced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorFlowEvent {
    pub kind: ErrorFlowEventKind,
    pub word: Option<String>,
    pub error_category: Option<ErrorCategory>,
    pub nil_reason: Option<NilReason>,
    pub stack_len_before: usize,
    pub stack_len_after: usize,
    pub message: String,
    pub diagnosis: Option<DebugDiagnosis>,
}
