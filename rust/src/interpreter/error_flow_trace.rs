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
    /// The raised error as it renders, for a `WordError`; empty otherwise.
    ///
    /// An error unwinds through every frame that reached it, and each frame
    /// used to record it again under its own name. Comparing what the frame is
    /// about to record against what the frame below already did is how the
    /// outer frames are recognised as *enclosing* rather than *failing* — see
    /// `Interpreter::attribute_enclosing_word`. Matching on the rendered
    /// `message` instead would be matching on prose that already embeds a word
    /// name, so the error text is kept as its own field.
    pub error_text: String,
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
