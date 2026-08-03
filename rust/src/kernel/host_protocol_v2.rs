//! HostProtocolV2 — the host wire format rendered directly from the Semantic
//! Spine (migration plan §15).
//!
//! V2 reflects the six canonical value domains and the outward-observable
//! [`Observation`], and nothing else. It carries no legacy concept — no module
//! catalog, no tensor shape, no logical-unknown axis, no absence
//! origin/recoverability, no capability set — because those cannot be expressed
//! by the spine types it serializes. HostProtocolV2 is the sole current host protocol and stays in lockstep with the language.
//!
//! This module is a pure `Observation -> serde_json::Value` mapping; no consumer
//! reads it as an independent semantic authority.

use serde_json::{json, Value as Json};

use super::observation::{Observation, ObservedValue, PresentationHint};
use super::value::KernelValue;

/// The protocol identifier carried by every V2 document.
pub const PROTOCOL_VERSION: &str = "ajisai.host.v2";

fn presentation_str(hint: PresentationHint) -> &'static str {
    match hint {
        PresentationHint::Structural => "structural",
        PresentationHint::Interval => "interval",
        PresentationHint::Timestamp => "timestamp",
    }
}

/// Render a [`KernelValue`] into its V2 value node. Total over the six domains;
/// no legacy field can appear because the spine cannot express one.
pub fn value_to_v2(value: &KernelValue) -> Json {
    match value {
        KernelValue::Scalar(scalar) => match scalar.as_fraction() {
            Some(fraction) => json!({
                "type": "scalar",
                "numerator": fraction.numerator().to_string(),
                "denominator": fraction.denominator().to_string(),
            }),
            // An exact-real scalar's rational backing is not part of the public
            // spine API; V2 marks the representation and defers its wire value
            // to a later phase that widens the exact-real surface.
            None => json!({ "type": "scalar", "exact": true }),
        },
        KernelValue::Boolean(value) => json!({ "type": "boolean", "value": value }),
        KernelValue::String(text) => json!({ "type": "string", "value": text.as_ref() }),
        KernelValue::Vector(items) => json!({
            "type": "vector",
            "items": items.iter().map(value_to_v2).collect::<Vec<_>>(),
        }),
        KernelValue::Nil(reason) => json!({
            "type": "nil",
            "reason": reason.as_ref().map(|reason| reason.as_protocol_str()),
        }),
        KernelValue::CodeBlock(_) => json!({ "type": "codeBlock" }),
    }
}

fn observed_to_v2(observed: &ObservedValue) -> Json {
    json!({
        "value": value_to_v2(&observed.value),
        "presentation": presentation_str(observed.presentation),
    })
}

/// Render an [`Observation`] into a complete HostProtocolV2 document.
pub fn observation_to_v2(observation: &Observation) -> Json {
    json!({
        "protocol": PROTOCOL_VERSION,
        "stack": observation
            .stack
            .iter()
            .map(observed_to_v2)
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::NilReason;
    use crate::kernel::scalar::Scalar;
    use crate::kernel::value::CodeBlock;
    use crate::types::fraction::Fraction;
    use std::sync::Arc;

    const GOLDEN_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../spec/freeze/host-protocol-v2.golden.json"
    );

    /// One value of each of the six domains plus a display-hinted vector, so the
    /// golden exercises the whole wire surface.
    fn canonical_observation() -> Observation {
        fn scalar(n: i64) -> KernelValue {
            KernelValue::Scalar(Scalar::from_fraction(Fraction::from(n)))
        }
        Observation {
            stack: vec![
                ObservedValue {
                    value: scalar(3),
                    presentation: PresentationHint::Structural,
                },
                ObservedValue {
                    value: KernelValue::Boolean(true),
                    presentation: PresentationHint::Structural,
                },
                ObservedValue {
                    value: KernelValue::String(Arc::from("hi")),
                    presentation: PresentationHint::Structural,
                },
                ObservedValue {
                    value: KernelValue::Vector(Arc::from([scalar(1), scalar(2)])),
                    presentation: PresentationHint::Interval,
                },
                ObservedValue {
                    value: KernelValue::Nil(Some(NilReason::DivisionByZero)),
                    presentation: PresentationHint::Structural,
                },
                ObservedValue {
                    value: KernelValue::Nil(None),
                    presentation: PresentationHint::Structural,
                },
                ObservedValue {
                    value: KernelValue::CodeBlock(CodeBlock::new(Arc::from(Vec::new()))),
                    presentation: PresentationHint::Structural,
                },
            ],
        }
    }

    #[test]
    fn v2_document_matches_the_frozen_golden() {
        let document = observation_to_v2(&canonical_observation());
        let golden: Json = serde_json::from_str(
            &std::fs::read_to_string(GOLDEN_PATH).expect("V2 golden is present"),
        )
        .expect("V2 golden is valid JSON");
        assert_eq!(
            document, golden,
            "V2 output drifted from the golden; regenerate with \
             `cargo test regenerate_v2_golden -- --ignored`"
        );
    }

    #[test]
    fn v2_carries_no_legacy_field() {
        // The V1-only concepts the spine dropped must never surface in a V2
        // document, by construction. This is a belt-and-braces check over the
        // serialized text alongside the type-level guarantee.
        let text = serde_json::to_string(&observation_to_v2(&canonical_observation())).unwrap();
        for legacy in [
            "module",
            "tensor",
            "unknown",
            "origin",
            "recoverab",
            "capabilit",
            "importedModules",
            "semanticKind",
        ] {
            assert!(!text.contains(legacy), "V2 leaked a legacy field: {legacy}");
        }
    }

    #[test]
    fn v2_value_types_conform_to_the_schema() {
        // Keep the schema and the serializer from drifting: every value node the
        // serializer emits must use a `type` the schema declares. Reads the
        // domain enum straight from spec/host-protocol-v2.schema.json.
        const SCHEMA_PATH: &str = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../spec/host-protocol-v2.schema.json"
        );
        let schema: Json =
            serde_json::from_str(&std::fs::read_to_string(SCHEMA_PATH).expect("schema present"))
                .expect("schema is valid JSON");
        let allowed: Vec<&str> = schema["$defs"]["value"]["properties"]["type"]["enum"]
            .as_array()
            .expect("schema declares a value type enum")
            .iter()
            .map(|entry| entry.as_str().expect("type enum entries are strings"))
            .collect();

        fn walk(node: &Json, allowed: &[&str]) {
            let type_str = node["type"].as_str().expect("value node has a type");
            assert!(
                allowed.contains(&type_str),
                "value type `{type_str}` is not declared in the schema"
            );
            if let Some(items) = node["items"].as_array() {
                for item in items {
                    walk(item, allowed);
                }
            }
        }

        let document = observation_to_v2(&canonical_observation());
        for observed in document["stack"].as_array().unwrap() {
            walk(&observed["value"], &allowed);
        }
    }

    #[test]
    #[ignore = "regenerates the V2 golden; run with `-- --ignored`"]
    fn regenerate_v2_golden() {
        let document = observation_to_v2(&canonical_observation());
        let pretty = serde_json::to_string_pretty(&document).unwrap();
        std::fs::write(GOLDEN_PATH, format!("{pretty}\n")).unwrap();
    }
}
