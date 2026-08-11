// `js_sys::Reflect::set(...).unwrap()` 群について:
// 直前に `js_sys::Object::new()` で生成したフレッシュなプレーン JS オブジェクト
// に対する set のため、Proxy ハンドラや凍結など失敗要因は実質的に発生しない。
// それでも万一 set が失敗した場合は console_error_panic_hook 経由で
// ブラウザコンソールにスタックトレースが出るので、原因解析は可能。

use crate::types::arena::{NodeId, NodeKind, ValueArena};
use crate::types::value_protocol::{
    exact_terms, interpretation_protocol_str, value_to_protocol, ProtocolNode, ProtocolValue,
};
use crate::types::{Interpretation, Value, ValueData};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct UserWordData {
    pub(crate) dictionary: Option<String>,
    pub(crate) name: String,
    pub(crate) definition: Option<String>,
}

#[cfg(test)]
pub(crate) fn build_bracket_structure_from_shape(shape: &[usize]) -> String {
    fn build_level(shape: &[usize]) -> String {
        if shape.len() == 1 {
            let empty = "[ ]";
            std::iter::repeat_n(empty, shape[0])
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            let inner = build_level(&shape[1..]);
            let one_element = format!("[ {} ]", inner);
            std::iter::repeat_n(one_element.as_str(), shape[0])
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
    if shape.is_empty() {
        return "[ ]".to_string();
    }
    build_level(shape)
}

fn set_prop(obj: &js_sys::Object, key: &str, value: &JsValue) {
    js_sys::Reflect::set(obj, &key.into(), value).unwrap();
}

fn diagnosis_to_protocol_js(
    diagnosis: &crate::interpreter::debug_diagnosis::DebugDiagnosis,
) -> JsValue {
    let obj = js_sys::Object::new();
    set_prop(&obj, "when", &diagnosis.when.as_protocol_str().into());
    set_prop(&obj, "why", &diagnosis.why.as_protocol_str().into());
    set_prop(&obj, "summary", &diagnosis.summary.clone().into());

    let where_obj = js_sys::Object::new();
    set_prop(
        &where_obj,
        "kind",
        &diagnosis.where_.kind.as_protocol_str().into(),
    );
    if let Some(word) = &diagnosis.where_.word {
        set_prop(&where_obj, "word", &word.clone().into());
    }
    if let Some(dictionary) = &diagnosis.where_.dictionary {
        set_prop(&where_obj, "dictionary", &dictionary.clone().into());
    }
    set_prop(&obj, "where", &where_obj.into());

    let evidence_arr = js_sys::Array::new();
    for item in &diagnosis.evidence {
        evidence_arr.push(&JsValue::from_str(item));
    }
    set_prop(&obj, "evidence", &evidence_arr.into());

    let checks_arr = js_sys::Array::new();
    for c in &diagnosis.next_checks {
        let check_obj = js_sys::Object::new();
        set_prop(&check_obj, "code", &JsValue::from_str(c.code));
        set_prop(&check_obj, "title", &localized_to_protocol_js(&c.title));
        set_prop(&check_obj, "detail", &localized_to_protocol_js(&c.detail));
        checks_arr.push(&check_obj);
    }
    set_prop(&obj, "nextChecks", &checks_arr.into());

    let candidates_arr = js_sys::Array::new();
    for candidate in &diagnosis.candidates {
        candidates_arr.push(&JsValue::from_str(candidate));
    }
    set_prop(&obj, "candidates", &candidates_arr.into());

    if let Some(facts) = &diagnosis.resource_limit {
        let limit_obj = js_sys::Object::new();
        set_prop(&limit_obj, "resource", &facts.resource.clone().into());
        set_prop(&limit_obj, "limit", &(facts.limit as f64).into());
        if let Some(observed) = facts.observed {
            set_prop(&limit_obj, "observed", &(observed as f64).into());
        }
        set_prop(&obj, "resourceLimit", &limit_obj.into());
    }

    // CF-comparison agreed-prefix (SPEC §4.5.0 / §7.4.1): machine-readable
    // count of leading partial quotients that matched before an Unknown (U)
    // comparison gave up. Emitted only when present.
    if let Some(prefix) = diagnosis.agreed_prefix {
        set_prop(&obj, "agreedPrefix", &(prefix as f64).into());
    }
    obj.into()
}

/// One locale-keyed display string. The stable identity of a next-check is its
/// `code`; this carries only what a host displays.
fn localized_to_protocol_js(text: &crate::interpreter::debug_diagnosis::LocalizedText) -> JsValue {
    let obj = js_sys::Object::new();
    set_prop(&obj, "en", &text.en.clone().into());
    set_prop(&obj, "ja", &text.ja.clone().into());
    obj.into()
}

/// The absence envelope the current protocol observes: the reason, plus the
/// diagnosis when the runtime produced one. An absence's `origin` and
/// `recoverability` are diagnostic state rather than wire fields, so they are
/// not reconstructed here.
fn absence_to_protocol_js(absence: &crate::semantic::AbsenceMetadata) -> JsValue {
    let obj = js_sys::Object::new();
    if let Some(reason) = &absence.reason {
        set_prop(&obj, "reason", &reason.as_protocol_str().into());
    }
    if let Some(diagnosis) = &absence.diagnosis {
        set_prop(&obj, "diagnosis", &diagnosis_to_protocol_js(diagnosis));
    }
    obj.into()
}

/// The `semantics` metadata bag the current protocol carries. The retired
/// HostProtocolV1 also spelled `semanticKind`, `shape`, `capabilities`, and
/// `origin` here; the value domains discriminate themselves through `type`, so
/// those axes described the same six domains a second time and no reader ever
/// consulted them.
fn value_semantics_to_js(value: &Value, effective: Interpretation) -> JsValue {
    let obj = js_sys::Object::new();
    // The `truthValue` axis (SPEC §2.3) is the only observable surface for
    // the three-valued logic: `true` / `false` / `unknown`. It is derived
    // from the *effective* interpretation role, because a definite boolean
    // carries the `TruthValue` role in the semantic plane rather than on the
    // value's own hint (SPEC §12.2). Present only on truth-valued values.
    let truth = value.truth_value_for_role(effective);
    if let Some(truth) = truth {
        set_prop(&obj, "truthValue", &truth.into());
    }
    if let Some(absence) = value.normalized_absence_metadata() {
        set_prop(&obj, "absence", &absence_to_protocol_js(&absence));
    }
    // Exact-irrational firewall marker (SPEC §2.3): an `ExactScalar` rendered
    // under any role other than the lossless ContinuedFraction form is shown
    // as a *best rational approximation* (see `value_to_protocol`). Without a
    // marker its `number` value is indistinguishable from an exact rational,
    // which contradicts Ajisai's "no hidden truncation" guarantee. This is an
    // additive, optional field on the `semantics` metadata bag: existing
    // consumers ignore it; the GUI can use it to prefix an `≈`. ContinuedFraction
    // nodes carry no `semantics` block, so they never reach here.
    if matches!(value.data, ValueData::ExactScalar(_))
        && effective != Interpretation::ContinuedFraction
    {
        set_prop(&obj, "approximate", &JsValue::TRUE);
    }
    // The exact value itself, when there is a short way to write it. An
    // algebraic irrational is *stored* as the multiquadratic normal form
    // Σ c_m √m (SPEC §4.2), so these pairs are the number rather than a view of
    // it, and a host given them can draw `√3` or `1/2 + 1/3√5` instead of
    // choosing between a thirty-line continued fraction and an approximation.
    // Additive and optional: a host that ignores it sees exactly what it saw
    // before.
    if let Some(exact_terms) = exact_terms(value) {
        let terms = js_sys::Array::new();
        for exact_term in exact_terms {
            let term = js_sys::Object::new();
            set_prop(&term, "numerator", &exact_term.numerator.into());
            set_prop(&term, "denominator", &exact_term.denominator.into());
            set_prop(&term, "radicand", &exact_term.radicand.into());
            terms.push(&term.into());
        }
        set_prop(&obj, "exactTerms", &terms.into());
    }
    obj.into()
}

// The pure (Value, hint) -> protocol mapping (`ProtocolNode`,
// `value_to_protocol`) lives in `crate::types::value_protocol` so the native
// CLI shares the exact same wire format. Extracting it out of the `JsValue`
// glue also lets the entire decision be unit / MC/DC / property tested
// natively (AQ-REQ-003, `types/value_protocol_tests.rs`), with
// `protocol_to_js` reduced to a mechanical shim.

/// Mechanical shim: render a `ProtocolNode` into the `JsValue` the GUI
/// receives. Carries no decision logic — every behavioral choice lives in
/// `value_to_protocol`, which is verified natively.
fn protocol_to_js(node: &ProtocolNode) -> JsValue {
    let obj = js_sys::Object::new();
    set_prop(
        &obj,
        "displayHint",
        &interpretation_protocol_str(node.display_hint).into(),
    );
    if let Some(source) = &node.semantics {
        set_prop(
            &obj,
            "semantics",
            &value_semantics_to_js(source, node.display_hint),
        );
    }
    set_prop(&obj, "type", &node.type_str.into());
    match &node.value {
        ProtocolValue::Null => set_prop(&obj, "value", &JsValue::NULL),
        ProtocolValue::Bool(b) => set_prop(&obj, "value", &(*b).into()),
        ProtocolValue::Text(s) => set_prop(&obj, "value", &s.clone().into()),
        ProtocolValue::Number {
            numerator,
            denominator,
        } => {
            let num_obj = js_sys::Object::new();
            set_prop(&num_obj, "numerator", &numerator.clone().into());
            set_prop(&num_obj, "denominator", &denominator.clone().into());
            set_prop(&obj, "value", &num_obj.into());
        }
        ProtocolValue::Children(kids) => {
            let arr = js_sys::Array::new();
            for kid in kids {
                arr.push(&protocol_to_js(kid));
            }
            set_prop(&obj, "value", &arr.into());
        }
    }
    obj.into()
}

pub(crate) fn value_to_js(value: &Value, external_hint_opt: Option<Interpretation>) -> JsValue {
    protocol_to_js(&value_to_protocol(value, external_hint_opt))
}

fn tensor_data_to_js_array(
    data: &[crate::types::fraction::Fraction],
    shape: &[usize],
    leaf_hint: Interpretation,
) -> js_sys::Array {
    // Mirror the Vector serialization path: only the TruthValue role is
    // propagated to leaves (numbers otherwise). A promoted dense boolean
    // vector must render its elements as booleans, matching the Display
    // path's `format_as_boolean`.
    let leaves_are_bool = leaf_hint == Interpretation::TruthValue;
    let arr = js_sys::Array::new();
    if shape.is_empty() || shape.len() == 1 {
        for f in data {
            let elem = js_sys::Object::new();
            if leaves_are_bool {
                js_sys::Reflect::set(&elem, &"type".into(), &"boolean".into()).unwrap();
                js_sys::Reflect::set(&elem, &"value".into(), &(!f.is_zero()).into()).unwrap();
                js_sys::Reflect::set(&elem, &"displayHint".into(), &"truthValue".into()).unwrap();
            } else {
                let num_obj = js_sys::Object::new();
                js_sys::Reflect::set(
                    &num_obj,
                    &"numerator".into(),
                    &f.numerator().to_string().into(),
                )
                .unwrap();
                js_sys::Reflect::set(
                    &num_obj,
                    &"denominator".into(),
                    &f.denominator().to_string().into(),
                )
                .unwrap();
                js_sys::Reflect::set(&elem, &"type".into(), &"number".into()).unwrap();
                js_sys::Reflect::set(&elem, &"value".into(), &num_obj).unwrap();
                js_sys::Reflect::set(&elem, &"displayHint".into(), &"rawNumber".into()).unwrap();
            }
            let element_value = Value::from_fraction(f.clone());
            let leaf_role = if leaves_are_bool {
                Interpretation::TruthValue
            } else {
                Interpretation::RawNumber
            };
            js_sys::Reflect::set(
                &elem,
                &"semantics".into(),
                &value_semantics_to_js(&element_value, leaf_role),
            )
            .unwrap();
            arr.push(&elem);
        }
    } else {
        let outer = shape[0];
        let rest = &shape[1..];
        let stride: usize = rest.iter().product();
        let inner_hint_str = if leaves_are_bool {
            "truthValue"
        } else {
            "unassigned"
        };
        for i in 0..outer {
            let inner =
                tensor_data_to_js_array(&data[i * stride..(i + 1) * stride], rest, leaf_hint);
            let elem = js_sys::Object::new();
            js_sys::Reflect::set(&elem, &"type".into(), &"vector".into()).unwrap();
            js_sys::Reflect::set(&elem, &"value".into(), &inner).unwrap();
            js_sys::Reflect::set(&elem, &"displayHint".into(), &inner_hint_str.into()).unwrap();
            arr.push(&elem);
        }
    }
    arr
}

#[allow(dead_code)]
pub(crate) fn arena_node_to_js(
    arena: &ValueArena,
    root_id: NodeId,
    external_hint_opt: Option<Interpretation>,
) -> JsValue {
    let obj = js_sys::Object::new();
    // external_hint_opt が無い場合は必ず Arena 側の hint を参照する。
    // 子ノード再帰では None を渡し、各 NodeId の明示 hint を尊重する。
    let effective_hint = resolve_effective_hint(arena, root_id, external_hint_opt);

    let hint_str: &str = interpretation_protocol_str(effective_hint);
    js_sys::Reflect::set(&obj, &"displayHint".into(), &hint_str.into()).unwrap();

    match arena.kind(root_id) {
        // The node carries its reason, but the protocol spells absence through
        // the separate `absence` envelope rather than on this display object.
        // Surfacing the reason here would be a protocol change, not a
        // rendering one.
        NodeKind::Nil(_) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        }
        NodeKind::Boolean(b) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"boolean".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &(*b).into()).unwrap();
        }
        NodeKind::Text(text) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"string".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &(&**text).into()).unwrap();
        }
        NodeKind::Scalar(f) => {
            let scalar_type = match effective_hint {
                Interpretation::TruthValue => "boolean",
                Interpretation::Timestamp => "datetime",
                _ => "number",
            };
            js_sys::Reflect::set(&obj, &"type".into(), &scalar_type.into()).unwrap();
            match scalar_type {
                "boolean" => {
                    js_sys::Reflect::set(&obj, &"value".into(), &(!f.is_zero()).into()).unwrap();
                }
                "string" => {
                    let as_char = f
                        .to_i64()
                        .and_then(|n| char::from_u32(n as u32))
                        .map(|c| c.to_string())
                        .unwrap_or_default();
                    js_sys::Reflect::set(&obj, &"value".into(), &as_char.into()).unwrap();
                }
                _ => {
                    let num_obj = js_sys::Object::new();
                    js_sys::Reflect::set(
                        &num_obj,
                        &"numerator".into(),
                        &f.numerator().to_string().into(),
                    )
                    .unwrap();
                    js_sys::Reflect::set(
                        &num_obj,
                        &"denominator".into(),
                        &f.denominator().to_string().into(),
                    )
                    .unwrap();
                    js_sys::Reflect::set(&obj, &"value".into(), &num_obj).unwrap();
                }
            }
        }
        NodeKind::Vector { children } => {
            let child_external: Option<Interpretation> = match effective_hint {
                Interpretation::TruthValue => Some(Interpretation::TruthValue),
                _ => None,
            };
            let js_array = js_sys::Array::new();
            for child in children {
                js_array.push(&arena_node_to_js(arena, *child, child_external));
            }
            js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
        }
        NodeKind::Tensor { data, shape } => {
            // Hydrate a dense Tensor at the WASM boundary so the GUI/TS layer
            // can keep treating values uniformly as nested Vectors.
            let js_array = tensor_data_to_js_array(data, shape, effective_hint);
            js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
        }
        NodeKind::CodeBlock(_) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        }
    }

    obj.into()
}

