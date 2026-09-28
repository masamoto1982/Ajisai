pub mod absence;
pub mod protocol;

pub use absence::{minted_absence_count, AbsenceMetadata, AbsenceOrigin, Recoverability};
#[cfg(test)]
mod absence_metadata_tests;
#[cfg(test)]
mod protocol_string_tests;
