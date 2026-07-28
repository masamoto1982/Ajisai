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

**Effect.** Rendering; the role words; and admissibility in the two
role-sensitive words, `DEF` and `DEL` (`SPECIFICATION.md` §6.3). Never a Data
Plane *result* — a role decides whether an operand is acceptable, never what is
computed from it.

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

## `WordContract.stak`

**Definition.** What `STAK` means for this word: map across every cell of the
flow, fold left across the whole flow, or nothing.

**Produced by.** The registry, per word.

**Read by.** The operand layer, which does exactly what the declaration says;
the package-registration check, which refuses a declaration the word cannot
support; the manifest.

**Effect.** Execution, directly.

**Invariants.** `MapEach` requires one input. `FoldLeft` requires a **closed**
operation — two in, one out, and an output type identical to the first input
type — so that each result is a legitimate operand for the next step. Only an
operand-to-result word can carry either. All three are held by tests, and by
the registration check for package words.

**Why this is a field and not a rule.** It used to be derived from arity: two
in and one out meant foldable. That is the same error as Flow Mass
Conservation — deriving a meaning from a count of operands — and it made
`1 1 1 STAK EQ` compute `EQ(EQ(1, 1), 1)` and answer `FALSE` about three equal
values.

---

## `WordContract.role_required`

**Definition.** Which operand a word reads a role from, and which role. `None`
for every word that does not read one.

**Produced by.** The registry. Only `DEF` and `DEL` declare it.

**Read by.** The words themselves, to refuse an inadmissible name; the lint,
which distinguishes text from a bare vector for exactly this reason; the
manifest; a test that asserts the set of role-sensitive words is `{DEF, DEL}`.

**Effect.** Execution — a name that is not read as text is refused — and
diagnostics.

**Invariants.** The named operand exists, and its declared type is the matching
`TypeSpec`.

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

**`WordContract.effect` was one of them, and it is gone.** It claimed `Pure` or
`Dictionary`, and only `DEF` and `DEL` were `Dictionary` — but `EXEC`, `MAP`,
`FILTER`, and `FOLD` all run a quote, and `{ { 1 } "X" DEF } EXEC` changes the
dictionary while `EXEC` declared itself pure. Making it accurate would have
meant effect polymorphism through quotes, for a field nothing acted on. The
specification's one use of it was an argument that a blocked vent is safe
because the unit is pure; the real reason is better, and is now stated
directly: **a blocked unit is not evaluated at all**, so its effects never
arise whatever they would have been.
