//! Temporary bridge between the legacy value model and the Semantic Spine.
//!
//! Phase 2 introduces the spine's [`KernelValue`] alongside the pre-reduction
//! [`Value`]/[`ValueData`] model and lets the two coexist while consumers
//! migrate one at a time (migration plan §Phase 2). This module is the seam:
//! it lowers a legacy `Value` to a `KernelValue` and raises a `KernelValue`
//! back to a `Value`. Nothing is rewired yet — the impls are available
//! crate-wide (trait coherence), but no runtime path calls them in Phase 2.
//!
//! The conversion unit is the whole [`Value`] — its `data`, `hint`, and
//! `absence` — not a bare [`ValueData`]. That is itself evidence for the
//! refactor: in the legacy model a value's *domain* is not determined by
//! `ValueData` alone. A string is a `Vector` of codepoints wearing an
//! `Interpretation::Text` hint, and a NIL's reason lives in `absence`, off to
//! the side. The spine folds both back into the value itself
//! (`KernelValue::String`, `KernelValue::Nil(reason)`).
//!
//! ## Deliberate collapses (legacy → spine)
//! - `ValueData::ExactScalar` and `ValueData::Scalar` both lower to
//!   `KernelValue::Scalar`: the exact/rational split is a representation, not a
//!   domain.
//! - `ValueData::Tensor` lowers to `KernelValue::Vector`: dense numeric storage
//!   is a vector optimization, observed as a vector.
//!
//! ## Known legacy quirks preserved on raise (spine → legacy)
//! - An empty `KernelValue::String` and an empty `KernelValue::Vector` raise to
//!   `NIL(EmptySequence)`, because the legacy model has no empty
//!   string/vector value — `Value::from_string`/`from_vector` project them to
//!   NIL. The round-trip tests pin this rather than pretend it is identity.
//!
//! ## Scope
//! The adapter converts value *structure* (the six domains). It does not model
//! interpretation-role-driven leaf retyping (e.g. a `TruthValue`-hinted vector
//! whose numeric leaves display as booleans, or timestamp/interval rendering):
//! those are presentation concerns that later phases relocate to
//! [`Observation`](super::observation)/presentation metadata.

use std::sync::Arc;

use crate::semantic::AbsenceMetadata;
use crate::types::{Interpretation, Value, ValueData};

use super::observation::{Observation, ObservedValue, PresentationHint};
use super::scalar::Scalar;
use super::value::{CodeBlock, KernelValue};

/// Lower a legacy [`Value`] onto the Semantic Spine.
impl From<&Value> for KernelValue {
    fn from(value: &Value) -> Self {
        match &value.data {
            ValueData::Boolean(b) => KernelValue::Boolean(*b),
            ValueData::Scalar(f) => KernelValue::Scalar(Scalar::from_fraction(f.clone())),
            ValueData::ExactScalar(er) => KernelValue::Scalar(Scalar::from_exact(er.clone())),
            ValueData::Nil => KernelValue::Nil(value.nil_reason().cloned()),
            ValueData::CodeBlock(tokens) => {
                KernelValue::CodeBlock(CodeBlock::new(Arc::from(tokens.as_slice())))
            }
            ValueData::Vector(children) => {
                // The canonical legacy string is a `Text`-hinted vector of
                // codepoints; fold it back into a first-class string.
                if value.hint == Interpretation::Text {
                    KernelValue::String(Arc::from(vector_to_string(children).as_str()))
                } else {
                    KernelValue::Vector(children.iter().map(KernelValue::from).collect())
                }
            }
            ValueData::Tensor { data, shape } => dense_to_kernel(data, shape, 0),
        }
    }
}

/// Raise a Semantic Spine [`KernelValue`] back into the legacy model.
impl From<&KernelValue> for Value {
    fn from(value: &KernelValue) -> Self {
        match value {
            KernelValue::Boolean(b) => Value::from_bool(*b),
            KernelValue::Scalar(scalar) => {
                if let Some(er) = scalar.exact_backing() {
                    Value::from_exact_real(er.clone())
                } else if let Some(f) = scalar.as_fraction() {
                    Value::from_fraction(f.clone())
                } else {
                    unreachable!("a spine Scalar is always a rational or an exact real")
                }
            }
            KernelValue::String(s) => Value::from_string(s),
            KernelValue::Vector(items) => {
                Value::from_children(items.iter().map(Value::from).collect())
            }
            KernelValue::Nil(Some(reason)) => Value::nil_with_reason(reason.clone()),
            // `Nil(None)` is not the written literal — the literal carries the
            // reason `literal`. It is the Spine's residual "absence with no
            // reason recorded", which only the legacy side can still produce
            // (an absent tensor lane), so it maps to the legacy reasonless
            // shape rather than minting a reason the value never had.
            KernelValue::Nil(None) => {
                Value::nil_with_absence(AbsenceMetadata::with_reasonless_unknown())
            }
            KernelValue::CodeBlock(block) => Value {
                data: ValueData::CodeBlock(block.tokens().to_vec()),
                hint: Interpretation::Unassigned,
                absence: None,
            },
        }
    }
}

