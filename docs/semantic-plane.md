# The Semantic Plane

Companion to `SPECIFICATION.md` §6, which is canonical. This document explains
the design and records the decisions behind it.

## The problem it solves

A vector of three numbers is a vector of three numbers. It is also, sometimes,
a word; sometimes a range; sometimes just three numbers. The data does not say
which, and a language that only has the Data Plane must either invent a new
value shape for every reading or lose the reading entirely.

Ajisai keeps the reading and calls it a **role**. Roles are the Semantic Plane.

## One canonical home

**A value's role is stored on the value.** That is the whole storage design.

This is worth stating as a rule because the alternatives are so tempting. A
stack that keeps a parallel array of roles is faster to render. A registry that
maps positions to roles survives a value being replaced. A hint on the value
*plus* a role on the stack *plus* a registry entry covers every case at once —
and then three things have to agree, forever, across every save, restore, clone,
snapshot, and boundary in the interpreter, and one of them will drift.

Because the role lives on the value:

- it travels into a vector and back out of it with no rule needed;
- it survives a basin, a quote boundary, and the dictionary for free;
- `DUP` and `SWAP` carry it without knowing it exists;
- there is no synchronisation code, so there is no synchronisation bug.

`crates/ajisai-core/tests/semantic_plane.rs` holds this from the outside.

## Exactly three roles

`RAW`, `TEXT`, `INTERVAL`.

Each has a generator reachable from source, a consumer that observes it, and a
propagation rule. A role that could not be produced, or that nothing read, would
be a name in an enum rather than a part of the language — and this rebuild was
largely an exercise in removing those.

Adding a role is a language change and belongs in `SPECIFICATION.md`. A package
cannot add one (§13) — which is a real limit on what a package can be, and
`SPECIFICATION.md` §13 states it rather than leaving it to be discovered.

## What roles affect, stated honestly

Roles affect **rendering**, the **role words**, and **two role-sensitive
words**: `DEF` and `DEL`, which require a name to be read as `TEXT`.

That third item is the one worth defending, because the tidier position — "a
role never touches computation" — was what an earlier draft claimed while the
implementation did something else. `DEF` declared a `Text` input and then
accepted any vector of codepoints, so `{ 2 MUL } [ 68 79 85 66 76 69 ] DEF`
defined `DOUBLE`. Four different things said four different things: the
specification said roles change nothing, the contract said the name is text,
the implementation ignored the role, and a comment claimed the Semantic Plane
was load bearing there.

The honest resolutions were two, and the tidier one is the weaker one. If a
bare vector of numbers can be a name, then the reading a program asserts about
its own data counts for nothing at exactly the point a language most needs a
name to be a name — and the plane is decoration after all. So `DEF` requires
the role, the specification says which words are role-sensitive, and the set is
enumerable from the contracts rather than discoverable by reading source.

What roles still never do is change a **result**. A role decides whether an
operand is admissible; nothing computes a different value because of one.
Equality is the clearest case: `"A" [ 65 ] EQ` is `TRUE`, deliberately, because
if equality consulted the role then a reading would be deciding a computation.

## One well-formedness rule

A role is **admitted** by a value when the value's shape satisfies the role's
condition. That single predicate drives both directions:

- asserting a role (`>TEXT`, `>INTERVAL`) checks it and raises `BadRole` on
  failure;
- propagating a role (`REST`, `REVERSE`, `CONCAT`, `MAP`, `FILTER`) checks it
  and drops to `RAW` on failure.

So `REST` of a text is a text, `REST` of an interval is raw, and neither case
needed its own rule. In the implementation this is
`role::admits` and `role::retain`, and a test asserts the two agree.

## Why not more readings

Candidates that were considered and left out, each for the same reason — nothing
would read them:

- a `TIMESTAMP` role: renders as a date, but Ajisai Core has no clock, so the
  role would exist only to be displayed by a UI the language does not specify;
- a `RATIO` role: every number is already an exact rational, so the reading adds
  nothing to the rendering;
- a `MATRIX` role: vectors already nest, and a role asserting rectangularity
  would have to be re-checked on every operation to stay true.

If one of these later gains a real consumer, it is a specification change with a
generator, a consumer, and a propagation rule — not a variant added in advance.
