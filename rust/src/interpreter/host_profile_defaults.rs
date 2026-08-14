//! The default resource ceilings a host gets if it does not inject its own
//! `RuntimeLimits` — i.e. the playground / native-CLI (`ajisai run`) profile.
//! Split out of `runtime_limits.rs` (SPECIFICATION §14.1's 500-line budget)
//! and re-exported from there (`pub use host_profile_defaults::*;`), so every
//! existing `runtime_limits::DEFAULT_MAX_*` path keeps resolving unchanged.
//!
//! `docs/dev/mcp-host-profiles.md` is the live comparison against the MCP
//! profile (`LOCAL_AGENT_RUNTIME_LIMITS`, `rust/src/agent/api.rs`); the two
//! disagree on purpose (SPEC §2.5: limits are a host safety control, not
//! language semantics). `docs/dev/host-profile-derivation-2026-08-14.md` is
//! the record of how the values below were derived (or, for three of them,
//! deliberately *not* derived — see each constant's own doc comment).

/// Default cap on elements a single generative built-in (`RANGE`, `FILL`,
/// `RESHAPE`, …) may materialize in one call. Mirrors the historical
/// `MAX_MATERIALIZED_ELEMENTS` constant; each generated `Value` costs a few
/// hundred bytes, so one million elements bounds a call to a few hundred MiB
/// rather than a multi-gigabyte OOM abort.
///
/// Unlike [`DEFAULT_MAX_NUMERIC_WORK`] / [`DEFAULT_MAX_COLLECTION_WORK`] /
/// [`super::interpreter_core::DEFAULT_MAX_EXECUTION_STEPS`], this is **not**
/// derived from [`DEFAULT_HOST_TIME_BUDGET_MS`]: it bounds the size of one
/// value, not accumulated time, so a time budget has nothing to say about it.
/// The right basis would be something like "elements a browser can hold and a
/// human can still make sense of on screen" — nobody has measured that yet
/// (`docs/dev/host-profile-derivation-handoff.md` §6), so this stays at its
/// prior value. It was re-checked against the 2026-08-14 re-derivation of
/// [`DEFAULT_MAX_COLLECTION_WORK`]: one linear pass over a
/// `DEFAULT_MAX_MATERIALIZED_ELEMENTS`-sized vector still costs well under 2%
/// of the new budget, so lowering the work budget did not strand this ceiling
/// out of reach (`profile_liveness_tests`).
pub const DEFAULT_MAX_MATERIALIZED_ELEMENTS: usize = 1_000_000;

/// Default cap on the byte length of a single source program handed to
/// `execute`, checked before tokenization allocates per-character buffers.
///
/// Was 64 MiB — deliberately generous, but by nobody's measurement: not a
/// program-size observation, just a round number that happened to be far
/// above the perf-benchmark's largest chain (~1.77 MB) without saying why
/// *that* margin was the right one
/// (`docs/dev/host-profile-derivation-handoff.md` §4, item 3). 16 MiB keeps
/// the same generosity in kind — about 9x the largest known legitimate
/// machine-generated program, so nothing that has ever actually been written
/// against Ajisai gets close to it — while being an amount someone could
/// state a reason for instead of a bare "large enough". Still a judgment
/// call, not a measurement of what a human pastes into a textarea; nobody has
/// done that study either. Memory-constrained hosts — the MCP server in
/// particular, which is tighter still — should inject an even lower
/// `max_source_bytes` via `Interpreter::set_runtime_limits`; that is exactly
/// why the limit is a per-interpreter injectable field rather than a global.
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;

/// Default cap on the digit count of a single numeric literal in source. A
/// 4096-digit integer is astronomically large for any legitimate program,
/// while the ceiling stops a megabyte-long literal from driving an expensive
/// BigInt parse (`Fraction::from_str`) before the value is ever built.
pub const DEFAULT_MAX_NUMERIC_LITERAL_DIGITS: usize = 4_096;

