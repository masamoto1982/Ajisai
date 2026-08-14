//! Unified resource-control limits for internal computation cost (CS5).
//!
//! The execution-step budget (`Interpreter::max_execution_steps`, charged once
//! per word in `execute_builtin.rs`) prices *how many words* run, but not the
//! expensive work performed **inside** a single word: algebraic term×term
//! products, reciprocal/conjugate recursion, sign/bounds precision doubling,
//! BigInt blow-up, huge materializations, and huge numeric-literal parses. A
//! word that loops internally counts as one step, so those costs bypass the
//! step budget entirely. Ajisai must stay exact but return a **diagnosable**
//! runtime failure at a resource ceiling rather than an approximation,
//! wraparound, panic, OOM, or WASM trap.
//!
//! [`RuntimeLimits`] gathers those ceilings in one place. It lives on the
//! interpreter (and every child runtime inherits a copy — it is **not** a
//! global), and small limits can be injected in tests to fire a guard without
//! actually allocating or computing anything huge.
//!
//! Limits are a safety control, never value semantics: conformance results must
//! not depend on a specific limit value, and all conformance must pass under
//! the documented defaults (SPEC §2.5).

use crate::error::{AjisaiError, ResourceLimit, Result};
use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;

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

/// Width of the machine word the bignum arithmetic underneath actually works
/// in. Work is priced in limb×limb products because that is what a bignum
/// multiply costs.
pub const WORK_LIMB_BITS: u64 = 64;

/// Limbs needed to hold `bits`, never zero — every operation costs at least one.
///
/// This is the unit the work meter counts in, and the reason it counts in this
/// unit rather than in operations: the meter used to charge a *term-pair*
/// count, which prices the loop and ignores the size of the numbers inside it.
/// Multiplying two 4096-digit integers and two single-digit ones both charged
/// one unit while differing by orders of magnitude in cost, so the meter could
/// not bound the thing it exists to bound.
pub fn work_limbs(bits: u64) -> u64 {
    bits.div_ceil(WORK_LIMB_BITS).max(1)
}

/// What one algebraic term pair costs, in limb-multiply units.
///
/// A Tier 1 term pair is not one bignum multiply. It multiplies two
/// coefficients *and* two radicands, then square-free-decomposes the product
/// against a basis that grows with the number of distinct primes in play, and
/// inserts the result into an ordered term map. Measured against the rational
/// path — which is one bignum multiply and therefore the meter's natural unit —
/// a term pair costs roughly three orders of magnitude more. Without this
/// factor the two paths ran on the same meter at wildly different prices, so a
/// ceiling calibrated for one was meaningless for the other.
///
/// Empirical, and rounded to a power of two. It was chosen by measuring both
/// paths on one container: the rational chain charged ~80,000 units/ms and the
/// algebraic cascade ~56 units/ms, a ratio near 1,400. The exact figure is a
/// property of that machine and this `num-bigint`, and the meter only needs to
/// be right to within the order of magnitude that keeps a ceiling meaningful
/// for both paths. It is deliberately *not* pinned by a timing test: a wall
/// clock in CI would be flaky, and a stale constant is a worse failure than an
/// unpinned one only if nobody re-measures — so re-measure when either path's
/// representation changes.
pub const ALGEBRAIC_PAIR_UNITS: u64 = 1_024;

/// The work a binary exact-arithmetic operation of these two operand widths
/// costs, in limb-multiply units.
///
/// Every schema is priced limb×limb, including addition and subtraction. That
/// looks wrong for a moment and is the whole point: Ajisai's `+` is *rational*
/// addition, not integer addition. `a/b + c/d` cross-multiplies into
/// `(ad + cb)/(bd)` and then normalizes by a gcd — three multiplications and a
/// Euclid, none of them linear in the wider operand.
///
/// Pricing it as linear was not a rounding error. Measured on the reference
/// container with the literal parsed once (`examples/work_meter_calibration`),
/// a 4096-digit rational addition takes 745 µs — *twice* what a multiplication
/// of the same width takes — and used to be charged 213 units against that
/// multiplication's ~430,000. A meter that prices the cheaper operation four
/// thousand times higher than the dearer one is not bounding anything; it was
/// possible to spend the whole `wallTimeMs` budget on additions while the work
/// meter read a fraction of a percent.
///
/// Deliberately an upper bound on the real cost and deliberately cheap to
/// compute: it is charged before the operation runs, so a runaway computation
/// is refused rather than measured. The bound is loosest for very wide
/// multiplications, where `num-bigint` switches to Karatsuba and beats the
/// limb×limb estimate — over-charging is the safe direction, and
/// `examples/work_meter_calibration` is where to see by how much.
pub fn binary_numeric_work(left_bits: u64, right_bits: u64) -> u64 {
    work_limbs(left_bits).saturating_mul(work_limbs(right_bits))
}

