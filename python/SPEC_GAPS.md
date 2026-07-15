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

> **RESOLVED (Section 7.7.1).** A new normative subsection *"7.7.1 Stack
> signatures and consumption"* now fixes the COND contract in the same form as
> the Section 7.1.1 vector-word contracts: COND consumes *all* consecutive
> clause blocks on top of the stack, then consumes the subject beneath them;
> each guard runs in isolation on a *fresh copy* of the subject (so a guard that
> consumes it does not disturb later guards); the matched body runs in isolation
> seeded with the subject and contributes exactly one value; `IDLE` is the else
> clause; a U guard does not fire (Section 7.4.3); and exhaustion raises
> `CondExhausted`. The subject *is* consumed — `[ 2 ] CLASSIFY → [ 200/1 ]`, not
> `[ 2/1 ] [ 200/1 ]`. This port and the conformance suite (`core-cond-*`) now
> match the implementation; the reference interpreter collects consecutive
> clause blocks and consumes the subject accordingly.

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

> **RESOLVED (Section 6.4).** Section 6.4 now specifies `VENT` (`^`) as a *lazy
> control directive*, not a two-operand stack word. On a non-NIL top the value
> is kept and the *following source unit* (one token, or one balanced `[ ]` /
> `{ }` group) is skipped **unevaluated**; on a NIL top the NIL is discarded and
> the following unit is evaluated as the fallback. The "next stack value"
> wording is explicitly superseded. The normative traps are documented too:
> only one source unit is skipped (`1 ^ 2 3 ADD → 4/1`), and a `{ }` fallback on
> the NIL branch is pushed as a code block rather than run. This port and the
> conformance suite (`core-vent-*`) implement the directive form: the fallback
> after `^` is skipped on the non-NIL branch and evaluated on the NIL branch.

"If the top of the stack is NIL, replace it with the next stack value." The spec
defines only the NIL branch. It does not say whether the fallback (the next
value) is consumed when the top is non-NIL. As a coalescing operator that should
yield one value, this port consumes both operands and pushes the survivor. An
implementation that leaves the fallback on the stack when the top is non-NIL
diverges. A two-operand stack contract for VENT would settle it.

## 3. Higher-order word stack signatures are undefined — *high impact*
**Section 7.7.**

> **RESOLVED (Section 7.7.1).** The new Section 7.7.1 gives normative stack
> signatures for every higher-order word: `vector block MAP`,
> `vector block FILTER`, `vector init block FOLD`, `seed block UNFOLD`,
> `vector block ANY` / `ALL`, `vector block COUNT` → `[ n ]`, and
> `vector init block SCAN`. It fixes the argument order (accumulator init before
> the block for FOLD/SCAN), what the block observes (`acc elem` for FOLD/SCAN),
> the one-value-out rule, the one-element-vector unwrap for MAP/SCAN, the
> per-word NIL-target results, and the UNFOLD generator protocol
> (`state → [ element next-state ] | NIL`). The block argument may be a `{ ... }`
> code block *or* a quoted word name. This port implements all eight words to
> match (`core-map-block`, `core-fold-bare-init`, `core-unfold-generator`, etc.).

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

> **RESOLVED (re-confirmed, spec version 2026-07-15).** CLI probes re-verified
> the Section 7.4 "Exactness over the admitted domain" contract against the
> implementation: `2 MATH@SQRT 2 MATH@SQRT EQ → TRUE`,
> `2 MATH@SQRT 2 MATH@SQRT SUB 0 EQ → TRUE`, and multi-surd equality
> `√2+√3 = √3+√2 → TRUE`, all with no budget in play. No further spec change
> was needed; the 2026-07-01 resolution below stands.

> **RESOLVED (spec version 2026-07-01).** Section 7.4 now carries a normative
> paragraph *"Exactness over the admitted domain"*: over the admitted domain `D`
> (new Section 4.2.7) the six relations are total and exact and never return
> `unknown`; the budget and `unknown` are confined to `COMPARE-WITHIN` and to
> exact reals outside `D`. Section 7.4.1's "U outcome is required for all six
> relations" is now scoped to the lazy case, and Section 16 adds a matching
> conformance item. The port verifies this (`test_spec_examples.py`).

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

