# Benchmark results — 2026-07-08 — Claude tier sweep (Haiku/Sonnet/Opus) — condition A

- Date (UTC): 2026-07-08
- Model / version: Claude Haiku 4.5 (T1), Claude Sonnet 4.5 (T2), Claude Opus 4.8 (T3)
- Model capability tier: T1 / T2 / T3 (protocol §1.1) — one row per tier below
- Condition: A = spec only (SPECIFICATION.html + task .md; no SKILL.md)
- ajisai commit: ef9edcb
- TRANSITION_METRICS_VERSION: 1 (design memo §6)
- Notes (system prompt, temperature, tool access, draft/repair cut-point rule used):
  - Each tier was an **independent, fresh-context agent session** (protocol §1),
    run as a context-isolated subagent whose only reference document was
    `SPECIFICATION.html` and whose only task text was the
    `exact-rational-calculator` spec. Tool access: read files + the `ajisai`
    CLI (`run`/`check --json`). Each was explicitly barred from reading the
    grading harness (`*.cases.tsv`, `verify.sh`, `verify-lib.sh`), `SKILL.md`
    (that is condition B), the `examples/` directory, and the `public/docs/`
    Reference — so condition A is preserved and the self-reported process
    metrics are honest.
  - The tiers are relative within the Claude family (memo §1.1's explicit
    Haiku-/Sonnet-/Opus-class example). **Caveat:** they span the 4.5/4.8
    generation boundary (Haiku 4.5, Sonnet 4.5, Opus 4.8), not a single
    same-generation family; read the tier axis as an ordinal capability
    proxy, not a controlled parameter count.
  - `passed` is the independent `verify.sh` verdict run by the recorder, not
    the subagent's self-report. `onePass` / `fixCount` are the subagent's
    honestly-reported process (each cross-checked for internal consistency:
    a onePass run has fixCount 0). `finalLines` was computed by the recorder.
  - **Draft/repair cut-point:** not reconstructable from the data available
    to the recorder. The harness reports only a *combined* per-session token
    total (in+out, not split) and forbids reading the full transcripts, so
    `tokensIn` / `tokensOut` / `draftTokensOut` / `repairTokensOut` are left
    blank (protocol §5: never invent). Combined session tokens / tool-uses,
    recorded for context only: T1 ≈ 53.9k / 12, T2 ≈ 54.6k / 14,
    T3 ≈ 37.0k / 8.

## Results

One row per (task, trial). `passed` is the `verify.sh` exit verdict (✅/❌).
Leave a cell blank if not measured — never invent a value (protocol §5).
`turbulence` = repairTokensOut / draftTokensOut; `contractCov` = contract
coverage ratio of the final solution, as reported by
`ajisai coverage <final-solution> --json` (definitions:
`docs/dev/capability-transition-measurement-design.md` §3–§4).

| tier | task | trial | passed | onePass | fixCount | finalLines | tokensIn | tokensOut | draftTokensOut | repairTokensOut | turbulence | contractCov | selfResolved | errorQuality(1-5) | energyProxyScore |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| T1 (Haiku) | exact-rational-calculator | 1 | ✅ | ✅ | 0 | 3 | | | | | 0 (onePass) | 9/9 | ✅ | n/a (no errors) | n/a |
| T2 (Sonnet) | exact-rational-calculator | 1 | ✅ | ✅ | 0 | 3 | | | | | 0 (onePass) | 9/9 | ✅ | n/a (no errors) | n/a |
| T3 (Opus) | exact-rational-calculator | 1 | ✅ | ✅ | 0 | 3 | | | | | 0 (onePass) | 9/9 | ✅ | n/a (no errors) | n/a |