/// How long a host with nobody waiting on the other end of a call — a user in
/// their own tab or terminal, free to close it or hit abort at any point —
/// should let a computation run before answering with a named reason instead
/// of continuing silently. The single dial every ceiling below is derived
/// from, per `docs/dev/host-profile-derivation-handoff.md` §4 item 1: not a
/// value to tune per-ceiling, a budget of *time* that gets turned into a
/// budget of *units* once per meter, at that meter's measured rate.
///
/// This is a **judgment call, not a measurement** — say so plainly, per that
/// document's §6 requirement, because nobody has usage data on how long an
/// Ajisai learner actually waits before assuming a computation is stuck
/// rather than genuinely large. What is known: the playground runs execution
/// off the main thread in a Worker (`src/workers/execution-worker-manager.ts`)
/// behind an explicit abort control
/// (`ExecutionController.abortExecution`), so the relevant threshold is not
/// "the tab looks frozen" — Nielsen/Miller's classic ~10s attention limit for
/// a UI with *no* feedback and no way out — but "how long a user who chose to
/// run something and can already see it running, and can already stop it, is
/// willing to wait before the ceiling should do it for them." 30 seconds is
/// picked as several multiples of that 10s baseline: long enough that a
/// program a curious learner deliberately made bigger doesn't hit a wall for
/// being merely ambitious (§2's rejected "align down to MCP" option is
/// rejected for exactly this reason — the teaching surface should stay wide),
/// short enough that it is still a real ceiling and not a number nobody will
/// ever reach. Re-derive this from real usage data if it ever becomes
/// available; until then, treat it as what it is.
pub const DEFAULT_HOST_TIME_BUDGET_MS: u64 = 30_000;

/// `numericWork`'s units/ms floor on this container, this build, this
/// session (release, rustc 1.94.1, 2026-08-14) — the slowest measured rate
/// among the paths only `numericWork` bounds, the same role `dense tensor
/// lanes` plays in `docs/dev/collection-word-billing-2026-08-13.md` §6.
/// Read from `examples/work_meter_calibration`'s `dense tensor lanes (100k x
/// i64 add)` row, minimum of several runs (4,660–6,197 units/ms observed;
/// this container is shared/virtualized and visibly noisier than the
/// reference container that document used, so the minimum — the safe
/// direction, since a lower rate means a *smaller* derived budget — is kept
/// rather than an average). **Container-specific: re-measure when deploying
/// on different hardware, per the same document's own instruction to
/// re-measure rather than trust a stale constant.**
const NUMERIC_WORK_FLOOR_RATE_UNITS_PER_MS: u64 = 4_660;

/// `collectionWork`'s units/ms floor on this container, this build, this
/// session — measured the same way as [`NUMERIC_WORK_FLOOR_RATE_UNITS_PER_MS`]
/// but read from `examples/collection_word_calibration`'s new §6 section
/// (added in this session), which reads `collection_work_used()` back
/// directly rather than hand-deriving units from the pricing formula: the
/// scan family's algorithm and per-element charge both changed in the
/// 2026-08-14 de-quadraticization follow-up, so the pre-dequadraticization
/// 30,800 units/ms no longer describes this build. `REVERSE 100k` was the
/// floor across every run (30,176–50,915 units/ms observed; minimum kept for
/// the same reason as the numeric floor) — the scan family, once hashed
/// instead of scanned, is no longer the cheapest-per-unit path the way it was
/// before. **Container-specific: re-measure on deployment hardware.**
const COLLECTION_WORK_FLOOR_RATE_UNITS_PER_MS: u64 = 30_176;

/// Default cap on accumulated internal numeric work units charged through the
/// work meter (algebraic products, reciprocal recursion, precision doubling,
/// enclosure refinement).
///
/// Derived, not chosen: [`DEFAULT_HOST_TIME_BUDGET_MS`] ×
/// [`NUMERIC_WORK_FLOOR_RATE_UNITS_PER_MS`]. At the floor rate this spends
/// the full time budget; every other unbounded path is faster per unit and
/// so exhausts this budget in *less* than [`DEFAULT_HOST_TIME_BUDGET_MS`],
/// never more — the same safety direction
/// `LOCAL_AGENT_RUNTIME_LIMITS.max_numeric_work` uses for the MCP profile.
/// Notably **smaller** than the pre-derivation value of 1,000,000,000: that
/// number was never derived (`docs/dev/host-profile-derivation-handoff.md`
/// §3.2 called it "a post-hoc interpretation, not a derived value"), and this
/// container's floor rate is well under the faster reference container the
/// old number implicitly assumed. A faster deployment target would derive a
/// larger number from the same formula; re-measure there rather than raise
/// this by hand.
pub const DEFAULT_MAX_NUMERIC_WORK: u64 =
    DEFAULT_HOST_TIME_BUDGET_MS * NUMERIC_WORK_FLOOR_RATE_UNITS_PER_MS;

