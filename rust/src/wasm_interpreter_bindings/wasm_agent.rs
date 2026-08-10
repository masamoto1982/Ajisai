//! One-shot WASM entry point for the host-neutral agent boundary
//! (`crate::agent`). Returns the same schema-1 JSON envelope the native
//! `ajisai agent <operation>` CLI emits
//! (`docs/dev/agent-cli-output-contract.md`), serialized as a JSON string so
//! a Node host parses it identically to the native CLI's stdout — no
//! bespoke per-host JS object shape, no separate normalizer.
//!
//! Deliberately separate from [`super::AjisaiInterpreter`]: that struct is
//! the browser playground's stateful, step-mode session API and keeps its
//! own JS-object result shape. This module creates one fresh interpreter per
//! call, mirroring the native CLI's one-process-per-call model.

use crate::agent::api;
use wasm_bindgen::prelude::*;

/// Execute one Ajisai source document under the same tightened
/// agent-profile runtime limits the native `ajisai agent compute` CLI
/// applies. `step_limit` overrides the default execution step budget when
/// positive; `0` or omitted keeps the interpreter default.
#[wasm_bindgen]
pub async fn agent_compute(source: &str, step_limit: Option<u32>) -> String {
    let options = api::ComputeOptions {
        step_limit: step_limit.filter(|&n| n > 0).map(|n| n as usize),
        runtime_limits: Some(api::LOCAL_AGENT_RUNTIME_LIMITS),
    };
    api::compute(source, options).await.to_json().to_string()
}

/// Parse and resolve `source` without executing it; also verifies declared
/// `#:contract` declarations conservatively, matching `ajisai agent check`.
#[wasm_bindgen]
pub fn agent_check(source: &str) -> String {
    api::check(source, true).to_json().to_string()
}

/// Infer machine-readable contracts for user-defined Words without
/// executing their bodies, matching `ajisai agent infer-contracts`.
#[wasm_bindgen]
pub fn agent_infer_contracts(source: &str) -> String {
    api::infer_contracts(source).to_json().to_string()
}
