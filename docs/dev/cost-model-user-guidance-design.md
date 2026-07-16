# Cost Model — user-facing performance guidance (design note)

Status: **Non-canonical** (developer note).
Authority: this document does not define Ajisai semantics. The canonical
source is `SPECIFICATION.html`. If anything here conflicts with the
specification, the specification wins. This note records **why** the
Reference carries a user-facing cost model and **where** each claim in it
comes from; it is not itself the cost model.

## Why this exists

An external review raised a real tension. Ajisai pursues, at once:

- exact real numbers (continued fractions);
- lazy infinite values (square roots and reserved lazy CFs);
- a dense, SIMD-oriented tensor representation;
- implicit, shape-aware parallelism (VTU).

Dense acceleration wants fixed-width, homogeneous lanes. Arbitrary-precision
and lazy values do not have fixed cost per value. Ajisai's answer is a **fast
lane for small rationals and no silent approximation for anything else** —
which is the right call, but it leaves the *user* unable to predict:

1. which values ride the fast lane;
2. which operations promote a value to a slower representation;
3. how much exactness costs;
4. when the comparison budget is spent.

The specification answers all four, but only implicitly and scattered across
several sections written for porters, not users. The review's concrete
recommendation was: **document a user-facing cost model separately from the
language specification.** This note and the new Reference page do exactly
that. The cost model is *guidance*, never a semantic guarantee: it may not
name a promotion boundary that a future optimizer moves, and it must never be
read as changing an observable value.

## Placement decision

The cost model lives in the **Reference** (`public/docs/index.html`), not in
`SPECIFICATION.html` and not as a dev note. Rationale:

- The audience is Ajisai *users* (README document-role table). Performance
  intuition is learning material, the Reference's remit.
- Keeping it out of the specification preserves the specification's role as
  the semantic authority. Performance is not semantics: the cost model
  describes representation *speed*, and every claim it makes is invariant
  under the "internal representation is not observable" rule (§4.2.2, §4.3.1).
- A single Reference page, added to the site nav, is discoverable and sits
  beside the numeric and comparison pages it depends on.

## Source mapping (every user-facing claim → spec anchor)

| Reference claim | Canonical source |
|---|---|
| Small rationals (i64/i64 reduced fraction) are the fast representation | §4.2.2 Internal representation; §4.3.1 dense lane admission |
| Values needing arbitrary-precision integers stay exact but leave the small-lane path | §4.2.2 (Rational → BigInt); §4.3.1 (BigInt lanes not dense-admitted) |
| `SQRT` of a non-square, and arithmetic that mixes such a value, is a lazy CF | §4.2.2 (AlgebraicSqrt, Gosper); §7.6 |
| A dense vector needs uniform small-rational lanes and a rectangular shape | §4.3.1 dense representation class + exactness rule |
| A lane becoming NIL does **not** rebuild a dense vector into nested form | §4.3.1 No-Rebuild Principle |
| Mixed-type / ragged vectors are nested, not dense | §4.3 Role of nesting; §4.3.1 nested class |
| Finite rationals always decide a comparison; no observable budget | §7.4 Exactness over the admitted domain; §4.2.7 |
| Lazy irrationals run under a partial-quotient budget and can yield `UNKNOWN` | §7.4.1 Decidability and comparison budget |
| Budget unit is one nearest-integer CF (NICF) term | §4.2.5; §7.4.1.1 |
| `COMPARE-WITHIN` names the budget explicitly | §7.4.2 |
| Exactness is preserved — no truncation, rounding, or overflow wraparound | §4.2.6 Numeric error policy |

The measured "small-rational fast path is ~1.21× the broadcast path in a
countdown loop" figure comes from `docs/dev/scalar-fastpath-d1.md` (D1 A/B
run). The Reference states it as an illustration with that caveat, not as a
guaranteed ratio; the cost model gives *direction and triggers*, and only one
concrete number, deliberately.

## Non-goals

- Not a benchmark suite and not a promise of any specific speedup.
- Does not introduce, rename, or gate any word or numeric type.
- Does not touch VTU classification behavior; VTU stays observational
  (`docs/dev/virtual-tensor-unit-design.md`).
- Does not restate the specification; it links to it as the authority.
