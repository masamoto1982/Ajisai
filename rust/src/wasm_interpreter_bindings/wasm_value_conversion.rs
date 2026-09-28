// `js_sys::Reflect::set(...).unwrap()` 群について:
// 直前に `js_sys::Object::new()` で生成したフレッシュなプレーン JS オブジェクト
// に対する set のため、Proxy ハンドラや凍結など失敗要因は実質的に発生しない。
// それでも万一 set が失敗した場合は console_error_panic_hook 経由で
// ブラウザコンソールにスタックトレースが出るので、原因解析は可能。

use crate::types::value_protocol::{value_to_protocol, ProtocolNode, ProtocolValue};
use crate::types::Value;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct UserWordData {
    pub(crate) name: String,
    pub(crate) definition: Option<String>,
    /// The `#:contract`-derived hover text (see `execute_def::set_word_
    /// description`), round-tripped through save/export so it survives a
    /// restore rather than existing only for the session that typed it.
    #[serde(default)]
    pub(crate) description: Option<String>,
}

fn set_prop(obj: &js_sys::Object, key: &str, value: &JsValue) {
    js_sys::Reflect::set(obj, &key.into(), value).unwrap();
}

/// The `semantics` bag, rendered by the one serializer both hosts share
/// (`agent::report::semantics_json`) and converted to a plain JS object.
///
/// This boundary used to build the same bag by hand, field by field, and the
/// two copies had drifted: the WASM absence dropped `origin` and
/// `recoverability`, and its diagnosis omitted `progress` from a resource
/// limit — two spellings of one protocol, where LANG.OBSERVATION.PROTOCOL
/// promises one. Converting the shared rendering makes a third copy
/// impossible to write by accident.
fn value_semantics_to_js(value: &Value) -> JsValue {
    use serde::Serialize as _;
    crate::agent::report::semantics_json(value)
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .expect("a serde_json value always converts to a JS value")
}

// The pure Value -> protocol mapping (`ProtocolNode`,
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
    if let Some(source) = &node.semantics {
        set_prop(&obj, "semantics", &value_semantics_to_js(source));
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
        ProtocolValue::Record { keys, values } => {
            let record_obj = js_sys::Object::new();
            let key_arr = js_sys::Array::new();
            for key in keys {
                key_arr.push(&protocol_to_js(key));
            }
            let value_arr = js_sys::Array::new();
            for value in values {
                value_arr.push(&protocol_to_js(value));
            }
            set_prop(&record_obj, "keys", &key_arr.into());
            set_prop(&record_obj, "values", &value_arr.into());
            set_prop(&obj, "value", &record_obj.into());
        }
    }
    obj.into()
}

pub(crate) fn value_to_js(value: &Value) -> JsValue {
    protocol_to_js(&value_to_protocol(value))
}
