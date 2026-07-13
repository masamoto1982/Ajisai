//! Property-based observation-foundation laws (Phase 1).
//!
//! These encode the algebraic content of the observation function and the
//! renderer from `docs/dev/ajisai-mathematical-formalization.md` §9-ter D
//! (Phase 1): `observe(p) = (render(π_Stack ⟦p⟧ σ₀), π_Eff)` with
//! `render : (data, role) → display` a **pure** function over **all** SPEC
//! §12.2 roles, observed through the SPEC §2.3 semantic axes only.
//!
//! Unlike `algebraic_laws.rs` — which observes through whole-stack
//! `Value::to_string()` (a *display* surface, non-canonical per §2.3) — this
//! file observes through protocol axes and treats `render` as the explicit
//! `(data, role)` function. It is the firewall-clean basis later phases reuse
//! by adding domain generators (`test_support::generators`).
//!
//! Every law below was checked against the reference implementation with a
//! throwaway probe before being written (roadmap §1.2-(T) discipline).

mod test_support;

use ajisai_core::semantic::Capability;
use ajisai_core::types::Interpretation;
use proptest::prelude::*;
use test_support::generators::*;
use test_support::observe::{observe_axes, render, run, run_one, ALL_ROLES};

// ─────────────────────────── concrete-witness laws ───────────────────────────

/// Render is genuinely role-sensitive (a positive control): it is not a
/// constant function that ignores its role argument. A definite Boolean renders
/// as the bare truth word under `TruthValue` but as quoted text under `Text`.
#[test]
fn render_is_role_sensitive() {
    let t = run_one("TRUE");
    assert_ne!(
        render(&t, Interpretation::TruthValue),
        render(&t, Interpretation::Text),
        "render must depend on its role argument"
    );
}

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
    assert_ne!(
        render(&run_one("TRUE"), Interpretation::Unassigned),
        render(&run_one("1"), Interpretation::Unassigned),
    );
}

/// NIL is an operational absence, not FALSE and not logical UNKNOWN. The
/// distinction is visible through protocol axes, not through Rust storage or
/// display internals.
#[test]
fn nil_is_observably_operational_absence() {
    let nil = observe_axes(&run_one("NIL"));
    let false_value = observe_axes(&run_one("FALSE"));
    let unknown = observe_axes(&run_one(
        "'math' IMPORT 2 MATH@SQRT 1 ADD 2 MATH@SQRT 1 ADD 8 COMPARE-WITHIN",
    ));

    assert_eq!(nil.semantic_kind, "absence");
    assert_eq!(nil.shape, "absence");
    assert_eq!(nil.truth_value, None);
    assert!(nil.capabilities.contains(&"nilPassthrough"));
    assert!(nil.capabilities.contains(&"diagnosable"));
    assert!(!nil.capabilities.contains(&"truthValued"));

    assert_eq!(false_value.truth_value, Some("false"));
    assert_eq!(unknown.truth_value, Some("unknown"));
    assert!(false_value.capabilities.contains(&"truthValued"));
    assert!(unknown.capabilities.contains(&"truthValued"));
}

/// Every observed protocol string is canonical lower-camelCase (SPEC §2.3):
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
        "{ 1 ADD }",
        "'math' IMPORT 2 MATH@SQRT",
        "'math' IMPORT 2 MATH@SQRT 1 ADD 2 MATH@SQRT 1 ADD 8 COMPARE-WITHIN",
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

    // ───────────────────────── render is a pure (data, role) function ─────────

    /// **Totality + determinism**: `render` is defined on every SPEC §12.2 role
    /// for every well-formed value (the role match is exhaustive over the 8
    /// roles) and is a deterministic pure function — two calls agree. (Probe
    /// finding: an empty code block `{ }` renders to the empty string, so
    /// nonemptiness is *not* a render law; totality and determinism are.)
    #[test]
    fn render_total_and_deterministic_over_all_roles(src in any_value_src()) {
        let v = run_one(&src);
        for role in ALL_ROLES {
            prop_assert_eq!(render(&v, role), render(&v, role));
        }
    }

    /// **Purity / hint-independence** (SPEC §12.2: "two values are displayed
    /// identically whenever their data and role are equal"). `render(v, r)`
    /// depends only on `(data, role)`, never on the value's stored hint, so
    /// re-roling the carrier value leaves every rendering fixed.
    #[test]
    fn render_depends_only_on_data_and_role(src in any_value_src()) {
        let v = run_one(&src);
        let mut reroled = v.clone();
        reroled.hint = if v.hint == Interpretation::RawNumber {
            Interpretation::Text
        } else {
            Interpretation::RawNumber
        };
        for role in ALL_ROLES {
            prop_assert_eq!(render(&v, role), render(&reroled, role));
        }
    }

    /// The default observation `Value::to_string()` is exactly `render` at the
    /// value's own role: `observe`'s display half factors through `render`.
    #[test]
    fn default_observation_is_render_at_own_role(src in any_value_src()) {
        let v = run_one(&src);
        prop_assert_eq!(v.to_string(), render(&v, v.hint));
    }

    /// **U is render-absorbing** (SPEC §2.3, §7.5): the logical Unknown renders
    /// as `UNKNOWN` under *every* role — its truth surface is role-invariant and
    /// never leaks as `NIL` or a numeric form.
    #[test]
    fn unknown_renders_absorbingly(src in unknown_src()) {
        let u = run_one(&src);
        prop_assert_eq!(u.truth_value(), Some("unknown"));
        for role in ALL_ROLES {
            prop_assert_eq!(render(&u, role), "UNKNOWN");
        }
    }

    // ───────────────────────── semantic firewall on the axes ─────────────────

    /// **Structural axes are role-orthogonal** (semantic firewall): the
    /// data-plane axes `semanticKind`, `shape`, and `origin` read only the data
    /// and absence metadata, so assigning a display role never changes them.
    #[test]
    fn structural_axes_are_role_orthogonal(src in any_value_src()) {
        let v = run_one(&src);
        let mut reroled = v.clone();
        reroled.hint = if v.hint == Interpretation::TruthValue {
            Interpretation::Unassigned
        } else {
            Interpretation::TruthValue
        };
        let a = observe_axes(&v);
        let b = observe_axes(&reroled);
        prop_assert_eq!(a.semantic_kind, b.semantic_kind);
        prop_assert_eq!(a.shape, b.shape);
        prop_assert_eq!(a.origin, b.origin);
    }

    /// **Axis coherence** on runtime-produced values (SPEC §2.3: a truth-valued
    /// value "also carries the `truthValued` capability"): the `truthValue` axis
    /// is present iff the `truthValued` capability is present.
    #[test]
    fn truth_axis_and_capability_cohere(src in any_value_src()) {
        let v = run_one(&src);
        let has_axis = v.truth_value().is_some();
        let has_cap = v.has_capability(Capability::TruthValued);
        prop_assert_eq!(has_axis, has_cap);
    }

    /// Every value advertises the universal stack capabilities (§2.3 baseline):
    /// it is a `stackItem`, `serializable`, and `displayable`.
    #[test]
    fn every_value_is_a_displayable_stack_item(src in any_value_src()) {
        let o = observe_axes(&run_one(&src));
        for cap in ["stackItem", "serializable", "displayable"] {
            prop_assert!(o.capabilities.contains(&cap), "missing {cap}");
        }
    }
}
