# Analysis (off-tree probe) — kleene-triage tier sweep, condition A

- Date (UTC): 2026-07-08
- Type: **one-off off-tree probe** (Plan 2). The task spec, `cases.tsv`, and
  reference solution were built and validated in the recorder's scratchpad and
  are **not** committed to `tasks/` (per the one-off decision — promote to a
  permanent suite task only if it earns a slot). This file records the results
  and the concrete findings.
- Model / version: Claude Haiku 4.5 (T1), Claude Sonnet 4.5 (T2), Claude Opus 4.8 (T3)
- Condition: A = spec only (SPECIFICATION.html + task text; no SKILL.md)
- ajisai commit: 42dbe98 (measured with the CLI built at that base)
- Purpose: the first task **deliberately engineered to move the binary** — a
  three-valued (Kleene/NIL) alarm-classification pipeline with short-circuit
  traps a naive "bubble on any NIL" implementation gets wrong (e.g.
  `FALSE NIL NIL → FALSE`, `NIL FALSE FALSE → FALSE`). Also built to have a
  large solution space so the Plan-1 conservation question becomes non-trivial.

## The task (kept off-tree)

Define `ALARM` over three flags `a b c`, each `TRUE`/`FALSE`/`NIL`, realizing
`a AND (b OR c)` under strong-Kleene three-valued logic with `NIL` as the
undecided value, then fail-safe venting undecided (`NIL`) results to `TRUE`.
12 acceptance cases chosen to probe the NIL short-circuit traps. A validated
reference solution passes 12/12.

## Results

Verified by the recorder with the off-tree verifier (concat solution +
invocation, run `--json`, compare `stackDisplay`). Effort is harness-measured.

| tier | casesPassing | onePass | tool-uses | tokens | sec | words | ncLines | final solution |
|---|---|---|---|---|---|---|---|---|
| T1 (Haiku) | **12/12** | yes | 21 | 91.9k | 155 | 4 | 1 | `{ OR AND ^ TRUE }` |
| T2 (Sonnet) | **12/12** | **no** (fixed a `^` bug) | 25 | 78.9k | 285 | 4 | 1 | `{ OR AND ^ TRUE }` |
| T3 (Opus) | **12/12** | yes | 9 | 103.4k | 200 | 4 | 1 | `{ OR AND ^ TRUE }` |

All three solutions are **byte-identical** (`md5` collapses to one hash).

## Observations

- **The binary floored a FIFTH time — even on a purpose-built trap task.** All
  three tiers pass 12/12. The Kleene/NIL traps did not catch any tier's *final*
  solution. No pass/fail transition.

- **Why: Ajisai's native operators absorbed the difficulty.** The whole
  trap-handling — `FALSE AND NIL = FALSE`, `TRUE OR NIL = TRUE` short-circuits —
  is done *by the language's own `AND`/`OR`*. The agents did not hand-roll the
  three-valued table; they composed the native words, which handle it for free.
  So the intended discriminator never reached the agent. This crystallizes the
  cross-task pattern: **Ajisai's "value integrity first" design systematically
  removes the error classes that would discriminate tiers** — exact arithmetic
  (exact-rational-calculator), the JSON module (json-parser), and now
  native strong-Kleene logic. The runtime handles the hard part, so
  tier-discrimination is suppressed *by design*.

- **The solution space collapsed to a single point again — Plan-1's
  conservation question stays vacuous.** Despite a large *potential* solution
  space, all three tiers converged on the identical 4-word minimal composition.
  The complexity bundle (words = 4, ncLines = 1, energyProxyScore = 0) is
  trivially identical across tiers. The minimal Kleene composition is too
  strong an attractor: when the language's operators already express the logic,
  there is only one short way to write it. **A conserved-quantity / soliton
  test still has no medium with genuine dynamics here.**

- **The effort gradient did NOT replicate — it scattered.** In json-parser the
  effort ordered cleanly by tier (34/23/6 tool-uses). Here it does not:
  tool-uses 21 / 25 / 9 (Sonnet highest), tokens 91.9k / 78.9k / 103.4k (Opus
  highest), duration 155 / 285 / 200 (Sonnet highest) — **none monotonic in
  tier.** So the earlier "effort gradient" is task-specific, not a robust
  tier-invariant. For the soliton framing this is a further negative: the
  continuous "signal" is not conserved and not even monotonic; on a
  reasoning-heavy task it dissipates into task-specific noise.

- **The friction hit the MIDDLE tier, via a specific trap, not a capability
  cliff.** Sonnet was the *only* tier not to one-pass — but not because it
  failed the Kleene logic. It stumbled on the **`^` (VENT) operand-order sugar**
  and recovered. This is not a capability gradient; it is task-specific friction
  landing on one tier.

## Concrete tooling findings (the actionable deliverable of this probe)

Both were independently surfaced by the agents and reconfirmed by the recorder:

1. **`^` (VENT) operand-order misuse silently produces a wrong two-item stack
   with `status: ok` — not a diagnostic.** `FALSE ^ TRUE` → `[FALSE]` (correct,
   one item), but the reversed `TRUE FALSE ^` → `[TRUE, FALSE]` (two items),
   `status: ok`, no error. A malformed VENT use flows on as a plausible-but-wrong
   value instead of being caught. This is a genuine **value-integrity gap** — the
   one place in these tasks where the language did *not* absorb a mistake, it
   absorbed it silently and wrongly. Both Sonnet and Opus lost time to it
   (Sonnet's only non-onePass; Opus's careful pre-probing). Candidate fix: make
   a two-operand `^` with a non-NIL top either a structure error or at least an
   errorFlowTrace note.

2. **`VENT` (the canonical word named in SPEC §6.4) is not registered in the
   CLI build — only the `^` sugar works.** `NIL TRUE VENT` → `status: error`,
   `unknownWord`. This is a spec/impl drift: the reader/spec name a word the
   runtime does not expose. (The `unknownWord` diagnosis itself was rated clear
   and actionable — errorQuality high *for that* error.)

## Meta-conclusion (five sweeps)

The binary has floored on every task, including one built specifically to break
it, because Ajisai keeps removing the discriminating error classes at the
language level. Two honest consequences:

1. **For the transition/grokking hunt:** to see a capability cliff in the
   Haiku→Opus band you must *forbid the safety nets* — force agents to
   reimplement what the runtime provides (hand-rolled Kleene tables with the
   module and native `AND`/`OR` disallowed, exact arithmetic from scratch,
   etc.). Only then is the hard part in the agent's hands where a tier can fail.
2. **For Ajisai itself:** the flip side of "no tier discrimination" is the
   language's actual strength — the hard parts are correct by construction. The
   one leak found (silent-wrong `^` misuse, finding 1) is worth closing precisely
   because it violates that very promise.
