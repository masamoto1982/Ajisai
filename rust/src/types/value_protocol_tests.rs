// AQ-VER-003-C: native verification of the pure serialization mapping
// `value_to_protocol`. The historical WASM boundary left the (Value, hint)
// -> wire-format decision untested (only `cargo check --target wasm32`),
// which allowed a promoted boolean tensor to serialize as numbers. These
// tests pin the type/value/displayHint decision for every ValueData kind,
// the four scalar interpretation arms, the Vector/Tensor text projections,
// the TruthValue leaf propagation through promoted tensors (the regression),
// and the external-vs-value hint precedence.
//
// Trace: docs/quality/TRACEABILITY_MATRIX.md, requirement AQ-REQ-003.

use crate::types::fraction::Fraction;
use crate::types::value_protocol::{value_to_protocol, ProtocolNode, ProtocolValue};
use crate::types::{DenseTensor, Interpretation, Value, ValueData};
use std::sync::Arc;

fn frac(n: i64) -> Fraction {
    Fraction::from(n)
}

fn scalar(n: i64) -> Value {
    Value::from_fraction(frac(n))
}

fn with_hint(mut v: Value, hint: Interpretation) -> Value {
    v.hint = hint;
    v
}

fn vector(children: Vec<Value>) -> Value {
    Value::from_children(children)
}