#[allow(dead_code)]
fn resolve_effective_hint(
    arena: &ValueArena,
    root_id: NodeId,
    external_hint_opt: Option<Interpretation>,
) -> Interpretation {
    external_hint_opt.unwrap_or_else(|| arena.hint(root_id))
}

#[cfg(test)]
mod test_input_helper {
    use super::{build_bracket_structure_from_shape, resolve_effective_hint};
    use crate::types::arena::ValueArena;
    use crate::types::Interpretation;

    #[test]
    fn test_build_bracket_structure_from_shape() {
        assert_eq!(build_bracket_structure_from_shape(&[1]), "[ ]");
        assert_eq!(build_bracket_structure_from_shape(&[2]), "[ ] [ ]");
        assert_eq!(build_bracket_structure_from_shape(&[3]), "[ ] [ ] [ ]");

        assert_eq!(build_bracket_structure_from_shape(&[1, 1]), "[ [ ] ]");
        assert_eq!(build_bracket_structure_from_shape(&[1, 2]), "[ [ ] [ ] ]");
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 3]),
            "[ [ ] [ ] [ ] ]"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[2, 3]),
            "[ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ]"
        );

        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 1]),
            "[ [ [ ] ] ]"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 2]),
            "[ [ [ ] [ ] ] ]"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 2, 3]),
            "[ [ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ] ]"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[2, 2, 3]),
            "[ [ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ] ] [ [ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ] ]"
        );

        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 1, 1]),
            "[ [ [ [ ] ] ] ]"
        );
    }

    #[test]
    fn effective_hint_prefers_external_otherwise_uses_arena() {
        let mut arena = ValueArena::new();
        let id = arena.alloc_string("AB");
        assert_eq!(
            resolve_effective_hint(&arena, id, None),
            Interpretation::Text
        );
        assert_eq!(
            resolve_effective_hint(&arena, id, Some(Interpretation::RawNumber)),
            Interpretation::RawNumber
        );
    }
}

