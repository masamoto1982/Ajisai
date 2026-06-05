//! Property-based child-runtime (concurrency) observation-contract laws
//! (Phase 8).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-septies H (Phase 8):
//! `SPAWN`/`AWAIT`/`STATUS`/`KILL`/`MONITOR`/`SUPERVISE` (SPEC §10). Per the
//! roadmap, Phase 8 fixes the **observation contract** rather than a full
//! denotation: snapshot isolation, the child state machine, and
//! `AWAIT` as the projection of the child's ⟦block⟧ final configuration.
//!
//! Observation stays firewall-clean: a `ProcessHandle` is read through the
//! Phase 1 semantic axes (`semanticKind = process`, `shape = handle`); the
//! `AWAIT` result tuple `[status result-stack]` is decomposed and each part is
//! read through the pure `render`. We never branch on a Rust enum or `Debug`.
//!
//! Every law was probe-confirmed (`_probe_child.rs`, deleted) first
//! (roadmap §1.2-(T)). Surprises pinned as §9-septies H.5 guarded oracles:
//!   * **H1** — the child runs **lazily at `AWAIT`**, not at `SPAWN`: a freshly
//!     spawned child reads `STATUS = running`.
//!   * **H2** — a **domain failure does not fail the child**: `x 0 /` projects
//!     to NIL (Bubble Rule) and the child still `completed`s with `[NIL]`. Only
//!     a *genuine* error (unknown word, underflow) yields `failed`.

mod test_support;

use ajisai_core::interpreter::Interpreter;
use ajisai_core::types::ValueData;
use proptest::prelude::*;
use test_support::generators::{completing_block_body, failing_block_body, small};
use test_support::observe::{observe_axes, render, run};

// ─────────────────────────── observation helpers ───────────────────────────

/// Whole-stack rendering (one value per element).
fn obs(src: &str) -> Vec<String> {
    run(src).iter().map(|v| render(v, v.hint)).collect()
}

/// Run a program, returning `Err(())` if execution failed (firewall-clean: we
/// never read the error text).
fn ok(src: &str) -> bool {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(async { Interpreter::new().execute(src).await.is_ok() })
}

/// Decompose the top-of-stack `AWAIT` / `SUPERVISE` result tuple
/// `[status result-stack]` into `(status_render, result_stack_renders)`. The
/// status string is rendered with its own hint (→ `'completed'` etc., Text);
/// the result-stack is the child's final stack, each value rendered with its
/// hint (an empty child stack renders as an empty list / NIL → `vec![]`).
fn await_tuple(src: &str) -> (String, Vec<String>) {
    let stack = run(src);
    let top = stack.last().expect("AWAIT leaves a result");
    let ValueData::Vector(els) = &top.data else {
        panic!(
            "AWAIT result must be a vector, got {}",
            render(top, top.hint)
        );
    };
    assert_eq!(els.len(), 2, "AWAIT result must be [status result-stack]");
    let status = render(&els[0], els[0].hint);
    let result_stack = match &els[1].data {
        ValueData::Vector(inner) => inner.iter().map(|v| render(v, v.hint)).collect(),
        _ => vec![], // empty child stack → empty vector renders as NIL
    };
    (status, result_stack)
}

// ───────────────── ProcessHandle is observable (SPEC §4.7) ───────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    /// **`SPAWN` pushes a `ProcessHandle`** observable through the Phase 1 axes:
    /// `semanticKind = process`, `shape = handle`, with the base capabilities
    /// every value carries. (SPEC §4.7 / §10.3.)
    #[test]
    fn spawn_pushes_a_process_handle(body in completing_block_body()) {
        let stack = run(&format!("{{ {body} }} SPAWN"));
        prop_assert_eq!(stack.len(), 1);
        let axes = observe_axes(&stack[0]);
        prop_assert_eq!(axes.semantic_kind, "process");
        prop_assert_eq!(axes.shape, "handle");
        for cap in ["stackItem", "serializable", "displayable"] {
            prop_assert!(axes.capabilities.contains(&cap), "missing capability {cap}");
        }
    }

    // ───────────── AWAIT projects the child's ⟦block⟧ final config ────────────

    /// **`AWAIT` observes the child's ⟦block⟧ final configuration** (SPEC §10.4).
    /// For a completing block the result tuple is `['completed', S]` where `S`
    /// is exactly the stack of running `block` standalone — the child computes
    /// the same denotation in isolation.
    #[test]
    fn await_projects_child_final_config(body in completing_block_body()) {
        let (status, result_stack) = await_tuple(&format!("{{ {body} }} SPAWN AWAIT"));
        prop_assert_eq!(status, "'completed'");
        prop_assert_eq!(result_stack, obs(&body));
    }

    /// **Parent ⫫ child stack isolation** (SPEC §10.1): the child starts with an
    /// empty stack, so a parent value sitting beneath `SPAWN` is invisible to
    /// the child — the result-stack is unchanged by anything below it, and the
    /// parent value remains on the parent stack under the result tuple.
    #[test]
    fn parent_child_stack_isolation(body in completing_block_body(), guard in small()) {
        let plain = await_tuple(&format!("{{ {body} }} SPAWN AWAIT"));
        let with_guard = await_tuple(&format!("{guard} {{ {body} }} SPAWN AWAIT"));
        prop_assert_eq!(&plain, &with_guard); // child sees nothing of the parent
        // the guard value is still on the parent stack, beneath the result tuple.
        let full = run(&format!("{guard} {{ {body} }} SPAWN AWAIT"));
        prop_assert_eq!(full.len(), 2);
        prop_assert_eq!(render(&full[0], full[0].hint), format!("{guard}/1"));
    }

    /// **Reproducibility under a deterministic block** (SPEC §10, roadmap
    /// Phase 8): `{ body } SPAWN AWAIT` observed twice (fresh interpreter each
    /// run) yields the identical tuple — the child runtime is deterministic for
    /// a deterministic block.
    #[test]
    fn child_execution_is_reproducible(body in completing_block_body()) {
        let prog = format!("{{ {body} }} SPAWN AWAIT");
        prop_assert_eq!(await_tuple(&prog), await_tuple(&prog));
    }

    /// **`MONITOR` is observationally pass-through** (SPEC §10.7): inserting
    /// `MONITOR` between `SPAWN` and `AWAIT` does not change the observed
    /// result tuple.
    #[test]
    fn monitor_is_pass_through(body in completing_block_body()) {
        prop_assert_eq!(
            await_tuple(&format!("{{ {body} }} SPAWN AWAIT")),
            await_tuple(&format!("{{ {body} }} SPAWN MONITOR AWAIT"))
        );
    }

    /// **`SUPERVISE` of a completing block returns the same exit tuple as a
    /// direct await** (SPEC §10.7): `{ body } [ n ] SUPERVISE` ≡ the
    /// `['completed', S]` projection.
    #[test]
    fn supervise_completing_block_matches_await(body in completing_block_body(), n in 1i64..3) {
        let sup = await_tuple(&format!("{{ {body} }} [ {n} ] SUPERVISE"));
        prop_assert_eq!(&sup.0, "'completed'");
        prop_assert_eq!(sup.1, obs(&body));
    }
}