> **RESOLVED (re-confirmed, spec version 2026-07-15).** CLI probes re-verified
> every Section 4.2.7 boundary against the implementation: `√2+√3` is
> in-domain, division by the multi-surd `√2+√3` is exact
> (`1 (√2+√3) DIV (√2+√3) MUL 1 EQ → TRUE`), `SQRT` of a non-rational
> (`√√2`) is malformed use → error, and `MATH@POW` rejects a non-integer
> exponent. No further spec change was needed. One implementation wrinkle was
> found and reported (not fixed here): `-4 MATH@SQRT` correctly projects to
> Bubble/NIL, but with `reason = divisionByZero` where a domain-miss reason
> would be expected.

> **RESOLVED (spec version 2026-07-01).** New Section 4.2.7 defines the admitted
> domain `D` as the multiquadratic closure of ℚ — the field ℚ(√d₁, √d₂, …)
> generated by square roots of square-free positive integers — closed under
> `+ - * /`, with `MATH@SQRT` defined on rational operands only and `MATH@POW`
> on integer exponents only; leaving `D` (e.g. `SQRT` of a non-rational) is
> malformed use. The port's `AlgebraicReal` now realises `D` exactly, including
> division by multi-surd denominators via Galois conjugation
> (`1 (√2+√3) DIV (√2+√3) MUL → 1`).

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

> **RESOLVED (Section 12.2, spec version 2026-07-15).** A new normative
> paragraph *"Default role of a computed scalar"* fixes both points from CLI
> probes: a computed **rational** scalar renders as its reduced
> `numerator/denominator` (the `RawNumber` rendering — `3` renders `3/1`),
> and a computed **non-rational** scalar renders as the truncated
> `ContinuedFraction` nested form (`2 MATH@SQRT` displays
> `( 1 ( 2 ( 2 ...`). Whether the internal role tag is `Unassigned` or
> `RawNumber`/`ContinuedFraction` is declared not observable (implementation
> freedom), because `Unassigned`'s "raw structural form" for a scalar is
> defined to coincide with these renderings. The "RawNumber surface of a lazy
> irrational" is declared unreachable — no current Coreword assigns that role
> to a non-rational — and the only compact text surface for a non-rational,
> `STR`, is normatively an implementation-defined non-canonical approximation
> (Section 7.6.1; the probe shows the reference implementation prints a
> rational convergent such as `665857/470832` for `√2`). Conformance case:
> `core-scalar-default-rendering`.

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

> **RESOLVED (Section 7.6.1, spec version 2026-07-15).** A new normative
> subsection gives per-type conversion tables for both words, fixed by CLI
> probes of the implementation. `STR` is total: Text is identity; a rational
> renders compactly (`42 STR → '42'` — deliberately *not* the Stack surface
> `42/1` — and `1 2 DIV STR → '1/2'`); a non-rational yields an
> implementation-defined, non-canonical approximation Text; Booleans render
> by spelling; NIL and U yield a *fresh, reasonless* NIL
> (`0 0 DIV STR NIL-REASON → NIL NIL`); a Vector/Record flattens to its
> space-joined leaves with Text elements decaying to code points
> (`[ 'AB' 'CD' ] STR → '65 66 67 68'`); CodeBlocks/handles yield an
> implementation-defined placeholder. `BOOL` is defined over Booleans,
> rational scalars (zero → FALSE, else TRUE), and Text (case-insensitive
> `'TRUE'`/`'FALSE'`; **any** other text — including `'42'` — is a
> well-formed failure → NIL); NIL, U, non-rational scalars, and containers
> are malformed use → error. This port's guesses ("Output-surface rendering"
> for STR; "NIL → FALSE, other → TRUE" for BOOL) are superseded; the
> conformance reference interpreter (`tools/ajisai-repro/ajisai.py`) now
> implements the adjudicated tables. Conformance cases:
> `core-str-nil-yields-fresh-nil`, `core-str-flattens-container-leaves`,
> `core-str-nil-leaf-renders-letters`, `core-bool-numeric-text-is-nil`
> (plus the pre-existing `core-str-*` / `core-bool-*` cases).