/// What the work meter needs to know about one operand, independent of the
/// shape it arrived in.
///
/// The meter used to read a `Fraction` and stop there, which made it a meter on
/// the *representation* rather than on the arithmetic: `2 3 *` was charged and
/// `[ 2 ] 3 *` was free, because the second one leaves the scalar path and
/// every other path charged nothing. Whether an operand is stored as a scalar,
/// a one-element vector or an N-lane tensor is an internal decision
/// (LANG.AUTHORITY.FREEDOM says it is unobservable), and a safety control whose
/// price turns on an unobservable decision is not a control. A broadcast
/// performs one operation per lane, so lanes are what it is charged for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperandWork {
    /// Numeric leaves the operand carries. A scalar is one lane.
    pub lanes: u64,
    /// The widest leaf, in bits.
    pub bits: u64,
    /// The largest algebraic term count among the leaves, or 0 when every leaf
    /// is rational.
    pub terms: u64,
}

impl OperandWork {
    /// The measure of a single rational leaf.
    pub const fn leaf(bits: u64) -> Self {
        Self {
            lanes: 1,
            bits,
            terms: 0,
        }
    }

    /// Combine sibling leaves: lanes add, width and term count take the worst.
    pub fn join(self, other: Self) -> Self {
        Self {
            lanes: self.lanes.saturating_add(other.lanes),
            bits: self.bits.max(other.bits),
            terms: self.terms.max(other.terms),
        }
    }
}

/// The work a binary arithmetic operation costs across every lane a broadcast
/// will touch.
///
/// `pair_units` is what one lane pair costs *above* the bignum operation: 1 for
/// a rational pair, and the term-pair count times [`ALGEBRAIC_PAIR_UNITS`] for
/// an algebraic one. A scalar operation is `lanes = 1` of this, so shape
/// changes nothing about the price.
pub fn broadcast_numeric_work(left: OperandWork, right: OperandWork, pair_units: u64) -> u64 {
    // Broadcast repeats the narrower operand across the wider one, so the
    // wider lane count is how many operations actually run.
    let lanes = left.lanes.max(right.lanes).max(1);
    binary_numeric_work(left.bits, right.bits)
        .saturating_mul(pair_units)
        .saturating_mul(lanes)
}

/// What one boxed element copy costs, in collection units.
///
/// A collection unit is the cheapest thing this meter counts: one equality
/// probe between two machine-word scalars, measured at 5.8 ns for a 16,000
/// element `UNIQUE` and 9.1 ns at 100,000 — call it 6 ns.
///
/// Copying one element costs 80 ns at 4,000 elements and 389 ns at 100,000; the
/// spread is the allocator and the cache, not the copy. 16 units (~96 ns) sits
/// inside that range and is the smallest power of two that keeps the dearest
/// measured copy — 100,000 elements through `REVERSE`, 38.9 ms — charging
/// 43,700 units/ms, above the 30,800 units/ms floor of the scan family and well
/// above the 14,465 units/ms floor the arithmetic meter already accepts on an
/// unbounded path.
///
/// Empirical, and re-measured by `examples/collection_word_calibration`.
pub const COLLECTION_COPY_UNITS: u64 = 16;

/// What comparing one *algebraic* element costs, in collection units.
///
/// `Algebraic::eq` is `Algebraic::cmp`: deciding it rebases both sides over a
/// common refinement of their radical bases, which is nothing like a limb
/// compare. Measured at 3.0 µs through `UNIQUE` and 3.3 µs through `SORT`
/// (after correcting for Rust's sort detecting the already-ascending run that
/// `[ 2 n ] RANGE { SQRT } MAP` produces) — 500 to 550 units at 6 ns each.
///
/// Distinct from [`ALGEBRAIC_PAIR_UNITS`], which prices an algebraic *product*.
/// A product and a comparison are different operations and the measurements
/// disagree by a factor of two; one constant serving both would be a rounding
/// of two numbers into one.
pub const ALGEBRAIC_ELEMENT_UNITS: u64 = 512;

