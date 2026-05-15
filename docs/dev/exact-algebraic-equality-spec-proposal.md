# Spec Proposal: Decidable Exact Equality over the Algebraic Domain

Status: **Proposal (non-canonical)**
Target: amendment to `SPECIFICATION.md` §4.2, §7.3, §7.4, §7.4.1, §11.2, §16
Supersedes: the rejected memo `undecidable-fallback-implementation-instruction.md`

## 0. Authority note

This document is a design proposal. Per `SPECIFICATION.md` §2.2 it is
non-canonical and defines no semantics until its amendments are merged
into `SPECIFICATION.md` itself. Nothing here may be implemented as
observable behavior before that merge.

## 1. Problem

`SQRT` of a rational produces an irrational continued fraction. Arithmetic
on such values currently collapses into an opaque `Gosper` node
(`continued_fraction.rs`, `ExactReal::Gosper`). Once a value is a `Gosper`,
equality is only **semi-decidable**: comparison emits partial quotients in
parallel and, for two equal irrationals, never finds a difference. §7.4.1
bounds this with a partial-quotient budget and projects exhaustion onto
`NilReason::Undecidable` / `AbsenceOrigin::ComparisonBudget`.

The consequence is that mathematically exact identities are reported as
undecidable:

```ajisai
2 MATH@SQRT 2 MATH@SQRT SUB 0 EQ
```

`√2 − √2` is exactly `0`, but the bihomographic Gosper transform treats its
two operands as independent CF streams and cannot prove the difference is
zero. The comparison budget is exhausted and the result is
`NIL (undecidable)`.

A previously circulated memo proposed catching this NIL and silently
falling back to an epsilon comparison. That proposal was rejected: it
violates §1 (exact reals, no rounding), §4.3 (no approximate numeric
types), §11.3 (`SAFE` is not a generic swallower), and Conformance #11
(budget exhaustion must not become "a non-deterministic answer"). See
the review in the session that produced this document.

## 2. Mathematical scope (honest boundary)

Two properties must not be conflated:

| Property | Rationals | Irrational algebraics (`SQRT`) | Transcendentals |
|----------|-----------|--------------------------------|-----------------|
| Exact representation, no rounding | yes (today) | yes (today) | yes (achievable) |
| **Decidable** equality / ordering | yes (today) | **yes (this proposal)** | **no — undecidable in principle** |

Equality of transcendental numbers is not decidable: whether `e + π` is
rational is an open mathematical problem, and no representation can
provide a total equality test for arbitrary transcendentals. Therefore
this proposal does **not** claim to remove `Undecidable`. It claims
something precise and fully achievable:

> Every value the current Ajisai Coreword set can construct is an
> **algebraic number** (a rational, a `SQRT` of a rational, or an
> arithmetic combination thereof). Equality and ordering over that
> domain are decidable. This proposal makes them decidable in the
> implementation, so `Undecidable` becomes **unreachable for the
> current Coreword set** and is retained in the spec solely as the
> defined behavior for a future transcendental domain.

`SPECIFICATION.md` already supports this framing: §4.2.2 lists `LazyCf`
as "reserved for future words; not produced by the Coreword set defined
in this document." Transcendentals are exactly that future domain.

## 3. Key mathematical fact

The arithmetic closure of the rationals and `{√r : r ∈ ℚ, r ≥ 0}` under
`+ − × ÷` lies inside **multiquadratic fields**
`ℚ(√d₁, √d₂, …, √dₖ)`, where each `dᵢ` is a squarefree positive integer.

Every element of such a field has a unique representation as a
finite ℚ-linear combination of square-root monomials:

```
x = Σ  q_D · √D        ( D squarefree, D ≥ 1; √1 = 1; q_D ∈ ℚ )
```

The square roots of distinct squarefree integers are **linearly
independent over ℚ** (classical theorem). Consequently:

- the representation is **canonical** — each algebraic value in the
  domain has exactly one such coefficient map;
- **equality is structural**: `x = y` iff their coefficient maps are
  identical after normalization;
- **`x = 0`** iff the map is empty after dropping zero coefficients;
- **ordering** reduces to the sign of `x − y`; once `x − y ≠ 0` is
  established structurally, its sign is found by interval evaluation
  that is *guaranteed to terminate* because the value is known nonzero.

