# Task: nil-fallback-pipeline

Ajisai-favorable task. Exercises NIL-as-a-value: partial operations project
to NIL (the "Bubble Rule"), `^` supplies a fallback, and an uncaught NIL
propagates through the rest of a pipeline.

## Background (language-independent)

In Ajisai a failed partial operation does not throw — it produces `NIL`, an
ordinary value carrying a machine-readable reason. Division by zero is the
canonical case: `[ 10 ] [ 0 ] DIV` succeeds and leaves `NIL`. The
nil-coalescing operator `^` replaces a `NIL` on top of the stack with a
fallback value; a non-`NIL` value passes through unchanged. If a `NIL` is
never coalesced, it bubbles through subsequent operations to the end.

## Solution contract

Write an Ajisai source file that defines:

- `SAFE-DIV` — given `[ a ] [ b ]`, leave `[ a/b ]`, or `[ 0 ]` when `b` is
  zero (catch the projected `NIL` with a fallback). A reciprocal is then
  just `[ 1 ] [ x ] SAFE-DIV`.

The harness appends an invocation that may chain further arithmetic to show
that a caught NIL keeps the pipeline alive while an uncaught one bubbles.

## Acceptance cases

| id | invocation | expected final stack |
|---|---|---|
| div-ok | `[ 20 ] [ 4 ] SAFE-DIV` | `[ 5 ]` |
| div-zero-fallback | `[ 20 ] [ 0 ] SAFE-DIV` | `[ 0 ]` |
| reciprocal-ok | `[ 1 ] [ 4 ] SAFE-DIV` | `[ 1/4 ]` |
| reciprocal-zero | `[ 1 ] [ 0 ] SAFE-DIV` | `[ 0 ]` |
| pipeline-caught | `[ 20 ] [ 0 ] SAFE-DIV [ 100 ] +` | `[ 100 ]` (fallback 0, then +100) |
| pipeline-raw-bubble | `[ 20 ] [ 0 ] DIV [ 100 ] +` | `NIL` (uncaught NIL bubbles through `+`) |

The last case uses raw `DIV` (not `SAFE-DIV`) on purpose: it pins that an
uncaught NIL survives later operations and reaches the final stack as `NIL`.

## Run

```sh
./verify.sh nil-fallback-pipeline your-solution.ajisai
```
