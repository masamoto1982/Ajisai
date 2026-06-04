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
//!     resolves it after `IMPORT` (`4 MATH@SQRT` is `Unknown` un-imported).
//!   * **F2** — an imported module word *shadows a same-named user word* (the
//!     resolver checks imported modules before user dictionaries).
//!
//! Observation stays firewall-clean: laws compare whole-stack renders / the
//! Ok-vs-Err resolution outcome, never a Rust enum or `Debug` string. The few
//! registry-level invariants read the public contract metadata (SPEC §7.14
//! listing fields), mirroring `contract_modifier_laws.rs`.

mod test_support;

use ajisai_core::coreword_registry::{
    get_builtin_word_registry, get_coreword_metadata, CanonicalHome,
};
use ajisai_core::interpreter::Interpreter;
use proptest::prelude::*;
use test_support::generators::{module_word_call, small, user_word_body, user_word_name};
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

    /// **Boundary-word resolution `bare ≡ MODULE@WORD`** (roadmap Phase 6,
    /// SPEC §7.14). After `'M' IMPORT`, a canonical module word is reachable
    /// under its bare name and under `M@WORD`, and both observe identically.
    #[test]
    fn bare_equals_qualified_after_import((m, bare, qual) in module_word_call()) {
        let via_bare = obs(&format!("'{m}' IMPORT {bare}"));
        let via_qual = obs(&format!("'{m}' IMPORT {qual}"));
        prop_assert_eq!(via_bare, via_qual);
    }

    /// **IMPORT is idempotent** (monotone visibility operator, SPEC §9.2):
    /// importing a module `k ≥ 1` times leaves the same vocabulary as importing
    /// it once. Observed by running the same word call after `k` imports.
    #[test]
    fn import_is_idempotent((m, bare, _q) in module_word_call(), k in 1usize..4) {
        let once = obs(&format!("'{m}' IMPORT {bare}"));
        let imports = format!("'{m}' IMPORT ").repeat(k);
        prop_assert_eq!(once, obs(&format!("{imports}{bare}")));
    }

    /// **Resolution determinism.** `resolve` is a function of the import/def
    /// state, not of evaluation history: the same program in the same starting
    /// state always observes the same stack (run twice, fresh interpreter each
    /// time — the harness builds a new `Interpreter` per `run`).
    #[test]
    fn resolution_is_deterministic((m, bare, qual) in module_word_call()) {
        let prog = format!("'{m}' IMPORT {bare} {qual}");
        prop_assert_eq!(obs(&prog), obs(&prog));
    }

    /// **Core precedence — import never shadows a Canonical Core word.** Bare
    /// `GET` is core (`canonical_home = Core`), so it resolves to the core
    /// vector index whether or not `JSON` (which also lists a `GET`) is
    /// imported. The observation is invariant under the import.
    #[test]
    fn core_word_is_not_shadowed_by_import(xs in prop::collection::vec(small(), 1..4), i in 0usize..3) {
        let body = xs.iter().map(i64::to_string).collect::<Vec<_>>().join(" ");
        let plain = obs(&format!("[ {body} ] {i} GET"));
        let after_import = obs(&format!("'json' IMPORT [ {body} ] {i} GET"));
        prop_assert_eq!(plain, after_import);
    }

    /// **F2 (finding) — an imported module word shadows a same-named user
    /// word.** The resolver order is Core → imported module → user, so after a
    /// user `SQRT` is defined *and* `MATH` is imported, bare `SQRT` runs the
    /// module word, not the user word. Pinned as a guarded oracle.
    #[test]
    fn imported_module_shadows_user_word(n in 2i64..12) {
        let sq = n * n;
        // user SQRT would add 99; module SQRT takes the root.
        let prog = format!("{{ 99 ADD }} 'SQRT' DEF 'math' IMPORT {sq} SQRT");
        prop_assert_eq!(obs(&prog), vec![format!("{n}/1")]);
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

// ─────────── dependency guard `FORC` (§8.2) — DEF/DEL need `!` ────────────────

/// **Redefining a word with active dependents requires the force modifier `!`.**
/// Without `!` the redefinition is rejected and the dictionary is unchanged;
/// with `!` it succeeds and the dependent observes the new definition
/// (SPEC §8.2). This is the `FORC` guard of the dependency graph.
#[test]
fn redefine_with_dependents_needs_force() {
    // INC2 depends on INC. Redefining INC without `!` is rejected.
    let no_force = "{ 1 ADD } 'INC' DEF { INC INC } 'INC2' DEF { 2 ADD } 'INC' DEF 5 INC2";
    assert!(
        outcome(no_force).is_err(),
        "redefining a dependency without ! must fail"
    );

    // With `!`, the redefinition lands and INC2 sees the new INC (5+2+2 = 9).
    let forced = "{ 1 ADD } 'INC' DEF { INC INC } 'INC2' DEF { 2 ADD } 'INC' ! DEF 5 INC2";
    assert_eq!(outcome(forced), Ok(vec!["9/1".to_string()]));
}

/// **Deleting a word with active dependents requires `!`** (SPEC §8.2/§8.3).
/// Without `!`, deletion is rejected (the dependent keeps working); with `!`
/// the word is gone and the dependent can no longer resolve it.
#[test]
fn delete_with_dependents_needs_force() {
    // Without `!`, the guard fires and the delete is rejected (raises an error).
    let no_force = "{ 1 ADD } 'INC' DEF { INC INC } 'INC2' DEF 'INC' DEL";
    assert!(
        outcome(no_force).is_err(),
        "deleting a referenced word without ! must fail"
    );

    // With `!`, the delete succeeds and the program completes cleanly.
    let forced_ok = "{ 1 ADD } 'INC' DEF { INC INC } 'INC2' DEF 'INC' ! DEL 42";
    assert_eq!(outcome(forced_ok), Ok(vec!["42/1".to_string()]));

    // …and after the forced delete the dependent can no longer resolve INC.
    let forced_orphan = "{ 1 ADD } 'INC' DEF { INC INC } 'INC2' DEF 'INC' ! DEL 5 INC2";
    assert!(
        outcome(forced_orphan).is_err(),
        "after forced delete the dependent must not resolve"
    );
}

/// **Built-in words cannot be redefined** (SPEC §8.2): `DEF` of a Canonical
/// Core name is rejected outright, independent of `!`. The core vocabulary is
/// immutable from user space.
#[test]
fn builtin_words_cannot_be_redefined() {
    for w in ["ADD", "GET", "EQ"] {
        assert!(
            outcome(&format!("{{ 0 }} '{w}' DEF")).is_err(),
            "redefining built-in {w} must be rejected"
        );
    }
}

// ────────── IMPORT / UNIMPORT visibility operators (§9.2) ────────────────────

/// **UNIMPORT of an unreferenced full import restores the pre-import state.**
/// After `'M' IMPORT … 'M' UNIMPORT`, the module word is hidden again under
/// both its bare and qualified names — `UNIMPORT` is the inverse of `IMPORT`
/// on unreferenced visibility (SPEC §9.2).
#[test]
fn unimport_inverts_unreferenced_import() {
    // qualified hidden before import, visible after import, hidden after unimport.
    assert!(outcome("'[1]' JSON@PARSE").is_err());
    assert!(outcome("'json' IMPORT '[1]' JSON@PARSE").is_ok());
    assert!(outcome("'json' IMPORT 'json' UNIMPORT '[1]' JSON@PARSE").is_err());
    // bare name too.
    assert!(outcome("'json' IMPORT 'json' UNIMPORT '[1]' PARSE").is_err());
}

/// **UNIMPORT keeps words referenced by a user word visible** (SPEC §9.2):
/// a full import shrinks to an explicit partial-import that preserves exactly
/// the referenced module word, hiding the rest.
#[test]
fn unimport_preserves_referenced_words() {
    // USE-PARSE references JSON@PARSE; after UNIMPORT the reference still resolves.
    let keep = "'json' IMPORT { JSON@PARSE } 'USE-PARSE' DEF 'json' UNIMPORT '[1]' USE-PARSE";
    assert_eq!(outcome(keep), Ok(vec!["[ 1/1 ]".to_string()]));
    // …but an unreferenced sibling (STRINGIFY) is hidden by the same UNIMPORT.
    let drop_sibling =
        "'json' IMPORT { JSON@PARSE } 'USE-PARSE' DEF 'json' UNIMPORT '[1]' JSON@STRINGIFY";
    assert!(outcome(drop_sibling).is_err());
}

/// **UNIMPORT-ONLY of a referenced word is rejected** (SPEC §9.2): a selector
/// pinned by a user word cannot be hidden piecemeal; dictionary-level UNIMPORT
/// is required. The vocabulary is unchanged on rejection.
#[test]
fn unimport_only_rejects_referenced_selector() {
    let prog =
        "'json' IMPORT { JSON@PARSE } 'USE-PARSE' DEF 'json' [ 'parse' ] UNIMPORT-ONLY '[1]' USE-PARSE";
    assert!(
        outcome(prog).is_err(),
        "UNIMPORT-ONLY of a referenced selector must be rejected"
    );
}

/// **IMPORT-ONLY brings exactly the selected word, not its siblings** (SPEC
/// §9.2): `'math' [ 'sqrt' ] IMPORT-ONLY` exposes bare `SQRT` but leaves `GCD`
/// (another MATH word) `Unknown`.
#[test]
fn import_only_is_selective() {
    assert!(outcome("'math' [ 'sqrt' ] IMPORT-ONLY 4 SQRT").is_ok());
    assert!(outcome("'math' [ 'sqrt' ] IMPORT-ONLY 3 4 GCD").is_err());
}

/// **IMPORT-ONLY of a core-listed selector is a silent no-op** (SPEC §7.14,
/// §9.2): `PRINT` is a Canonical Core word merely *listed* in the `IO` view, so
/// selecting it is skipped with a warning and the rest of the run proceeds —
/// the word was already available without import.
#[test]
fn import_only_core_listed_selector_is_noop() {
    // The run succeeds (no error) and PRINT is usable regardless.
    assert_eq!(
        outcome("'io' [ 'print' ] IMPORT-ONLY 7"),
        Ok(vec!["7/1".to_string()])
    );
    // A selector matching neither a canonical word nor a listing is an error.
    assert!(outcome("'json' [ 'nope' ] IMPORT-ONLY 7").is_err());
}

// ──────────── registry oracles: canonical_home / listing fields (§7.14) ──────

/// **Every word declares a `canonical_home`, and bare lookup of an overlapping
/// name resolves to Core** (SPEC §7.14): a bare name that exists both as a
/// Canonical Core word and a module word (e.g. `GET` = core / `JSON@GET`)
/// resolves to the Core entry under `get_coreword_metadata`, matching the
/// runtime resolution order.
#[test]
fn registry_canonical_home_and_core_preference() {
    let reg = get_builtin_word_registry();
    assert!(!reg.is_empty());

    // Bare names that appear under more than one canonical home.
    let mut homes: std::collections::BTreeMap<&str, Vec<&CanonicalHome>> = Default::default();
    for m in reg {
        homes
            .entry(m.name.as_str())
            .or_default()
            .push(&m.canonical_home);
    }
    let overlapping: Vec<&str> = homes
        .iter()
        .filter(|(_, hs)| hs.len() > 1)
        .map(|(n, _)| *n)
        .collect();
    // There is at least one overlapping name (GET) and bare lookup prefers Core.
    assert!(
        overlapping.contains(&"GET"),
        "expected GET to overlap Core/Module"
    );
    for name in overlapping {
        let bare = get_coreword_metadata(name).unwrap_or_else(|| panic!("no contract {name}"));
        if homes[name].iter().any(|h| matches!(h, CanonicalHome::Core)) {
            assert_eq!(
                bare.canonical_home,
                CanonicalHome::Core,
                "bare {name} must resolve to its Core home (SPEC §7.14)"
            );
        }
    }
}

/// **`MODULE@WORD` always reaches the module entry in the contract registry,
/// and a canonical module word lists its own module** (SPEC §7.14). This is the
/// *static* reachability that finding F1 contrasts with import-gated runtime
/// resolution.
#[test]
fn registry_qualified_reaches_module_entry() {
    for (q, module) in [
        ("MATH@SQRT", "MATH"),
        ("JSON@PARSE", "JSON"),
        ("ALGO@SORT", "ALGO"),
    ] {
        let m = get_coreword_metadata(q).unwrap_or_else(|| panic!("no contract {q}"));
        assert_eq!(
            m.canonical_home,
            CanonicalHome::Module(module.to_string()),
            "{q}"
        );
        assert!(
            m.listed_in_modules.iter().any(|x| x == module),
            "{q} must list its own module"
        );
    }
}

/// **Finding F1 (guarded oracle).** Runtime `MODULE@WORD` resolution is
/// import-gated even though the static registry always reaches the entry:
/// `4 MATH@SQRT` is `Unknown` until `'math' IMPORT`. This pins the
/// registry-reachability / runtime-visibility distinction so a future drift is
/// loud.
#[test]
fn finding_f1_qualified_resolution_is_import_gated() {
    // Static registry: reachable without any import.
    assert!(get_coreword_metadata("MATH@SQRT").is_some());
    // Runtime: not resolvable until imported.
    assert!(
        outcome("4 MATH@SQRT").is_err(),
        "F1: qualified call must need IMPORT"
    );
    assert!(outcome("'math' IMPORT 4 MATH@SQRT").is_ok());
}