/// `executionSteps`' steps/ms floor on this container, this build, this
/// session — the same idea as [`NUMERIC_WORK_FLOOR_RATE_UNITS_PER_MS`] above,
/// but for word dispatch rather than operand size. `executionSteps` prices
/// raw word count and cannot see what a word does internally (the other two
/// meters already price that), so the only path it alone has to bound is a
/// long loop of the *cheapest* word — the dispatch-bound analogue of `dense
/// tensor lanes`.
///
/// Measured from `examples/work_meter_calibration`'s steps/ms section, which
/// tries two shapes and keeps the slower: a flat loop of machine-word
/// additions (~1,470–1,838 steps/ms) and a trampolined user-word tail call —
/// the same construction as `cli::step_limit_tests::DOWN_PROBE` — which is
/// dearer because every iteration pays a dictionary lookup and a frame push,
/// not just an arithmetic op (773–890 steps/ms observed). The trampoline sets
/// the floor, same as the numeric and collection meters: minimum of several
/// runs kept for the same conservative reason. **Container-specific:
/// re-measure on deployment hardware.**
const EXECUTION_STEPS_FLOOR_RATE_STEPS_PER_MS: u64 = 773;

/// Default cap on words executed in one `execute` call — `executionSteps`.
/// [`super::interpreter_core::DEFAULT_MAX_EXECUTION_STEPS`] re-exports this
/// value, mirroring the existing
/// [`super::interpreter_core::MAX_MATERIALIZED_ELEMENTS`] pattern.
///
/// Derived the same way as the two work budgets: [`DEFAULT_HOST_TIME_BUDGET_MS`]
/// × [`EXECUTION_STEPS_FLOOR_RATE_STEPS_PER_MS`]. Previously 100,000, shared
/// verbatim with the MCP profile's own `executionSteps` — not because the two
/// hosts were declared to agree, but because nobody had threaded a
/// playground-specific value through
/// (`docs/dev/host-profile-derivation-handoff.md` §4 item 3: "a long loop of
/// cheap Words hits the same wall on its own tab as it does through MCP").
/// The MCP profile still passes its own `executionSteps = 100_000` explicitly
/// at every call site (`tools/mcp-server/index.js` `LIMITS.executionSteps`,
/// threaded through `--step-limit` / `stepLimit`), so raising this default
/// only widens the playground and native-CLI budget; MCP's is untouched.
///
/// A budget this large is not something a fast test should try to exhaust by
/// actually running it — unlike the two work meters, which a single wide
/// operand can reach in microseconds, reaching a step *count* costs real wall
/// time roughly proportional to dispatch speed, by construction. Tests that
/// need to observe `ExecutionLimitExceeded` inject a small limit instead
/// (`Interpreter::set_max_execution_steps`), the same pattern already used
/// throughout `collection_meter_tests.rs` and `arithmetic_meter_tests.rs` for
/// the same reason.
pub const DEFAULT_MAX_EXECUTION_STEPS: usize =
    (DEFAULT_HOST_TIME_BUDGET_MS * EXECUTION_STEPS_FLOOR_RATE_STEPS_PER_MS) as usize;

