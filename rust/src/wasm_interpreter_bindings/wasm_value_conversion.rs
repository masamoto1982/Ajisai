// `js_sys::Reflect::set(...).unwrap()` 群について:
// 直前に `js_sys::Object::new()` で生成したフレッシュなプレーン JS オブジェクト
// に対する set のため、Proxy ハンドラや凍結など失敗要因は実質的に発生しない。
// それでも万一 set が失敗した場合は console_error_panic_hook 経由で
// ブラウザコンソールにスタックトレースが出るので、原因解析は可能。

use crate::types::value_protocol::{
    exact_display, exact_terms, interpretation_protocol_str, value_to_protocol, ProtocolNode,
    ProtocolValue,
};
use crate::types::{Interpretation, Value, ValueData};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct UserWordData {
    pub(crate) dictionary: Option<String>,
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

    // CF-comparison agreed-prefix (LANG.VALUES.NIL / LANG.VALUES.EXACT): machine-readable
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
    if let Some(detail) = &absence.detail {
        set_prop(&obj, "detail", &detail.as_str().into());
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
    // The `truthValue` axis (LANG.VALUES.TRUTH) is the only observable surface
    // for the three-valued logic: `true` / `false` / `unknown`. It is derived
    // from the *effective* interpretation role, because a definite boolean
    // carries the `TruthValue` role in the semantic plane rather than on the
    // value's own hint. Present only on truth-valued values.
    let truth = value.truth_value_for_role(effective);
    if let Some(truth) = truth {
        set_prop(&obj, "truthValue", &truth.into());
    }
    if let Some(absence) = value.normalized_absence_metadata() {
        set_prop(&obj, "absence", &absence_to_protocol_js(&absence));
    }
    // Exact-irrational firewall marker (LANG.OBSERVATION.FIREWALL): an `ExactScalar` rendered
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
    // Σ c_m √m (LANG.VALUES.EXACT), so these pairs are the number rather than a view of
    // it, and a host given them can draw `√3` or `1/2 + 1/3√5` instead of
    // choosing between a thirty-line continued fraction and an approximation.
    // Additive and optional: a host that ignores it sees exactly what it saw
    // before.
    if let Some(display) = exact_display(value) {
        set_prop(&obj, "exactDisplay", &display.into());
    }
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
