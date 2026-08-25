//! Temporary bridge between the legacy value model and the Semantic Spine.
//!
//! Phase 2 introduces the spine's [`KernelValue`] alongside the pre-reduction
//! [`Value`]/[`ValueData`] model and lets the two coexist while consumers
//! migrate one at a time (migration plan §Phase 2). This module is the seam:
//! it lowers a legacy `Value` to a `KernelValue` and raises a `KernelValue`
//! back to a `Value`. Nothing is rewired yet — the impls are available
//! crate-wide (trait coherence), but no runtime path calls them in Phase 2.
//!
//! The conversion unit is the whole [`Value`] — its `data` and `absence` —
//! not a bare [`ValueData`], because a NIL's reason lives in `absence`, off to
//! the side, and the spine folds it back into the value itself
//! (`KernelValue::Nil(reason)`).
//!
//! String no longer needs folding. It used to be a `Vector` of codepoints
//! wearing an `Interpretation::Text` hint, so the legacy domain was not a
//! function of `ValueData` alone and this adapter had to reconstruct it;
//! `ValueData::Text` now mirrors `KernelValue::String` one-to-one.
//!
//! ## Deliberate collapses (legacy → spine)
//! - `ValueData::ExactScalar` and `ValueData::Scalar` both lower to
//!   `KernelValue::Scalar`: the exact/rational split is a representation, not a
//!   domain.
//! - `ValueData::Tensor` lowers to `KernelValue::Vector`: dense numeric storage
//!   is a vector optimization, observed as a vector.
//!
//! ## Known legacy quirks preserved on raise (spine → legacy)
//! - None remain for String. The empty String used to raise to
//!   `NIL(EmptySequence)`, because the legacy model had no empty string value;
//!   the round-trip test pinned that rather than pretend it was an identity.
//!   `''` is now a String on both sides and round-trips as itself.
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
            // The spine has no Symbol variant of its own yet — this bridge
            // is documented dead code on the runtime path ("no runtime path
            // calls them in Phase 2"), so a lone Symbol round-trips through
            // the nearest existing shape (a one-token CodeBlock) rather than
            // widening KernelValue's own domain, which is out of scope for
            // the CodeBlock/Vector unification this adapts to.
            ValueData::Symbol(name) => {
                KernelValue::CodeBlock(CodeBlock::new(Arc::from([crate::types::Token::Symbol(
                    Arc::clone(name),
                )])))
            }
            ValueData::Text(s) => KernelValue::String(Arc::clone(s)),
            ValueData::Vector(children) => {
                KernelValue::Vector(children.iter().map(KernelValue::from).collect())
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
            KernelValue::Nil(Some(reason)) => Value::nil_with_reason(*reason),
            // `Nil(None)` is not the written literal — the literal carries the
            // reason `literal`. It is the Spine's residual "absence with no
            // reason recorded", which only the legacy side can still produce
            // (an absent tensor lane), so it maps to the legacy reasonless
            // shape rather than minting a reason the value never had.
            KernelValue::Nil(None) => {
                Value::nil_with_absence(AbsenceMetadata::with_reasonless_unknown())
            }
            // Mirror of the lowering above — unwrap the one-token encoding
            // back to a Symbol when possible, otherwise fall back to an
            // empty Symbol (this path is unreachable from any runtime caller
            // today; see the lowering-side note).
            KernelValue::CodeBlock(block) => {
                let name: Arc<str> = match block.tokens() {
                    [crate::types::Token::Symbol(s)] => Arc::clone(s),
                    _ => Arc::from(""),
                };
                Value {
                    data: ValueData::Symbol(name),
                    hint: Interpretation::Unassigned,
                    absence: None,
                }
            }
        }
    }
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
    fn the_empty_string_round_trips_as_itself() {
        // This used to be half of `empty_string_and_vector_project_to_nil_on_raise`:
        // the legacy model had no empty String, so `''` came back as
        // NIL(EmptySequence) and the quirk had to be pinned. The String domain
        // has an empty element, so the round trip is now an identity.
        let round_tripped = KernelValue::from(&Value::from(&KernelValue::String(Arc::from(""))));
        assert_eq!(round_tripped, KernelValue::String(Arc::from("")));
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
    async fn a_string_observes_as_a_string_value_rendered_structurally() {
        // The String domain carries the "render me as text" information, so
        // the presentation hint has nothing left to add and stays structural.
        let observation = observe_program("'hi'").await;
        assert_eq!(
            observation.stack,
            vec![ObservedValue {
                value: KernelValue::String(Arc::from("hi")),
                presentation: PresentationHint::Structural,
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
