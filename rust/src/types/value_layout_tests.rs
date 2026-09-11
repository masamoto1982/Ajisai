//! How wide a `Value` is allowed to be.
//!
//! A `Value` is copied and moved constantly — every stack slot, every element
//! of an AoS `Vector`, every intermediate a Word builds — so its width is a
//! multiplier on almost all memory traffic the runtime generates. It used to be
//! 344 bytes around a 72-byte `ValueData`, because `Option<AbsenceMetadata>`
//! inlined a 256-byte `DebugDiagnosis`: a summary string, an evidence list, a
//! next-check list and a candidate list, reserved in full on every value
//! whether or not it was absent. A vector of 262,144 numbers moved 90 MB to
//! carry 19 MB of numerators, and `[ 0 262143 ] RANGE` took 89.8 ms; boxed, the
//! same line takes 9.9 ms.
//!
//! These are layout assertions, not wall clocks (`work_meter_calibration_tests`
//! says why timing tests do not belong in CI). They pin the shape that made the
//! difference: the absence envelope is a *pointer* on a present value, not a
//! reserved diagnosis.

#[cfg(test)]
mod value_layout_tests {
    use crate::semantic::AbsenceMetadata;
    use crate::types::{Value, ValueData};
    use std::mem::size_of;

    /// The payload plus a pointer-sized envelope and the role, rounded to
    /// alignment. Anything materially wider means something rare was inlined
    /// into every value again.
    const ENVELOPE_BUDGET: usize = 24;

    #[test]
    fn a_value_is_its_payload_plus_a_pointer_sized_envelope() {
        let payload = size_of::<ValueData>();
        let value = size_of::<Value>();
        assert!(
            value <= payload + ENVELOPE_BUDGET,
            "Value is {value} bytes around a {payload}-byte ValueData: the \
             absence envelope must stay a pointer (boxed diagnosis), because \
             this width multiplies every stack slot and every AoS lane"
        );
    }

    /// Stated separately from the budget above so a regression names its cause.
    /// `AbsenceMetadata` is a reason, an origin, a recoverability and a
    /// *pointer* to a diagnosis — the diagnosis is the rarest thing a value can
    /// carry and pays for its own allocation when it exists.
    #[test]
    fn absence_metadata_holds_its_diagnosis_behind_a_pointer() {
        let metadata = size_of::<AbsenceMetadata>();
        assert!(
            metadata <= ENVELOPE_BUDGET,
            "AbsenceMetadata is {metadata} bytes: it must not inline \
             DebugDiagnosis, which is hundreds of bytes of strings and lists"
        );
    }
}