- `STR` "convert value to its string representation" does not say which surface
  or role drives the text. Is `42 STR` the text `42/1` (RawNumber)? The
  ContinuedFraction form? The port uses the Output-surface rendering.
- `BOOL` "convert to boolean" gives no rule: which numbers are true, what NIL or
  a vector converts to. The port uses "non-zero scalar → TRUE, NIL → FALSE,
  other → TRUE", which is a pure guess.

## 8. `JOIN` argument shape — *medium impact*
**Section 7.6.**

> **RESOLVED (Section 7.6.1, spec version 2026-07-15).** Probes settled the
> ambiguity in the simplest possible way: **there is no separator operand at
> all.** `JOIN` consumes exactly one value — the vector on top of the stack —
> and the "optional separator" wording is superseded in the Section 7.6 table
> and Section 7.6.1 contract. `[ 'A' 'B' 'C' ] ',' JOIN` joins the `','`
> itself (a Text is a code-point vector, joining to itself) and leaves the
> vector untouched. Element rule: Text elements append as content; integer
> scalars append as Unicode code points; invalid code points and all other
> element types (NIL, Boolean, nested Vector, …) are malformed use → error;
> a NIL target is malformed use → error. The type-sniffing separator
> detection this port implemented is superseded; the reference interpreter
> now matches. Conformance cases: `core-join-no-separator-operand`,
> `core-join-mixed-text-and-codepoints`, `core-join-rejects-boolean-element`,
> `core-join-rejects-invalid-codepoint`.

"Join a vector of strings, with optional separator" — no stack signature and no
rule for detecting whether the optional separator is present. The port detects it
by type (a top-of-stack Text is the separator), which is ambiguous if the vector
itself could be confused with a separator.

## 9. NIL equality vs. NIL-passthrough on `EQ` — *medium impact*
**Sections 4.5.0, 7.4, 7.12.**

> **RESOLVED (Sections 4.5.0 and 7.4, spec version 2026-07-15).** A new
> normative paragraph *"NIL operands and NIL equality"* in Section 7.4
> disambiguates the two surfaces exactly as this finding suspected. The `EQ`
> word is NIL-passthrough for **top-level** operands: `NIL NIL EQ → NIL`
> (never TRUE), preserving the leftmost reason
> (`1 0 DIV 1 EQ NIL-REASON → NIL 'divisionByZero'`). Section 4.5.0's
> "uniform" equality governs **structural equality** — the element-wise
> equality inside containers (and hashing/dedup) — where two NIL elements
> compare equal regardless of diagnostic metadata:
> `0 0 DIV 1 COLLECT NIL 1 COLLECT EQ → TRUE`. Section 4.5.0 now
> cross-references this split. The reference interpreter's structural
> equality was aligned (NIL elements were previously never equal).
> Conformance cases: `core-eq-nil-nil-passthrough`,
> `core-eq-nil-passthrough-preserves-reason`,
> `core-eq-structural-nil-uniform`.

Section 4.5.0 says "equality … treat all NIL values uniformly", suggesting
`NIL NIL EQ` is meaningful, but `EQ` is NIL-passthrough (7.12), so any NIL operand
makes `EQ` return NIL — the two NILs are never actually compared. So a user can
**never** observe `TRUE` from comparing NILs. The "uniform equality" statement
appears to be about hashing/dedup, not the `EQ` word; the spec should disambiguate
which surface it governs.

## 10. Child-runtime concurrency model is unobservable / unspecified — *low impact*
**Section 10.**

