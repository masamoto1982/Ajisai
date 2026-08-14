# Ajisai host resource profiles

Status date: 2026-08-13. Updated 2026-08-14: the playground / native-default
profile's ceilings are now derived, not chosen — see
[`host-profile-derivation-handoff.md`](./host-profile-derivation-handoff.md)
for the work and the reasoning behind each number below.

Ajisai's resource ceilings are **host safety controls, not language semantics**
(`SPECIFICATION.html` §2.5). A conforming host chooses its own, and all
conformance must pass under the documented defaults. Two official hosts
therefore disagree about them on purpose.

That is only safe when each host says what it applies. This page is the
comparison; each host also publishes its own profile at runtime, so nobody has
to trust a document that could drift:

| host | how to read the applied profile |
|---|---|
| MCP stdio server | `mcp.limits` on every tool result, and the `ajisai://limits` resource |
| browser playground | the `profile:` badge in the header (hover for the full table); `AjisaiInterpreter.host_profile()` |
| native CLI (`ajisai run`) | interpreter defaults, `--step-limit` overrides the step budget |
| native CLI (`ajisai agent …`) | the same profile the MCP server applies |

## The profiles

| ceiling | MCP / `ajisai agent` | playground and `ajisai run` | what it bounds | derived? |
|---|---:|---:|---|---|
| `sourceBytes` | 65,536 | 16,777,216 | byte length of one program | judgment call (§ below) |
| `executionSteps` | 100,000 | 23,190,000 | Words executed | **yes**, from the host time budget |
| `materializedElements` | 100,000 | 1,000,000 | elements one generative Word may build | no — size, not time (§ below) |
| `numericLiteralDigits` | 4,096 | 4,096 | digits in one numeric literal | no |
| `numericWork` | 10,000,000 | 139,800,000 | accumulated arithmetic work, in limb-multiply units, in every operand shape | **yes**, from the host time budget |
| `collectionWork` | 20,000,000 | 905,280,000 | accumulated element operations inside collection Words — copies, order comparisons, equality probes | **yes**, from the host time budget |
| `bigintBits` | 262,144 | 1,000,000 | coefficient width of one exact arithmetic result | no — size, not time (§ below) |
| `algebraicTerms` | 512 | 10,000 | term count of one exact algebraic value | no — size, not time (§ below) |

Both profiles' pair of work ceilings bound the same amount of *time*, not the
same number of units — the two paths do not count the same thing, so setting
them equal would have made one of them mean something different from the
other. Each host has its own single time budget, and each work ceiling is that
budget times the meter's own measured floor rate (its slowest unbounded path,
in units/ms, on the container it was measured on):

| host | time budget | `numericWork` floor rate | `collectionWork` floor rate |
|---|---:|---:|---:|
| MCP / `ajisai agent` | 5,000 ms (`wallTimeMs`, adapter-enforced) | 14,465 units/ms | 30,800 units/ms |
| playground / `ajisai run` | 30,000 ms (`DEFAULT_HOST_TIME_BUDGET_MS`) | 4,660 units/ms | 30,176 units/ms |