fn tensor(nums: &[i64], shape: &[usize]) -> Value {
    let fracs: Vec<Fraction> = nums.iter().map(|n| frac(*n)).collect();
    let dense =
        DenseTensor::from_fractions(fracs, shape.to_vec()).expect("rectangular tensor for test");
    Value {
        data: ValueData::Tensor {
            data: Arc::new(dense),
            shape: Arc::new(shape.to_vec()),
        },
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

fn num(numerator: &str, denominator: &str) -> ProtocolValue {
    ProtocolValue::Number {
        numerator: numerator.to_string(),
        denominator: denominator.to_string(),
    }
}

fn children_of(node: &ProtocolNode) -> &[ProtocolNode] {
    match &node.value {
        ProtocolValue::Children(kids) => kids,
        other => panic!("expected Children, got {:?}", other),
    }
}

// --- logical Unknown (U) serialization (SPEC §7.5, §2.3) ---

#[test]
fn unknown_serializes_as_truth_value_unknown() {
    let node = value_to_protocol(&Value::unknown(), None);
    assert_eq!(node.type_str, "truthValue");
    assert_eq!(node.value, ProtocolValue::Text("unknown".to_string()));
    assert_eq!(node.display_hint, Interpretation::TruthValue);
}

#[test]
fn unknown_serializes_as_truth_value_even_under_external_hint() {
    // Detection is reason-based, so U is observed as `unknown` regardless
    // of any external hint override (SPEC §2.3 firewall).
    let node = value_to_protocol(&Value::unknown(), Some(Interpretation::Nil));
    assert_eq!(node.type_str, "truthValue");
    assert_eq!(node.value, ProtocolValue::Text("unknown".to_string()));
}

#[test]
fn plain_nil_is_still_nil_not_unknown() {
    let node = value_to_protocol(&Value::nil(), None);
    assert_eq!(node.type_str, "nil");
    assert_eq!(node.value, ProtocolValue::Null);
}

// --- ExactScalar approximation marker (SPEC §2.3) ---

/// √2 as an exact irrational (AlgebraicSqrt), the canonical ExactScalar.
fn sqrt2() -> Value {
    use crate::types::continued_fraction::ExactReal;
    let er = ExactReal::from_sqrt_rational(frac(2)).expect("√2 is a valid exact real");
    let v = Value::from_exact_real(er);
    assert!(
        matches!(v.data, ValueData::ExactScalar(_)),
        "√2 must remain an ExactScalar, not collapse to a rational"
    );
    v
}

/// Under `RawNumber`, an ExactScalar serializes as a `number` (its best
/// rational approximation) but its `semantics` block must carry the
/// original exact value, so the GUI can reference the exact source rather
/// than a silent truncation (Option 1 / SPEC §2.3 firewall).
#[test]
fn exact_scalar_rawnumber_carries_exact_source_in_semantics() {
    let node = value_to_protocol(&sqrt2(), Some(Interpretation::RawNumber));
    assert_eq!(node.type_str, "number", "RawNumber ExactScalar -> number");
    assert!(
        matches!(node.value, ProtocolValue::Number { .. }),
        "value is the rational approximation, got {:?}",
        node.value
    );
    let semantics = node
        .semantics
        .as_ref()
        .expect("ExactScalar node must carry a semantics source");
    assert!(
        matches!(semantics.data, ValueData::ExactScalar(_)),
        "semantics must preserve the exact ExactScalar source, got {:?}",
        semantics.data
    );
}

/// Under the `ContinuedFraction` role the value is rendered losslessly as
/// the canonical nested-form string and carries no `semantics` block, so
/// it is never marked approximate (regression guard: unchanged behavior).
#[test]
fn exact_scalar_continued_fraction_role_is_lossless_nested_form() {
    let node = value_to_protocol(&sqrt2(), Some(Interpretation::ContinuedFraction));
    assert_eq!(node.type_str, "string", "CF role -> nested-form string");
    assert!(
        matches!(node.value, ProtocolValue::Text(_)),
        "CF role yields the nested-form text, got {:?}",
        node.value
    );
    assert_eq!(node.display_hint, Interpretation::ContinuedFraction);
    assert!(
        node.semantics.is_none(),
        "CF nodes carry no semantics block (and thus no approximate marker)"
    );
}

#[test]
fn truth_value_axis_uses_effective_role_for_definite_booleans() {
    // A comparison/logic boolean carries the TruthValue role in the
    // semantic plane (here passed as the external hint), not on the
    // value's own RawNumber hint. The truthValue axis must still resolve.
    assert_eq!(
        scalar(1).truth_value_for_role(Interpretation::TruthValue),
        Some("true")
    );
    assert_eq!(
        scalar(0).truth_value_for_role(Interpretation::TruthValue),
        Some("false")
    );
    // Without the TruthValue role it is a plain number, not a truth value.
    assert_eq!(
        scalar(1).truth_value_for_role(Interpretation::RawNumber),
        None
    );
    // U is `unknown` regardless of the role.
    assert_eq!(
        Value::unknown().truth_value_for_role(Interpretation::RawNumber),
        Some("unknown")
    );
}

// --- scalar interpretation arms (MC/DC on the 4-way match) ---

#[test]
fn scalar_default_hint_is_number() {
    let node = value_to_protocol(&scalar(7), None);
    assert_eq!(node.type_str, "number");
    assert_eq!(node.value, num("7", "1"));
    assert_eq!(node.display_hint, Interpretation::RawNumber);
}

#[test]
fn scalar_truthvalue_is_boolean() {
    let node = value_to_protocol(&scalar(1), Some(Interpretation::TruthValue));
    assert_eq!(node.type_str, "boolean");
    assert_eq!(node.value, ProtocolValue::Bool(true));
    let zero = value_to_protocol(&scalar(0), Some(Interpretation::TruthValue));
    assert_eq!(zero.value, ProtocolValue::Bool(false));
}

#[test]
fn scalar_timestamp_is_datetime_with_number_value() {
    let node = value_to_protocol(&scalar(123), Some(Interpretation::Timestamp));
    assert_eq!(node.type_str, "datetime");
    assert_eq!(node.value, num("123", "1"));
}

#[test]
fn scalar_text_is_string_codepoint() {
    // 65 -> 'A'
    let node = value_to_protocol(&scalar(65), Some(Interpretation::Text));
    assert_eq!(node.type_str, "string");
    assert_eq!(node.value, ProtocolValue::Text("A".to_string()));
}

// --- hint precedence: external Some wins, None falls back to value.hint ---

#[test]
fn external_hint_overrides_value_hint() {
    let v = with_hint(scalar(1), Interpretation::RawNumber);
    let node = value_to_protocol(&v, Some(Interpretation::TruthValue));
    assert_eq!(node.type_str, "boolean");
}

#[test]
fn absent_external_hint_falls_back_to_value_hint() {
    let v = with_hint(scalar(1), Interpretation::TruthValue);
    let node = value_to_protocol(&v, None);
    assert_eq!(node.type_str, "boolean");
}

// --- Vector branch: structural vs Text projection vs TruthValue children ---

#[test]
fn vector_structural_renders_number_children() {
    let node = value_to_protocol(&vector(vec![scalar(1), scalar(2)]), None);
    assert_eq!(node.type_str, "vector");
    let kids = children_of(&node);
    assert_eq!(kids.len(), 2);
    assert_eq!(kids[0].type_str, "number");
    assert_eq!(kids[1].value, num("2", "1"));
}

#[test]
fn vector_truthvalue_propagates_to_children() {
    let node = value_to_protocol(
        &vector(vec![scalar(1), scalar(0)]),
        Some(Interpretation::TruthValue),
    );
    let kids = children_of(&node);
    assert_eq!(kids[0].type_str, "boolean");
    assert_eq!(kids[0].value, ProtocolValue::Bool(true));
    assert_eq!(kids[1].value, ProtocolValue::Bool(false));
    assert_eq!(kids[0].display_hint, Interpretation::TruthValue);
}

#[test]
fn vector_text_projects_to_string() {
    // 'A','B' codepoints
    let node = value_to_protocol(
        &vector(vec![scalar(65), scalar(66)]),
        Some(Interpretation::Text),
    );
    assert_eq!(node.type_str, "string");
    assert_eq!(node.value, ProtocolValue::Text("AB".to_string()));
}

// --- Tensor branch: the regression that motivated this layer ---

#[test]
fn tensor_1d_default_renders_numbers() {
    let node = value_to_protocol(&tensor(&[1, 2, 3], &[3]), None);
    let kids = children_of(&node);
    assert_eq!(kids.len(), 3);
    assert!(kids.iter().all(|k| k.type_str == "number"));
    assert_eq!(kids[2].value, num("3", "1"));
}

#[test]
fn tensor_1d_truthvalue_renders_booleans() {
    // Regression: a promoted dense boolean vector ([ TRUE ], AND/OR/NOT
    // results) must serialize its leaves as booleans, not 1/1 numbers.
    let node = value_to_protocol(&tensor(&[1, 0, 1], &[3]), Some(Interpretation::TruthValue));
    assert_eq!(node.type_str, "vector");
    let kids = children_of(&node);
    assert_eq!(kids[0].value, ProtocolValue::Bool(true));
    assert_eq!(kids[1].value, ProtocolValue::Bool(false));
    assert_eq!(kids[2].value, ProtocolValue::Bool(true));
    assert!(kids.iter().all(|k| k.type_str == "boolean"));
    assert!(kids
        .iter()
        .all(|k| k.display_hint == Interpretation::TruthValue));
}

#[test]
fn tensor_2d_numbers_nest_with_unassigned_interior_hint() {
    let node = value_to_protocol(&tensor(&[1, 2, 3, 4], &[2, 2]), None);
    let rows = children_of(&node);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].type_str, "vector");
    assert_eq!(rows[0].display_hint, Interpretation::Unassigned);
    assert!(
        rows[0].semantics.is_none(),
        "interior tensor nodes carry no semantics"
    );
    let leaves = children_of(&rows[0]);
    assert_eq!(leaves[0].value, num("1", "1"));
    assert_eq!(leaves[1].value, num("2", "1"));
}

