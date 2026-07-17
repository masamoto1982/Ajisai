//! Property-based effect / observation laws (Phase 7).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-sexies G (Phase 7): the
//! observation `observe(p) = (render*(π_Stack ⟦p⟧ σ₀), π_Eff ⟦p⟧ σ₀)` of
//! §9-ter D gets its **effect half** `π_Eff` here. `π_Eff` is the ordered
//! sequence of *outbound* host effects (the `HostEffect` log), modeled as a
//! free monoid; `IMPORT`/`UNIMPORT`/`UNIMPORT-ONLY` … as monotone operators were
//! Phase 6, and rendering / the semantic plane were Phase 1.
//!
//! Observation stays firewall-clean: the effect列 is observed through the
//! stable protocol tags `HostEffect::kind()` / `::payload()` (the strings the
//! conformance suite carries in `data-kind` / `data-payload`, see
//! `src/interpreter/host.rs`), never a Rust enum or `Debug`. The data half uses
//! the Phase 1 pure `render`. Registry-level invariants read the public
//! contract metadata (SPEC §7.14 / §5.2 Portability Profiles).
//!
//! Every law was probe-confirmed (`_probe_effects.rs`, deleted) first
//! (roadmap §1.2-(T)). Surprises pinned as §9-sexies G.5 guarded oracles:
//!   * **G1** — π_Eff records only *outbound* host effects. `NOW`/`CSPRNG`
//!     (host *reads*, `Observable`) and `DEF`/`IMPORT` (internal-Σ mutation,
//!     `Effectful` but `Core`) emit **nothing** to the log.
//!   * **G2** — `WordProfile::Core ⊋ Pure`: Core means *host-independent*
//!     (`required_capability = None`), and includes deterministic-given-Σ
//!     effectful words (`DEF`, `IMPORT`, …). Host-dependence is exactly
//!     `Hosted ⟺ required_capability = Some`.
//!
//! The SERIAL receive buffer (`serial_inbox`) is a `pub(crate)` host boundary
//! not reachable from the integration harness, so the full drain law lives in
//! the crate-internal `serial/serial_command_tests.rs`; here we pin the
//! integration-observable `READ` no-data projection and run-internal
//! determinism.

mod test_support;

use std::sync::Arc;

use ajisai_core::coreword_registry::{
    get_builtin_word_registry, get_coreword_metadata, WordProfile, WordPurity,
};
use ajisai_core::interpreter::{DeterministicHostEnv, HostCapability, Interpreter};
use proptest::prelude::*;
use test_support::generators::{effect_free_src, serial_outbound_call, small};
use test_support::observe::{observe_axes, render};

// ─────────────────────────── observation helpers ───────────────────────────

/// Run with the default host (all capabilities) and return both observation
/// channels: the data-plane stack renders and the ordered effect列 as
/// `(kind, payload)` protocol tags. Panics on execution error so a malformed
/// law program is loud (mirrors the existing `run` harness).
fn obs(src: &str) -> (Vec<String>, Vec<(String, String)>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        let program = test_support::observe::prepare(&mut interp, src);
        interp
            .execute(program)
            .await
            .unwrap_or_else(|e| panic!("program failed: {src:?}: {e}"));
        let stack = interp
            .get_stack()
            .iter()
            .map(|v| render(v, v.hint))
            .collect();
        let effects = interp
            .host_effects()
            .iter()
            .map(|e| (e.kind().to_string(), e.payload().to_string()))
            .collect();
        (stack, effects)
    })
}

/// Just the ordered effect列 (the `π_Eff` component).
fn eff(src: &str) -> Vec<(String, String)> {
    obs(src).1
}

/// Just the data-plane stack renders (the `render*(π_Stack)` component).
fn stk(src: &str) -> Vec<String> {
    obs(src).0
}

