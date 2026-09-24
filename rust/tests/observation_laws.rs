//! Property-based observation-foundation laws (Phase 1).
//!
//! These encode the algebraic content of the observation function and the
//! renderer (Phase 1): `observe(p) = (render(π_Stack ⟦p⟧ σ₀), π_Eff)` with
//! `render : value → display` a **pure** function of the value
//! (LANG.VALUES.DENOTATION), observed through the LANG.OBSERVATION.FIREWALL
//! semantic axes only.
//!
//! Unlike `algebraic_laws.rs` — which observes through whole-stack
//! `Value::to_string()` (a *display* surface, non-canonical per LANG.OBSERVATION.FIREWALL) — this
//! file observes through protocol axes and treats `render` as an explicit
//! function of the value. It is the firewall-clean basis later phases reuse
//! by adding domain generators (`test_support::generators`).
//!
//! Every law below was checked against the reference implementation with a
//! throwaway probe before being written (roadmap §1.2-(T) discipline).

mod test_support;

use ajisai_core::semantic::Capability;
use proptest::prelude::*;
use test_support::generators::*;
use test_support::observe::{observe_axes, render, run, run_one};

// ─────────────────────────── concrete-witness laws ───────────────────────────

/// Finding B at the observation layer: a truth value is observably **not** a
/// number. `TRUE` carries the `truthValue` axis and the `truthValued`
/// capability; the scalar `1` carries neither, and they render differently.
#[test]
fn truth_value_is_observably_not_a_number() {
    let t = observe_axes(&run_one("TRUE"));
    let one = observe_axes(&run_one("1"));
    assert_eq!(t.truth_value, Some("true"));
    assert_eq!(one.truth_value, None);
    assert!(t.capabilities.contains(&"truthValued"));
    assert!(!one.capabilities.contains(&"truthValued"));
    assert_ne!(render(&run_one("TRUE")), render(&run_one("1")));
}
/// Every observed protocol string is canonical lower-camelCase (LANG.OBSERVATION.FIREWALL):
/// nonempty, lowercase first letter, ASCII-alphanumeric only (no `_`, no `-`).
#[test]
fn protocol_strings_are_lower_camel_case() {
    fn ok(s: &str) -> bool {
        let mut chars = s.chars();
        matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
            && s.chars().all(|c| c.is_ascii_alphanumeric())
    }
    for src in [
        "5",
        "TRUE",
        "FALSE",
        "1 0 /",
        "[ 1 2 3 ]",
        "[ 1 ADD ]",
        "2 SQRT",
    ] {
        for v in run(src) {
            let o = observe_axes(&v);
            assert!(ok(o.semantic_kind), "semanticKind {:?}", o.semantic_kind);
            assert!(ok(o.shape), "shape {:?}", o.shape);
            assert!(ok(o.origin), "origin {:?}", o.origin);
            for c in &o.capabilities {
                assert!(ok(c), "capability {c:?}");
            }
            if let Some(tv) = o.truth_value {
                assert!(ok(tv), "truthValue {tv:?}");
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    // ───────────────────────── render is a pure function ─────────────────────

    /// **Totality + determinism**: `render` is defined for every well-formed
    /// value and is a deterministic pure function — two calls agree.
    #[test]
    fn render_total_and_deterministic(src in any_value_src()) {
        let v = run_one(&src);
        prop_assert_eq!(render(&v), render(&v));
    }

    // ───────────────────────── semantic firewall on the axes ─────────────────

    /// **The truth axis is the Boolean domain** (LANG.VALUES.TRUTH): a value
    /// reports `truthValue` exactly when it is a Boolean. UNKNOWN is a NIL and
    /// reports none.
    #[test]
    fn truth_axis_is_present_exactly_on_booleans(src in any_value_src()) {
        let v = run_one(&src);
        let is_boolean = matches!(v.data, ajisai_core::types::ValueData::Boolean(_));
        prop_assert_eq!(v.truth_value().is_some(), is_boolean);
    }

    /// **Axis coherence** on runtime-produced values (LANG.OBSERVATION.FIREWALL: a truth-valued
    /// value "also carries the `truthValued` capability"): the `truthValue` axis
    /// is present iff the `truthValued` capability is present.
    #[test]
    fn truth_axis_and_capability_cohere(src in any_value_src()) {
        let v = run_one(&src);
        let has_axis = v.truth_value().is_some();
        let has_cap = v.has_capability(Capability::TruthValued);
        prop_assert_eq!(has_axis, has_cap);
    }

    /// Every value advertises the universal stack capabilities (LANG.OBSERVATION.FIREWALL baseline):
    /// it is a `stackItem`, `serializable`, and `displayable`.
    #[test]
    fn every_value_is_a_displayable_stack_item(src in any_value_src()) {
        let o = observe_axes(&run_one(&src));
        for cap in ["stackItem", "serializable", "displayable"] {
            prop_assert!(o.capabilities.contains(&cap), "missing {cap}");
        }
    }
}
