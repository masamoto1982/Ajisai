// `js_sys::Reflect::set(...).unwrap()` 群について:
// 直前に `js_sys::Object::new()` で生成したフレッシュなプレーン JS オブジェクト
// に対する set のため、Proxy ハンドラや凍結など失敗要因は実質的に発生しない。
// それでも万一 set が失敗した場合は console_error_panic_hook 経由で
// ブラウザコンソールにスタックトレースが出るので、原因解析は可能。

use crate::types::arena::{NodeId, NodeKind, ValueArena};
use crate::types::fraction::Fraction;
use crate::types::value_protocol::{
    interpretation_protocol_str, value_to_protocol, ProtocolNode, ProtocolValue,
};
use crate::types::{Interpretation, Value, ValueData};
use num_bigint::BigInt;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// Cap on how deeply a JS value may nest when deserialized at the boundary
/// (`restore_stack` / `restore_user_words`). `js_value_to_value` recurses one
/// frame per `vector` level, and the resulting `Value` is then traversed
/// recursively (display, the derived `Drop`, JSON conversions). A hostile or
/// corrupted restored snapshot with thousands of nesting levels would overflow
/// the WASM stack — an unrecoverable trap — instead of yielding a recoverable
/// error. Matches the interpreter's WASM-vetted recursion envelope.
const MAX_BOUNDARY_NESTING_DEPTH: usize = 256;

/// Parse a `{ numerator, denominator }` JS object into a `Fraction`, rejecting a
/// zero denominator before it reaches `Fraction::new` (which *panics* on a zero
/// denominator — an unrecoverable WASM trap when the value comes from untrusted
/// restored state).
fn parse_js_fraction(obj: &js_sys::Object) -> Result<Fraction, String> {
    let num_str = js_sys::Reflect::get(obj, &"numerator".into())
        .map_err(|_| "No numerator".to_string())?
        .as_string()
        .ok_or("Numerator not string")?;
    let den_str = js_sys::Reflect::get(obj, &"denominator".into())
        .map_err(|_| "No denominator".to_string())?
        .as_string()
        .ok_or("Denominator not string")?;
    let numerator = BigInt::from_str(&num_str).map_err(|e| e.to_string())?;
    let denominator = BigInt::from_str(&den_str).map_err(|e| e.to_string())?;
    if denominator.is_zero() {
        return Err("denominator is zero".to_string());
    }
    Ok(Fraction::new(numerator, denominator))
}

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

pub(crate) fn js_value_to_value(js_val: JsValue) -> Result<Value, String> {
    js_value_to_value_with_depth(js_val, 0)
}

