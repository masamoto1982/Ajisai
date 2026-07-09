# Analysis (not a trial) — conserved-quantity hunt over the json-parser A/B solutions

- Date (UTC): 2026-07-08
- Type: **re-analysis of existing data** — no new agent sessions. The six
  final solutions from the two json-parser sweeps
  (`2026-07-08-claude-tier-sweep-json-parser-A.md` and `-B.md`) were
  reconstructed verbatim from the recorded session transcripts and measured
  with `ajisai coverage --json` and `ajisai run --json`.
- Motivation: the discussion framed the recurring pattern (binary saturates
  while signal lives in the continuous metrics) as **soliton-like**, with the
  conjecture that a soliton — a *conserved, non-dispersing* excitation — is the
  key to grokking. A soliton test needs a **conserved quantity**. This note
  asks: across the SKILL.md (A→B) intervention, is any non-trivial continuous
  functional of the *solution* invariant?
- ajisai commit: 42dbe98

## What was measured (per tier × condition)

`words` = counted word occurrences (`coverage.total`); `bytes` = source
length; `ncLines` = non-comment/non-blank lines; `contractCov` from
`ajisai coverage`; `score` = `energyProxyScore` of the solution run with a
fixed representative invocation (`'{"x":{"y":5}}' JSON@PARSE 'x' FIELD-JSON`).

| tier-cond | words | bytes | ncLines | contractCov | score |
|---|---|---|---|---|---|
| A-haiku | 12 | 178 | 5 | 12/12 | 0 |
| A-sonnet | 12 | 178 | 5 | 12/12 | 0 |
| A-opus | 12 | 178 | 5 | 12/12 | 0 |
| B-haiku | 13 | 441 | 5 | 13/13 | 0 |
| B-sonnet | 13 | 185 | 5 | 13/13 | 0 |
| B-opus | 12 | 178 | 5 | 12/12 | 0 |

## Conservation verdict across the A→B intervention

Splitting the quantities by whether they stayed invariant:

**Invariant — but only trivially (each forced by a task constraint, not an
emergent law):**
- `energyProxyScore` = 0 for all six. Conserved because JSON string ops move
  **no tensor data** — the structural-cost proxy is at its floor and cannot
  distinguish anything here. Uninformative.
- `ncLines` = 5 for all six. Conserved because the solution contract
  **dictates** exactly one `IMPORT` + four `DEF`s. The skeleton is fixed by the
  task, not chosen.
- `contractCov` **ratio** = 1.0 for all six (12/12 or 13/13). Conserved because
  every solution uses only registered words. Forced, not emergent.

**Not conserved (every quantity with genuine freedom to vary moved with the
intervention):**
- `words` (occurrence count): 12 → 13 for Haiku and Sonnet (they added a
  redundant `STR` after `JSON@HAS` in condition B), 12 → 12 for Opus.
- `bytes`: 178 → 441 for Haiku (verbose comments in B), 178 → 185 Sonnet,
  178 → 178 Opus.
- `contractCov` **count** (numerator/denominator absolute): 12 → 13 for two
  tiers.

## Conclusion — a null for the conservation hypothesis at this scale, and the
## requirement it imposes on the next probe

**There is no non-trivial conserved continuous quantity in this data.** Every
invariant we found is forced by the task's constraints (four DEFs, all-covered
words, zero tensor movement); every quantity with real freedom (word count,
bytes) was *not* conserved — it varied with the SKILL.md intervention. The one
genuinely conserved-and-meaningful thing is the **graded I/O function itself**
(all six solutions compute the identical map, all pass 13/13) — but that is
conserved *by construction*, because it is the pass criterion, so it is not
evidence of a conservation law either.

The deeper reason is structural: **`json-parser`'s solution space is
essentially a single point** — the canonical four-word composition — with one
trivial degree of freedom (add the redundant `STR` or not). A conservation law
is only a non-trivial claim over a space with real dynamics, where correct
solutions can genuinely differ and one asks what stays invariant as they vary.
Our favorable/ported tasks are too constrained to *have* such a space; we have
been looking for a wave in a medium with no room to propagate.

**Requirement this places on Plan 2 (the transition-task design):** to make the
soliton/grokking conservation question testable, the next task must have a
**large, genuinely multi-valued solution space** — different tiers should
produce *structurally different* correct solutions — so that a candidate
conserved functional (e.g. a semantic invariant of the solution that is *not*
the graded output, yet stays fixed while surface form and effort vary across
tiers) becomes a non-trivial, falsifiable claim. Concretely, the proposed
multi-step NIL/`^`/three-valued-logic pipeline should be designed with several
inequivalent correct strategies, and the measurement should record a
solution-level invariant candidate (e.g. the count of decision points, or the
truth-table the pipeline realizes) alongside the binary verdict and the
continuous effort metrics.

This keeps the soliton framing where memo §5 requires — a metaphor guiding
metric design, not a claimed mechanism — while turning it into a concrete,
testable requirement for the next probe.