// ───────────────── child state machine (SPEC §10.2) ──────────────────────────

/// **State machine — `STATUS`/`KILL`/`AWAIT` status strings** (SPEC §10.2).
/// A freshly spawned child is `running` (finding H1: it runs lazily at AWAIT);
/// `KILL` drives it to `killed`; a completing block awaits to `completed`; a
/// genuinely erroring block awaits to `failed`.
#[test]
fn child_state_machine_status_strings() {
    assert_eq!(
        obs("{ 1 2 ADD } SPAWN STATUS"),
        vec!["'running'".to_string()]
    );
    assert_eq!(obs("{ 1 2 ADD } SPAWN KILL"), vec!["'killed'".to_string()]);
    assert_eq!(await_tuple("{ 1 2 ADD } SPAWN AWAIT").0, "'completed'");
    assert_eq!(await_tuple("{ NOPEWORD } SPAWN AWAIT").0, "'failed'");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// **A genuine error terminates the child `failed`** (SPEC §10.2), while the
    /// partial stack at the point of failure is still projected (probe: an
    /// underflowing `1 ADD` keeps the `1` it pushed). The tuple shape is the
    /// same `[status result-stack]`.
    #[test]
    fn failing_block_awaits_to_failed(body in failing_block_body()) {
        let (status, _result) = await_tuple(&format!("{{ {body} }} SPAWN AWAIT"));
        prop_assert_eq!(status, "'failed'");
    }
}

// ───────────── snapshot isolation of the dictionary (SPEC §10.1) ─────────────

/// **Parent ⫫ child dictionary isolation, both directions** (SPEC §10.1):
/// a word defined *inside* the child block is **not** visible in the parent
/// afterwards (the child gets a private snapshot it may mutate freely); but a
/// word defined in the parent *before* `SPAWN` **is** visible to the child
/// (the snapshot is taken at spawn time).
#[test]
fn dictionary_snapshot_isolation() {
    // (a) child-local DEF does not leak to the parent.
    assert!(
        !ok("{ { 1 ADD } 'CHILD-ONLY' DEF } SPAWN AWAIT 5 CHILD-ONLY"),
        "a word DEF'd inside the child must not resolve in the parent"
    );
    // (b) a parent word DEF'd before SPAWN is in the child's snapshot.
    let (status, result) = await_tuple("{ 1 ADD } 'PARENT-INC' DEF { 5 PARENT-INC } SPAWN AWAIT");
    assert_eq!(status, "'completed'");
    assert_eq!(result, vec!["6/1".to_string()]);
}

// ─────────────────────── findings as guarded oracles ────────────────────────

/// **Finding H1 (guarded oracle): the child runs lazily at `AWAIT`.** `SPAWN`
/// only records the child; `STATUS` immediately after `SPAWN` is `running`, and
/// the work happens when `AWAIT` pulls and runs it.
#[test]
fn finding_h1_child_runs_at_await() {
    assert_eq!(
        obs("{ 1 2 ADD } SPAWN STATUS"),
        vec!["'running'".to_string()]
    );
}

/// **Finding H2 (guarded oracle): a domain failure does not fail the child.**
/// `x 0 /` projects to a reasoned NIL (Bubble Rule §11.2), so the child
/// *completes* with `[NIL]` rather than terminating `failed`. Contrast a
/// genuine error (`NOPEWORD`), which does fail.
#[test]
fn finding_h2_domain_failure_still_completes() {
    let (status, result) = await_tuple("{ 7 0 / } SPAWN AWAIT");
    assert_eq!(
        status, "'completed'",
        "division-by-zero must not fail the child"
    );
    assert_eq!(result, vec!["NIL".to_string()]);

    assert_eq!(await_tuple("{ NOPEWORD } SPAWN AWAIT").0, "'failed'");
}