fn js_value_to_value_with_depth(js_val: JsValue, depth: usize) -> Result<Value, String> {
    // Bound recursion before descending: a deeply nested untrusted value would
    // otherwise overflow the WASM stack (an unrecoverable trap) here and in
    // every later traversal of the resulting Value.
    if depth > MAX_BOUNDARY_NESTING_DEPTH {
        return Err("value nesting too deep".to_string());
    }

    let obj = js_sys::Object::from(js_val);
    let type_str = js_sys::Reflect::get(&obj, &"type".into())
        .map_err(|_| "Failed to get 'type' property".to_string())?
        .as_string()
        .ok_or("Type not string")?;
    let value_js = js_sys::Reflect::get(&obj, &"value".into())
        .map_err(|_| "Failed to get 'value' property".to_string())?;

    match type_str.as_str() {
        "number" => {
            let fraction = parse_js_fraction(&js_sys::Object::from(value_js))?;
            Ok(Value::from_fraction(fraction))
        }
        "datetime" => {
            let fraction = parse_js_fraction(&js_sys::Object::from(value_js))?;
            Ok(Value::from_datetime(fraction))
        }
        "string" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_string(&s))
        }
        "boolean" => {
            let b = value_js.as_bool().ok_or("Value not boolean")?;
            Ok(Value::from_bool(b))
        }
        "symbol" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_symbol(&s))
        }
        "vector" => {
            let js_array = js_sys::Array::from(&value_js);
            let mut vec = Vec::new();
            for i in 0..js_array.length() {
                vec.push(js_value_to_value_with_depth(js_array.get(i), depth + 1)?);
            }
            Ok(Value::from_vector(vec))
        }
        "tensor" => {
            let tensor_obj = js_sys::Object::from(value_js);

            let data_js = js_sys::Reflect::get(&tensor_obj, &"data".into())
                .map_err(|_| "No data in tensor".to_string())?;
            let data_array = js_sys::Array::from(&data_js);
            let mut fractions = Vec::new();
            for i in 0..data_array.length() {
                let fraction = parse_js_fraction(&js_sys::Object::from(data_array.get(i)))?;
                fractions.push(fraction);
            }

            let children: Vec<Value> = fractions.into_iter().map(Value::from_fraction).collect();

            Ok(Value::from_children(children))
        }
        "nil" => Ok(Value::nil()),
        "process_handle" => {
            let id = value_js.as_f64().ok_or("Process handle id is not number")? as u64;
            Ok(Value::from_process_handle(id))
        }
        "supervisor_handle" => {
            let id = value_js
                .as_f64()
                .ok_or("Supervisor handle id is not number")? as u64;
            Ok(Value::from_supervisor_handle(id))
        }
        _ => Err(format!("Unknown type: {}", type_str)),
    }
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
    if let Some(module) = &diagnosis.where_.module {
        set_prop(&where_obj, "module", &module.clone().into());
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
        set_prop(&check_obj, "label", &c.label.clone().into());
        set_prop(&check_obj, "detail", &c.detail.clone().into());
        checks_arr.push(&check_obj);
    }
    set_prop(&obj, "nextChecks", &checks_arr.into());

    // CF-comparison agreed-prefix (SPEC §4.5.0 / §7.4.1): machine-readable
    // count of leading partial quotients that matched before an Unknown (U)
    // comparison gave up. Emitted only when present.
    if let Some(prefix) = diagnosis.agreed_prefix {
        set_prop(&obj, "agreedPrefix", &(prefix as f64).into());
    }
    obj.into()
}

fn absence_to_protocol_js(absence: &crate::semantic::AbsenceMetadata) -> JsValue {
    let obj = js_sys::Object::new();
    if let Some(reason) = &absence.reason {
        set_prop(&obj, "reason", &reason.as_protocol_str().into());
    }
    set_prop(&obj, "origin", &absence.origin.as_protocol_str().into());
    set_prop(
        &obj,
        "recoverability",
        &absence.recoverability.as_protocol_str().into(),
    );
    if let Some(diagnosis) = &absence.diagnosis {
        set_prop(&obj, "diagnosis", &diagnosis_to_protocol_js(diagnosis));
    }
    obj.into()
}