/// Decode a `Text`-hinted codepoint vector into a Rust string, matching the
/// legacy string encoding (`Value::from_string`). Non-scalar or out-of-range
/// entries are skipped.
fn vector_to_string(children: &[Value]) -> String {
    children
        .iter()
        .filter_map(|child| match &child.data {
            ValueData::Scalar(f) => f.to_i64().and_then(|n| char::from_u32(n as u32)),
            _ => None,
        })
        .collect()
}

/// Materialize a dense tensor (or a rectangular slice of one) into a nested
/// `KernelValue::Vector`, row-major. `offset` is the flat start index of the
/// slice `shape` describes. A tensor lane that the valid-mask marks absent
/// becomes `KernelValue::Nil(None)`.
fn dense_to_kernel(
    data: &crate::types::DenseTensor,
    shape: &[usize],
    offset: usize,
) -> KernelValue {
    if shape.len() <= 1 {
        let len = if shape.is_empty() {
            data.len()
        } else {
            shape[0]
        };
        let lanes = (0..len).map(|i| match data.get_small_fraction(offset + i) {
            Some(f) => KernelValue::Scalar(Scalar::from_fraction(f)),
            None => KernelValue::Nil(None),
        });
        KernelValue::Vector(lanes.collect())
    } else {
        let outer = shape[0];
        let rest = &shape[1..];
        let stride: usize = rest.iter().product();
        let rows = (0..outer).map(|i| dense_to_kernel(data, rest, offset + i * stride));
        KernelValue::Vector(rows.collect())
    }
}

// ── Observation bridge (migration plan §17–18) ────────────────────────────
// Projecting the runtime into an `Observation` is the seam that lets
// conformance, host protocols, the CLI, and the GUI compare *observed values*
// rather than the engine's private representation. The legacy display intent
// (an `Interpretation` role) is demoted here to a presentation-only hint: it
// selects rendering, never execution. The absence origin/recoverability a
// legacy NIL carries is intentionally not observed — a NIL's reason is on the
// value; its provenance is diagnostic state (§9).

/// Demote a legacy interpretation role to a presentation-only hint. Roles that
/// carry no display intent (a plain number, a truth value, an unassigned or NIL
/// role, the machine continued-fraction serialization) observe structurally.
fn presentation_hint(role: Interpretation) -> PresentationHint {
    match role {
        Interpretation::Text => PresentationHint::Text,
        Interpretation::Interval => PresentationHint::Interval,
        Interpretation::Timestamp => PresentationHint::Timestamp,
        Interpretation::Unassigned
        | Interpretation::RawNumber
        | Interpretation::TruthValue
        | Interpretation::Nil
        | Interpretation::ContinuedFraction => PresentationHint::Structural,
    }
}

/// Observe a single legacy [`Value`]: its domain becomes a [`KernelValue`], its
/// display role a [`PresentationHint`].
impl From<&Value> for ObservedValue {
    fn from(value: &Value) -> Self {
        ObservedValue {
            value: KernelValue::from(value),
            presentation: presentation_hint(value.hint),
        }
    }
}