The two hosts' floor rates were measured on different containers at different
times (MCP's in `docs/dev/collection-word-billing-2026-08-13.md` §6, before
the scan family was de-quadraticized; the playground's in this session, after)
and are not comparable to each other — only within a host, between its own
`numericWork` and `collectionWork` rows, does the ratio mean anything.
Notably, the playground's two floor rates are *not* a clean multiple of one
another (30,176 / 4,660 ≈ 6.5) the way the MCP profile's used to look
(30,800 / 14,465 ≈ 2.1, once hard-coded as `collectionWork = 2 × numericWork`).
That 2× relationship was itself a derived-but-coincidental ratio of two
measured rates, not a rule, and the 2026-08-14 de-quadraticization of
`UNIQUE`/`TALLY`/`GROUP` moved it, exactly as
[`collection-word-dequadraticization-2026-08-14.md`](./collection-word-dequadraticization-2026-08-14.md)
§5 predicted it might. Each work ceiling is now derived independently from its
own host's time budget and its own floor rate; the two only need to agree on
the *time* they bound, and they do that by construction.

`DEFAULT_HOST_TIME_BUDGET_MS` (30 s) is a **judgment call, not a
measurement** — nobody has usage data on how long an Ajisai learner tolerates
a running computation before assuming it is stuck. It is several multiples of
the classic ~10 s "the UI looks frozen" threshold (Nielsen/Miller), extended
because the playground shows a running state and an abort control rather than
giving no feedback at all — see the constant's doc comment in
`rust/src/interpreter/runtime_limits.rs` (`DEFAULT_HOST_TIME_BUDGET_MS`) for
the full reasoning, and treat it as replaceable the moment real usage data
exists.

**`executionSteps` is now derived too**, closing the one ceiling that used to
be identical across hosts for no stated reason (a shared Rust constant nobody
had threaded a playground-specific value through, not a decision that the two
hosts should agree). It is derived the same way as the two work ceilings — the
host time budget times a measured floor rate — except the unit being priced is
raw word-dispatch count rather than operand size, and the floor path is
therefore a long loop of the *cheapest* word rather than the widest operand
(measured at 773 steps/ms for a trampolined user-word call, the dispatch-bound
analogue of `numericWork`'s "dense tensor lanes"; see
`rust/examples/work_meter_calibration.rs`). The MCP profile's own
`executionSteps` is untouched: every real MCP call site threads its own
`100_000` explicitly (`tools/mcp-server/index.js` `LIMITS.executionSteps`),
independent of whatever the interpreter's own default is, so raising the
playground/native default did not move it.

**Three ceilings are deliberately *not* derived from the time budget, and stay
at their prior values (mostly) or a differently-reasoned one:**

- `materializedElements` and `bigintBits` bound the size of *one value*, not
  accumulated time, so a time budget has nothing to say about them. The right
  basis would be something like "elements a browser can hold and a human can
  still make sense of on screen" — nobody has measured that, so both stay at
  their prior values, re-checked only for the weaker property of staying
  reachable inside the new `numericWork`/`collectionWork` budgets (they do:
  see the doc comments on `DEFAULT_MAX_MATERIALIZED_ELEMENTS` and
  `DEFAULT_MAX_BIGINT_BITS` in `runtime_limits.rs`).
- `algebraicTerms` moved (100,000 → 10,000) for the same reason it moved on
  the MCP side once before: the old value stopped being *live* once
  `numericWork` shrank to match this container's measured rate — the doubling
  that first exceeds 100,000 terms costs 520,124,416 units, nearly 4× the new
  139,800,000 budget, so `numericWork` would always answer first and the term
  ceiling would never fire (`profile_liveness_tests` exists to catch exactly
  this). 10,000 is chosen the same way the MCP profile's 512 was: live with
  comfortable margin (the crossing cascade costs ~36% of the work budget), not
  derived from a stated size-legibility criterion either.
- `sourceBytes` (playground: 64 MiB → 16 MiB) is a **judgment call with a
  stated anchor**, replacing a round number that had no stated reason at all:
  16 MiB is about 9× the largest known legitimate machine-generated Ajisai
  program (the perf-benchmark's ~1.77 MB chain), generous in the same
  direction the old 64 MiB was, but for a reason someone can point to instead
  of "large enough". Not a measurement of what a human actually pastes into a
  textarea — nobody has done that study either.

None of the above changes anything about *why* the two profiles are allowed to
differ — `SPECIFICATION.html` §2.5 still says a conforming host chooses its
own ceilings, and the rejected alternatives (align the profiles to each other,
or collapse them into one) are unchanged from
`host-profile-derivation-handoff.md` §2. What changed is that "the playground
is generous because 67 seconds felt right" is no longer the explanation for
any number in this table.

The MCP server declares four further ceilings that exist only at the adapter,
because they bound the *call* rather than the computation: `wallTimeMs`
(5,000), `responseBytes` (1,048,576), `concurrentExecutions` (4), and the
adapter-side `sourceBytes` check that rejects an oversized program before a
backend is entered. The playground has no equivalent: it runs in the user's
own tab, at their own pace, for their own eyes.

## Intended divergences

These are decisions, not defects. Each is recorded where it can be seen:

- **`[ 0 100001 ] RANGE`** succeeds in the playground, materializing 100,002
  elements, and answers `NIL(spaceExhausted)` under the MCP profile. Pinned as
  a golden case with an explicit `hostDivergence` block
  (`tools/mcp-server/golden/cases.json`).
- **A 1 MiB program** runs in the playground and is refused by the MCP server
  as `sourceTooLarge`.
- **A computation past MCP's 5-second-equivalent work budget** completes in
  the playground and is refused by the MCP server. Since the work meters
  cover every expensive path it is refused *by name* — `numericWork` or
  `collectionWork` — rather than by the clock; the playground's budgets are
  several times to over an order of magnitude larger (§ above; the exact
  ratio moves with re-measurement, unlike the old "100x" which was itself
  just the pre-derivation numbers' coincidence), so the same program runs
  there.

The practical consequence is worth stating plainly: **a program prototyped in
the playground can behave differently through MCP.** It will never compute a
*different value* — that is what the shared value protocol and the backend
parity tests guarantee — but it can succeed in one host and answer a
reason-carrying `NIL`, or fail as a host error, in the other. Prototype
against the tighter profile if the program is destined for MCP.

## What each MCP-declared limit is actually pinned by

`tools/mcp-server/golden/limits.json` holds one entry per declared limit, and
`selftest.js` fails if that set and the served `LIMITS` set ever differ — so a
newly declared ceiling cannot be added without saying how it is exercised.
Three kinds of coverage appear there:

- **`boundary`** — a real source just under the declared value that succeeds,
  and one just over it that fails in the stated way. Run against the live
  server on every self-test, and compared across both backends in
  `backend/parity-test.js`. Covers `sourceBytes`, `responseBytes`,
  `executionSteps`, `materializedElements`, `numericLiteralDigits`,
  `numericWork`, `collectionWork`, `bigintBits` and `algebraicTerms`.
- **`hostGate`** — enforced by the adapter's admission path rather than by a
  program, and exercised through that path in `selftest.js`. Covers
  `concurrentExecutions` and `wallTimeMs`.
- **`injectedLimit`** — pinned in Rust with a small injected ceiling, because
  the declared value is not reachable within `wallTimeMs`. Covers nothing at
  present.

Every declared ceiling now has coverage of one of the first two kinds, which
was not true for most of this document's life. Three earlier revisions recorded
the opposite state, and the reasons are worth keeping because each was a
different defect:

- `numericWork` was charged only on the scalar arithmetic path, so any operand
  in vector or tensor shape ran free — a ceiling turned off by a representation
  decision the language says is unobservable. It is now charged at one entry
  for every shape, priced as lanes × limb-multiply units.
- `algebraicTerms` was declared at 4,096, which the work budget could not pay
  to reach: the doubling that first exceeds it costs 16,799,744 units against
  10,000,000, so `numericWork` always answered first. Lowered to 512, and
  `rust/src/agent/profile_liveness_tests.rs` now fails the build if any size
  ceiling is declared past what the work budget can reach.
- `wallTimeMs` really was the ceiling that arrived first for anything
  expensive, because the expensive things were unpriced. The last of them was
  the collection family: `[ 0 99999 ] RANGE UNIQUE` is quadratic and took 48
  seconds as *one* execution step, which is why the clock was the only thing
  that noticed. With `collectionWork` charging it, the same program is refused
  by name in 164 ms, and no source program reaches the clock any more — so
  `wallTimeMs` moved to `hostGate`, exercised against a backend built with a
  1 ms deadline.

What the work meters charge is documented where it was measured:
[`collection-word-billing-2026-08-13.md`](./collection-word-billing-2026-08-13.md)
for the collection prices, and `rust/examples/work_meter_calibration.rs` /
`rust/examples/collection_word_calibration.rs` for the measurements themselves.
Both are runnable: re-measure rather than trust the constants when a
representation changes.

`UNIQUE` / `TALLY` / `GROUP` themselves stopped being a linear scan on
2026-08-14: [`collection-word-dequadraticization-2026-08-14.md`](./collection-word-dequadraticization-2026-08-14.md)
replaced it with a `Value: Hash`-backed `HashMap` lookup and re-derived the
per-element charge accordingly. The ceiling values in the table above did not
move; the golden boundary source that reaches `collectionWork` did.
