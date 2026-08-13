# Ajisai host resource profiles

Status date: 2026-08-13.

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

| ceiling | MCP / `ajisai agent` | playground and `ajisai run` | what it bounds |
|---|---:|---:|---|
| `sourceBytes` | 65,536 | 67,108,864 | byte length of one program |
| `executionSteps` | 100,000 | 100,000 | Words executed |
| `materializedElements` | 100,000 | 1,000,000 | elements one generative Word may build |
| `numericLiteralDigits` | 4,096 | 4,096 | digits in one numeric literal |
| `numericWork` | 10,000,000 | 1,000,000,000 | accumulated arithmetic work, in limb-multiply units, in every operand shape |
| `collectionWork` | 20,000,000 | 2,000,000,000 | accumulated element operations inside collection Words — copies, order comparisons, equality probes |
| `bigintBits` | 262,144 | 1,000,000 | coefficient width of one exact arithmetic result |
| `algebraicTerms` | 512 | 100,000 | term count of one exact algebraic value |

The two work ceilings are not the same number because they do not count the
same thing, and setting them equal would have made one of them mean something
different from the other. They are set to bound the same amount of *time*: at
each meter's slowest measured path — 14,465 units/ms for `numericWork`, 30,800
for `collectionWork` — 10,000,000 and 20,000,000 both buy about 0.7 s. Their
sum is what `wallTimeMs` backstops.

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
- **A five-second computation** completes in the playground and is refused by
  the MCP server. Since the work meters cover every expensive path it is
  refused *by name* — `numericWork` or `collectionWork` — rather than by the
  clock; the playground's budgets are a hundred times larger, so the same
  program runs there.

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
