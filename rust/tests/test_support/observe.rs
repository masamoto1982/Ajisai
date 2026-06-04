//! Firewall-clean observation (Phase 1).
//!
//! The formalization §8 defines the observation function
//! `observe(p) = (render(π_Stack ⟦p⟧ σ₀), π_Eff ⟦p⟧ σ₀)`. This module gives the
//! data-plane half of that: it reads a value **only** through the SPEC §2.3
//! semantic axes (`semanticKind`, `shape`, `capabilities`, `truthValue`,
//! `origin`, `absence`) and through the pure renderer `render : (data, role) →
//! display`. It never branches on a Rust enum name, `Debug` string, or display
//! text — the semantic-firewall discipline the roadmap §1.2-3 mandates.
//!
//! The effect component `π_Eff` is intentionally out of scope here; the effect
//! algebra is Phase 7. Phase 1 fixes the data-plane observation basis that all
//! later phases build on.

use ajisai_core::interpreter::Interpreter;
use ajisai_core::types::display::format_with_hint;
use ajisai_core::types::{Interpretation, Value};

/// Every interpretation role of SPEC §12.2, in table order.
pub const ALL_ROLES: [Interpretation; 8] = [
    Interpretation::Unassigned,
    Interpretation::RawNumber,
    Interpretation::ContinuedFraction,
    Interpretation::Interval,
    Interpretation::Text,
    Interpretation::TruthValue,
    Interpretation::Timestamp,
    Interpretation::Nil,
];

/// The pure renderer `render : (data, role) → display` (SPEC §12.1). Exposed as
/// a named function so laws read as equations over `render`, not over the
/// `Display` impl.
pub fn render(v: &Value, role: Interpretation) -> String {
    format_with_hint(v, role)
}

/// The protocol-level observation of one value: the SPEC §2.3 semantic axes as
/// canonical lower-camel-case protocol strings. Capabilities are sorted so the
/// observation is order-insensitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisObservation {
    pub semantic_kind: &'static str,
    pub shape: &'static str,
    pub capabilities: Vec<&'static str>,
    pub truth_value: Option<&'static str>,
    pub origin: &'static str,
}

/// Observe a value through the semantic axes only (firewall-clean).
pub fn observe_axes(v: &Value) -> AxisObservation {
    let mut capabilities: Vec<&'static str> = v
        .capabilities()
        .iter()
        .map(|c| c.as_protocol_str())
        .collect();
    capabilities.sort_unstable();
    AxisObservation {
        semantic_kind: v.semantic_kind().as_protocol_str(),
        shape: v.shape_kind().as_protocol_str(),
        capabilities,
        truth_value: v.truth_value(),
        origin: v.origin().as_protocol_str(),
    }
}

/// Run an Ajisai program and return the final stack. Panics on execution error
/// so a malformed law program is loud rather than silently skipped (mirrors the
/// existing `algebraic_laws.rs` harness).
pub fn run(src: &str) -> Vec<Value> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        interp
            .execute(src)
            .await
            .unwrap_or_else(|e| panic!("program failed: {src:?}: {e}"));
        interp.get_stack().clone()
    })
}

/// Run a program expected to leave exactly one value, returning that value.
pub fn run_one(src: &str) -> Value {
    let stack = run(src);
    assert_eq!(
        stack.len(),
        1,
        "generator program {src:?} must leave exactly one value, got {}",
        stack.len()
    );
    stack.into_iter().next().unwrap()
}
