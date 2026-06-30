# Specification gaps found while porting Ajisai to Python from the spec alone

This Python port (`python/ajisai/`) was written **only** from `SPECIFICATION.html`,
without reading the Rust/WASM/TypeScript implementation or any prior port. Per
the exercise's premise, every point at which the port's behaviour could diverge
from the original Ajisai is evidence that the specification is under-determined.

Each finding cites the section(s) it concerns, states what a from-spec-only
implementer is forced to guess, and records the choice this port made (search the
code for `SPEC-GAP`). Findings are ordered by how likely they are to cause an
observable divergence.

---

## 1. COND clause collection and stack discipline — *high impact*
**Sections 3.6, 7.7, 8.1.**

The spec says COND clauses are "separated by `|`", yet the only worked example
(Section 8.1) writes each clause as its own `{ guard | body }` block followed by
`COND`:

```
{ [ 1 ] EQ | [ 100 ] }
{ [ 2 ] EQ | [ 200 ] }
{ IDLE     | [ 0 ]   } COND
```

None of the following is defined normatively:

- **How many clauses COND consumes.** The clauses arrive as separate `CodeBlock`
  values on the stack; the spec never says COND consumes "all consecutive code
  blocks", nor gives a count word.
- **What stack each guard sees**, and whether a guard that consumes the subject
  (e.g. `[ 1 ] EQ` consumes the value being classified) must have it restored
  before the next guard runs.
- **Whether the matched body consumes the subject.** This port runs the body
  against the base stack with the subject still present, so
  `[ 2 ] CLASSIFY → [ 2/1 ] [ 200/1 ]` (subject retained beneath the result). An
  implementation that consumes the subject would yield `[ 200/1 ]` — a direct
  observable divergence.
- **How `IDLE` is recognised as the else clause.** `IDLE` pushes no truth value,
  so "execute the first whose guard is definitely true" cannot apply to it; the
  port special-cases a guard of exactly `IDLE` (or empty) as the else.

A normative stack contract for COND (like the one Section 7.1.1 gives the vector
words) is needed.

## 2. VENT (`^`) behaviour when the top is **not** NIL — *high impact*
**Section 6.4, 7.12, 11.2.**

"If the top of the stack is NIL, replace it with the next stack value." The spec
defines only the NIL branch. It does not say whether the fallback (the next
value) is consumed when the top is non-NIL. As a coalescing operator that should
yield one value, this port consumes both operands and pushes the survivor. An
implementation that leaves the fallback on the stack when the top is non-NIL
diverges. A two-operand stack contract for VENT would settle it.

## 3. Higher-order word stack signatures are undefined — *high impact*
**Section 7.7.**

Section 7.1.1 gives normative stack contracts for the vector words, but the
control/higher-order words (`MAP FILTER FOLD UNFOLD ANY ALL COUNT SCAN`) get only
one-line prose. Unspecified:

- **Argument order and arity.** Is it `vector block FOLD` or `vector init block
  FOLD`? Where does the initial accumulator go? This port chose
  `vector init block FOLD`, `vector block MAP/FILTER`.
- **What the block observes.** For `FOLD`, does the block see `acc elem` or
  `elem acc` on the stack, and must it leave exactly one value? The port pushes
  `acc` then `elem` and pops one result.
- **`UNFOLD` generator protocol** is entirely unspecified: the block's
  input/output shape and its termination condition. The port invented "block maps
  state → `[ value newstate ]`, or NIL to stop".

These are exactly the words a conformance suite would pin first, yet the spec
leaves their observable stack effect open.

## 4. "Exact/total" six relations vs. budgeted UNKNOWN — *high impact, interpretive fork*
**Sections 2.3.1, 2.3.1.1 vs. 7.4.1, 7.4.2.**

Section 2.3.1 states that "In the current algebraic domain, L1 comparisons are
total and exact", and 2.3.1.1 promises a user can rely on
`2 MATH@SQRT 2 MATH@SQRT SUB 0 EQ → TRUE`. Section 7.4.1 simultaneously frames
*all six relations* as running under a partial-quotient budget that yields
`UNKNOWN` on exhaustion. For two equal irrationals these conflict: a pure
CF-budget `EQ` would return `UNKNOWN`, but the exactness claim demands `TRUE`.

The only consistent reading is that the bare relations decide *exactly* over the
current algebraic domain while **only `COMPARE-WITHIN` exposes the budget** and
can return `UNKNOWN`. The spec never states this split explicitly; an implementer
who takes 7.4.1 literally for the bare relations produces different results
(`2 MATH@SQRT 2 MATH@SQRT EQ` → `UNKNOWN` instead of `TRUE`). This port decides
the six relations exactly and reserves budget/`UNKNOWN` for `COMPARE-WITHIN`.

## 5. The boundary of the "current algebraic domain" is never delimited — *high impact*
**Sections 4.2, 4.2.2, 7.3, 9.1 (MATH).**

The spec promises every number is an exact real (continued fraction) and that
arithmetic is closed and exact, but never says **which values the current
Coreword set can actually produce and operate on**. Concretely undefined:

