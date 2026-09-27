use super::{AbsenceOrigin, Recoverability};

impl AbsenceOrigin {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            AbsenceOrigin::Literal => "literal",
            AbsenceOrigin::DivisionByZero => "divisionByZero",
            AbsenceOrigin::NilPropagation => "nilPropagation",
            AbsenceOrigin::NotFound => "notFound",
            AbsenceOrigin::InvalidEncoding => "invalidEncoding",
            AbsenceOrigin::IndexOutOfBounds => "indexOutOfBounds",
            AbsenceOrigin::SpaceBudget => "spaceBudget",
            AbsenceOrigin::DomainMiss => "domainMiss",
            AbsenceOrigin::HostEnvironment => "hostEnvironment",
            AbsenceOrigin::UserDeclared => "userDeclared",
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
