pub mod absence;
pub mod capability;
pub mod protocol;
pub mod value_axes;

pub use absence::{AbsenceMetadata, AbsenceOrigin, Recoverability};
pub use capability::Capability;
pub use value_axes::{SemanticKind, ValueOrigin, ValueShape};
#[cfg(test)]
mod absence_metadata_tests;
#[cfg(test)]
mod protocol_string_tests;