/// Default cap on accumulated collection work units.
///
/// Derived the same way as [`DEFAULT_MAX_NUMERIC_WORK`]:
/// [`DEFAULT_HOST_TIME_BUDGET_MS`] ×
/// [`COLLECTION_WORK_FLOOR_RATE_UNITS_PER_MS`]. **No longer twice the numeric
/// budget.** It used to be defined as `2 * DEFAULT_MAX_NUMERIC_WORK`, which
/// was itself a derived relationship — the two floor rates measured 14,465
/// and 30,800 units/ms on the reference container in
/// `docs/dev/collection-word-billing-2026-08-13.md` §6, a ratio of ~2.1 that
/// the `2×` constant captured. The 2026-08-14 de-quadraticization follow-up
/// predicted this ratio would move once the scan family stopped being a
/// linear scan (`docs/dev/collection-word-dequadraticization-2026-08-14.md`
/// §5: "the 2 in `2 * DEFAULT_MAX_NUMERIC_WORK` may no longer hold"), and on
/// this container it did: the two floor rates now measure 4,660 and 30,176,
/// a ratio of ~6.5. Hard-coding a multiplier onto a number that already
/// depends on which path happens to be slowest today, and which changes
/// character with the algorithm, was recording a coincidence as a rule. Each
/// budget is now derived independently from its own measured floor, and they
/// only need to agree on the *time* they bound, which they do by
/// construction (both use the same [`DEFAULT_HOST_TIME_BUDGET_MS`]) rather
/// than by one being defined in terms of the other.
pub const DEFAULT_MAX_COLLECTION_WORK: u64 =
    DEFAULT_HOST_TIME_BUDGET_MS * COLLECTION_WORK_FLOOR_RATE_UNITS_PER_MS;

/// Default cap on the bit length of a BigInt arithmetic result. ~300k decimal
/// digits — generous for exact rationals, but bounded so a doubling cascade
/// cannot blow up to gigabytes.
///
/// Like [`DEFAULT_MAX_MATERIALIZED_ELEMENTS`], this bounds the size of one
/// value rather than accumulated time, so it is not derived from
/// [`DEFAULT_HOST_TIME_BUDGET_MS`] the way the two work budgets are — but it
/// still has to stay *reachable* inside [`DEFAULT_MAX_NUMERIC_WORK`], or it is
/// a claim rather than a control (`profile_liveness_tests`, and the account of
/// exactly this failure mode in `docs/dev/mcp-host-profiles.md`'s "what each
/// MCP-declared limit is pinned by" section). Re-checked against the
/// 2026-08-14 re-derivation of `DEFAULT_MAX_NUMERIC_WORK`: the widening chain
/// that first crosses 1,000,000 bits (74 repeated multiplications by a
/// 4096-digit constant) charges 122,321,640 units, ~87% of the new
/// 139,800,000 budget — close, the same margin `bigintBits` already runs at
/// under the MCP profile (86%), because an N-limb integer costs about N²/4
/// limb-operations by construction and the two ceilings are the closest pair
/// for exactly that reason. Left unchanged because it still fires by name
/// with the numeric-work ceiling as backstop, not because a size-based
/// criterion for it has been written down (`host-profile-derivation-handoff.md`
/// §6 lists this honestly as unmeasured).
pub const DEFAULT_MAX_BIGINT_BITS: u64 = 1_000_000;

/// Default cap on the number of algebraic terms a single continued-fraction /
/// polynomial value may carry.
///
/// Lowered from 100,000 on 2026-08-14. That value predates
/// [`DEFAULT_MAX_NUMERIC_WORK`]'s re-derivation and stopped being live once
/// the numeric-work budget shrank to match this container's measured floor
/// rate (`docs/dev/host-profile-derivation-handoff.md` §4): the doubling
/// cascade that first exceeds 100,000 terms (131,072 terms, at factor 17)
/// charges 520,124,416 units — nearly 4x the new 139,800,000 budget, so
/// `numericWork` would always answer first and the term ceiling would never
/// fire in its life, exactly the failure `profile_liveness_tests` exists to
/// catch (see the module-level docs on that file, and the identical MCP-side
/// history: `max_algebraic_terms` was 4,096 there for the same reason before
/// being lowered to 512). 10,000 is chosen the same way 512 was for the MCP
/// profile: the cascade that first crosses it (16,384 terms, at factor 14)
/// charges 50,356,224 units, ~36% of the work budget — comfortably reachable
/// with room to spare, not cut close the way `bigintBits` is. Like
/// `DEFAULT_MAX_BIGINT_BITS`, this is not itself derived from a stated
/// size-legibility criterion; it is chosen to be *live*, which is a weaker
/// and more urgent property than being *right*.
pub const DEFAULT_MAX_ALGEBRAIC_TERMS: usize = 10_000;
