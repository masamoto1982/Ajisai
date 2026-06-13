# Task: three-valued-logic

Ajisai-favorable task. Exercises the logical UNKNOWN (U) of Kleene
three-valued logic (SPEC §7.5) and budgeted continued-fraction comparison
(SPEC §7.4.1).

## Background (language-independent)

Comparing two exact real numbers is not always decidable within a fixed
budget. Ajisai models this honestly: an equality/ordering comparison that
cannot be settled within its partial-quotient budget yields the third truth
value `UNKNOWN`, distinct from both `TRUE`/`FALSE` and from `NIL`. Logical
connectives follow Kleene semantics over `{TRUE, FALSE, UNKNOWN}`.

`EQ` compares two numbers and yields a truth value. For exact rationals it
always decides. For two exact irrationals that are mathematically equal but
*constructed differently* (e.g. `√2` versus `√8 / 2`), the default budget
cannot certify equality and `EQ` yields `UNKNOWN`. `MATH@SQRT` (after
`'MATH' IMPORT`) produces exact irrationals.

## Solution contract

Write an Ajisai source file that defines these user words with `DEF`:

- `EQ?` — given two numbers on the stack (`[ a ] [ b ]`), leave their
  three-valued equality (`TRUE` / `FALSE` / `UNKNOWN`).
- `BOTH-TRUE` — given two truth values, leave their Kleene conjunction.
- `EITHER-TRUE` — given two truth values, leave their Kleene disjunction.

## Acceptance cases

| id | invocation | expected |
|---|---|---|
| eq-rationals-equal | `[ 2 ] [ 2 ] EQ?` | `TRUE` |
| eq-rationals-unequal | `[ 1 ] [ 2 ] EQ?` | `FALSE` |
| eq-irrationals-unknown | `'MATH' IMPORT 2 SQRT 8 SQRT 2 DIV EQ?` | `UNKNOWN` |
| kleene-and-tf | `TRUE FALSE BOTH-TRUE` | `FALSE` |
| kleene-and-unknown | `'MATH' IMPORT 2 SQRT 8 SQRT 2 DIV EQ? TRUE BOTH-TRUE` | `UNKNOWN` |
| kleene-or-unknown | `'MATH' IMPORT 2 SQRT 8 SQRT 2 DIV EQ? TRUE EITHER-TRUE` | `TRUE` |

Kleene note: `UNKNOWN AND TRUE = UNKNOWN`, but `UNKNOWN OR TRUE = TRUE`
(a definite `TRUE` absorbs disjunction). The last two cases pin that
distinction — the heart of why UNKNOWN is not just "another error".

## Run

```sh
./verify.sh three-valued-logic your-solution.ajisai
```
