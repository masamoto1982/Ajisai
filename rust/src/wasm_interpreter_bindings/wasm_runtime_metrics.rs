//! WASM surface for cost-model observability (SPECIFICATION.html §4.8 Cost
//! Model). Exposes the observational `RuntimeMetrics` counters that answer the
//! cost-model questions — which values were fast, how much shape-aware data
//! movement happened, and when the comparison budget was spent — to the
//! Playground.
//!
//! These counters are proxies for cost, never part of value identity (SPEC
//! §4.2.2 / §4.8): reading them changes no result, and no Coreword reads them.

use super::{set_js_prop, AjisaiInterpreter};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl AjisaiInterpreter {
    /// Cost-model counters for the Playground. Counts are session-cumulative
    /// and reset with the interpreter. Observational only (SPEC §4.8).
    #[wasm_bindgen]
    pub fn collect_runtime_metrics(&self) -> JsValue {
        let m = self.interpreter.runtime_metrics();
        let num = |v: u64| JsValue::from_f64(v as f64);
        let obj = js_sys::Object::new();

        // Fast lane: small-rational scalar ops and dense tensor kernels.
        set_js_prop(&obj, "scalarFastpathCount", &num(m.scalar_fastpath_count));
        set_js_prop(
            &obj,
            "bulkKernelUseCount",
            &num(m.vtu_bulk_kernel_use_count),
        );
        set_js_prop(
            &obj,
            "simdKernelUseCount",
            &num(m.vtu_simd_kernel_use_count),
        );

        // Data movement: dense<->nested round trips and sparse candidates.
        set_js_prop(&obj, "tensorFlattenCount", &num(m.vtu_tensor_flatten_count));
        set_js_prop(&obj, "tensorRebuildCount", &num(m.vtu_tensor_rebuild_count));
        set_js_prop(
            &obj,
            "sparseCandidateCount",
            &num(m.vtu_sparse_candidate_count),
        );

        // Comparison budget: only COMPARE-WITHIN spends it.
        set_js_prop(&obj, "compareWithinCount", &num(m.compare_within_count));
        set_js_prop(
            &obj,
            "compareWithinLazyCount",
            &num(m.compare_within_lazy_count),
        );
        set_js_prop(
            &obj,
            "compareWithinUnknownCount",
            &num(m.compare_within_unknown_count),
        );
        set_js_prop(
            &obj,
            "compareWithinBudgetTermsConsumed",
            &num(m.compare_within_budget_terms_consumed),
        );

        obj.into()
    }
}