Solutions are kept **off-tree** (in the recorder's session scratchpad), not
committed, per the no-solutions rule (protocol §5). All three tiers
independently converged on the same minimal shape: three `DEF`s that fold the
single-element rational vectors with elementwise `ADD` and divide by a pushed
`[ 2 ]` / `[ 3 ]` — differing only in the surface spelling `DIV` vs `/`.

## verify.sh transcripts (audit trail)

The recorder ran `verify.sh` once per tier's final solution; each returned
exit 0 with 7/7. Verbatim stdout (identical across the three tiers; the
transcript reveals case ids and pass/fail only, never the solution or the
expected values):

```
$ bench/agent-suite/tasks/verify.sh exact-rational-calculator <tier-solution.ajisai>
== verifying task 'exact-rational-calculator' with solution '<...>' ==
PASS  sum3-basic             stack
PASS  sum3-fractions         stack
PASS  avg3-exact             stack
PASS  avg3-whole             stack
PASS  half-int               stack
PASS  half-frac              stack
PASS  no-float-drift         stack
----
7/7 cases passed
```

## Observations

- **errorQuality rationale:** unratable this round. No tier hit a single CLI
  diagnosis — all three read the spec, inferred that `[ n ]` is a
  single-element exact-rational vector and that `ADD`/`DIV` apply elementwise,
  and wrote a correct solution with zero repair cycles. There were no
  diagnoses to rate (hence `n/a`, not a score).

- **Tier delta (the headline, and a NULL result for H1 on this task):** there
  is **no transition** across T1→T2→T3. Both the binary verdict (pass) *and*
  every continuous metric (fixCount 0, finalLines 3, turbulence 0,
  contractCov 9/9) are flat across all three tiers. Under the mirage guard
  (memo §5) a transition claim requires binary and continuous metrics to move
  *together*; here neither moves at all. The honest reading: this task sits
  **below the capability floor of even the smallest tier measured (Haiku)**,
  so it cannot reveal where reduced ambiguity would shift a transition — there
  is no struggling tier to shift. This is a real, publishable finding about
  the *benchmark design*, not about Ajisai: to observe a capability transition
  (and thus test whether Ajisai's low ambiguity moves it), the suite needs a
  task hard enough that a small tier actually fails or thrashes. `energy-refactor`
  or a deliberately under-specified task is a better transition probe than an
  Ajisai-favorable one that every tier one-shots.

- **Definitional gap surfaced by running the measurement:** T2 and T3 spent
  tokens *after* writing their first complete (correct) solution — T2 ran
  seven per-case self-tests, T3 ran one exploratory snippet then verified. The
  memo §3 cut-point defines `repairTokensOut` mechanically as "everything
  after the first submission," which would count that read-only *verification*
  as repair and make turbulence > 0, contradicting the same section's
  "onePass ⟹ turbulence 0" convention. These sessions are genuinely onePass
  (fixCount 0: no edit→re-verify cycle ever changed the solution), so the
  table records turbulence 0 per the convention. **Recommendation for
  TRANSITION_METRICS_VERSION 2:** clarify §3 so `repairTokensOut` counts
  tokens spent in edit→re-verify (fix) cycles only, excluding read-only
  post-submission self-verification — otherwise "diligent agent tests its
  own answer" is misread as turbulence.

- **A-vs-B delta:** not measured this round (condition A only). Because all
  three tiers already one-pass under condition A, this task cannot show a
  SKILL.md benefit either — a floor effect, not evidence that SKILL.md is
  useless. Any A/B or Ajisai-vs-other-language comparison needs a
  discriminating task first.

- **Contract coverage:** 9/9 (100%) for all three. The counted occurrences are
  the six arithmetic corewords (`ADD`/`ADD`/`ADD`/`ADD`/`DIV`/`DIV` or the
  `/` spellings) plus three `DEF`s; the defined names `SUM3`/`AVG3`/`HALF`
  appear only as `DEF` string arguments, so they are not word occurrences and
  do not enter the denominator (memo §4). A solution that leaned on
  user-word *calls* would score below 100%; this one does not call its own
  words.