/// `executionSteps`' steps/ms floor on this container, this build, this
/// session — the same idea as the two work-meter floors above, but for word
/// dispatch rather than operand size. `executionSteps` prices raw word count
/// and cannot see what a word does internally (the other two meters already
/// price that), so the only path it alone has to bound is a long loop of the
/// *cheapest* word — the dispatch-bound analogue of `dense tensor lanes`.
///
/// Measured from `examples/work_meter_calibration`'s new steps/ms section
/// (added in this session), which tries two shapes and keeps the slower: a
/// flat loop of machine-word additions (~1,470–1,838 steps/ms) and a
/// trampolined user-word tail call — the same construction as
/// `cli::step_limit_tests::DOWN_PROBE` — which is dearer because every
/// iteration pays a dictionary lookup and a frame push, not just an
/// arithmetic op (773–890 steps/ms observed). The trampoline sets the floor,
/// same as the numeric and collection meters: minimum of several runs kept
/// for the same conservative reason. **Container-specific: re-measure on
/// deployment hardware.**
const EXECUTION_STEPS_FLOOR_RATE_STEPS_PER_MS: u64 = 773;

/// Default cap on words executed in one `execute` call — `executionSteps`.
///
/// Lives here, not beside [`super::interpreter_core::MAX_USER_WORD_DEPTH`],
/// because the derivation needs [`DEFAULT_HOST_TIME_BUDGET_MS`] and the two
/// floor-rate constants above; [`super::interpreter_core::DEFAULT_MAX_EXECUTION_STEPS`]
/// re-exports this value, mirroring the existing
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

/// The units a collection Word is charged for touching one element, given the
/// measure of the operand it came from.
///
/// Two prices, because the measurements are two different shapes. Copying is
/// `constant + width`: duplicating a 213-limb BigInt costs 3.4x what
/// duplicating a machine word does, not 213x, because the wide part is a
/// memcpy. Comparing is `width`: an equality test walks limbs until they
/// differ, so it is the width that is traversed, not a constant plus it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementCost {
    /// Numeric leaves one element carries. 1 for a scalar; `k` for an element
    /// that is itself a `k`-element vector, because an equality test on that
    /// element is a loop over `k` leaves and a copy of it copies `k` of them.
    pub leaves: u64,
    /// What one leaf costs to look at: limbs for a rational, and
    /// `terms × ALGEBRAIC_ELEMENT_UNITS` for an algebraic, whichever is dearer.
    pub width: u64,
}

impl ElementCost {
    /// The cost of one element in a collection of `element_count` elements
    /// whose combined measure is `work`.
    ///
    /// `OperandWork::lanes` counts leaves across the whole operand, so the
    /// leaves *per element* are that divided by the element count — which is
    /// how a vector of 4,000 sixty-four-element rows is told apart from a
    /// vector of 256,000 scalars, two operands with the same leaf count and a
    /// 64x difference in what one element costs to compare.
    pub fn measure(work: OperandWork, element_count: usize) -> Self {
        let elements = (element_count as u64).max(1);
        let width = work_limbs(work.bits).max(work.terms.saturating_mul(ALGEBRAIC_ELEMENT_UNITS));
        Self {
            leaves: (work.lanes / elements).max(1),
            width: width.max(1),
        }
    }

    /// What one equality probe or order comparison against one element costs.
    pub fn probe(self) -> u64 {
        self.leaves.saturating_mul(self.width)
    }

    /// What copying one element into a result costs.
    pub fn copy(self) -> u64 {
        self.leaves
            .saturating_mul(COLLECTION_COPY_UNITS.saturating_add(self.width))
    }

    /// What copying `count` elements costs.
    pub fn copies(self, count: usize) -> u64 {
        self.copy().saturating_mul(count as u64)
    }