This gives total, exact, budget-free equality and ordering over the
whole algebraic domain — no epsilon, no rounding, no non-determinism.

Worked examples (all decided exactly):

| Expression | Normal form | Result |
|------------|-------------|--------|
| `√2 − √2` | `{}` | `0` |
| `√8 − 2·√2` | `√8 = 2√2`, so `{2: 2} − {2: 2} = {}` | `0` |
| `(√2 + 1)·(√2 − 1)` | `{1:1, 2:1} × {1:−1, 2:1}` → `{1:1}` | `1` |
| `√2 + √3` vs `√5` | `{2:1, 3:1}` ≠ `{5:1}` | not equal |

## 4. Proposed representation

Add a normalized algebraic representation to `ExactReal`:

```
ExactReal::Algebraic(AlgebraicReal)
```

where `AlgebraicReal` is the multiquadratic normal form:

- an ordered map `squarefree_radicand (BigInt ≥ 1) → coefficient (Fraction)`;
- the key `1` carries the rational part;
- zero coefficients are never stored;
- an empty map denotes `0`;
- a map with the single key `1` is a rational and is re-tagged as
  `ExactReal::Rational` so existing fast paths are unaffected.

Construction and arithmetic:

- `SQRT(r)` for rational `r ≥ 0`: factor out perfect-square divisors,
  yielding `coeff · √(squarefree)`; perfect squares and `0` collapse to
  `Rational` (as `from_sqrt_rational` already does).
- `ADD` / `SUB`: merge coefficient maps.
- `MUL`: distribute; `√a · √b = (gcd-extracted) · √(squarefree(ab))`.
- `DIV`: rationalize the denominator via the field's finite-dimensional
  ℚ-algebra structure (inversion through the multiplication matrix /
  conjugate product). Division by structural `0` keeps the existing
  `divisionByZero` Bubble.

`Algebraic` is **closed** under `+ − × ÷`: the result of an operation on
two `Algebraic`/`Rational` operands is always `Algebraic`/`Rational`,
never `Gosper`.

`AlgebraicSqrt { radicand }` becomes a degenerate single-term
`Algebraic` and can be folded into it, or kept as a constructor shortcut.

`Gosper` is retained unchanged. It is reached only when an operand is
**not** in the algebraic domain — which the current Coreword set never
produces. It is the designated representation for the future
transcendental domain.

### 4.1 Display / serialization unchanged

`AlgebraicReal` still produces a CF stream on demand for the
`ContinuedFraction` display hint and for WASM interop. Per §2.3 the
internal representation is not observable; only the canonical CF
sequence and the value are. This proposal adds an internal
representation — it does not change any observable serialization.

## 5. Amendments to `SPECIFICATION.md`

**§4.2.2 Internal representation** — add a fourth bullet:

> - **Algebraic** — a normalized element of a multiquadratic field
>   `ℚ(√d₁, …, √dₖ)`, stored as a canonical map from squarefree radicand
>   to rational coefficient. Closed under `+ − × ÷`. Equality of two
>   `Algebraic` values is decided structurally on the normal form.

**§4.2.4 Equivalence of representations** — add:

> When both operands are `Rational` or `Algebraic`, equality is decided
> structurally on the normal form and always terminates. The
> partial-quotient procedure of §7.4.1 applies only when at least one
> operand is `Gosper`/`LazyCf`.

**§7.3 Arithmetic** — note that arithmetic over the algebraic domain
stays in the algebraic domain and never builds a `Gosper` node; Gosper's
algorithm is used only for operands outside that domain.

**§7.4.1 Decidability and comparison budget** — prepend:

> Comparison of two values that are both in the algebraic domain
> (`Rational` or `Algebraic`) is **total**: it is decided structurally
> on the normal form and never consumes the budget. The budget and the
> `Undecidable` outcome below apply only when at least one operand is a
> lazy non-algebraic CF (`Gosper`/`LazyCf`), i.e. a transcendental value
> introduced by a future Coreword. With the Coreword set defined in this
> document, the `Undecidable` comparison outcome is unreachable.