// AQ-VER-003: WASM boundary MC/DC tests for QL-B pure helpers.
//
// Scope: the JS-bridge conversion layer is reachable natively only for
// its pure helpers (`resolve_effective_hint`,
// `build_bracket_structure_from_shape`). JsValue-based entry points
// (`js_value_to_value`, `arena_node_to_js`, `extract_display_hint_from_js`)
// exercise `wasm_bindgen` runtime glue and are verified by the
// `cargo check --target wasm32-unknown-unknown` step in
// `.github/workflows/test.yml` (AQ-REQ-003). They are intentionally not
// asserted here.
//
// Trace: docs/quality/TRACEABILITY_MATRIX.md, requirement AQ-REQ-003.
#[cfg(test)]
mod mcdc_tests {
    use super::{build_bracket_structure_from_shape, resolve_effective_hint};
    use crate::types::arena::ValueArena;
    use crate::types::Interpretation;

    // AQ-VER-003-A
    // DUT: `resolve_effective_hint`
    //     external_hint_opt.unwrap_or_else(|| arena.hint(root_id))
    //
    // One atomic condition C = external_hint_opt.is_some().
    //   row 1: C=T -> return external value verbatim
    //   row 2: C=F -> fall back to arena hint
    //
    // Additional row 3 pins that C=T ignores the arena hint even when
    // the external value disagrees — this matters because a caller
    // passing an explicit hint must win over arena state.
    mod aq_ver_003_a_resolve_effective_hint {
        use super::*;

