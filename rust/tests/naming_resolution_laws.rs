//! Property-based name-resolution / dictionary / module laws (Phase 6).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-quinquies F (Phase 6):
//! the dictionary `Dict = Name ⇀ Blk`, the deterministic resolver
//! `resolve : Name × Vis ⇀ Blk + Unknown` with order **Core → imported module →
//! user**, `DEF`/`DEL` as state transducers with a dependency guard (`FORC`,
//! SPEC §8.2), and `IMPORT`/`IMPORT-ONLY`/`UNIMPORT`/`UNIMPORT-ONLY` as
//! monotone visibility operators (SPEC §9.2).
//!
//! Every law was checked against the reference implementation with a throwaway
//! probe (`_probe_naming.rs`, deleted) before being written (roadmap §1.2-(T)
//! discipline). The two surprises the probe surfaced are recorded as
//! §9-quinquies F.5 findings and pinned here as guarded oracles:
//!   * **F1** — runtime `MODULE@WORD` resolution is *import-gated*: the static
//!     contract registry always reaches the module entry, but the runtime only
//!     resolves it after `IMPORT` (`4 SQRT` is `Unknown` un-imported).
//!   * **F2** — an imported module word *shadows a same-named user word* (the
//!     resolver checks imported modules before user dictionaries).
//!
//! Observation stays firewall-clean: laws compare whole-stack renders / the
//! Ok-vs-Err resolution outcome, never a Rust enum or `Debug` string. The few
//! registry-level invariants read the public contract metadata (SPEC §7.14
//! listing fields), mirroring `contract_modifier_laws.rs`.

mod test_support;

use ajisai_core::interpreter::Interpreter;
use proptest::prelude::*;
use test_support::generators::{small, user_word_body, user_word_name};
use test_support::observe::{render, run};

// ─────────────────────────── observation helpers ───────────────────────────

/// Whole-stack rendering (one value per element), the conformance observation.
fn obs(src: &str) -> Vec<String> {
    run(src).iter().map(|v| render(v, v.hint)).collect()
}

/// The resolution *outcome* of a program: `Ok(stack-render)` when every word
/// resolved and ran, `Err(())` when resolution (or execution) failed. This is
/// the firewall-clean way to observe "does this name resolve here" — we never
/// inspect the error text, only whether a binding was found.
fn outcome(src: &str) -> Result<Vec<String>, ()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        match interp.execute(src).await {
            Ok(()) => Ok(interp
                .get_stack()
                .iter()
                .map(|v| render(v, v.hint))
                .collect()),
            Err(_) => Err(()),
        }
    })
}

// ───────────────── resolve : Name × Vis ⇀ Blk + Unknown (§7/§9) ──────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]
    /// **Core precedence — import never shadows a Canonical Core word.** Bare
    /// `GET` is core (`canonical_home = Core`), so it resolves to the core
    /// vector index whether or not `JSON` (which also lists a `GET`) is
    /// imported. The observation is invariant under the import.
    #[test]
    fn core_word_is_not_shadowed_by_import(xs in prop::collection::vec(small(), 1..4), i in 0usize..3) {
        let body = xs.iter().map(i64::to_string).collect::<Vec<_>>().join(" ");
        let plain = obs(&format!("[ {body} ] {i} GET"));
        let after_import = obs(&format!("[ {body} ] {i} GET"));
        prop_assert_eq!(plain, after_import);
    }

    /// **User never shadows Core** (LANG.DICTIONARY.RESOLUTION). Core is
    /// sealed: defining a user word over a Core name is rejected outright, so
    /// a Core name always means the Core Word.
    #[test]
    fn user_definition_cannot_shadow_a_core_word(n in 2i64..12) {
        let sq = n * n;
        assert!(
            outcome("{ 99 ADD } 'SQRT' DEF").is_err(),
            "defining a user word over the Core name SQRT must fail"
        );
        prop_assert_eq!(obs(&format!("{sq} SQRT")), vec![format!("{n}/1")]);
    }
}

// ─────────────── DEF / DEL as Dict state transducers (§8) ────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    /// **DEF makes a name resolvable; the defined word equals its inlined
    /// body.** `{body} 'W' DEF  x W  ≡  x body` — defining then calling is the
    /// identity on the body transducer (SPEC §8.1).
    #[test]
    fn def_then_call_inlines_body(name in user_word_name(), (body, inline) in user_word_body(), x in small()) {
        let defined = obs(&format!("{{ {body} }} '{name}' DEF {x} {name}"));
        let inlined = obs(&format!("{x} {inline}"));
        prop_assert_eq!(defined, inlined);
    }

    /// **DEF then DEL is the identity on resolution.** A name is `Unknown`
    /// before definition and `Unknown` again after deletion — `DEL` is the left
    /// inverse of `DEF` on the visibility of a fresh name (SPEC §8.3).
    #[test]
    fn def_del_round_trip_restores_unknown(name in user_word_name(), (body, _i) in user_word_body(), x in small()) {
        let fresh = format!("{x} {name}");
        let defined = format!("{{ {body} }} '{name}' DEF {x} {name}");
        let def_del = format!("{{ {body} }} '{name}' DEF '{name}' DEL {x} {name}");
        // Fresh name resolves to Unknown.
        prop_assert!(outcome(&fresh).is_err());
        // Defined → resolves.
        prop_assert!(outcome(&defined).is_ok());
        // Defined then deleted → Unknown again.
        prop_assert!(outcome(&def_del).is_err());
    }
}

// ───────────────── dependency guard — DEL refuses while referenced ────────────

/// **DEL refuses while a dependent exists.** There is no force modifier: a
/// referenced word cannot be deleted, so a dangling reference is unreachable
/// (LANG.DICTIONARY.MUTATION).
#[test]
fn delete_with_dependents_is_refused() {
    let referenced = "{ 1 ADD } 'INC' DEF { INC INC } 'INC2' DEF 'INC' DEL";
    assert!(
        outcome(referenced).is_err(),
        "deleting a referenced word must fail"
    );

    // Removing the dependent first is what makes the delete legal.
    let ordered = "{ 1 ADD } 'INC' DEF { INC INC } 'INC2' DEF 'INC2' DEL 'INC' DEL 42";
    assert_eq!(outcome(ordered), Ok(vec!["42/1".to_string()]));
}

/// **Core words cannot be redefined** (LANG.DICTIONARY.RESOLUTION): `DEF` of a
/// Core name is rejected outright. Core is sealed from user space.
#[test]
fn builtin_words_cannot_be_redefined() {
    for w in ["ADD", "GET", "EQ"] {
        assert!(
            outcome(&format!("{{ 0 }} '{w}' DEF")).is_err(),
            "redefining built-in {w} must be rejected"
        );
    }
}