The rest of §7.4.1 (budget, `Undecidable`, `comparisonBudget`) stays as
the defined behavior for the future transcendental domain.

**§7.3 `DIV`** — the clause "division by an irrational that cannot be
distinguished from zero within the comparison budget produces NIL with
`reason = undecidable`" is similarly scoped to non-algebraic divisors.

**§11.2 Bubble Rule table** — the `EQ`/`NEQ`/`LT`/… `Undecidable` row is
annotated as reachable only for non-algebraic (transcendental) operands.

**§16 Conformance Checklist #11** — amend to:

> …comparison-budget exhaustion produces `NilReason::Undecidable`
> Bubble/NIL rather than an error or a non-deterministic answer; and
> comparison of two algebraic-domain operands is total and never
> produces `Undecidable`.

No protocol strings, no display forms, no Coreword names change.
`NilReason::Undecidable` and `AbsenceOrigin::ComparisonBudget` are kept.

## 6. What does NOT change

- The six comparison words `EQ NEQ LT LTE GT GTE` — same names, same
  contracts, same modifiers. No new `EXACT_EQ` / `APPROX_EQ` word.
- No epsilon, no approximate numeric type, no rounding (§1, §4.3).
- `Undecidable` / `comparisonBudget` plumbing stays in the codebase
  (`error.rs`, `absence.rs`, `value-operations.rs`) for the future
  transcendental domain.
- `=>` (`OR-NIL`) remains the explicit, user-chosen fallback for any
  genuine future `Undecidable`.
- The continued-fraction value model and `ContinuedFraction` display
  hint are unchanged; `Algebraic` is an internal representation only.

## 7. Implementation phases

1. **Phase A — representation.** Add `ExactReal::Algebraic(AlgebraicReal)`
   and the normal-form type, with normalization and equality. Re-tag
   single-`{1:…}` maps as `Rational`. Per-type unit tests for the normal
   form. ~1 file under the 500-line limit (§14.1).
2. **Phase B — `+ − ×`.** Route `Algebraic`/`Rational` operand pairs
   through algebraic arithmetic; keep the Gosper path for any other
   operand. Differential tests against the existing Gosper results for
   the rational sub-cases.
3. **Phase C — `÷`.** Field inversion / denominator rationalization.
4. **Phase D — comparison.** `comparison.rs`: when both operands are
   algebraic, decide structurally; otherwise fall through to the
   existing budgeted CF path. Update Coreword contract narrative.
5. **Phase E — tests.** MC/DC for the algebraic-vs-Gosper branch in
   each comparison word; pin that `√2−√2 == 0` → `TRUE`,
   `√8−2√2 == 0` → `TRUE`, `(√2+1)(√2−1) == 1` → `TRUE`,
   `√2+√3 == √5` → `FALSE`, all without producing NIL. Keep the
   existing `Undecidable` tests, re-scoped to a synthetic non-algebraic
   operand or marked as future-domain.

Each phase is independently testable and separately reviewable; semantic
changes are kept apart from structural cleanup (§14.1).

## 8. Conformance impact

| Checklist item | Effect |
|----------------|--------|
| #2 equal-value output not an error | reinforced — `√2−√2` now yields exact `0` |
| #8 contract metadata | comparison contracts: narrative updated, fields unchanged |
| #11 CF arithmetic, no rounding | reinforced — algebraic path is exact, BigInt, no budget |
| #1 single design authority | satisfied once §4.2/§7.4 amendments merge |

## 9. Open questions for the user

1. **`DIV` scope.** Phase C (field inversion) is the most involved part.
   Acceptable to land Phases A, B, D first (so `√2−√2`, sums, products
   decide) and treat algebraic `÷` as a follow-up, with `Gosper`
   division remaining correct-but-budgeted in the interim?
2. **`AlgebraicSqrt`.** Fold it into `Algebraic` as a single-term value
   (cleaner, one representation), or keep it as a constructor shortcut
   (smaller diff)?
3. **Future transcendentals.** Confirm the intended long-term policy:
   transcendental equality stays `Undecidable` + explicit `=>`, with no
   silent approximate fallback ever. This proposal assumes yes.
