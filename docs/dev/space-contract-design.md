# Space Contracts — Static Footprint Bounds (design note, non-canonical)

> Canonical semantics live in `SPECIFICATION.html`. This note designs Phase 2 of
> the structural-memory-safety roadmap (`structural-memory-safety-roadmap.md`).
> It makes no language-level guarantee on its own; each increment carries its own
> spec edit (if any) and conformance tests when it lands. The `#:contract`
> surface described here is tooling — it adds no runtime semantics.

## Goal — the beyond-Rust target

Rust removes undefined behavior but does **not** bound memory: a Rust program can
OOM at run time with nothing said ahead of time. Ajisai's identity is *check the
computation before it runs*. Phase 2 extends that check from *value* correctness
(the existing `#:contract` arity/purity/nil/linearity axes) and *runtime* space
safety (Phase 3, where a materialization over the water level becomes a bubble)
to a **static, pre-execution space bound**: a word may declare how its extra
materialization grows with its input, and `ajisai check` verifies the
declaration against conservative inference *without running the program*.

Phase 2 and Phase 3 are complementary, not redundant:

- **Phase 3 (runtime, done):** an over-budget materialization *at run time*
  projects onto a diagnosable `NIL` (reason `spaceExhausted`) instead of an
  abort — the safety net when a bound is exceeded.
- **Phase 2 (static, this note):** *before* running, prove the word stays within
  a declared growth class — the guarantee that the net is rarely needed.

## Why a coarse class, not a precise `f(shape)` (first)

A precise symbolic footprint `f(shape)` (e.g. "≤ 3·n + 2 nodes") is the eventual
target, but it is not the honest *first* increment: inferring and comparing
symbolic polynomials over shape variables is a large piece of machinery, and an
unsound first cut would violate this project's "never a false error" contract for
`#:contract`. So Phase 2 starts with a **coarse growth class** — the same shape
the cost model already uses for parallel dispatch (`ParallelOpClass`) and the
cost tiers of SPEC §4.8 — which is soundly inferable by composition and still
catches the bug that matters: *an unbounded materialization hiding in a word that
looks cheap.*

### The class lattice

Ordered from tightest to loosest; inference widens monotonically (a word is only
ever assigned the loosest class any path forces), exactly like the existing
contract lattice (`word_contract_lattice.rs`).

| Class | Meaning | Examples |
| --- | --- | --- |
| `const` | Extra materialization is bounded independent of input size — O(1) new nodes. | `ADD`, `DUP`, `GET`, scalar ops |
| `linear` | Extra materialization is bounded by the total input size — O(n). | `MAP` of a `const` body, `REVERSE`, `CONCAT` |
| `superlinear` | Grows faster than input but still a function of it — O(n²)+. | an outer-product / cross build |
| `unbounded` | Materialization is a function of a *value*, not the input structure's size, so no static bound over input size exists. | `RANGE`, `FILL` (a numeric operand sets the length) |

`unbounded` is the crucial class: `RANGE` and `FILL` take a small input (a pair,
a shape vector) but materialize a length set by the *numeric value* of that
input, so their footprint is not bounded by input *size*. A word that calls them
is `unbounded` unless it constrains that value. This is precisely the footprint
that Phase 3 catches at run time; Phase 2 lets a word *declare* it is not
`unbounded` and have that checked before running.

## Surface syntax

The `#:contract` grammar gains one optional term, written as a single token to
avoid colliding with the bare `linear` of the linearity axis:

```text
#:contract W ( 1 -- 1 ) pure space:const
#:contract BUILD ( 1 -- 1 ) space:linear
#:contract SEQ space:unbounded
```

`space:<class>` where `<class>` ∈ `const` / `linear` / `superlinear` /
`unbounded`. Optional and additive: an omitted term is `None` and unchecked,
matching every other `#:contract` axis.

## Increment plan

1. **This increment (2.1).** Grammar: parse `space:<class>` into
   `ContractDecl.space` (new `contract_space.rs` module, mirroring
   `contract_linearity.rs`). Additive, non-breaking. Inference is not wired yet,
   so a declared class is surfaced as a `note`, never a false `error` —
   preserving the module invariant.
2. **Done (2.2).** Inference (`rust/src/interpreter/word_space.rs`): each
   built-in carries a space class keyed on its `BuiltinExecutorKey`
   (`RANGE`/`FILL` = `unbounded`, structure/movement words = `const`/`linear`),
   and a user word's class is derived by a **provenance-aware slot simulation**
   folded into the existing execution-free contract walk
   (`word_contract.rs`). The simulation tracks whether each simulated stack slot
   is a compile-time literal, a moved word input, or unknown, so it distinguishes
   `[ 0 10 ] RANGE` (literal operand → `const`) from a bare `RANGE` (input
   operand → `unbounded`) — the distinction §"Why a coarse class" flagged as the
   crux. The bound carries an **exactness witness**: `ajisai check` reports a
   declaration the inference provably *attains above* as an `error`, and an
   *unproven* upper bound (higher-order body, recursion, an unresolved
   dependency) as a `note`, never a false error. Any construct the simulation
   cannot model (a lazy `^` fallback, a COND clause split, a dynamic-arity word)
   soundly degrades to the conservative top without a witness. `ajisai contract`
   now reports the inferred class and suggests it when proven.
3. **Later (2.3).** `ajisai check --space` summary surface; then, incrementally,
   refine `unbounded` into a value-parametric bound where the constraining value
   is statically known, moving toward a precise `f(shape)`.

## Soundness stance

Inference is conservative: an unknown or dynamic word widens to `unbounded`, so
the checker never certifies a bound it cannot prove and never emits a false
`error`. This mirrors the arity/purity/nil/linearity axes and the "conservative
is always the safe side" rule the parallel gate already follows.

## Relationship to the spec

Increment 2.1 was tooling only (no spec change). With inference landed (2.2), the
space class joins the `#:contract` axes documented at SPEC §7.14 (the "Space
growth (opt-in contract discipline)" paragraph), cross-referencing the Water
Levels table (Phase 3) as the runtime companion. Unlike Phase 1's split between a
normative property (§4.7) and its opt-in check (§7.14), the space contract has no
separate normative surface: it is purely an opt-in, checked declaration over the
existing materialization semantics, so it lives wholly in §7.14 as tooling.
