# The semantic ontology

Every semantic field Ajisai retains, and what reads it.

The test for keeping a field is not whether it is meaningful, well named, or
plausibly useful later. It is whether something reads it and acts differently
because of what it says. Everything below passes that test; the fields that did
not are gone, and `docs/migration.md` records the ones whose removal is
observable.

---

## `Value.role` — the Semantic Plane

**Definition.** How a value is read, as distinct from what it is. One of `RAW`,
`TEXT`, `INTERVAL`.

**Produced by.** Text literals (`TEXT`); `>TEXT`, `>INTERVAL`, `>RAW`;
propagation through `REST`, `REVERSE`, `APPEND`, `CONCAT`, `MAP`, `FILTER`.

**Read by.** The renderer, which prints `"hi"` rather than `[ 104 105 ]` and
`1..3` rather than `[ 1 3 ]`; the word `ROLE`; `role::admits`, which decides
whether an assertion succeeds and whether a propagated role survives.

**Effect.** Rendering, and the role words. Never a Data Plane result
(`SPECIFICATION.md` §6.3).

**Invariants.** A value's role is always admitted by its shape. There is exactly
one storage location. Equality ignores it.

---

## `Mode.selection` and `Mode.retention`

**Definition.** Where the next word draws from (`TOP`/`STAK`) and whether it
consumes what it drew (`EAT`/`KEEP`).

**Produced by.** The four directive words and their aliases.

**Read by.** The operand layer in the interpreter, once, for every word with a
fixed stack effect; and the words with dynamic effects, which reject a
non-default mode.

**Effect.** Changes which operands a word receives and what remains afterwards —
directly, on every affected execution.

**Invariants.** The axes are independent. A mode applies to one word invocation.
A body starts and must end at the default.

---

## `WordContract.arity` and `.stack_effect`

**Definition.** How many values a word draws and leaves, as a machine value and
as prose notation.

**Produced by.** The word registry.

**Read by.** The operand layer, to select operands; the contract lint, to track
the abstract flow; the manifest; the diagnostics, which quote the notation.

**Effect.** Execution (operand selection) and diagnostics.

**Invariants.** The prose notation and the machine arity agree — checked against
each other by a test rather than both trusted. An `Op` word always declares a
fixed effect.

Two views of one fact are usually a smell. They are kept here because the
notation names its operands (`( vector quote -- vector )` says more than
`2 -> 1`) and because the check between them is cheap and mechanical.

---

## `WordContract.input_types` and `.output_types`

**Definition.** The kind of value each position accepts or yields.

**Produced by.** The registry.

**Read by.** The contract lint, to report a definite type contradiction; the
manifest.

**Effect.** Diagnostics only. **The types are not enforced at run time by this
field** — each word validates its own operands and raises its own
`TypeMismatch`. The declaration exists so the lint can see a mismatch without
running the program.

---

## `WordContract.nil_policy` and `.unknown_policy`

**Definition.** Two terms each: `rejects` (the word errors when this value
reaches an input) and `may_produce` (the word can put this value into the flow).

**Produced by.** The registry.

**Read by.** The lint, which raises an advisory when a value that may be `NIL`
reaches a word that rejects `NIL`, and the same for `UNKNOWN`; the manifest,
where `may_produce_unknown` is the machine-readable statement of which words are
canonical `UNKNOWN` sources (`SPECIFICATION.md` §7.2).

**Effect.** Diagnostics and documentation.

**Note on what was removed.** An earlier draft of this type distinguished
"propagates" from "accepts". It read well and changed nothing: no caller could
act on the difference. Two booleans is what the readers actually use.

---

## `WordContract.effect`

**Definition.** `Pure` or `Dictionary`.

**Produced by.** The registry. Only `DEF` and `DEL` are `Dictionary`.

**Read by.** The specification's argument that a blocked vent is
observationally identical to a unit that was never written — which holds only
because everything else is pure; a test asserts the set is exactly `{DEF, DEL}`.

**Effect.** Documentation and that invariant.

---

## `Word.package`

**Definition.** Which package owns the word.

**Produced by.** Registration.

**Read by.** The manifest; the duplicate-name check that makes registration fail
rather than shadow.

**Effect.** Registration, and the ability to say what is Ajisai Core and what is
not.

---

## What is deliberately absent

There is no field on any word or value recording a stability tier, a portability
profile, an exploratory classification, a confidence level, a capability set, a
resource linearity, a complexity class, a determinism claim, a backend
suitability, a content-addressed identity, a provenance chain, a safety level, a
recoverability class, or a conserved quantity.

Nothing read any of them.
