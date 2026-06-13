# Task: energy-refactor

Ajisai-specific task. Exercises the energyProxyScore (Phase 3): keep a
program's output identical while lowering its structural-cost proxy.

## Background (language-independent)

The CLI reports `runtimeMetrics.vtu.energyProxyScore` — a deterministic
integer aggregating observed structural work (tensor flatten/rebuild
round-trips, allocation, dispatch). It is a proxy for data movement, not a
joule measurement (see `docs/quality/energy-proxy-score.md`). Two programs
that compute the same result can have very different scores when one moves
data redundantly.

The starting point is this **naive** program, which scales a 2×3 matrix by
10 but inserts redundant identity broadcasts (`[ 1 ] *`) first:

```ajisai
[ [ 1 2 3 ] [ 4 5 6 ] ] [ 1 ] * [ 1 ] * [ 1 ] * [ 10 ] *
```

It produces `[ [ 10 20 30 ] [ 40 50 60 ] ]` with an energyProxyScore of
**304** (each redundant broadcast moves the whole matrix again).

## Solution contract

Write an Ajisai source file (a complete program, no word definitions
required) that:

1. produces the **same** final stack — `[ [ 10/1 20/1 30/1 ] [ 40/1 50/1 60/1 ] ]`, and
2. has `energyProxyScore` **≤ 76** (drop the redundant broadcasts; the direct
   `[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 ] *` scores 76).

The harness checks both the output and the score of your program directly
(its cases use an empty invocation, so the score measured is your program's).

## Acceptance cases

| id | check | expected |
|---|---|---|
| output-unchanged | final stack | `[ [ 10/1 20/1 30/1 ] [ 40/1 50/1 60/1 ] ]` |
| score-within-budget | energyProxyScore ≤ | `76` |

## Notes

- Scores are comparable only within one `proxyVersion` (currently 1). If the
  scoring formula changes, re-derive the naive/target numbers from the CLI.
- This task rewards the verification-then-low-movement quadrant Ajisai aims
  for: the refactor must not change a single output value.

## Run

```sh
./verify.sh energy-refactor your-solution.ajisai
```