#[test]
fn tensor_2d_truthvalue_nests_booleans() {
    let node = value_to_protocol(
        &tensor(&[1, 0, 0, 1], &[2, 2]),
        Some(Interpretation::TruthValue),
    );
    let rows = children_of(&node);
    assert_eq!(rows[0].display_hint, Interpretation::TruthValue);
    let leaves = children_of(&rows[1]);
    assert_eq!(leaves[0].value, ProtocolValue::Bool(false));
    assert_eq!(leaves[1].value, ProtocolValue::Bool(true));
}

#[test]
fn tensor_1d_text_projects_to_string() {
    let node = value_to_protocol(&tensor(&[72, 105], &[2]), Some(Interpretation::Text));
    assert_eq!(node.type_str, "string");
    assert_eq!(node.value, ProtocolValue::Text("Hi".to_string()));
}

// --- remaining ValueData kinds ---

#[test]
fn nil_and_handles() {
    assert_eq!(value_to_protocol(&Value::nil(), None).type_str, "nil");
    assert_eq!(
        value_to_protocol(&Value::from_process_handle(7), None).value,
        ProtocolValue::Handle(7)
    );
    assert_eq!(
        value_to_protocol(&Value::from_supervisor_handle(9), None).type_str,
        "supervisor_handle"
    );
}

#[test]
fn top_level_node_always_carries_semantics() {
    assert!(value_to_protocol(&scalar(1), None).semantics.is_some());
    assert!(value_to_protocol(&tensor(&[1, 2], &[2]), None)
        .semantics
        .is_some());
}

mod protocol_property_tests {
    use super::*;
    use proptest::prelude::*;

    fn tensor_1d(nums: &[i64]) -> Value {
        let fracs: Vec<Fraction> = nums.iter().map(|n| Fraction::from(*n)).collect();
        let len = fracs.len();
        let dense = DenseTensor::from_fractions(fracs, vec![len]).expect("1d tensor");
        Value {
            data: ValueData::Tensor {
                data: Arc::new(dense),
                shape: Arc::new(vec![len]),
            },
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    proptest! {
        // TruthValue role => every leaf is a boolean whose truth equals
        // (element != 0); never a number. Drives broad input coverage of
        // the tensor leaf decision beyond the hand-picked MC/DC rows.
        #[test]
        fn truthvalue_tensor_leaves_are_boolean(nums in proptest::collection::vec(-5i64..5, 1..12)) {
            let node = value_to_protocol(&tensor_1d(&nums), Some(Interpretation::TruthValue));
            let kids = match node.value {
                ProtocolValue::Children(k) => k,
                other => panic!("expected Children, got {:?}", other),
            };
            prop_assert_eq!(kids.len(), nums.len());
            for (kid, n) in kids.iter().zip(nums.iter()) {
                prop_assert_eq!(kid.type_str, "boolean");
                prop_assert_eq!(&kid.value, &ProtocolValue::Bool(*n != 0));
            }
        }

        // Default (numeric) role => every leaf is a number, never a boolean.
        #[test]
        fn default_tensor_leaves_are_number(nums in proptest::collection::vec(-5i64..5, 1..12)) {
            let node = value_to_protocol(&tensor_1d(&nums), None);
            let kids = match node.value {
                ProtocolValue::Children(k) => k,
                other => panic!("expected Children, got {:?}", other),
            };
            for kid in &kids {
                prop_assert_eq!(kid.type_str, "number");
                let is_number = matches!(kid.value, ProtocolValue::Number { .. });
                prop_assert!(is_number);
            }
        }
    }
}
