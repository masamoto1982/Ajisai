pub mod absence;
pub mod protocol;

pub use absence::{AbsenceMetadata, AbsenceOrigin, Recoverability};
#[cfg(test)]
mod absence_metadata_tests;
#[cfg(test)]
mod protocol_string_tests;