/// Run under a fixed `DeterministicHostEnv` (all capabilities) and return
/// `(stack renders, effect kinds)`. Used to show the host isolates all
/// non-determinism: under a fixed host, ⟦·⟧ is reproducible.
fn det(src: &str, now_millis: i64, random_bytes: Vec<u8>) -> (Vec<String>, Vec<String>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let host = Arc::new(DeterministicHostEnv::all_capabilities(
            now_millis,
            random_bytes,
        ));
        let mut interp = Interpreter::with_host(host);
        interp
            .execute(src)
            .await
            .unwrap_or_else(|e| panic!("program failed: {src:?}: {e}"));
        let stack = interp
            .get_stack()
            .iter()
            .map(|v| render(v, v.hint))
            .collect();
        let kinds = interp
            .host_effects()
            .iter()
            .map(|e| e.kind().to_string())
            .collect();
        (stack, kinds)
    })
}

// ───────────────── π_Eff is an ordered free monoid (§5.2) ────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// **Effect列 is a monoid homomorphism (order-preserving concatenation).**
    /// `π_Eff(p ++ q) = π_Eff(p) ++ π_Eff(q)`: running two programs in sequence
    /// concatenates their effect logs in order. Built from `PRINT`s so the log
    /// is non-trivial.
    #[test]
    fn effect_log_is_concatenative(xs in prop::collection::vec(small(), 0..4),
                                   ys in prop::collection::vec(small(), 0..4)) {
        let p = xs.iter().map(|x| format!("{x} PRINT")).collect::<Vec<_>>().join(" ");
        let q = ys.iter().map(|y| format!("{y} PRINT")).collect::<Vec<_>>().join(" ");
        let mut concat = eff(&p);
        concat.extend(eff(&q));
        prop_assert_eq!(eff(&format!("{p} {q}")), concat);
    }

    /// **Effect列 order matches program order, and each `PRINT` payload is the
    /// pure render of the printed value** (the semantic plane meets π_Eff only
    /// at the render boundary). `x PRINT` logs exactly `("print", render(x))`.
    #[test]
    fn print_log_is_ordered_and_payload_is_render(xs in prop::collection::vec(small(), 0..5)) {
        let prog = xs.iter().map(|x| format!("{x} PRINT")).collect::<Vec<_>>().join(" ");
        let effects = eff(&prog);
        prop_assert_eq!(effects.len(), xs.len());
        for (e, x) in effects.iter().zip(&xs) {
            prop_assert_eq!(&e.0, "print");
            // payload equals the data-plane render of the printed value.
            let rendered = stk(&x.to_string()).pop().unwrap();
            prop_assert_eq!(&e.1, &rendered);
        }
    }

    /// **Pure / internal-Σ programs emit the empty effect列** (π_Eff = ε). The
    /// data plane (stack) is non-empty while the outbound effect channel is
    /// empty — the two observation channels are independent (§5.2 two planes).
    #[test]
    fn effect_free_programs_emit_no_outbound_effect(src in effect_free_src()) {
        let (stack, effects) = obs(&src);
        prop_assert!(effects.is_empty(), "{src:?} unexpectedly emitted {effects:?}");
        prop_assert!(!stack.is_empty(), "{src:?} left no value");
    }

    /// **A pure context does not perturb the effect列.** Surrounding an
    /// effectful `PRINT` with pure computation leaves π_Eff unchanged: effects
    /// depend only on the effectful words, in order, not on pure neighbours.
    #[test]
    fn pure_context_does_not_change_effects(a in small(), b in small(), x in small()) {
        let bare = eff(&format!("{x} PRINT"));
        // pure prefix and suffix add nothing to the log.
        prop_assert_eq!(&bare, &eff(&format!("{a} {b} ADD {x} PRINT")));
        prop_assert_eq!(&bare, &eff(&format!("{x} PRINT {a} {b} MUL")));
    }
}

// ──────────────── SERIAL effect列 ordering & noData (§9.4) ────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    /// **SERIAL outbound effects appear once each, in program order** (§9.4).
    /// `OPEN … WRITE … CLOSE` logs exactly three `serial` effects in that order.
    #[test]
    fn serial_outbound_effects_are_ordered(bytes in prop::collection::vec(0i64..256, 1..4)) {
        let body = bytes.iter().map(i64::to_string).collect::<Vec<_>>().join(" ");
        let prog = format!("'serial' IMPORT 'P1' OPEN [ {body} ] WRITE CLOSE");
        let kinds: Vec<String> = eff(&prog).into_iter().map(|(k, _)| k).collect();
        prop_assert_eq!(kinds, vec!["serial".to_string(); 3]);
    }

    /// **Boundary `bare ≡ SERIAL@WORD` on effectful words**: the bare and
    /// qualified spellings of a SERIAL program emit the identical effect列
    /// (Phase 6 boundary law extended to the effect channel).
    #[test]
    fn serial_bare_equals_qualified((bare, qual) in serial_outbound_call()) {
        prop_assert_eq!(
            eff(&format!("'serial' IMPORT {bare}")),
            eff(&format!("'serial' IMPORT {qual}"))
        );
    }
}

