# Task: exact-rational-calculator

Ajisai-favorable task. Exercises exact rational arithmetic with no
floating-point rounding.

## Background (language-independent)

Implement a small calculator over exact rational numbers. The required
behaviors are defined purely by input/output; how you express them is up to
you. In Ajisai, numbers are exact rationals by construction, so the
intended difficulty is *expressing the operations*, not avoiding float
drift.

## Solution contract

Write a single Ajisai source file that defines these user words with `DEF`:

- `SUM3` — given three numbers on the stack (pushed as `[ a ] [ b ] [ c ]`),
  leave their exact sum `[ a+b+c ]`.
- `AVG3` — given three numbers, leave their exact arithmetic mean
  `[ (a+b+c)/3 ]` as an exact rational (never a rounded decimal).
- `HALF` — given one number `[ x ]`, leave its exact half `[ x/2 ]`.

The harness appends an invocation after your definitions and inspects the
final stack (`stackDisplay`).

## Acceptance cases

| id | invocation | expected final stack |
|---|---|---|
| sum3-basic | `[ 1 ] [ 2 ] [ 3 ] SUM3` | `[ 6 ]` |
| sum3-fractions | `[ 1 ] [ 2 ] / [ 1 ] [ 3 ] / [ 1 ] [ 6 ] / SUM3` | `[ 1 ]` |
| avg3-exact | `[ 1 ] [ 2 ] [ 4 ] AVG3` | `[ 7/3 ]` (exact, not 2.33…) |
| avg3-whole | `[ 2 ] [ 4 ] [ 6 ] AVG3` | `[ 4 ]` |
| half-int | `[ 1 ] HALF` | `[ 1/2 ]` |
| half-frac | `[ 1 ] [ 3 ] / HALF` | `[ 1/6 ]` |
| no-float-drift | `[ 1 ] [ 3 ] / [ 3 ] *` | `[ 1 ]` (exactly 1, never 0.999…) |

Display note: integers render as `[ n/1 ]` on the stack. The harness compares
against the literal display strings in `cases.tsv` (e.g. `[ 6/1 ]`).

## Run

```sh
bench/agent-suite/tasks/verify.sh exact-rational-calculator path/to/solution.ajisai
# or, from this directory:
./verify.sh your-solution.ajisai
```
