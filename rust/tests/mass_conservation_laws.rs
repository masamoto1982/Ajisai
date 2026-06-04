//! Property-based mass-conservation laws (SPEC §13, finding E1 resolved).
//!
//! Encodes the static mass contract (`§9-quater E.3`) now that arity/production
//! are surfaced into the §7.14 Coreword contract (`CorewordMetadata.mass`) and a
//! diagnostic-only validator (`interpreter::mass_conservation`) consumes them.
//!
//! The central soundness law ties the *static* contract to *observed* mass: for
//! a flow of fixed-arity words the validator's abstract net depth equals the
//! real runtime stack depth, and static over-consumption coincides with a
//! runtime stack underflow. Probed against the reference implementation before
//! being written (roadmap §1.2-(T) discipline).

mod test_support;

use ajisai_core::coreword_registry::{get_coreword_metadata, mass_contract, MassContract};
use ajisai_core::interpreter::mass_conservation::analyze_source;
use ajisai_core::interpreter::Interpreter;
use proptest::prelude::*;
use test_support::generators::small;

/// Runtime stack depth, or `None` if the flow errors (e.g. underflow).
fn runtime_depth(src: &str) -> Option<usize> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        interp
            .execute(src)
            .await
            .ok()
            .map(|_| interp.get_stack().len())
    })
}

fn analyze(src: &str) -> ajisai_core::interpreter::mass_conservation::MassReport {
    analyze_source(&Interpreter::new(), src).expect("tokenizes")
}

// ───────────────────────── contract is surfaced (§7.14/§13.1) ───────────────

/// The mass contract is now a machine-readable §7.14 field, and it is the same
/// single source the compiled-plan analyzer reads. Fixed for pinned words,
/// `Dynamic` for data-dependent ones (finding E1 resolved).
#[test]
fn mass_contract_is_surfaced_in_the_registry() {
    let add = get_coreword_metadata("ADD").unwrap();
    assert_eq!(
        add.mass,
        MassContract::Fixed {
            consumes: 2,
            produces: 1
        }
    );
    assert_eq!(add.mass, mass_contract("ADD"));

    let not = get_coreword_metadata("NOT").unwrap();
    assert_eq!(
        not.mass,
        MassContract::Fixed {
            consumes: 1,
            produces: 1
        }
    );

    // GET has a data-dependent / non-consuming profile (finding E3) → Dynamic.
    assert_eq!(
        get_coreword_metadata("GET").unwrap().mass,
        MassContract::Dynamic
    );
}

// ─────────────────── validator soundness vs observed mass ───────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// **Soundness.** For a flow built only from fixed-arity words, the static
    /// net mass equals the observed runtime stack depth, and the flow is not
    /// flagged as over-consuming.
    #[test]
    fn static_net_mass_matches_runtime_depth(
        a in small(), b in small(), c in small(),
        w1 in prop_oneof![Just("ADD"), Just("MUL"), Just("SUB")],
        w2 in prop_oneof![Just("ADD"), Just("MUL"), Just("SUB")],
    ) {
        // a b w1 c w2 : push a,b -> w1 (->1) ; push c -> w2 (->1). Net = 1.
        let src = format!("{a} {b} {w1} {c} {w2}");
        let report = analyze(&src);
        prop_assert!(report.all_known, "should be fully known: {src}");
        prop_assert!(!report.over_consumes_from_empty(), "{src}");
        prop_assert_eq!(report.net_mass, runtime_depth(&src).unwrap() as i64);
    }

    /// **KEEP changes net mass by exactly the production** (§13.2 bifurcation):
    /// `a b KEEP w` retains its `consumes` operands, so its static net mass
    /// exceeds `a b w` by `consumes` (= 2 for a binary word).
    #[test]
    fn keep_raises_static_net_mass_by_arity(a in small(), b in small()) {
        let eat = analyze(&format!("{a} {b} ADD")).net_mass;
        let keep = analyze(&format!("{a} {b} KEEP ADD")).net_mass;
        prop_assert_eq!(keep - eat, 2);
    }
}

/// **Over-consumption ⇔ runtime underflow.** A flow that reads more operands
/// than it is given dips below zero statically and underflows at runtime.
#[test]
fn static_over_consumption_coincides_with_runtime_underflow() {
    // `5 ADD`: one operand for a binary word.
    let report = analyze("5 ADD");
    assert!(
        report.over_consumes_from_empty(),
        "static must flag over-consumption"
    );
    assert_eq!(runtime_depth("5 ADD"), None, "runtime must underflow");

    // A balanced flow does neither.
    let ok = analyze("2 3 ADD");
    assert!(!ok.over_consumes_from_empty());
    assert_eq!(runtime_depth("2 3 ADD"), Some(1));
}

/// The validator **abstains** (does not claim mass knowledge) on data-dependent
/// arity: the `STAK` count-fold and runtime-shaped vector ops are `Dynamic`.
#[test]
fn validator_abstains_on_dynamic_arity() {
    assert!(
        !analyze("1 2 3 3 STAK ADD").all_known,
        "STAK is count-driven"
    );
    assert!(!analyze("[ 1 2 3 ] 0 GET").all_known, "GET is Dynamic");
}
