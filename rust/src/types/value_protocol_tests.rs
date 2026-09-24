// AQ-VER-003-C: native verification of the pure serialization mapping
// `value_to_protocol`. These tests pin the type/value decision for every
// ValueData kind: each is a function of the value's domain alone
// (LANG.VALUES.DENOTATION, LANG.VALUES.DISJOINT).
//
// Trace: docs/quality/TRACEABILITY_MATRIX.md, requirement AQ-REQ-003.

use crate::types::fraction::Fraction;
use crate::types::value_protocol::{value_to_protocol, ProtocolNode, ProtocolValue};
use crate::types::{DenseTensor, Value, ValueData};
use std::sync::Arc;

fn frac(n: i64) -> Fraction {
    Fraction::from(n)
}

fn scalar(n: i64) -> Value {
    Value::from_fraction(frac(n))
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
#[test]
fn plain_nil_is_still_nil_not_unknown() {
    let node = value_to_protocol(&Value::nil());
    assert_eq!(node.type_str, "nil");
    assert_eq!(node.value, ProtocolValue::Null);
}

/// A Symbol is its own domain on the wire, carrying its bare name. It used
/// to serialize as `nil` (as part of the pre-unification CodeBlock domain),
/// so every host drew `NIL` for a value `NIL?` answers FALSE for and `EXEC`
/// runs — the internal representation observed as the wrong domain, which
/// LANG.OBSERVATION.FIREWALL rules out.
#[test]
fn a_symbol_serializes_as_its_own_domain_with_its_bare_name() {
    let node = value_to_protocol(&Value::from_symbol("MUL"));
    assert_eq!(node.type_str, "symbol");
    assert_eq!(node.value, ProtocolValue::Text("MUL".to_string()));
}

// --- ExactScalar approximation marker (LANG.OBSERVATION.FIREWALL) ---

/// √2 as an exact irrational (AlgebraicSqrt), the canonical ExactScalar.
fn sqrt2() -> Value {
    use crate::types::exact::ExactReal;
    let er = ExactReal::from_sqrt_rational(frac(2)).expect("√2 is a valid exact real");
    let v = Value::from_exact_real(er);
    assert!(
        matches!(v.data, ValueData::ExactScalar(_)),
        "√2 must remain an ExactScalar, not collapse to a rational"
    );
    v
}

/// An ExactScalar serializes as a `number` (its best rational approximation)
/// but its `semantics` block must carry the original exact value, so the GUI
/// can reference the exact source rather than a silent truncation
/// (LANG.OBSERVATION.FIREWALL).
#[test]
fn exact_scalar_carries_exact_source_in_semantics() {
    let node = value_to_protocol(&sqrt2());
    assert_eq!(node.type_str, "number", "ExactScalar -> number");
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

#[test]
fn algebraic_exact_terms_are_lossless_decimal_strings() {
    use crate::types::value_protocol::exact_terms;

    assert_eq!(
        exact_terms(&sqrt2()).expect("sqrt(2) has an algebraic normal form"),
        vec![crate::types::value_protocol::ProtocolExactTerm {
            numerator: "1".to_string(),
            denominator: "1".to_string(),
            radicand: "2".to_string(),
        }]
    );
}

/// `exactDisplay` renders the same normal form `exactTerms` carries, short
/// enough to read. The continued-fraction projection a consumer meets first is
/// truncated and the node's own `value` is an approximation, so this is the
/// only rendering of an algebraic value that is both short and complete.
///
/// Every rendering decision is pinned here rather than through the CLI,
/// because both host serializers call this one function and neither adds any
/// formatting of its own.
#[test]
fn algebraic_exact_display_writes_the_normal_form_short() {
    use crate::types::exact::ExactReal;
    use crate::types::value_protocol::{exact_display, exact_terms};

    let display = |value: &Value| exact_display(value).expect("an algebraic value renders short");
    let sqrt_of = |n: i64| {
        Value::from_exact_real(
            ExactReal::from_sqrt_rational(frac(n)).expect("a supported algebraic square root"),
        )
    };

    // A unit coefficient is left unwritten: `1/1*sqrt(2)` says nothing more.
    assert_eq!(display(&sqrt2()), "sqrt(2)");

    // A non-unit coefficient keeps Ajisai's own `numerator/denominator`
    // rendering. `2*sqrt(2)` would be the only place in the language where a
    // number is written without its denominator.
    let two_sqrt2 = match sqrt2().data {
        ValueData::ExactScalar(ExactReal::Algebraic(algebraic)) => {
            match algebraic.mul_fraction(&Fraction::new(2.into(), 1.into())) {
                crate::types::exact::AlgebraicResult::Irrational(scaled) => {
                    Value::from_exact_real(ExactReal::Algebraic(scaled))
                }
                other => panic!("2·√2 stays irrational, got {other:?}"),
            }
        }
        ref other => panic!("√2 is an algebraic ExactScalar, got {other:?}"),
    };
    assert_eq!(display(&two_sqrt2), "2/1*sqrt(2)");

    // Present exactly when `exactTerms` is: one fact in two shapes, so a
    // reader is never left choosing which field to believe.
    for value in [scalar(3), vector(vec![scalar(1)]), Value::nil()] {
        assert_eq!(exact_display(&value), None, "{value:?} has no normal form");
        assert_eq!(exact_terms(&value), None, "{value:?} has no normal form");
    }

    // The stored form is rendered faithfully, including the case where two
    // equal values hold different terms. Reducing `sqrt(8)` to `2/1*sqrt(2)`
    // here would make the string disagree with the `exactTerms` beside it.
    assert_eq!(display(&sqrt_of(8)), "sqrt(8)");
}

// --- scalar and truth domains ---

#[test]
fn scalar_is_number() {
    let node = value_to_protocol(&scalar(7));
    assert_eq!(node.type_str, "number");
    assert_eq!(node.value, num("7", "1"));
}

/// LANG.VALUES.DISJOINT: TRUE is not scalar one and FALSE is not scalar zero,
/// on the wire as anywhere else.
#[test]
fn zero_and_one_are_numbers_and_booleans_are_booleans() {
    for n in [0, 1] {
        let node = value_to_protocol(&scalar(n));
        assert_eq!(node.type_str, "number");
        assert_eq!(node.value, num(&n.to_string(), "1"));
    }
    for b in [true, false] {
        let node = value_to_protocol(&Value::from_bool(b));
        assert_eq!(node.type_str, "boolean");
        assert_eq!(node.value, ProtocolValue::Bool(b));
    }
}

/// UNKNOWN is a NIL (LANG.VALUES.TRUTH), so it is observed as one: there is
/// no truth-valued NIL on the wire.
#[test]
fn a_nil_in_truth_position_is_observed_as_a_nil() {
    let unknown = Value::nil_with_reason_unknown(crate::error::NilReason::Undecidable);
    let node = value_to_protocol(&unknown);
    assert_eq!(node.type_str, "nil");
    assert_eq!(node.value, ProtocolValue::Null);
    assert_eq!(unknown.truth_value(), None);
}

#[test]
fn a_string_projects_to_a_string_leaf() {
    let node = value_to_protocol(&Value::from_string("A"));
    assert_eq!(node.type_str, "string");
    assert_eq!(node.value, ProtocolValue::Text("A".to_string()));
}

#[test]
fn a_scalar_never_projects_as_a_string() {
    let node = value_to_protocol(&scalar(65));
    assert_eq!(node.type_str, "number");
}

// --- Vector branch ---

#[test]
fn vector_structural_renders_number_children() {
    let node = value_to_protocol(&vector(vec![scalar(1), scalar(2)]));
    assert_eq!(node.type_str, "vector");
    let kids = children_of(&node);
    assert_eq!(kids.len(), 2);
    assert_eq!(kids[0].type_str, "number");
    assert_eq!(kids[1].value, num("2", "1"));
}

#[test]
fn a_vector_of_booleans_has_boolean_children() {
    let node = value_to_protocol(&vector(vec![
        Value::from_bool(true),
        Value::from_bool(false),
        Value::nil(),
    ]));
    assert_eq!(node.type_str, "vector");
    let kids = children_of(&node);
    assert_eq!(kids[0].type_str, "boolean");
    assert_eq!(kids[0].value, ProtocolValue::Bool(true));
    assert_eq!(kids[1].value, ProtocolValue::Bool(false));
    assert_eq!(kids[2].type_str, "nil");
}

#[test]
fn a_codepoint_vector_projects_as_a_vector() {
    // `[ 65 66 ]` is a Vector of two numbers, not the string `'AB'`. This is
    // the protocol-side face of `'A' [ 65 ] EQ` answering false.
    let node = value_to_protocol(&vector(vec![scalar(65), scalar(66)]));
    assert_eq!(node.type_str, "vector");
}

// --- Tensor branch: the regression that motivated this layer ---

#[test]
fn tensor_1d_default_renders_numbers() {
    let node = value_to_protocol(&tensor(&[1, 2, 3], &[3]));
    let kids = children_of(&node);
    assert_eq!(kids.len(), 3);
    assert!(kids.iter().all(|k| k.type_str == "number"));
    assert_eq!(kids[2].value, num("3", "1"));
}

#[test]
fn tensor_2d_numbers_nest() {
    let node = value_to_protocol(&tensor(&[1, 2, 3, 4], &[2, 2]));
    let rows = children_of(&node);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].type_str, "vector");
    assert!(
        rows[0].semantics.is_none(),
        "interior tensor nodes carry no semantics"
    );
    let leaves = children_of(&rows[0]);
    assert_eq!(leaves[0].value, num("1", "1"));
    assert_eq!(leaves[1].value, num("2", "1"));
}

#[test]
fn a_dense_tensor_projects_as_a_vector() {
    // Strings are never densified now, so a dense numeric tensor has no text
    // reading available to it at all.
    let node = value_to_protocol(&tensor(&[72, 105], &[2]));
    assert_eq!(node.type_str, "vector");
}

// --- remaining ValueData kinds ---
#[test]
fn top_level_node_always_carries_semantics() {
    assert!(value_to_protocol(&scalar(1)).semantics.is_some());
    assert!(value_to_protocol(&tensor(&[1, 2], &[2]))
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
            absence: None,
        }
    }

    proptest! {
        // A dense tensor holds numbers: every leaf is a number, never a
        // boolean, whatever produced it.
        #[test]
        fn default_tensor_leaves_are_number(nums in proptest::collection::vec(-5i64..5, 1..12)) {
            let node = value_to_protocol(&tensor_1d(&nums));
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
