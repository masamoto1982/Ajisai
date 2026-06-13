# Benchmark results — <date> — <model> — condition <A|B>

- Date (UTC):
- Model / version:
- Condition: <A = spec only | B = with SKILL.md>
- ajisai commit:
- Notes (system prompt, temperature, tool access):

## Results

One row per (task, trial). `passed` is the `verify.sh` exit verdict (✅/❌).
Leave a cell blank if not measured — never invent a value (protocol §5).

| task | trial | passed | onePass | fixCount | finalLines | tokensIn | tokensOut | selfResolved | errorQuality(1-5) | energyProxyScore |
|---|---|---|---|---|---|---|---|---|---|---|
| json-parser | 1 | | | | | | | | | n/a |
| bank-account | 1 | | | | | | | | | n/a |
| exact-rational-calculator | 1 | | | | | | | | | n/a |
| three-valued-logic | 1 | | | | | | | | | n/a |
| nil-fallback-pipeline | 1 | | | | | | | | | n/a |
| energy-refactor | 1 | | | | | | | | | |

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
