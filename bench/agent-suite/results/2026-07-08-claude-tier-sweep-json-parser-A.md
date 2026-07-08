# Benchmark results — 2026-07-08 — Claude tier sweep (Haiku/Sonnet/Opus) — json-parser, condition A

- Date (UTC): 2026-07-08
- Model / version: Claude Haiku 4.5 (T1), Claude Sonnet 4.5 (T2), Claude Opus 4.8 (T3)
- Model capability tier: T1 / T2 / T3 (protocol §1.1) — one row per tier below
- Condition: A = spec only (SPECIFICATION.html + task .md; no SKILL.md)
- ajisai commit: cc852cc
- TRANSITION_METRICS_VERSION: 1 (design memo §6)
- Notes (system prompt, temperature, tool access, draft/repair cut-point rule used):
  - Each tier was an **independent, fresh-context agent session** (protocol §1),
    run as a context-isolated subagent whose only reference was
    `SPECIFICATION.html` + the `json-parser` spec, barred from the grading
    harness (`*.cases.tsv`, `verify.sh`), `SKILL.md`, `examples/`, the
    Reference, and everything under `bench/`. Tool access: read files + the
    `ajisai` CLI.
  - `passed` and `CASES_PASSING` were re-measured by the recorder via
    `verify.sh` (the official verdict), not taken from self-report.
  - **Objective effort metrics** (`toolUses`, combined session tokens,
    duration) are **harness-measured**, not self-reported, and are the
    reliable effort signal this round. `tokensIn`/`tokensOut` are left blank
    (only a combined figure is exposed); the combined figure is recorded in
    the effort column for context.
  - Tiers are relative within the Claude family (memo §1.1 example), spanning
    the 4.5/4.8 generation boundary (same caveat as prior sweeps).

## Results

`passed` is the `verify.sh` verdict (✅/❌). `casesPassing` is out of 13,
re-measured by the recorder. `effort` = harness-measured (tool-uses / combined
session tokens / wall-clock) — the objective work to reach the solution.

| tier | task | passed | casesPassing | onePass | finalLines | effort (toolUses / tokens / sec) | turbulence | contractCov | selfResolved | errorQuality(1-5) | energyProxyScore |
|---|---|---|---|---|---|---|---|---|---|---|---|
| T1 (Haiku) | json-parser | ✅ | 13/13 | ⚠ see note | 5 | **34 / 75.3k / 198** | | 12/12 | ✅ | n/a (no errors) | n/a |
| T2 (Sonnet) | json-parser | ✅ | 13/13 | ✅ | 5 | **23 / 47.9k / 99** | ~0 (onePass) | 12/12 | ✅ | n/a (no errors) | n/a |
| T3 (Opus) | json-parser | ✅ | 13/13 | ✅ | 5 | **6 / 35.7k / 71** | ~0 (onePass) | 12/12 | ✅ | n/a (no errors) | n/a |

All three converged on the identical canonical solution — `'JSON' IMPORT`
plus four `DEF`s composing `JSON@PARSE`/`STRINGIFY`/`GET`/`DELETE`/`HAS` in
`value key OP` stack order. Solutions kept off-tree per §5.

## verify.sh transcripts (audit trail)

All three returned exit 0 (13/13). Verbatim tail (identical across tiers;
reveals case ids and pass/fail only):

```
$ bench/agent-suite/tasks/verify.sh json-parser <tier-solution.ajisai>
== verifying task 'json-parser' with solution '<...>' ==
PASS  rt-object              stack
... (11 more) ...
PASS  has-absent             stack
----
13/13 cases passed
```

## Observations

- **The binary metric floors a third time — but this is the first sweep with a
  clean continuous tier gradient.** All three tiers reached 13/13, so pass rate
  and `casesPassing` are flat (no capability transition). What is *not* flat is
  the **objective, harness-measured effort**: tool-uses 34 → 23 → 6 and
  duration 198s → 99s → 71s fall **monotonically** from Haiku to Opus, and
  combined tokens 75.3k → 47.9k → 35.7k likewise. Haiku spent ~5.7× the
  tool-uses and ~2.1× the tokens of Opus to arrive at the *same five-line
  program*. Unlike the `energy-refactor` score split (a judgment artifact),
  this gradient is a genuine effort-scaling signal in the expected direction:
  the smaller the tier, the more thrashing to reach an identical result.

- **Mirage guard (memo §5) holds — this is a cost gradient, not a transition.**
  A transition claim requires the binary *and* continuous metrics to move
  together. Here the binary is flat and only the effort moves, so this is **not**
  reported as emergence. The honest framing: `json-parser` sits just above the
  point where the task starts to *cost* the smallest tier materially more,
  while still being within its capability floor. It is the closest of the three
  tasks measured to a transition — which suggests the actual pass/fail cliff
  (where Haiku fails outright) lies on a task only modestly harder than this
  one.

- **Weaker-tier self-report was unreliable — itself a tier signal, recorded
  with care.** Haiku reported `FIRST_RUN_ALL_CORRECT: yes` *and* `ITERATIONS: 15`
  in the same block — internally contradictory (a genuine one-pass cannot take
  15 iterations). Its final solution does pass 13/13 (recorder-verified), but
  the onePass claim is **not credited**: the 34 harness-measured tool-uses and
  198s corroborate substantial thrashing, not a clean one-shot. Sonnet's 6 and
  Opus's 1 self-reported iterations are consistent with their low tool-use
  counts. Lesson for the protocol: prefer harness-measured effort
  (tool-uses, tokens, duration) over self-reported `onePass`/`ITERATIONS` for
  the smaller tiers, whose process introspection is less reliable. `turbulence`
  is left blank for Haiku (its draft/repair split is unknowable and its onePass
  status is disputed) and marked ~0 for the two clean onePass tiers.

- **errorQuality unratable again:** no tier hit an Ajisai CLI error — the
  friction was in *discovering* `JSON@` operand order empirically (Sonnet and
  Haiku both probed with throwaway scripts), not in decoding diagnoses. The
  spec not documenting per-word JSON stack order is the real cost driver here;
  a documented stack effect would likely compress the Haiku effort gradient
  most. (Candidate concrete win, cf. the SKILL.md / condition-B lever.)

- **Cross-task arc (three sweeps).** exact-rational-calculator: both metrics
  floor (trivial). energy-refactor (A−): binary floors, continuous diverges as
  a *judgment* artifact. json-parser: binary floors, continuous shows a clean
  *effort* gradient. Progression is real — each harder task pushes more signal
  into the continuous metrics while the binary stays saturated. To finally move
  the binary (a real transition), the next probe needs a task whose correct
  solution requires multi-step reasoning a small tier cannot complete at all —
  e.g. a task combining NIL bubbling, `^` fallback, and three-valued logic in
  one pipeline with an easy-to-get-wrong middle step.