impl Observation {
    /// Project a runtime stack (bottom → top) into a spine [`Observation`].
    pub fn from_stack(values: &[Value]) -> Self {
        Observation {
            stack: values.iter().map(ObservedValue::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::exact::ExactReal;
    use crate::types::fraction::Fraction;
    use crate::types::DenseTensor;
    use crate::NilReason;

    /// spine -> legacy -> spine is the identity on the cases the legacy model
    /// can represent without projection.
    fn assert_round_trips(kernel: KernelValue) {
        let legacy = Value::from(&kernel);
        let back = KernelValue::from(&legacy);
        assert_eq!(back, kernel);
    }

    #[test]
    fn round_trips_the_representable_domains() {
        assert_round_trips(KernelValue::Boolean(true));
        assert_round_trips(KernelValue::Boolean(false));
        assert_round_trips(KernelValue::Scalar(Scalar::from_fraction(Fraction::from(
            42_i64,
        ))));
        assert_round_trips(KernelValue::String(Arc::from("hello")));
        assert_round_trips(KernelValue::Vector(Arc::from([
            KernelValue::Scalar(Scalar::from_fraction(Fraction::from(1_i64))),
            KernelValue::Boolean(true),
        ])));
        assert_round_trips(KernelValue::Nil(None));
        assert_round_trips(KernelValue::Nil(Some(NilReason::DivisionByZero)));
    }

    #[test]
    fn exact_scalar_round_trips_and_stays_exact() {
        let sqrt2 = ExactReal::from_sqrt_rational(Fraction::from(2_i64))
            .expect("sqrt(2) is a well-defined irrational");
        let kernel = KernelValue::Scalar(Scalar::from_exact(sqrt2));
        let back = KernelValue::from(&Value::from(&kernel));
        assert_eq!(back, kernel);
        match back {
            KernelValue::Scalar(scalar) => assert!(scalar.is_exact()),
            other => panic!("expected an exact scalar, got {other:?}"),
        }
    }

    #[test]
    fn both_legacy_scalar_variants_lower_to_the_one_scalar_domain() {
        let rational = KernelValue::from(&Value::from_fraction(Fraction::from(3_i64)));
        let irrational = KernelValue::from(&Value::from_exact_real(
            ExactReal::from_sqrt_rational(Fraction::from(3_i64)).unwrap(),
        ));
        assert!(matches!(rational, KernelValue::Scalar(_)));
        assert!(matches!(irrational, KernelValue::Scalar(_)));
    }

    #[test]
    fn a_dense_tensor_lowers_to_an_equivalent_vector() {
        let dense = DenseTensor::from_fractions(
            vec![Fraction::from(1_i64), Fraction::from(2_i64)],
            vec![2],
        )
        .unwrap();
        let tensor = Value {
            data: ValueData::Tensor {
                data: Arc::new(dense),
                shape: Arc::new(vec![2]),
            },
            hint: Interpretation::Unassigned,
            absence: None,
        };
        let lowered = KernelValue::from(&tensor);
        let expected = KernelValue::Vector(Arc::from([
            KernelValue::Scalar(Scalar::from_fraction(Fraction::from(1_i64))),
            KernelValue::Scalar(Scalar::from_fraction(Fraction::from(2_i64))),
        ]));
        assert_eq!(lowered, expected);
    }

    #[test]
    fn a_legacy_string_is_a_text_hinted_codepoint_vector() {
        // The legacy encoding, built the way the runtime builds it…
        let legacy = Value::from_string("hi");
        assert_eq!(
            KernelValue::from(&legacy),
            KernelValue::String(Arc::from("hi"))
        );
    }

    #[test]
    fn empty_string_and_vector_project_to_nil_on_raise() {
        // The legacy model has no empty string/vector value: it projects to
        // NIL(EmptySequence). Pin the quirk rather than pretend it round-trips.
        let from_empty_string =
            KernelValue::from(&Value::from(&KernelValue::String(Arc::from(""))));
        assert_eq!(
            from_empty_string,
            KernelValue::Nil(Some(NilReason::EmptySequence))
        );
    }
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    use crate::kernel::observation::{ObservedValue, PresentationHint};
    use crate::kernel::scalar::Scalar;
    use crate::types::fraction::Fraction;
    use crate::types::ValueData;

    async fn observe_program(program: &str) -> Observation {
        let mut interp = crate::interpreter::Interpreter::new();
        interp.execute(program).await.expect("program runs");
        Observation::from_stack(interp.stack.as_slice())
    }

    fn scalar(n: i64) -> KernelValue {
        KernelValue::Scalar(Scalar::from_fraction(Fraction::from(n)))
    }

    #[tokio::test]
    async fn observation_reflects_the_live_stack_values() {
        let observation = observe_program("3 4 +").await;
        assert_eq!(
            observation.stack,
            vec![ObservedValue {
                value: scalar(7),
                presentation: PresentationHint::Structural,
            }]
        );
    }

    #[tokio::test]
    async fn a_string_observes_as_a_string_value_with_a_text_hint() {
        let observation = observe_program("'hi'").await;
        assert_eq!(
            observation.stack,
            vec![ObservedValue {
                value: KernelValue::String(Arc::from("hi")),
                presentation: PresentationHint::Text,
            }]
        );
    }

    #[tokio::test]
    async fn a_multi_value_stack_observes_in_bottom_to_top_order() {
        let observation = observe_program("1 2 3").await;
        let values: Vec<&KernelValue> = observation.stack.iter().map(|o| &o.value).collect();
        assert_eq!(values, vec![&scalar(1), &scalar(2), &scalar(3)]);
    }

    #[test]
    fn presentation_hint_captures_display_intent_without_changing_the_value() {
        // Interval/Timestamp display intents ride as presentation hints; the
        // observed value stays a plain Vector/Scalar (no second type system).
        let interval = Value {
            data: ValueData::Vector(std::sync::Arc::new(vec![
                Value::from_int(1),
                Value::from_int(2),
            ])),
            hint: Interpretation::Interval,
            absence: None,
        };
        let observed = ObservedValue::from(&interval);
        assert_eq!(observed.presentation, PresentationHint::Interval);
        assert!(matches!(observed.value, KernelValue::Vector(_)));

        let timestamp = Value {
            data: ValueData::Scalar(Fraction::from(1_700_000_000_i64)),
            hint: Interpretation::Timestamp,
            absence: None,
        };
        let observed = ObservedValue::from(&timestamp);
        assert_eq!(observed.presentation, PresentationHint::Timestamp);
        assert!(matches!(observed.value, KernelValue::Scalar(_)));
    }
}