    /// What a comparison sort over `count` elements costs at its worst case.
    ///
    /// `n⌈log₂n⌉` is the bound, not the count Rust's sort actually performs —
    /// it detects sorted runs and can finish in `n`. A safety control prices
    /// the worst case, because the input that reaches it is the one that did
    /// not have a sorted run.
    pub fn comparison_sort(self, count: usize) -> u64 {
        let n = count as u64;
        let depth = u64::from(n.max(1).ilog2()) + 1;
        self.probe()
            .saturating_mul(n)
            .saturating_mul(depth)
            .saturating_add(self.copies(count))
    }
}

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

/// Operand width for the work meter on an exact (Tier 0 or Tier 1) value:
/// the wider of what it stores in coefficients and in radicands.
pub fn exact_work_bits(value: &ExactReal) -> u64 {
    match value {
        ExactReal::Algebraic(a) => a.max_coefficient_bits().max(a.max_radicand_bits()),
        other => other.max_coefficient_bits(),
    }
}

/// Operand width for the work meter, without paying to measure a small one.
///
/// `Fraction` keeps machine-word values in a `Small` representation, which is
/// the overwhelming majority of ordinary arithmetic; asking a `BigInt` for its
/// bit length on every add in a hot loop would cost more than the guard saves.
/// A small fraction is one limb by definition.
pub fn fraction_work_bits(f: &Fraction) -> u64 {
    if f.is_small() {
        return 1;
    }
    fraction_result_bits(f)
}

/// The width a fraction actually occupies: the wider of its two halves.
pub fn fraction_result_bits(f: &Fraction) -> u64 {
    if f.is_nil() {
        return 0;
    }
    f.numerator().bits().max(f.denominator().bits())
}

/// Unified internal-computation-cost ceilings (CS5).
///
/// This deliberately does **not** include the execution-step budget, which
/// remains the adjacent `Interpreter::max_execution_steps` field: the step
/// budget prices word count, whereas `RuntimeLimits` prices the per-word
/// internal work the step budget cannot see. The two together are the
/// interpreter's complete runtime-safety envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeLimits {
    /// Max elements a single generative built-in may materialize in one call.
    /// Folds the former `MAX_MATERIALIZED_ELEMENTS` guard into this structure.
    pub max_materialized_elements: usize,
    /// Max byte length of a source program handed to `execute` (checked before
    /// tokenization).
    pub max_source_bytes: usize,
    /// Max digit count of a single numeric literal (checked before the BigInt
    /// parse builds the value).
    pub max_numeric_literal_digits: usize,
    /// Max accumulated internal numeric work units. Consumed by the work meter
    /// in the CS5 follow-up.
    pub max_numeric_work: u64,
    /// Max accumulated collection work units: the element copies, comparisons
    /// and equality probes performed *inside* collection Words, which the step
    /// budget cannot see because each such Word is one step however much it
    /// does.
    pub max_collection_work: u64,
    /// Max bit length of a BigInt arithmetic result. Consumed by the work meter
    /// in the CS5 follow-up.
    pub max_bigint_bits: u64,
    /// Max algebraic-term count of a single continued-fraction / polynomial
    /// value. Consumed by the work meter in the CS5 follow-up.
    pub max_algebraic_terms: usize,
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            max_materialized_elements: DEFAULT_MAX_MATERIALIZED_ELEMENTS,
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_numeric_literal_digits: DEFAULT_MAX_NUMERIC_LITERAL_DIGITS,
            max_numeric_work: DEFAULT_MAX_NUMERIC_WORK,
            max_collection_work: DEFAULT_MAX_COLLECTION_WORK,
            max_bigint_bits: DEFAULT_MAX_BIGINT_BITS,
            max_algebraic_terms: DEFAULT_MAX_ALGEBRAIC_TERMS,
        }
    }
}

impl RuntimeLimits {
    /// Reject a source program larger than `max_source_bytes`, before
    /// tokenization. Returns a diagnosable `AjisaiError` (never a panic/OOM).
    pub fn check_source_bytes(&self, byte_len: usize) -> Result<()> {
        if byte_len > self.max_source_bytes {
            return Err(AjisaiError::ResourceLimitExceeded {
                resource: ResourceLimit::SourceBytes,
                limit: self.max_source_bytes as u64,
                observed: Some(byte_len as u64),
                progress: None,
            });
        }
        Ok(())
    }