/// **READ with no injected data projects a `noData` Bubble/NIL** (§9.4,
/// Bubble Rule §11.2): with an empty inbox, `READ` is total-by-projection —
/// it leaves an absence value, not an error, and emits **no** outbound effect
/// (it is a data-plane drain, not a host command).
#[test]
fn serial_read_no_data_projects_absence() {
    let (stack, effects) = obs("'serial' IMPORT 'P1' OPEN READ");
    // top of stack is an absence (NIL) value.
    let top = run_one_axes("'serial' IMPORT 'P1' OPEN READ");
    assert_eq!(
        top.semantic_kind, "absence",
        "READ no-data must project NIL, stack={stack:?}"
    );
    // only the OPEN produced an effect; READ itself emits none.
    assert_eq!(effects.iter().filter(|(k, _)| k == "serial").count(), 1);
}

/// Observe just the top value's axes (firewall-clean).
fn run_one_axes(src: &str) -> test_support::observe::AxisObservation {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut interp = Interpreter::new();
        interp.execute(src).await.unwrap();
        observe_axes(interp.get_stack().last().expect("non-empty stack"))
    })
}

/// **HostedEffect firewall:** a missing capability stops before request
/// construction. The word emits only a structured diagnostic effect and leaves
/// the data stack untouched; no outbound PRINT effect is appended.
#[test]
fn missing_capability_stops_before_hosted_request_construction() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let host = Arc::new(DeterministicHostEnv::new(0, vec![], vec![]));
        let mut interp = Interpreter::with_host(host);
        let result = interp.execute("7 PRINT").await;
        assert!(result.is_err(), "missing PRINT capability must error");

        let effects = interp
            .host_effects()
            .iter()
            .map(|e| (e.kind().to_string(), e.payload().to_string()))
            .collect::<Vec<_>>();
        assert_eq!(effects.len(), 1, "only diagnostic effect is emitted");
        assert_eq!(effects[0].0, "diagnostic");
        assert!(effects[0].1.contains("missingCapability"));
        assert!(
            !effects.iter().any(|(kind, _)| kind == "print"),
            "PRINT effect must not be appended when capability is missing"
        );

        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "capability failure must not consume stack");
        assert_eq!(observe_axes(&stack[0]).semantic_kind, "number");
    });
}

// ───────────── non-determinism isolation: ⟦·⟧ modulo host (§5.2) ─────────────