- Is `2 MATH@SQRT 3 MATH@SQRT ADD` (= √2 + √3) in-domain? (Sum of distinct
  surds.)
- Is division by such a value required to be exact?
- Does `MATH@POW` accept non-integer exponents (producing new irrationals)?

A finite implementation must commit to a representable sub-domain. This port
represents values as finite Q-combinations of square-free surds (closed under
`+ - *`, and division by divisors with ≤ 1 surd) and raises a domain error
otherwise. A different but equally spec-faithful choice would accept or reject
different programs. The spec should state the closed algebraic domain the current
Coreword set must support (and what happens at its edge — error vs. lazy CF).

## 6. Default interpretation role of a scalar, and RawNumber for irrationals — *medium impact*
**Section 12.2.**

- Every spec example renders a bare number as `n/1` (the `RawNumber` surface),
  but 12.2 says the default role is `Unassigned`, whose scalar rendering is "raw
  structural form" — left undefined for numbers. To reproduce `3/1` the port
  defaults freshly produced scalars to `RawNumber`. The spec should state the
  default role of a computed scalar.
- `RawNumber` "renders as a reduced numerator/denominator", which has **no
  meaning for an irrational** like √2. 12.2 admits "RawNumber may lose
  information for lazy irrationals" but specifies no actual surface. The port
  falls back to the truncated CF form; another implementation might print a
  decimal approximation or error, diverging. A defined RawNumber surface for
  irrationals (or a rule forcing the ContinuedFraction role) is needed.

## 7. `STR` and `BOOL` conversion semantics — *medium impact*
**Section 7.6.**

- `STR` "convert value to its string representation" does not say which surface
  or role drives the text. Is `42 STR` the text `42/1` (RawNumber)? The
  ContinuedFraction form? The port uses the Output-surface rendering.
- `BOOL` "convert to boolean" gives no rule: which numbers are true, what NIL or
  a vector converts to. The port uses "non-zero scalar → TRUE, NIL → FALSE,
  other → TRUE", which is a pure guess.

## 8. `JOIN` argument shape — *medium impact*
**Section 7.6.**

"Join a vector of strings, with optional separator" — no stack signature and no
rule for detecting whether the optional separator is present. The port detects it
by type (a top-of-stack Text is the separator), which is ambiguous if the vector
itself could be confused with a separator.

## 9. NIL equality vs. NIL-passthrough on `EQ` — *medium impact*
**Sections 4.5.0, 7.4, 7.12.**

Section 4.5.0 says "equality … treat all NIL values uniformly", suggesting
`NIL NIL EQ` is meaningful, but `EQ` is NIL-passthrough (7.12), so any NIL operand
makes `EQ` return NIL — the two NILs are never actually compared. So a user can
**never** observe `TRUE` from comparing NILs. The "uniform equality" statement
appears to be about hashing/dedup, not the `EQ` word; the spec should disambiguate
which surface it governs.

## 10. Child-runtime concurrency model is unobservable / unspecified — *low impact*
**Section 10.**

`AWAIT` "blocks until the child finishes" and `STATUS` reports state "without
blocking", but no concurrency or scheduling model is given. A conforming
single-threaded port (this one) runs the child eagerly at `SPAWN`, so the
`running` state is never observable and the `killed`/`timeout`/`failed`
transitions of an already-finished child are unreachable. The spec doesn't say
whether eager synchronous execution conforms or whether genuine concurrency (and
thus an observable `running` window) is required.

## 11. `IMPORT-ONLY` / `UNIMPORT-ONLY` produce no clear observable — *low impact*
**Section 9.2, 7.14.**

The partial-import / "shrink to explicit partial-import state" rules are described
narratively but do not reduce to a definite, testable stack/resolution observable
from the spec text alone. This port models per-word import coarsely as a full
import, which is observationally different for programs that import one word and
expect siblings to remain unresolved.

## 12. `PRECOMPUTE` staging has no observable contract — *low impact*
**Section 7.7.**

"Evaluate a code block at definition time and splice the resulting values into the
definition." The splicing mechanics — how staged values interleave with the rest
of the DEF body, and how to observe that staging happened versus ordinary
evaluation — are undefined. Outside a DEF it is an error, but the spec gives no
positive observable for the in-DEF case. The port approximates it as immediate
evaluation.

---

### Notes on things the spec pinned well

The following were specified precisely enough to implement without guessing, and
the port reproduces them exactly: the STAK count-fold and chained-comparison
rules (6.1); the vector-word stack contracts and inspection-retention rule
(7.1.1); `DIV`/`MOD` zero-divisor asymmetry (7.3); `ROUND` half-away-from-zero
(7.3); the K3 truth tables and NIL-over-U priority (7.5, 4.5.2); numeric literal
forms (3.2); string-literal boundary scanning (3.3); and the `PRINT` quote-
stripping surface vs. nested-element quoting (7.9, 12.2). These are reproduced
as passing assertions in `tests/test_spec_examples.py`.