    /// Reject a numeric literal with more than `max_numeric_literal_digits`
    /// digits, before the BigInt parse builds the value. `digit_len` counts
    /// digit characters only (sign, radix point, and separators excluded).
    pub fn check_numeric_literal_digits(&self, digit_len: usize) -> Result<()> {
        if digit_len > self.max_numeric_literal_digits {
            return Err(AjisaiError::ResourceLimitExceeded {
                resource: ResourceLimit::NumericLiteralDigits,
                limit: self.max_numeric_literal_digits as u64,
                observed: Some(digit_len as u64),
                progress: None,
            });
        }
        Ok(())
    }

    /// Reject an exact (Tier 1 algebraic) arithmetic result whose size crosses
    /// the internal-computation ceilings: `term_count` past
    /// `max_algebraic_terms` (multiplicative term explosion, e.g. repeatedly
    /// multiplying distinct `√p`), or `coeff_bits` past `max_bigint_bits`
    /// (BigInt blow-up). Bounds *accumulation* so operands feeding the next
    /// operation stay sane; the per-operation work is bounded separately by the
    /// work meter's pre-charge. Each ceiling reports itself by name through
    /// `ResourceLimitExceeded`: "this value carries too many algebraic terms"
    /// and "this coefficient is too many bits wide" are separately declared
    /// limits, and folding both into the step budget's category left a host
    /// unable to say which of the two an agent had actually hit.
    pub fn check_algebraic_size(&self, term_count: usize, coeff_bits: u64) -> Result<()> {
        if term_count > self.max_algebraic_terms {
            return Err(AjisaiError::ResourceLimitExceeded {
                resource: ResourceLimit::AlgebraicTerms,
                limit: self.max_algebraic_terms as u64,
                observed: Some(term_count as u64),
                progress: None,
            });
        }
        self.check_bigint_bits(coeff_bits)
    }

    /// Reject an arithmetic result wider than `max_bigint_bits`.
    ///
    /// Split out of `check_algebraic_size` because it is not an algebraic
    /// concern: a plain rational product blows a coefficient up exactly the
    /// same way, and used to reach this ceiling through no path at all. A
    /// repeated-multiplication chain was bounded only by the *step* budget,
    /// which prices word count — so four hundred multiplications, 0.4% of that
    /// budget, could spend tens of seconds building a multi-megabyte integer
    /// while every size ceiling stayed silent.
    pub fn check_bigint_bits(&self, bits: u64) -> Result<()> {
        if bits > self.max_bigint_bits {
            return Err(AjisaiError::ResourceLimitExceeded {
                resource: ResourceLimit::BigintBits,
                limit: self.max_bigint_bits,
                observed: Some(bits),
                progress: None,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_documented_ceilings() {
        let limits = RuntimeLimits::default();
        assert_eq!(
            limits.max_materialized_elements,
            DEFAULT_MAX_MATERIALIZED_ELEMENTS
        );
        assert_eq!(limits.max_source_bytes, DEFAULT_MAX_SOURCE_BYTES);
        assert_eq!(
            limits.max_numeric_literal_digits,
            DEFAULT_MAX_NUMERIC_LITERAL_DIGITS
        );
    }

    #[test]
    fn source_byte_ceiling_fires_at_a_low_injected_limit() {
        let limits = RuntimeLimits {
            max_source_bytes: 8,
            ..RuntimeLimits::default()
        };
        assert!(
            limits.check_source_bytes(8).is_ok(),
            "at the limit is allowed"
        );
        let err = limits
            .check_source_bytes(9)
            .expect_err("over the limit must error");
        assert!(err.to_string().contains("exceeds the limit"));
    }

    #[test]
    fn numeric_literal_digit_ceiling_fires_at_a_low_injected_limit() {
        let limits = RuntimeLimits {
            max_numeric_literal_digits: 4,
            ..RuntimeLimits::default()
        };
        assert!(limits.check_numeric_literal_digits(4).is_ok());
        let err = limits
            .check_numeric_literal_digits(5)
            .expect_err("over the digit limit must error");
        assert!(err.to_string().contains("exceeds the limit"));
    }
}
