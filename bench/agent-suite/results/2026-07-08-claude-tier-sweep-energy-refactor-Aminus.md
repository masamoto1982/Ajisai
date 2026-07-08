# Benchmark results — 2026-07-08 — Claude tier sweep (Haiku/Sonnet/Opus) — energy-refactor, condition A− (under-specified)

- Date (UTC): 2026-07-08
- Model / version: Claude Haiku 4.5 (T1), Claude Sonnet 4.5 (T2), Claude Opus 4.8 (T3)
- Model capability tier: T1 / T2 / T3 (protocol §1.1) — one row per tier below
- Condition: **A− (under-specified)** — a deliberate variant of condition A, see below
- ajisai commit: c84a95b
- TRANSITION_METRICS_VERSION: 1 (design memo §6)
- Notes (system prompt, temperature, tool access, draft/repair cut-point rule used):
  - Each tier was an **independent, fresh-context agent session** (protocol §1),
    run as a context-isolated subagent. Tool access: read files + the `ajisai`
    CLI (`run`/`check --json`). Each was barred from the grading harness
    (`*.cases.tsv`, `verify.sh`, `verify-lib.sh`), `SKILL.md`, `examples/`, the
    Reference, everything under `bench/`, **and** `docs/quality/energy-proxy-score.md`.
  - **Condition A− (what makes this "under-specified"):** the *standard*
    `energy-refactor.md` hands the answer away — its Background annotates the
    `[ 1 ] *` operations as "redundant identity broadcasts" and its Solution
    contract states the worked direct form `[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 ] *`
    "scores 76." Condition A− withholds **both**: agents received the naive
    304-scoring program, the target output, the ≤76 budget, and a general
    description of what `energyProxyScore` measures — but *not* the annotation
    that the broadcasts are redundant, and *not* the worked solution. They had
    to recognize the redundancy and construct the fix themselves. This is the
    H1 lever (memo §1): vary the information given, hold the task fixed.
  - Tiers are relative within the Claude family (memo §1.1's Haiku-/Sonnet-/
    Opus-class example); they span the 4.5/4.8 generation boundary (same
    caveat as the prior sweep) — read the axis as an ordinal capability proxy.
  - `passed` and the final `energyProxyScore` were re-measured by the recorder
    (`verify.sh` + `ajisai run --json`); `onePass` is the subagent's
    honestly-reported process. Token in/out and draft/repair splits are blank
    (only a combined session figure is available; §5 forbids inventing).
    Combined session tokens / tool-uses, for context: T1 ≈ 55.4k / 8,
    T2 ≈ 38.0k / 10, T3 ≈ 24.3k / 3.

## Results

One row per (task, trial). `passed` is the `verify.sh` exit verdict (✅/❌).
`turbulence` = repairTokensOut / draftTokensOut; `contractCov` from
`ajisai coverage --json`. The **energyProxyScore column is the point of this
task** — lower is a structurally cheaper program.

| tier | task | trial | passed | onePass | fixCount | finalLines | tokensIn | tokensOut | draftTokensOut | repairTokensOut | turbulence | contractCov | selfResolved | errorQuality(1-5) | energyProxyScore |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| T1 (Haiku) | energy-refactor (A−) | 1 | ✅ | ✅ | 0 | 1 | | | | | 0 (onePass) | 1/1 | ✅ | n/a (no errors) | **76** |
| T2 (Sonnet) | energy-refactor (A−) | 1 | ✅ | ✅ | 0 | 1 | | | | | 0 (onePass) | 1/1 | ✅ | n/a (no errors) | **76** |
| T3 (Opus) | energy-refactor (A−) | 1 | ✅ | ✅ | 0 | 1 | | | | | 0 (onePass) | 0/0 | ✅ | n/a (no errors) | **0** |

Solutions are already effectively public (both forms appear in the standard
`energy-refactor.md`), but the actual solution files are kept off-tree per §5:
- T1 Haiku & T2 Sonnet: `[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 ] *` — the intended
  refactor (strip the redundant broadcasts), landing exactly on the 76 budget.