fn value_semantics_to_js(value: &Value, effective: Interpretation) -> JsValue {
    let obj = js_sys::Object::new();
    set_prop(
        &obj,
        "semanticKind",
        &value.semantic_kind().as_protocol_str().into(),
    );
    set_prop(&obj, "shape", &value.shape_kind().as_protocol_str().into());
    // The `truthValue` axis (SPEC §2.3) is the only observable surface for
    // the three-valued logic: `true` / `false` / `unknown`. It is derived
    // from the *effective* interpretation role, because a definite boolean
    // carries the `TruthValue` role in the semantic plane rather than on the
    // value's own hint (SPEC §12.2). Present only on truth-valued values.
    let truth = value.truth_value_for_role(effective);
    if let Some(truth) = truth {
        set_prop(&obj, "truthValue", &truth.into());
    }
    let capabilities = js_sys::Array::new();
    let mut has_truth_valued = false;
    for capability in value.capabilities() {
        if capability == crate::semantic::Capability::TruthValued {
            has_truth_valued = true;
        }
        capabilities.push(&JsValue::from_str(capability.as_protocol_str()));
    }
    // A value rendered under the TruthValue role advertises `truthValued`
    // even when the role lives in the semantic plane (comparison/logic
    // booleans), not on the value's own hint.
    if truth.is_some() && !has_truth_valued {
        capabilities.push(&JsValue::from_str(
            crate::semantic::Capability::TruthValued.as_protocol_str(),
        ));
    }
    set_prop(&obj, "capabilities", &capabilities.into());
    set_prop(&obj, "origin", &value.origin().as_protocol_str().into());
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
        ProtocolValue::Handle(id) => set_prop(&obj, "value", &(*id as f64).into()),
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
        NodeKind::Nil => {
            js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        }
        NodeKind::Boolean(b) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"boolean".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &(*b).into()).unwrap();
        }
        NodeKind::Scalar(f) => {
            let scalar_type = match effective_hint {
                Interpretation::TruthValue => "boolean",
                Interpretation::Timestamp => "datetime",
                Interpretation::Text => "string",
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
            if effective_hint == Interpretation::Text {
                let text = children
                    .iter()
                    .filter_map(|child| match arena.kind(*child) {
                        NodeKind::Scalar(codepoint) => {
                            codepoint.to_i64().and_then(|n| char::from_u32(n as u32))
                        }
                        _ => None,
                    })
                    .collect::<String>();
                js_sys::Reflect::set(&obj, &"type".into(), &"string".into()).unwrap();
                js_sys::Reflect::set(&obj, &"value".into(), &text.into()).unwrap();
            } else {
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
        }
        NodeKind::Tensor { data, shape } => {
            // Hydrate a dense Tensor at the WASM boundary so the GUI/TS layer
            // can keep treating values uniformly as nested Vectors.
            if effective_hint == Interpretation::Text && shape.len() <= 1 {
                let text: String = data
                    .iter()
                    .filter_map(|f| f.to_i64().and_then(|n| char::from_u32(n as u32)))
                    .collect();
                js_sys::Reflect::set(&obj, &"type".into(), &"string".into()).unwrap();
                js_sys::Reflect::set(&obj, &"value".into(), &text.into()).unwrap();
            } else {
                let js_array = tensor_data_to_js_array(data, shape, effective_hint);
                js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
                js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
            }
        }
        NodeKind::Record { pairs, .. } => {
            let js_array = js_sys::Array::new();
            for pair_id in pairs {
                js_array.push(&arena_node_to_js(arena, *pair_id, None));
            }
            js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
        }
        NodeKind::CodeBlock(_) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        }
        NodeKind::ProcessHandle(id) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"process_handle".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &(*id as f64).into()).unwrap();
        }
        NodeKind::SupervisorHandle(id) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"supervisor_handle".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &(*id as f64).into()).unwrap();
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

pub(crate) fn extract_display_hint_from_js(js_val: &JsValue) -> Interpretation {
    let obj = js_sys::Object::from(js_val.clone());
    let hint_js = js_sys::Reflect::get(&obj, &"displayHint".into()).unwrap_or(JsValue::UNDEFINED);
    match hint_js.as_string().as_deref() {
        Some("rawNumber") => Interpretation::RawNumber,
        Some("interval") => Interpretation::Interval,
        Some("text") => Interpretation::Text,
        Some("truthValue") => Interpretation::TruthValue,
        Some("timestamp") => Interpretation::Timestamp,
        Some("nil") => Interpretation::Nil,
        // Legacy role names from snapshots persisted before the
        // interpretation-role redesign. Accepted so a saved stack restored
        // after an upgrade keeps its roles (a saved string would otherwise
        // restore as an Unassigned codepoint vector).
        Some("number") => Interpretation::RawNumber,
        Some("string") => Interpretation::Text,
        Some("boolean") => Interpretation::TruthValue,
        Some("datetime") => Interpretation::Timestamp,
        _ => Interpretation::Unassigned,
    }
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
