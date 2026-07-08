# Benchmark results — <date> — <model> — condition <A|B>

- Date (UTC):
- Model / version:
- Model capability tier: <T1 small | T2 mid | T3 frontier> (protocol §1.1)
- Condition: <A = spec only | B = with SKILL.md>
- ajisai commit:
- TRANSITION_METRICS_VERSION: 1 (design memo §6)
- Notes (system prompt, temperature, tool access, draft/repair cut-point rule used):

## Results

One row per (task, trial). `passed` is the `verify.sh` exit verdict (✅/❌).
Leave a cell blank if not measured — never invent a value (protocol §5).
`turbulence` = repairTokensOut / draftTokensOut; `contractCov` = contract
coverage ratio of the final solution (definitions:
`docs/dev/capability-transition-measurement-design.md` §3–§4).

| task | trial | passed | onePass | fixCount | finalLines | tokensIn | tokensOut | draftTokensOut | repairTokensOut | turbulence | contractCov | selfResolved | errorQuality(1-5) | energyProxyScore |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| json-parser | 1 | | | | | | | | | | | | | n/a |
| bank-account | 1 | | | | | | | | | | | | | n/a |
| exact-rational-calculator | 1 | | | | | | | | | | | | | n/a |
| three-valued-logic | 1 | | | | | | | | | | | | | n/a |
| nil-fallback-pipeline | 1 | | | | | | | | | | | | | n/a |
| energy-refactor | 1 | | | | | | | | | | | | | |

## verify.sh transcripts (audit trail)

Paste the verbatim stdout of the official `verify.sh` run for each task's
final solution.

```
$ bench/agent-suite/tasks/verify.sh <task> <final-solution.ajisai>
...
```

## Observations

- Where did the CLI diagnoses help or fall short (errorQuality rationale)?
- Losses: which tasks Ajisai handled poorly, and why (kept on purpose).
- A-vs-B delta: did SKILL.md reduce fixCount / tokens / first-try failures?
- Tier delta (if other tiers were measured): did pass rate AND the continuous
  metrics (fixCount, turbulence, tokensOut) move together across tiers, or
  only the binary pass rate (mirage guard, design memo §5)?