> **RESOLVED (Section 10.8, spec version 2026-07-15).** A new normative
> subsection fixes only the observable guarantees and declares scheduling
> implementation freedom, directly answering this finding's question: **eager
> synchronous execution at `SPAWN` conforms.** Normative surface: `AWAIT`'s
> `[status result-stack]` is deterministic for a deterministic block
> (`{ 1 2 ADD } SPAWN AWAIT → [ 'completed' [ 3/1 ] ]`; a raising child →
> `[ 'failed' [ stack-at-failure ] ]`; a Bubble/NIL is a value →
> `'completed'`); parent/child isolation per Section 10.1; and
> terminal-state stability. Whether `STATUS` ever reports `running` is
> schedule-dependent and explicitly **not** conformance surface — programs
> must not depend on observing it (the current Rust implementation happens
> to expose a `running` window; this port's never-observable `running` is
> equally conforming). Conformance cases (hosted category, outside this
> Core-only port's scope): `hosted-child-await-completed-deterministic`,
> `hosted-child-await-failed-child`, `hosted-child-bubble-completes`.

`AWAIT` "blocks until the child finishes" and `STATUS` reports state "without
blocking", but no concurrency or scheduling model is given. A conforming
single-threaded port (this one) runs the child eagerly at `SPAWN`, so the
`running` state is never observable and the `killed`/`timeout`/`failed`
transitions of an already-finished child are unreachable. The spec doesn't say
whether eager synchronous execution conforms or whether genuine concurrency (and
thus an observable `running` window) is required.

## 11. `IMPORT-ONLY` / `UNIMPORT-ONLY` produce no clear observable — *low impact*
**Section 9.2, 7.14.**

> **RESOLVED (Section 9.2, spec version 2026-07-15).** A new normative
> paragraph *"Observable resolution contract of a partial import"* reduces
> the narrative to a testable observable, fixed by CLI probes: after
> `'M' [ 'W' ] IMPORT-ONLY`, exactly the selected words resolve — in **both**
> bare and qualified form — and unselected siblings resolve in **neither**
> form (`'math' [ 'SQRT' ] IMPORT-ONLY -5 ABS` and `… -5 MATH@ABS` both raise
> `UnknownWord`). The qualified form is explicitly not a backdoor around a
> partial import, and Section 7.9's "always reachable in qualified form"
> wording was tightened to require an import that includes the word. The
> internal representation of the partial-import state is declared
> implementation freedom. This port's coarse full-import model is superseded;
> the reference interpreter now gates qualified resolution on the import set.
> Conformance cases: `core-import-only-selected-qualified`,
> `core-import-only-sibling-bare-unresolved`,
> `core-import-only-sibling-qualified-unresolved`.

The partial-import / "shrink to explicit partial-import state" rules are described
narratively but do not reduce to a definite, testable stack/resolution observable
from the spec text alone. This port models per-word import coarsely as a full
import, which is observationally different for programs that import one word and
expect siblings to remain unresolved.

## 12. `PRECOMPUTE` staging has no observable contract — *low impact*
**Section 7.7.**

> **RESOLVED (Section 7.7, spec version 2026-07-15).** A normative
> *"Observable staging contract"* now follows the `PRECOMPUTE` paragraph,
> fixed by CLI probes: (1) the staged block runs once at `DEF` time in an
> isolated evaluation seeded with an **empty** stack — it never sees the
> definition-time stack (`5 { { 2 MUL } PRECOMPUTE } '…' DEF` is a
> definition-time error); (2) all values the block leaves are spliced
> in place, in order (`{ { 1 2 } PRECOMPUTE 10 ADD } '…' DEF` then calling
> gives `1/1 12/1`); (3) staging failures — a raising block, an unsupported
> staged value such as NIL, or a non-definition-time-safe (effectful) word
> like `PRINT` — fail `DEF` itself, so staging *is* positively observable as
> the definition-time firing/rejection of what would otherwise happen at
> call time; (4) outside `DEF` it is an error. Everything else (caching
> layout, materialization timing) is implementation freedom. This port's
> "immediate evaluation" approximation is superseded; the reference
> interpreter now implements the isolation, error-wrapping, and
> effect-rejection rules. Conformance cases:
> `core-precompute-splices-values-in-order`,
> `core-precompute-empty-seed-stack`,
> `core-precompute-rejects-effectful-word` (plus the pre-existing
> `core-precompute-*` cases).

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
