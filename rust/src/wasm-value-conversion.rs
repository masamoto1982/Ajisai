use crate::types::arena::{NodeId, NodeKind, ValueArena};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, Value};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct UserWordData {
    pub(crate) dictionary: Option<String>,
    pub(crate) name: String,
    pub(crate) definition: Option<String>,
    pub(crate) description: Option<String>,
}

pub(crate) fn bracket_chars_for_depth(depth: usize) -> (char, char) {
    let _ = depth;
    ('[', ']')
}

pub(crate) fn build_bracket_structure_from_shape(shape: &[usize]) -> String {
    fn build_level(shape: &[usize], depth: usize) -> String {
        let (open, close) = bracket_chars_for_depth(depth);
        if shape.len() == 1 {
            let empty = format!("{} {}", open, close);
            (0..shape[0])
                .map(|_| empty.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            let inner = build_level(&shape[1..], depth + 1);
            let one_element = format!("{} {} {}", open, inner, close);
            (0..shape[0])
                .map(|_| one_element.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
    if shape.is_empty() {
        return "[ ]".to_string();
    }
    build_level(shape, 0)
}

pub(crate) fn is_vector_value(val: &Value) -> bool {
    val.is_vector()
}

pub(crate) fn js_value_to_value(js_val: JsValue) -> Result<Value, String> {
    let obj = js_sys::Object::from(js_val);
    let type_str = js_sys::Reflect::get(&obj, &"type".into())
        .map_err(|_| "Failed to get 'type' property".to_string())?
        .as_string()
        .ok_or("Type not string")?;
    let value_js = js_sys::Reflect::get(&obj, &"value".into())
        .map_err(|_| "Failed to get 'value' property".to_string())?;

    match type_str.as_str() {
        "number" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into())
                .map_err(|_| "No numerator".to_string())?
                .as_string()
                .ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into())
                .map_err(|_| "No denominator".to_string())?
                .as_string()
                .ok_or("Denominator not string")?;
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
            );
            Ok(Value::from_fraction(fraction))
        }
        "datetime" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into())
                .map_err(|_| "No numerator".to_string())?
                .as_string()
                .ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into())
                .map_err(|_| "No denominator".to_string())?
                .as_string()
                .ok_or("Denominator not string")?;
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
            );
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
                vec.push(js_value_to_value(js_array.get(i))?);
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
                let frac_obj = js_sys::Object::from(data_array.get(i));
                let num_str = js_sys::Reflect::get(&frac_obj, &"numerator".into())
                    .map_err(|_| "No numerator in tensor data".to_string())?
                    .as_string()
                    .ok_or("Numerator not string")?;
                let den_str = js_sys::Reflect::get(&frac_obj, &"denominator".into())
                    .map_err(|_| "No denominator in tensor data".to_string())?
                    .as_string()
                    .ok_or("Denominator not string")?;
                let fraction = Fraction::new(
                    BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                    BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
                );
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

pub(crate) fn arena_node_to_js(
    arena: &ValueArena,
    root_id: NodeId,
    external_hint_opt: Option<DisplayHint>,
) -> JsValue {
    let obj = js_sys::Object::new();
    // external_hint_opt が無い場合は必ず Arena 側の hint を参照する。
    // 子ノード再帰では None を渡し、各 NodeId の明示 hint を尊重する。
    let effective_hint = resolve_effective_hint(arena, root_id, external_hint_opt);

    let hint_str: &str = match effective_hint {
        DisplayHint::Auto => "auto",
        DisplayHint::Number => "number",
        DisplayHint::Interval => "interval",
        DisplayHint::String => "string",
        DisplayHint::Boolean => "boolean",
        DisplayHint::DateTime => "datetime",
        DisplayHint::Nil => "nil",
    };
    js_sys::Reflect::set(&obj, &"displayHint".into(), &hint_str.into()).unwrap();

    match arena.kind(root_id) {
        NodeKind::Nil => {
            js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        }
        NodeKind::Scalar(f) => {
            let scalar_type = match effective_hint {
                DisplayHint::Boolean => "boolean",
                DisplayHint::DateTime => "datetime",
                DisplayHint::String => "string",
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
            if effective_hint == DisplayHint::String {
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
                let child_external: Option<DisplayHint> = match effective_hint {
                    DisplayHint::Boolean => Some(DisplayHint::Boolean),
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

fn resolve_effective_hint(
    arena: &ValueArena,
    root_id: NodeId,
    external_hint_opt: Option<DisplayHint>,
) -> DisplayHint {
    external_hint_opt.unwrap_or_else(|| arena.hint(root_id))
}

pub(crate) fn extract_display_hint_from_js(js_val: &JsValue) -> DisplayHint {
    let obj = js_sys::Object::from(js_val.clone());
    let hint_js = js_sys::Reflect::get(&obj, &"displayHint".into()).unwrap_or(JsValue::UNDEFINED);
    match hint_js.as_string().as_deref() {
        Some("number") => DisplayHint::Number,
        Some("interval") => DisplayHint::Interval,
        Some("string") => DisplayHint::String,
        Some("boolean") => DisplayHint::Boolean,
        Some("datetime") => DisplayHint::DateTime,
        Some("nil") => DisplayHint::Nil,
        _ => DisplayHint::Auto,
    }
}

#[cfg(test)]
mod test_input_helper {
    use super::{build_bracket_structure_from_shape, resolve_effective_hint};
    use crate::types::arena::ValueArena;
    use crate::types::DisplayHint;

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
            DisplayHint::String
        );
        assert_eq!(
            resolve_effective_hint(&arena, id, Some(DisplayHint::Number)),
            DisplayHint::Number
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
    use crate::types::DisplayHint;

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
            let id = arena.alloc_nil(DisplayHint::Number);
            assert_eq!(
                resolve_effective_hint(&arena, id, Some(DisplayHint::Boolean)),
                DisplayHint::Boolean,
            );
        }

        #[test]
        fn row2_none_falls_back_to_arena_hint() {
            let mut arena = ValueArena::new();
            let id = arena.alloc_nil(DisplayHint::DateTime);
            assert_eq!(
                resolve_effective_hint(&arena, id, None),
                DisplayHint::DateTime,
            );
        }

        #[test]
        fn external_hint_wins_even_when_arena_disagrees() {
            // Guards against a regression where the fallback arm is
            // evaluated eagerly and overwrites the external value.
            let mut arena = ValueArena::new();
            let id = arena.alloc_nil(DisplayHint::Number);
            assert_eq!(
                resolve_effective_hint(&arena, id, Some(DisplayHint::String)),
                DisplayHint::String,
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
