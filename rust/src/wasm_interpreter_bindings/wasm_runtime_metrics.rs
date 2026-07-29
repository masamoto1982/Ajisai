//! WASM surface for the runtime's observational counters.
//!
//! These counters are diagnostics, never part of value identity
//! (LANG.AUTHORITY.FREEDOM): reading them changes no result, and no Word reads
//! them.

use super::{set_js_prop, AjisaiInterpreter};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl AjisaiInterpreter {
    /// Runtime counters for the Playground. Counts are session-cumulative and
    /// reset with the interpreter. Observational only.
    #[wasm_bindgen]
    pub fn collect_runtime_metrics(&self) -> JsValue {
        let m = self.interpreter.runtime_metrics();
        let num = |v: u64| JsValue::from_f64(v as f64);
        let obj = js_sys::Object::new();

        set_js_prop(&obj, "scalarFastpathCount", &num(m.scalar_fastpath_count));
        set_js_prop(
            &obj,
            "compiledPlanBuildCount",
            &num(m.compiled_plan_build_count),
        );
        set_js_prop(
            &obj,
            "compiledPlanCacheHitCount",
            &num(m.compiled_plan_cache_hit_count),
        );
        set_js_prop(
            &obj,
            "compiledPlanCacheMissCount",
            &num(m.compiled_plan_cache_miss_count),
        );
        set_js_prop(&obj, "tailCallJumpCount", &num(m.tail_call_jump_count));
        set_js_prop(&obj, "executionSteps", &num(m.execution_steps));

        obj.into()
    }
}
