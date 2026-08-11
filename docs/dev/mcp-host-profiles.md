# Ajisai host resource profiles

Status date: 2026-08-11.

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
| `numericWork` | 10,000,000 | 1,000,000,000 | accumulated internal work units, exact algebraic arithmetic only |
| `bigintBits` | 262,144 | 1,000,000 | coefficient width of one exact algebraic value |
| `algebraicTerms` | 4,096 | 100,000 | term count of one exact algebraic value |

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
- **A five-second computation** completes in the playground and is killed by
  the MCP server as `timeout`.

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
  `backend/parity-test.js`. Covers `sourceBytes`, `wallTimeMs`,
  `responseBytes`, `executionSteps`, `materializedElements` and
  `numericLiteralDigits`.
- **`hostGate`** — enforced by the adapter's admission path rather than by a
  program. Covers `concurrentExecutions`.
- **`injectedLimit`** — pinned in Rust with a small injected ceiling, because
  the declared value is not reachable within `wallTimeMs`. Covers
  `numericWork`, `bigintBits` and `algebraicTerms`.

The third group deserves the blunt version. `numericWork`, `bigintBits` and
`algebraicTerms` bound exact algebraic (Tier 1) values only, and the
per-operation cost the work meter charges is far below what the operation
actually costs: a product of two-radical sums grows about twelvefold in wall
time per additional factor while charging only a few work units. At the
declared values, `wallTimeMs` always arrives first. The numbers reported in
`mcp.limits` are the configured ceilings and are reported truthfully; they are
not bounds a caller can observe at this profile. Recalibrating the work
estimate so the size ceilings bind before the clock is engine work tracked in
[`mcp-readiness.md`](./mcp-readiness.md).

Two related gaps, recorded for the same reason:

- Plain rational arithmetic does not pass through the algebraic size guard, so
  a large integer product is bounded by `executionSteps` rather than by
  `bigintBits`.
- `numericWork` is charged only on the exact algebraic path.