        #[test]
        fn row1_some_external_is_returned_verbatim() {
            let mut arena = ValueArena::new();
            let id = arena.alloc_nil(Interpretation::RawNumber);
            assert_eq!(
                resolve_effective_hint(&arena, id, Some(Interpretation::TruthValue)),
                Interpretation::TruthValue,
            );
        }

        #[test]
        fn row2_none_falls_back_to_arena_hint() {
            let mut arena = ValueArena::new();
            let id = arena.alloc_nil(Interpretation::Timestamp);
            assert_eq!(
                resolve_effective_hint(&arena, id, None),
                Interpretation::Timestamp,
            );
        }

        #[test]
        fn external_hint_wins_even_when_arena_disagrees() {
            // Guards against a regression where the fallback arm is
            // evaluated eagerly and overwrites the external value.
            let mut arena = ValueArena::new();
            let id = arena.alloc_nil(Interpretation::RawNumber);
            assert_eq!(
                resolve_effective_hint(&arena, id, Some(Interpretation::Text)),
                Interpretation::Text,
            );
        }
    }

    // AQ-VER-003-B
    // DUT: `build_bracket_structure_from_shape`
    //
    // Outer decision: `if shape.is_empty()` — one atomic condition.
    //   row 1: empty shape -> literal "[ ]"
    //   row 2: non-empty shape -> recurse
    //
    // Inner decision (in `build_level`): `if shape.len() == 1`.
    //   row 3: tail dimension -> emit `[ ]` repeated `shape[0]` times
    //   row 4: non-tail dimension -> wrap the inner level
    //
    // The existing `test_build_bracket_structure_from_shape` covers
    // several combinations in row 3/4 already. This module adds the
    // outer-empty boundary (row 1), which was previously untested, and
    // asserts the leaf-count invariant to make the MC/DC intent explicit.
    mod aq_ver_003_b_bracket_structure {
        use super::*;

        #[test]
        fn row1_empty_shape_returns_single_pair() {
            assert_eq!(build_bracket_structure_from_shape(&[]), "[ ]");
        }

        #[test]
        fn row2_single_dim_emits_n_leaves() {
            // Complements row 1 by flipping `shape.is_empty()`.
            let out = build_bracket_structure_from_shape(&[4]);
            assert_eq!(out, "[ ] [ ] [ ] [ ]");
            assert_eq!(
                out.matches("[ ]").count(),
                4,
                "leaf count must equal shape[0] on the tail dimension"
            );
        }

        #[test]
        fn row3_row4_multi_dim_wraps_inner_levels() {
            // Non-tail dimension wraps tail output in brackets.
            // Shape [2, 3]: 2 outer frames, each containing 3 leaves.
            let out = build_bracket_structure_from_shape(&[2, 3]);
            assert_eq!(out, "[ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ]");
            assert_eq!(
                out.matches("[ ]").count(),
                6,
                "leaf count must equal the product of non-head dims"
            );
        }
    }
}