/// **Under a fixed host, otherwise non-deterministic words are reproducible.**
/// `NOW` and `CSPRNG` read the host; with a `DeterministicHostEnv` two runs
/// observe the identical stack — the non-determinism is isolated to the host,
/// so ⟦·⟧ modulo host is deterministic (Portability Profiles, §5.2).
#[test]
fn host_reads_are_deterministic_under_a_fixed_host() {
    let now = 1_700_000_000_123;
    assert_eq!(
        det("'time' IMPORT NOW", now, vec![]),
        det("'time' IMPORT NOW", now, vec![])
    );
    let rand = vec![3u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(
        det("'crypto' IMPORT [ 10 ] [ 1 ] CSPRNG", 0, rand.clone()),
        det("'crypto' IMPORT [ 10 ] [ 1 ] CSPRNG", 0, rand)
    );
}

/// **Finding G1 (guarded oracle): π_Eff records only outbound effects.** Host
/// *reads* (`NOW`, `CSPRNG` — `Observable`) and internal-Σ mutations (`DEF`,
/// `IMPORT` — `Effectful` but `Core`) push **nothing** to the effect log, while
/// `PRINT` (outbound) does. The log is the *outbound* projection of impurity,
/// not the full impurity classification.
#[test]
fn finding_g1_only_outbound_effects_are_logged() {
    // host reads → empty outbound log
    assert!(det("'time' IMPORT NOW", 1, vec![]).1.is_empty());
    assert!(det(
        "'crypto' IMPORT [ 10 ] [ 1 ] CSPRNG",
        0,
        vec![5, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    )
    .1
    .is_empty());
    // internal-Σ mutations → empty outbound log
    assert!(eff("{ 1 ADD } 'INC' DEF 5 INC").is_empty());
    assert!(eff("'math' IMPORT 4 MATH@SQRT").is_empty());
    // outbound effect → logged
    assert_eq!(
        eff("7 PRINT")
            .iter()
            .map(|(k, _)| k.as_str())
            .collect::<Vec<_>>(),
        vec!["print"]
    );
}

// ─────────── registry oracles: purity / profile / capability (§7.14) ─────────

/// **Impurity is always labeled, and `Pure` is total-clean** (SPEC §7.14):
/// every non-deterministic word carries a non-empty `effects` label, and every
/// `Pure` word is deterministic with no effect labels. This pins the
/// classification that lets the effect algebra separate pure from impure.
#[test]
fn purity_classification_is_consistent() {
    for m in get_builtin_word_registry() {
        if m.purity == WordPurity::Pure {
            assert!(m.deterministic, "{} Pure must be deterministic", m.name);
            assert!(
                m.effects.is_empty(),
                "{} Pure must have no effect labels",
                m.name
            );
        }
        if !m.deterministic {
            assert!(
                !m.effects.is_empty(),
                "{} is non-deterministic but unlabeled",
                m.name
            );
        }
    }
}

/// **Portability separation `Hosted ⟺ required_capability = Some`** (SPEC §5.2
/// Portability Profiles). Core-profile words are host-independent (no
/// capability), even when they are effectful internal-Σ mutators (finding G2);
/// Hosted-profile words declare exactly the host capability they need.
#[test]
fn profile_capability_separation_holds() {
    for m in get_builtin_word_registry() {
        match m.profile {
            WordProfile::Hosted => assert!(
                m.required_capability.is_some(),
                "{} Hosted must declare a capability",
                m.name
            ),
            WordProfile::Core => assert!(
                m.required_capability.is_none(),
                "{} Core must need no host capability",
                m.name
            ),
            WordProfile::PlatformSpecific => {}
        }
    }
}

/// **Finding G2 + §7.9 anchor (guarded oracle).** Pins the concrete
/// classification the effect model relies on: `PRINT`/`NOW`/`CSPRNG`/`OPEN` are
/// `Hosted` with their capability; `HASH`/`ADD` are pure `Core`; `DEF`/`IMPORT`
/// are `Core` (host-independent) yet `Effectful` — so `Core ⊋ Pure`.
#[test]
fn finding_g2_anchor_word_profiles() {
    let c = |n: &str| get_coreword_metadata(n).unwrap_or_else(|| panic!("no contract {n}"));

    for (w, cap) in [
        ("PRINT", HostCapability::Effect),
        ("NOW", HostCapability::Clock),
        ("CSPRNG", HostCapability::SecureRandom),
        ("OPEN", HostCapability::Serial),
    ] {
        let m = c(w);
        assert_eq!(m.profile, WordProfile::Hosted, "{w}");
        assert_eq!(m.required_capability, Some(cap), "{w}");
    }

    for w in ["HASH", "ADD"] {
        let m = c(w);
        assert_eq!(m.profile, WordProfile::Core, "{w}");
        assert_eq!(m.purity, WordPurity::Pure, "{w}");
        assert!(m.deterministic, "{w}");
    }

    // Core ⊋ Pure: Core, host-independent, but effectful (mutates Σ).
    for w in ["DEF", "IMPORT"] {
        let m = c(w);
        assert_eq!(m.profile, WordProfile::Core, "{w}");
        assert_eq!(m.required_capability, None, "{w}");
        assert_eq!(m.purity, WordPurity::Effectful, "{w}");
    }
}
