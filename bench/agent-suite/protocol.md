# Agent Benchmark Protocol

This protocol fixes *how* the agent benchmark is run so a third party can
reproduce it under the same conditions. It mirrors the method used by
external AI-language comparisons: hand an agent a specification, let it work
the write → run → read-error loop on the CLI, and measure how cheaply it
reaches a correct solution.

Authority for Ajisai semantics is `SPECIFICATION.html`. This document defines
procedure only.

## 0. What is measured

For each (task, condition, trial) the recorder logs, with no human judgement
of correctness (the `verify.sh` exit code is the verdict):

| metric | how obtained |
|---|---|
| passed | `verify.sh <task> <solution>` exits 0 |
| onePass | passed on the **first** submitted solution, no fixes |
| fixCount | number of edit→re-verify cycles before passing (0 if onePass) |
| finalLines | non-blank, non-comment lines in the final solution |
| tokensIn / tokensOut | agent prompt / completion tokens for the session |
| selfResolved | whether the agent recovered from its own errors without human hints |
| errorQuality | 1–5 rating of how actionable the CLI diagnoses were *during* the session (the one subjective field; rate the tool, not the solution) |
| energyProxyScore | for tensor tasks / energy-refactor, the score of the final solution |
| draftTokensOut | completion tokens up to and including the **first** submitted solution (cut-point rule: design memo §3) |
| repairTokensOut | `tokensOut − draftTokensOut` (everything after the first submission) |

Two derived values are computed from the recorded ones (definitions, exclusion
rules, and `TRANSITION_METRICS_VERSION` live in
`docs/dev/capability-transition-measurement-design.md`; records made under
different versions of those definitions are not comparable):

| derived metric | definition |
|---|---|
| sessionTurbulenceRatio | `repairTokensOut / draftTokensOut`; 0 for a onePass session |
| contractCoverageRatio | fraction of word occurrences in the final solution that resolve to a definition with complete §7.14 contract metadata |

`onePass`, `fixCount`, `finalLines`, and `energyProxyScore` are objective.
`errorQuality` is a tooling observation, not a pass/fail input. If the
draft/repair cut-point cannot be reconstructed from the session log, leave
`draftTokensOut` / `repairTokensOut` blank rather than estimating (§5).

## 1. Conditions

Two conditions per task, each run as an **independent agent session** (fresh
context, no memory of other trials):

- **Condition A — spec only.** The agent is given `SPECIFICATION.html` and
  the task's `.md` spec. No `SKILL.md`.
- **Condition B — with SKILL.md.** The agent is additionally given the
  generated `SKILL.md` (repo root).

The only difference between A and B is the presence of `SKILL.md`. Keep the
model, temperature, system prompt, tool access, and task text identical
across the two.

### 1.1 Model capability tiers (transition axis)

Orthogonal to A/B, each condition may be run at up to three **model
capability tiers**:

- **T1 — small**, **T2 — mid**, **T3 — frontier**, defined as relative
  tiers **within a single model family** (e.g. Haiku-class / Sonnet-class /
  Opus-class). Never compare tiers across families.
- Everything except the model stays identical across tiers: condition
  materials, temperature, system prompt, tool access, task text.
- Purpose: plot the **transition curve** — pass rate versus tier, per
  condition — and observe whether reduced-ambiguity conditions shift the
  curve toward smaller tiers. Rationale, the hypothesis under test, and the
  mirage guard (never claim a transition from the binary pass-rate alone;
  always co-report `fixCount` / turbulence / tokens across tiers) are in
  `docs/dev/capability-transition-measurement-design.md`.
- Single-tier measurements remain valid protocol runs; the tier is simply
  recorded. Tier-comparison claims require ≥ 3 trials per
  (task, condition, tier), all at the same `TRANSITION_METRICS_VERSION`.

## 2. Per-trial procedure

1. Start a fresh agent session. Provide the materials for the condition plus
   the task spec. Provide the `ajisai` CLI (built: `cargo build --bin ajisai`).
2. The agent writes a solution file and may run it with
   `ajisai run <file> --json` (and `ajisai check`) as often as it wants.
   Every run is part of the loop; count each edit→re-verify as one fix cycle.
3. The agent decides when it is done. Record the final solution file.
4. The recorder runs `bench/agent-suite/tasks/verify.sh <task> <final>` once
   for the official verdict. Do not let the agent see `verify.sh` internals
   or the `cases.tsv` expected values during the session — it gets the task
   `.md` (which lists the acceptance cases) only.
5. Log all metrics for the trial to a results file (§4).

Recommended: ≥ 3 trials per (task, condition) to average out variance.

## 3. Tasks

| task | origin | tests |
|---|---|---|
| `json-parser` | ported | composing JSON parse/query/transform/serialize |
| `bank-account` | ported | functional state + overdraft rejection via NIL |
| `exact-rational-calculator` | Ajisai-favorable | exact rational arithmetic |
| `three-valued-logic` | Ajisai-favorable | UNKNOWN / Kleene logic |
| `nil-fallback-pipeline` | Ajisai-favorable | NIL bubble + `^` fallback |
| `energy-refactor` | Ajisai-favorable | same output, lower energyProxyScore |

The ported tasks exist for head-to-head comparability with external
write-ups; expect Ajisai to do *worse* on some of them, and record that
honestly (§5).

## 4. Recording

Copy `results/TEMPLATE.md` to `results/<date>-<model>-<condition>.md` and fill
one row per trial (the model name in the filename identifies the tier; the
tier field in the header makes it explicit). Commit the filled results. The
`verify.sh` transcript (stdout) for the final solution should be pasted
verbatim — it is the audit trail for the `passed` column.

## 5. Honesty rules (binding)

- **Do not fabricate.** Only real agent sessions produce result rows. If a
  trial was not run, leave it blank — never invent numbers.
- **Publish losses.** A task where Ajisai scores poorly stays in the suite
  and its results stay in the file. Do not delete unfavorable rows or tasks.
- **No solutions in the repo.** Reference/answer solutions are never
  committed under `tasks/` — that would void the benchmark. The harness is
  validated separately (the maintainer confirms each `verify.sh` returns 0 on
  a known-correct solution and non-0 on a wrong one, without committing it).
- **Score honesty.** `energyProxyScore` is a structural proxy, not joules
  (see `docs/quality/energy-proxy-score.md`). Report it as such.
- **Transition honesty.** `sessionTurbulenceRatio` and
  `contractCoverageRatio` are observational proxies, not measures of model
  intelligence or "emergence"
  (see `docs/dev/capability-transition-measurement-design.md`). Do not use
  the word "emergence" in result files; a transition claim requires both the
  binary and the continuous metrics to move together (mirage guard, memo §5).
  Unfavorable transition results stay published like any other loss.

## 6. Reproducing from scratch

```sh
cargo build --bin ajisai --manifest-path rust/Cargo.toml
# (optional) regenerate the guide used in condition B:
npm run generate:skill
# run an agent session per §2, then:
bench/agent-suite/tasks/verify.sh <task> <final-solution.ajisai>
```

`verify.sh` builds the CLI itself if it is missing, or honours `AJISAI_BIN`.