- T3 Opus: `[ [ 10 20 30 ] [ 40 50 60 ] ]` — a **constant-folded literal**: the
  outputs are constants, so it emits them directly and moves no data at all
  (score 0). Note its `contractCov` is 0/0 — a pure literal, zero word
  occurrences.

## verify.sh transcripts (audit trail)

All three returned exit 0 (2/2). Verbatim stdout (identical shape across tiers;
reveals case ids and pass/fail only):

```
$ bench/agent-suite/tasks/verify.sh energy-refactor <tier-solution.ajisai>
== verifying task 'energy-refactor' with solution '<...>' ==
PASS  output-unchanged       stack
PASS  score-within-budget    scoreLE
----
2/2 cases passed
```

## Observations

- **Binary metric floors again — no pass/fail transition.** As with the prior
  `exact-rational-calculator` sweep, all three tiers produced a passing
  solution on the first complete run (onePass, fixCount 0). Even
  *under-specified*, this task is within every measured tier's capability
  floor for *finding a passing program*.

- **The continuous metric diverged (76 / 76 / 0) — but it is NOT a capability
  gradient, and the mirage guard (memo §5) forbids calling it a transition.**
  The final `energyProxyScore` split by tier, which looks like a signal. It is
  not a clean one: the divergence reflects a **judgment call about an
  under-specified task, not a capability threshold.** Decisive evidence is in
  the Sonnet (T2) session, which **explicitly found the score-0 literal
  loophole and deliberately rejected it** — "it bypasses the actual
  multiplication the task is about rather than optimizing it" — choosing the
  faithful 76 refactor instead. Opus (T3) took the literal (0). So the lowest
  score was produced by the *loophole-taker*, while the mid tier demonstrated
  arguably *deeper* task-intent reasoning by declining it. "Lower score = more
  capable" does not hold; the score column here measures interpretation of an
  ambiguous contract, not ability. Reporting this divergence as an emergence
  signal would be exactly the mirage the guard exists to prevent.

- **Task-design finding (the actionable output of this run):**
  `energy-refactor` is **under-constrained**. Its contract asks only for the
  same output stack and a score ≤ budget; nothing requires the *computation*
  to be preserved. So constant-folding the answer to a literal
  (`[ [ 10 20 30 ] [ 40 50 60 ] ]`, score 0) is a fully valid passing
  "solution" that bypasses the intended refactor — and `verify.sh` accepts it.
  The standard task's worked hint accidentally masks this by anchoring every
  solver to the 76 form. **Recommendation:** add a constraint to the task
  (e.g. "your program must contain the `*` multiply and be a transformation of
  the given program", or a lower-bound score floor so a bare literal is
  rejected) so it is a genuine refactor probe rather than a constant-folding
  exercise. Filed here rather than editing `tasks/energy-refactor.md` directly,
  since changing a suite task is a separate decision.

- **A-vs-A− delta (why the standard-A control was not run):** the plan was to
  run the hinted condition-A control only if A− produced a real pass/fail
  threshold movement to attribute to the withheld information. It did not
  (binary floors; the continuous divergence is a judgment artifact), so the
  control would only re-confirm the floor and was skipped (protocol §5 favors
  not manufacturing rows). One genuine open question it *could* answer: does
  the standard hint *suppress* the frontier tier's score-0 literal by anchoring
  it to 76? Left as a hypothesis, not run.

- **Cross-task meta-conclusion (two sweeps in).** Across
  `exact-rational-calculator` and under-specified `energy-refactor`, the
  binary transition metric has floored at every tier. The honest reading: the
  suite's Ajisai-favorable tasks are structurally easy enough — partly
  *because* Ajisai's exactness removes whole error classes — that they do not
  discriminate across the Haiku→Opus band. Testing H1 (does lower ambiguity
  move the tier threshold?) needs a task genuinely hard enough that a small
  tier **fails**; none of the current favorable tasks are. A ported/adversarial
  task, or a multi-step task with a real failure mode, is the next probe.
