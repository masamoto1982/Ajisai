use super::{AbsenceOrigin, Capability, Recoverability, SemanticKind, ValueOrigin, ValueShape};

impl SemanticKind {
    pub fn as_protocol_str(self) -> &'static str {
        match self {
            SemanticKind::Number => "number",
            SemanticKind::Collection => "collection",
            SemanticKind::Code => "code",
            SemanticKind::Absence => "absence",
        }
    }
}

impl ValueShape {
    pub fn as_protocol_str(self) -> &'static str {
        match self {
            ValueShape::Scalar => "scalar",
            ValueShape::Vector => "vector",
            ValueShape::Tensor => "tensor",
            ValueShape::CodeBlock => "codeBlock",
            ValueShape::Absence => "absence",
        }
    }
}

impl Capability {
    pub fn as_protocol_str(self) -> &'static str {
        match self {
            Capability::Numeric => "numeric",
            Capability::ExactNumeric => "exactNumeric",
            Capability::Iterable => "iterable",
            Capability::Indexable => "indexable",
            Capability::Callable => "callable",
            Capability::StackItem => "stackItem",
            Capability::NilPassthrough => "nilPassthrough",
            Capability::Diagnosable => "diagnosable",
            Capability::Serializable => "serializable",
            Capability::Displayable => "displayable",
            Capability::UserEditable => "userEditable",
            Capability::AiExplainable => "aiExplainable",
            Capability::TruthValued => "truthValued",
        }
    }
}

impl ValueOrigin {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ValueOrigin::Literal => "literal",
            ValueOrigin::NilPropagation => "nilPropagation",
            ValueOrigin::HostEnvironment => "hostEnvironment",
            ValueOrigin::Unknown => "unknown",
        }
    }
}

impl AbsenceOrigin {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            AbsenceOrigin::Literal => "literal",
            AbsenceOrigin::DivisionByZero => "divisionByZero",
            AbsenceOrigin::NilPropagation => "nilPropagation",
            AbsenceOrigin::EmptySequence => "emptySequence",
            AbsenceOrigin::MissingField => "missingField",
            AbsenceOrigin::InvalidEncoding => "invalidEncoding",
            AbsenceOrigin::InvalidLens => "invalidLens",
            AbsenceOrigin::StackUnderflow => "stackUnderflow",
            AbsenceOrigin::IndexOutOfBounds => "indexOutOfBounds",
            AbsenceOrigin::UnknownWord => "unknownWord",
            AbsenceOrigin::ExecutionFailure => "executionFailure",
            AbsenceOrigin::ComparisonBudget => "comparisonBudget",
            AbsenceOrigin::SpaceBudget => "spaceBudget",
            AbsenceOrigin::DomainMiss => "domainMiss",
            AbsenceOrigin::NotAvailable => "notAvailable",
            AbsenceOrigin::HostEnvironment => "hostEnvironment",
            AbsenceOrigin::Unknown => "unknown",
        }
    }
}

impl Recoverability {
    pub fn as_protocol_str(self) -> &'static str {
        match self {
            Recoverability::Recoverable => "recoverable",
            Recoverability::Retryable => "retryable",
            Recoverability::Fatal => "fatal",
            Recoverability::Unknown => "unknown",
        }
    }
}
