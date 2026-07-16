# Reference drift: square-root comparisons wrongly shown as UNKNOWN

Status: **Non-canonical** (developer note / drift record).
Authority: this document defines no semantics. The canonical source is
`SPECIFICATION.html`; the conformance suite (`tests/conformance/index.html`)
is the tie-breaker below it in the §2.5 authority order. This note records a
corrected divergence in the **Reference** (`public/docs/index.html`), a
Derived document.

## The divergence

The Reference taught, in the Comparison and the Three-Valued Logic pages,
that an ordinary relation over square roots can return `UNKNOWN`:

| Reference example (before) | Reference showed | Correct value |
|---|---|---|
| `'math' IMPORT 2 SQRT 2 SQRT SUB 0 EQ` | `UNKNOWN` | `TRUE` |
| `'math' IMPORT 2 SQRT 2 SQRT SUB 0 EQ TRUE AND` | `UNKNOWN` | `TRUE` |

with a concept note claiming "every comparison runs under a budget" and that
"equal irrationals exhaust it and yield `UNKNOWN`."

## Why the Reference was wrong (not the implementation)

This is **not** spec/implementation drift. All three authorities above the
Reference already agree that the value is `TRUE`:

- **Specification.** §4.2.7 defines the admitted domain \(D\) — rationals,
  `SQRT` of rationals, closed under \(+\,-\,\times\,\div\) — and §7.4
  ("Exactness over the admitted domain") makes the six relations *total and
  exact* over \(D\); `U` is reserved for exact lazy values outside \(D\)
  (future words) and for `COMPARE-WITHIN`. §2.3.1.1 gives this exact program
  as a worked example evaluating to `TRUE`.
- **Conformance suite.** `core-sqrt2-minus-sqrt2-eq-zero`
  (`tests/conformance/index.html`) pins `2 SQRT 2 SQRT SUB 0 EQ` → `TRUE`.
  Its comment states the six bare relations "can no longer produce UNKNOWN"
  over `SQRT`-built values, and names `COMPARE-WITHIN` as the current
  `UNKNOWN` producer (`core-unknown-spelling`,
  `core-unknown-propagation`).
- **Implementation.** `continued_fraction.rs::cmp_with_budget_tracked`
  short-circuits any Gosper operand through the multiquadratic normal form
  (`multiquadratic::algebraic_cmp`) *before* spending a partial quotient, so
  admitted-domain comparisons decide exactly. Unit tests assert
  `2 SQRT 2 SQRT EQ` → `TRUE` and `2 SQRT 2 SQRT SUB SIGN` / `... SUB ABS`
  → `0` (`arithmetic_operation_tests.rs`, `math_ops_tests.rs`).

The Reference examples were stale — a snapshot of behavior from before the
admitted-domain exact comparison landed. Per the §2.5 order (spec ▷ math ▷
conformance ▷ generated ▷ Reference ▷ style ▷ impl), the Reference is the
lowest doc surface here and simply had to be brought into line.

## Resolution

`public/docs/index.html`:

- Comparison page: `2 SQRT 2 SQRT SUB 0 EQ` now shows `TRUE`; the note
  explains that square roots and their field combinations compare exactly
  with no budget. A genuine `UNKNOWN` example was added using the
  conformance-verified `2 SQRT 1 ADD 2 SQRT 1 ADD 8 COMPARE-WITHIN`, and the
  section note now attributes `UNKNOWN` to `COMPARE-WITHIN` and to reserved
  future lazy values, not to the bare relations.
- Three-Valued Logic page: the Kleene-AND propagation example now derives its
  `UNKNOWN` from `... COMPARE-WITHIN TRUE AND` (fixture
  `core-unknown-propagation`) instead of the sqrt relation.

Every expected value now equals a conformance-suite observation.

## Related

- The Cost Model page and `docs/dev/cost-model-user-guidance-design.md`
  carried the same wrong framing when first drafted and were corrected in the
  same spirit (bare relations decide the admitted domain exactly;
  `COMPARE-WITHIN` is the budget window).
- Drift-handling policy: `docs/dev/spec-impl-drift-tactic.md`
  ("observable behavior must trace to a canonical anchor; the suite breaks
  ties").
