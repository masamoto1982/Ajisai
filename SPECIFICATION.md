# The Ajisai Language Specification

**Version 1.0. This document is canonical.**

Ajisai is a concatenative language in which exact values and vectors flow left
to right, and are observed, branched, and held at the points the program names.

This document defines the language: its lexis, its syntax, its values, its
evaluation model, its vocabulary, and what it takes to conform. It does not
describe any user interface, any package, any repository convention, or any
implementation technique. Those live elsewhere and none of them may be depended
on from here.

---

## 1. The water

Ajisai is described in terms of water, and the metaphor is load bearing: every
figure of speech in this document corresponds to a rule stated in it. Where the
two could disagree, the rule governs.

| Figure | Rule |
|---|---|
| the flow | evaluation order — values move left to right (§5) |
| the basin | the stack: the flow's current cross-section (§5.1) |
| `TOP` / `STAK` | where the next word draws from (§8) |
| `EAT` / `KEEP` | whether the next word swallows what it drew (§8) |
| `VENT` | release or block the next source unit (§9) |
| `UNKNOWN` | the flow reached the gauge and did not settle (§7) |
| `NIL` | the flow arrived carrying no value (§4.3) |
| an error | the flow never formed (§10) |

There is no word in Ajisai whose only purpose is to sound like water. A word
either has an execution rule or it is not in the language.

Every construct below is specified in the same order: **the execution rule
first, then the stack effect, then the type rule, then the error rule, and only
then the intuition.**

---

## 2. Lexis

Source is a sequence of Unicode characters. Tokens are separated by whitespace.

### 2.1 Comments

`#` begins a comment that runs to the end of the line. A comment is not part of
the program.

### 2.2 Numeric literals

```
12      -3      7/4     -1/2    2.5     -0.125
```

A numeric literal denotes an exact rational (§4.1). A decimal literal is exact:
`0.1` denotes `1/10`, not a binary approximation of one tenth. A token that
begins with a digit, or with `-` followed by a digit, and does not parse as a
number is an error — it is a malformed literal, never a word name.

### 2.3 Text literals

```
"hello"     "a\nb"      ""
```

A text literal denotes a vector of the Unicode scalar values of its characters,
carrying the `TEXT` role (§6). The escapes are `\n`, `\t`, `\"`, and `\\`; any
other escape is an error. An unterminated literal is an error.

Text is therefore not a separate value shape. `"A"` and `[ 65 ]` hold the same
data and compare equal (§6.4).

### 2.4 Words

Any other token is a word.

**Normative — normalization.** Word names fold case in **ASCII only**: `add`,
`Add`, and `ADD` are one word, and `倍` is itself. No other normalization is
applied, and source is required to be in Unicode Normalization Form C; two
names that differ only in decomposition are two names.

Full Unicode case conversion is deliberately not used. It is defined against a
particular Unicode version, is locale-sensitive in places, and can change a
string's length — so it would make word identity depend on which Unicode table
an implementation was built with, and two conforming implementations could
disagree about whether two names are the same word. ASCII case mapping has been
fixed forever.

### 2.5 Symbol aliases

Twelve tokens are aliases for canonical words. **They are a first-class surface
of the language, not a compatibility syntax and not a deprecated spelling.**

| Symbol | Word | Symbol | Word |
|---|---|---|---|
| `+` | `ADD` | `.` | `TOP` |
| `-` | `SUB` | `:` | `STAK` |
| `*` | `MUL` | `!` | `EAT` |
| `/` | `DIV` | `&` | `KEEP` |
| `=` | `EQ` | `^` | `VENT` |
| `<` | `LT` | | |
| `>` | `GT` | | |

An alias matches a whole token exactly. `-` is `SUB`; `-3` is a literal; `>TEXT`
is its own word.

**Normative:** an alias is normalized to its canonical word during parsing.
Every subsequent layer — evaluation, contracts, errors, formatting, linting —
sees only the canonical word. A program written in symbols and the same program
written in words are the same program, produce the same results, and produce
identical diagnostics. There is exactly one alias table.

### 2.6 Delimiters

`[` `]` `{` `}` are tokens and must be whitespace-separated.

---

## 3. Syntax

A program is a sequence of **source units**. There are exactly four:

| Unit | Written | Meaning |
|---|---|---|
| literal | `42`, `"hi"` | §5.2 |
| basin | `[ … ]` | §5.3 |
| quote | `{ … }` | §5.4 |
| word | `ADD`, `+` | §5.5 |

"Source unit" is defined once, here, and `VENT` (§9) uses this definition with
no looser variant.

An unbalanced `[`, `]`, `{`, `}`, or `"` is an error.

---

## 4. Values

A value has two planes: what it **is** (the Data Plane) and how it is **read**
(the Semantic Plane, §6).

The Data Plane has exactly six shapes.

### 4.1 Number

An exact rational, held as an arbitrary-precision numerator and denominator,
always reduced, with a positive denominator.

**There is no floating point in Ajisai.** No value approximates, no operation
rounds, and no result depends on how far a computation was carried. Division by
zero is an error (§10), not an infinity or a not-a-number.

Rendering: `3`, `3/4`, `-1/2`. An integer renders without a denominator.

### 4.2 Boolean

`TRUE` or `FALSE`. A Boolean is not a number: `TRUE` is not `1`.

### 4.3 Nil

The flow arrived carrying no value. Written `NIL`.

### 4.4 Unknown

The third truth value of Strong Kleene logic (§7): the flow reached the gauge
and the observation does not settle the question. Written `UNKNOWN`.

`NIL` and `UNKNOWN` are distinct values with distinct propagation rules
(§4.6). Neither is convertible to the other and neither is an error.

### 4.5 Vector

An ordered run of values. Vectors nest: a matrix is a vector of vectors, and
Ajisai defines no separate rank-aware type and no shape metadata.

Rendering: `[ 1 2 3 ]`, `[ ]`, `[ [ 1 2 ] [ 3 4 ] ]`.

### 4.6 Quote

An unevaluated flow held as a value. Rendering: `{ 1 2 ADD }`, `{ }`.

### 4.7 Absence and indeterminacy: the propagation rules

These two rules are separate, and they are stated separately because the
distinction is the point.

| Position | `NIL` operand | `UNKNOWN` operand | both |
|---|---|---|---|
| arithmetic (§11.2) | result is `NIL` | result is `UNKNOWN` | result is `NIL` |
| comparison (§11.3) | result is `UNKNOWN` | result is `UNKNOWN` | result is `UNKNOWN` |
| logic (§11.4) | **error** | K3 table (§7) | **error** |
| observation predicates (§11.5) | settles | settles | settles |
| vector words (§11.7) | **error** | **error** | **error** |
| `VENT` gate (§9) | **error** | blocks and marks | **error** |

**Conflict rule (arithmetic):** where absence and indeterminacy meet, absence
wins. Once an operand turns out not to exist there is nothing left to be
indeterminate about.

**Normative:** `NIL` in a logical position is an error, not a third reading of
falsity. This is what stops K3 collapsing back into two-valued logic.

---

## 5. Evaluation

### 5.1 The flow

Execution maintains one stack — **the flow**. Source units are evaluated left to
right. The flow persists between fragments submitted to one session.

### 5.2 A literal flows onto the stack

Execution: push the value. Stack effect: `( -- value )`. Errors: none.

### 5.3 A basin collects the flow it encloses

Execution: run the body on a **fresh, empty flow**; when it finishes, whatever
stands in that flow becomes a vector, in order, which is pushed onto the
enclosing flow.

Stack effect: `( -- vector )`. Errors: any error inside the body; the enclosing
flow is unchanged.

A vector literal is not a special case of the parser. `[ 1 2 3 ]` and
`[ 1 2 ADD ]` are the same construct.

> The basin is a pool the flow is diverted into; what stands in it when the
> diversion ends is what you carry away.

### 5.4 A quote is a value

Execution: push the quote. Stack effect: `( -- quote )`. Errors: none.

The body is not evaluated. `EXEC`, `MAP`, `FILTER`, `FOLD`, and `VENT` (§9)
evaluate quotes.

### 5.5 A word is invoked

Execution: resolve the name — first the dictionary of user definitions, then the
registered vocabulary — and invoke it under the armed mode (§8).

Errors: a name in neither is an error (§10).

### 5.6 Bodies and scopes

A **body** is a program, a basin body, a quote body, or a user definition's
body. Each body:

1. saves the armed mode and starts at the default mode `TOP EAT`;
2. runs its units;
3. is an error if a mode is still armed at the end (§8.5);
4. restores the saved mode.

Nesting of bodies is bounded by an implementation-defined budget; exceeding it
is an error (§10). The budget is not semantics: reaching it never truncates a
computation silently.

### 5.7 Word-level atomicity

**Normative:** a word that fails leaves the flow exactly as it found it. This
holds for every word, whatever it needs to do internally — a word that draws
its own operands before it can check them still leaves nothing half-consumed
behind, and a released `VENT` whose unit fails returns the gate as well.

This is a per-word guarantee, not a per-program one: when a program fails, the
flow holds whatever the words that completed left there.

**Normative — what it covers.** The guarantee is about the flow, and about the
flow only. The dictionary is not rolled back: a quote that defines a word and
then fails leaves the definition behind. Saying so is better than a promise
that would require snapshotting the whole dictionary at every step.

---

## 6. The Semantic Plane

### 6.1 What it is

Every value carries a **role**: how the value is read, as distinct from what it
is. There are exactly three roles.

| Role | Admits | Generated by | Renders as |
|---|---|---|---|
| `RAW` | anything | every numeric and vector literal, every computed result, `>RAW` | structurally |
| `TEXT` | a vector whose every element is a Unicode scalar value | text literals, `>TEXT` | `"hi"` |
| `INTERVAL` | a two-element vector `[ lo hi ]` of numbers with `lo <= hi` | `>INTERVAL` | `1..3` |

There is no role reserved for future use, and no role without a generator, a
consumer, and a propagation rule.

### 6.2 Where it lives

**Normative: a value's role is stored on the value, and nowhere else.** The
flow does not keep a parallel array of roles; no registry maps positions to
roles; no side table needs to be kept in sync. A role travels with its value
into and out of vectors, quotes, basins, and the dictionary, as a consequence of
where it is stored rather than as a rule that has to be maintained.

### 6.3 What it affects

**Normative.** Semantic information affects exactly three things:

1. **Rendering.** `[ 104 105 ]` and `"hi"` render differently.
2. **The role words** `ROLE`, `>TEXT`, `>INTERVAL`, `>RAW` (§11.8).
3. **The role-sensitive words**, which are `DEF` and `DEL` and no others
   (§11.9). A name must be read as `TEXT`.

It does not affect arithmetic, comparison, logic, indexing, length, or any other
computation over the Data Plane. **No word computes a different value because
of a role**; the role-sensitive words consult a role to decide whether the
operand is admissible at all, and then compute from the Data Plane as usual.

A word is role-sensitive only if this specification says it is, and each such
word declares which operand and which role in its contract, so the set is
enumerable rather than a matter of reading implementations.

**Why there is a role-sensitive word at all.** `[ 68 79 85 ]` is a vector of
three numbers that happens to spell `DOU`. If it could be a word's name, then
the reading a program asserts about its own data would count for nothing at
precisely the point a language most needs a name to be a name — and the
Semantic Plane would be decoration. One word's worth of reach, stated and
enumerable, is the honest position: not "roles never touch computation" when
they do, and not "roles are pervasive" when they are not.

### 6.4 Equality

**Normative:** `EQ` and `NE` compare Data Planes only. `"A"` and `[ 65 ]` are
equal. A role cannot be smuggled into a computation through equality.

### 6.5 Well-formedness, assertion, and propagation

One rule governs all three, stated once:

> A role is **admitted** by a value when the value's shape satisfies the role's
> condition in the table in §6.1.

* **Assertion** (`>TEXT`, `>INTERVAL`, `>RAW`): if the role is admitted, the
  value takes it; otherwise it is an error. A role never survives onto a value
  whose shape contradicts it.
* **Propagation** (`REST`, `REVERSE`, `APPEND`, `CONCAT`, `MAP`, `FILTER`): the
  result takes its source's role if the result admits it, and `RAW` if it does
  not. `REST` of a text is a text; `REST` of an interval is raw.
* **Joining** (`CONCAT`): two containers agree on a role only if they had the
  same one; otherwise the result is `RAW`.

### 6.6 Where a role is lost

A role is lost when — and only when — the result no longer admits it (§6.5),
when `>RAW` is applied, when a value is rebuilt by a word that declares a `RAW`
output (arithmetic, comparison, logic, `LENGTH`, `RANGE`), when a basin collects
a flow, or when `STAK` gathers one.

### 6.7 Conflict

The only conflict a role can have is with its own value's shape, and §6.5
resolves it: the shape governs, and the role drops to `RAW` or the assertion
fails. Two roles never compete for one value, because a value has exactly one.

---

## 7. K3: Strong Kleene three-valued logic

Ajisai's logic has three values: `TRUE`, `FALSE`, and `UNKNOWN`. `UNKNOWN` is a
truth value in its own right, not a placeholder awaiting resolution.

### 7.1 The tables

These are normative and complete.

**NOT**

| a | `NOT a` |
|---|---|
| `TRUE` | `FALSE` |
| `FALSE` | `TRUE` |
| `UNKNOWN` | `UNKNOWN` |

**AND** — the minimum under `FALSE < UNKNOWN < TRUE`

| a `AND` b | `TRUE` | `FALSE` | `UNKNOWN` |
|---|---|---|---|
| **`TRUE`** | `TRUE` | `FALSE` | `UNKNOWN` |
| **`FALSE`** | `FALSE` | `FALSE` | `FALSE` |
| **`UNKNOWN`** | `UNKNOWN` | `FALSE` | `UNKNOWN` |

**OR** — the maximum under `FALSE < UNKNOWN < TRUE`

| a `OR` b | `TRUE` | `FALSE` | `UNKNOWN` |
|---|---|---|---|
| **`TRUE`** | `TRUE` | `TRUE` | `TRUE` |
| **`FALSE`** | `TRUE` | `FALSE` | `UNKNOWN` |
| **`UNKNOWN`** | `TRUE` | `UNKNOWN` | `UNKNOWN` |

`UNKNOWN AND FALSE` is `FALSE` because whatever the unknown side turns out to
be, the conjunction is false. It is not `FALSE` because `UNKNOWN` was read as
falsity.

`p OR NOT p` is not a tautology in K3. That is the point of having a third
value.

### 7.2 The canonical sources of UNKNOWN

**Normative: `UNKNOWN` is reachable from source.** A K3 logic whose third value
could not be produced would not be a three-valued logic. The canonical
generating paths are:

1. **The word `UNKNOWN`.**
2. **A comparison whose operand is `NIL`** — there is no value to compare, so
   the question cannot be settled: `NIL 1 LT` is `UNKNOWN`, and so is
   `NIL NIL EQ`. Two absences are not evidence of sameness; ask `NIL?` when you
   want to observe absence itself, which is a question observation *can* settle.
3. **A comparison whose operand is `UNKNOWN`.**
4. **Propagation** through arithmetic (§4.7) and through the K3 tables.
5. **A vent blocked by an undetermined gate** (§9.3).

### 7.3 What UNKNOWN is not

**Normative.**

* `UNKNOWN` is never implicitly converted to a Boolean. `UNKNOWN BOOLEAN?` is
  `FALSE`.
* `UNKNOWN` is never treated as `FALSE`. Where a word must decide and the
  answer is `UNKNOWN`, the word raises an error rather than choosing a side —
  see `FILTER` (§11.6) and `VENT` (§9.3).
* `UNKNOWN` is not `NIL` (§4.7) and not an error (§10).

---

## 8. Flow modes: `TOP`/`STAK` × `EAT`/`KEEP`

A mode changes how the next word is fed and what it leaves behind. A mode is
not sugar for a stack shuffle, and the four words below are not substitutes for
`DUP` or `MAP`.

### 8.1 The two axes

The axes are independent and may be armed in either order.

**Selection** — where the next word draws from:

* `TOP` (`.`) — the surface. The word takes exactly the operands its stack
  effect declares. *This is the default.*
* `STAK` (`:`) — the whole standing flow.

**Retention** — what happens to what it drew:

* `EAT` (`!`) — the operands are consumed. *This is the default.*
* `KEEP` (`&`) — the operands stay standing, and the results are laid above
  them, so the flow branches instead of being swallowed.

### 8.2 Scope

**Normative:** arming a mode applies it to the **next word invocation** and to
that alone; the mode returns to `TOP EAT` afterwards.

Literals, basins, and quotes do not consume an armed mode. A mode is a
statement about the next *word*, and `KEEP [ 1 2 ] LENGTH` means what it reads.

Modes compose rather than override: `STAK KEEP ADD` arms both axes for the one
`ADD`, and `KEEP STAK ADD` is the same program.

### 8.3 What `STAK` means for each word

**Normative: what `STAK` means for a word is declared by the word, and is not
derived from how many operands the word takes.** Each word declares one of:

| Declaration | Reading | Requires |
|---|---|---|
| **map-each** | the word is applied to **every cell** of the flow, in order; the results are concatenated | exactly one input |
| **fold-left** | the word is **folded left** across the whole flow | a **closed** operation: two in, one out, and an output type identical to the first input type |
| **unsupported** | `ModeUnsupported` | — |

`1 2 3 STAK ADD` is `6`. `1 -2 3 STAK NEG` is `-1 2 -3`.
`TRUE FALSE UNKNOWN STAK AND` is `FALSE`. `[ 1 ] [ 2 ] [ 3 ] STAK CONCAT` is
`[ 1 2 3 ]`. `1 2 STAK SWAP`, `1 1 1 STAK EQ`, and `[ 1 2 ] 0 3 STAK NTH` are
all errors.

**Why closure is required.** A fold feeds each result back in as the next
step's first operand, so the operation has to accept its own output. Arity
alone does not establish that. `EQ` takes two values and leaves one, and
folding it computes `EQ(EQ(1, 1), 1)` — which is `EQ(TRUE, 1)`, and answers
`FALSE` about three equal values. Deriving a meaning from a count of operands
is the mistake this language removed when it deleted Flow Mass Conservation
(`docs/migration.md`), and it must not be reintroduced under another name.

A fold over a flow of one value yields that value, which is type-correct
precisely because the operation is closed. A fold over an empty flow is an
error.

### 8.4 Which words accept which modes

**Normative.**

* **`KEEP` applies to every word that declares a fixed stack effect.** The
  operands are remembered, the word runs, and they are laid back underneath
  whatever it produced. How the word is implemented does not enter into it.
* **`STAK` applies where the word declares a reading** (§8.3).
* **A word whose stack effect is not statically known rejects any non-default
  mode** with `ModeUnsupported`. In Ajisai Core that is `EXEC`, whose effect
  depends on the quote it is handed, and every user definition. There is no
  operand region for the mode to select, and silently ignoring the mode would
  be worse than refusing it.

A word's stack effect is a fact about the language. How an implementation
dispatches it is not, and must not decide which modes the word accepts.

### 8.5 Boundaries and errors

* **Quote boundary:** §5.6. The surrounding mode is saved, the body starts at
  the default, and the surrounding mode is restored on the way out.
* **Dangling mode:** a mode armed with no word to consume it before the end of
  a body is an error, raised at the end of the body in which it was armed.
  Ajisai does not carry source positions, so a diagnostic names the mode, not
  the line; an implementation that tracks positions may say more.
* **After an error:** the mode returns to `TOP EAT`. Nothing is inherited across
  a failure.

### 8.6 One implementation

**Normative:** an implementation applies modes in one common operand layer —
selecting operands, invoking the word, committing the result — rather than
branching per word. Individual words implement their operation only.

---

## 9. `VENT` and `^`

`VENT` releases or blocks the next source unit. It is the only construct in
Ajisai that decides whether something is evaluated at all.

### 9.1 Execution rule

1. Draw one value off the surface of the flow: the **gate**.
2. Read the gate as a truth value. Anything else, `NIL` included, is an error
   (§10), and the flow is left as it was found (§5.7).
3. Determine the unit that follows (§9.2).
4. Act on the gate (§9.3).
5. If the mode is `KEEP`, return the gate to the surface (§9.4).

Stack effect: `( truth -- )`, plus whatever the released unit does.

### 9.2 The unit

The unit is one source unit (§3), with two rules:

* **Modes attach to the word they govern.** Any run of mode words immediately
  following `VENT` is part of the unit, so `^ STAK ADD` is one unit and a
  blocked vent never leaves a mode armed with nothing to consume it.
* **A nested `VENT` carries its own unit.** `^ ^ X` is one unit.

If no unit follows, it is an error.

### 9.3 Acting on the gate

| Gate | Effect |
|---|---|
| `TRUE` | the unit is evaluated |
| `FALSE` | the unit is not evaluated and nothing is pushed |
| `UNKNOWN` | the unit is not evaluated and one `UNKNOWN` is pushed |
| anything else | error |

**A unit that is a single quote is entered, not pushed.** The flow goes through
the quoted channel. This is what makes `VENT` the branching construct; Ajisai
has no `IF`.

**Normative — laziness.** A blocked unit is not evaluated. It cannot divide by
zero, cannot name a word that does not exist, and cannot change the dictionary.
`FALSE ^ { 1 0 DIV }` is not an error.

**Why an undetermined gate leaves a mark.** Blocking silently would make a vent
that could not decide indistinguishable, in the flow, from one that decided not
to open — which is the operational form of reading `UNKNOWN` as `FALSE`, and
§7.3 forbids it. The pushed `UNKNOWN` records that what would have been released
is undetermined.

### 9.4 Interaction with the modes

* `STAK VENT` is an error: a gate is one value and a whole flow is not a reading
  of it.
* `KEEP VENT` draws the gate off the flow, runs the unit against the flow
  beneath it, and then returns the gate to the surface. **The gate therefore
  ends above whatever the unit released.**

That ordering gives two-branch selection out of two orthogonal features, with
no third feature added for it:

```
5 0 GT KEEP VENT { "positive" } NOT VENT { "not positive" }
```

The first vent observes the gate without swallowing it and runs the first
branch; `NOT` flips the returned gate; the second vent consumes it and runs the
other branch. When the gate is `UNKNOWN`, neither branch runs and the flow
records that twice.

---

## 10. Errors

An error is the third negative outcome and it is not a value.

* `NIL` — the flow arrived, and carried no value.
* `UNKNOWN` — the flow arrived, and observing it does not settle the question.
* **Error** — the flow never formed.

**Normative:** an error terminates the current execution. Nothing in Ajisai
converts an error into `NIL` or `UNKNOWN`, and Ajisai Core contains no word that
catches one.

The error conditions are: stack underflow, type mismatch, division by zero, an
unknown word, redefining a reserved word, an unsupported mode, a dangling mode,
a vent with no unit, a non-truth-value in a logical position, an index outside a
vector, a role a value does not admit, unbalanced source, a malformed token, an
undecided predicate, and the two budgets (nesting depth and vector size).

---

## 11. The vocabulary

Every word carries a machine-readable contract: name, stack effect, arity,
input and output types, its stance towards `NIL` and towards `UNKNOWN`, what
`STAK` means for it (§8.3), which operand it reads a role from if any (§6.3),
and a summary. See `docs/contracts.md`.

### 11.1 Constants

| Word | Effect | Meaning |
|---|---|---|
| `TRUE` | `( -- truth )` | the settled truth value `TRUE` |
| `FALSE` | `( -- truth )` | the settled truth value `FALSE` |
| `UNKNOWN` | `( -- truth )` | the third truth value |
| `NIL` | `( -- nil )` | the flow that carries no value |

### 11.2 Arithmetic

All exact (§4.1); absence and indeterminacy propagate per §4.7.

| Word | Alias | Effect |
|---|---|---|
| `ADD` | `+` | `( a b -- sum )` |
| `SUB` | `-` | `( a b -- difference )` |
| `MUL` | `*` | `( a b -- product )` |
| `DIV` | `/` | `( a b -- quotient )` — division by zero is an error |
| `MIN` | | `( a b -- lesser )` |
| `MAX` | | `( a b -- greater )` |
| `NEG` | | `( a -- negated )` |
| `ABS` | | `( a -- magnitude )` |

### 11.3 Comparison

`EQ` and `NE` accept any values and compare Data Planes (§6.4). The ordering
words require numbers. All five may produce `UNKNOWN` (§7.2).

| Word | Alias | Effect |
|---|---|---|
| `EQ` | `=` | `( a b -- truth )` |
| `NE` | | `( a b -- truth )` |
| `LT` | `<` | `( a b -- truth )` |
| `LE` | | `( a b -- truth )` |
| `GT` | `>` | `( a b -- truth )` |
| `GE` | | `( a b -- truth )` |

### 11.4 Logic

Per the K3 tables (§7.1). `NIL` in any operand position is an error.

| Word | Effect |
|---|---|
| `NOT` | `( a -- truth )` |
| `AND` | `( a b -- truth )` |
| `OR` | `( a b -- truth )` |

### 11.5 Observation predicates

Accept any value; always settle; never produce `UNKNOWN`.

`NIL?` `UNKNOWN?` `NUMBER?` `VECTOR?` `BOOLEAN?` `QUOTE?` — each
`( a -- truth )`. `UNKNOWN BOOLEAN?` is `FALSE`.

### 11.6 Flow shaping and quotes

| Word | Effect | Notes |
|---|---|---|
| `DUP` | `( a -- a a )` | the role travels with the value |
| `DROP` | `( a -- )` | |
| `SWAP` | `( a b -- b a )` | |
| `DEPTH` | `( -- count )` | |
| `MAP` | `( vector quote -- vector )` | |
| `FILTER` | `( vector quote -- vector )` | |
| `FOLD` | `( vector seed quote -- value )` | |
| `EXEC` | `( quote -- … )` | the one dynamic effect in Ajisai Core (§8.4) |

`MAP`, `FILTER`, and `FOLD` run the quote in a basin seeded with the operands,
so a quote cannot reach past its own operands into the surrounding flow. `EXEC`
deliberately runs against the current flow.

A quote handed to `MAP`, `FILTER`, or `FOLD` must leave exactly one value; any
other count is an error.

**`FILTER` and `UNKNOWN`.** A predicate that answers `UNKNOWN` is an error.
Keeping the element would read `UNKNOWN` as `TRUE` and dropping it would read
`UNKNOWN` as `FALSE`; §7.3 forbids both. Decide the predicate explicitly first.

`STAK` (§8) works across the standing flow; `MAP` works across a vector. They
are different axes and neither substitutes for the other.

### 11.7 Vectors

`NIL` and `UNKNOWN` are errors in the vector operand position.

| Word | Effect | Notes |
|---|---|---|
| `LENGTH` | `( vector -- count )` | |
| `NTH` | `( vector index -- element )` | zero-based; out of range is an error |
| `FIRST` | `( vector -- element )` | `NIL` when the vector is empty |
| `REST` | `( vector -- vector )` | role per §6.5 |
| `APPEND` | `( vector value -- vector )` | role per §6.5 |
| `CONCAT` | `( vector vector -- vector )` | role per §6.5 |
| `REVERSE` | `( vector -- vector )` | role per §6.5 |
| `RANGE` | `( from to -- vector )` | integers, `from` up to but excluding `to` |

`[ ] FIRST` is `NIL`, not an error: an empty vector's first element is a value
that is not there, which is exactly what `NIL` means. `[ 1 2 ] 9 NTH` is an
error: the rule did not hold.

### 11.8 The Semantic Plane

| Word | Effect | Notes |
|---|---|---|
| `>TEXT` | `( vector -- text )` | checked (§6.5) |
| `>INTERVAL` | `( vector -- interval )` | checked (§6.5) |
| `>RAW` | `( value -- value )` | forgets the reading |
| `ROLE` | `( value -- text )` | pair with `KEEP` to observe without eating |

### 11.9 The dictionary

| Word | Effect |
|---|---|
| `DEF` | `( quote name -- )` |
| `DEL` | `( name -- )` |

`{ 2 MUL } "DOUBLE" DEF` binds a quote to a name. There is no defining syntax
and no parser special case: a definition is two ordinary values and one ordinary
word.

**Normative — these are the role-sensitive words** (§6.3). The name operand
must carry the `TEXT` role. A bare vector of codepoints is refused, however it
would spell: write `"DOUBLE"`, or say `>TEXT` and mean it.

**Normative:** no word owned by Ajisai Core or by a registered package may be
redefined, under its canonical name or under an alias.

### 11.10 Directives

`TOP` (`.`), `STAK` (`:`), `EAT` (`!`), `KEEP` (`&`) — §8. `VENT` (`^`) — §9.

---

## 12. Ajisai Core

**Ajisai Core** is the semantics and vocabulary defined in this document, and
nothing else. The term has exactly one meaning.

The related terms, each used for one thing:

| Term | Meaning |
|---|---|
| **Ajisai Core** | the semantics and vocabulary required to be Ajisai — this document |
| **Package** | an external bundle of words a host may register (§13) |
| **Word Contract** | a word's machine-readable declaration (§11) |
| **Conformance Corpus** | the tests that fix conforming behaviour (§14) |

There is no minimal core, no core profile, no core word set, and no second
sense of the word "core".

---

## 13. Packages

A host may register a **package**: a bundle of words, each with a contract and
an implementation.

**Normative.**

* A package may add words. It may not add a value shape, a role, a mode, an
  error condition, or an execution path.
* A package may not take a name Ajisai Core, another registered package, or an
  existing user definition already answers to; registration fails rather than
  shadowing.
* **Registration is all or nothing.** Every word is checked first — for name
  collisions, for a canonical name, and for a contract that describes it — and
  only then is any of it committed. A rejected package leaves the vocabulary
  exactly as it was.
* Ajisai Core contains no knowledge that any package exists: no feature flag, no
  stub, no marker, no reserved namespace.
* An implementation that registers no package is complete.

**Normative — what a package is.** A package is a **vocabulary library over
Ajisai's existing values**, not a way to introduce a domain type. A package
that wants a "note" gets a two-element vector, and any two-element vector will
satisfy a word that expects one; the package can check the values but cannot
make the shape unforgeable, and cannot give it a reading of its own. This is a
deliberate limit — it is what keeps the extension surface to "words, and
nothing else" — and a package should say in its own documentation which value
conventions it expects.

Package words are ordinary words. The modes (§8), `VENT` (§9), the contract lint,
and the manifest all apply to them without the package opting in.

---

## 14. Conformance

An implementation conforms when it implements §§2–13 and passes the conformance
corpus.

**Normative — what conformance does not require.**

* No user interface. A headless implementation is fully conforming.
* No package. `ajisai-music` and `ajisai-audit` are not part of the language.
* No content addressing, lockfile, receipt, or attestation. These are not part
  of a word's identity or an execution's result.
* No particular execution strategy. **An implementation has one execution path;
  a conforming implementation may not offer a second one that a program can
  select.** Optimization is permitted inside the one path where it cannot be
  observed.

**Normative — the contract lint.** An implementation may provide a contract
lint. A lint reports obvious inconsistencies between declared stack effects and
types. It is not a verifier: it may not block execution, and it may not report
that a program will succeed. See `docs/contracts.md`.

---

## 15. Related documents

None of these is canonical for the language, and this document does not depend
on any of them.

| Document | Covers |
|---|---|
| `docs/semantic-plane.md` | the Semantic Plane in depth |
| `docs/semantic-ontology.md` | every retained semantic field, its producers and consumers |
| `docs/contracts.md` | the contract structure and the lint's guarantees |
| `docs/playground-ui.md` | the Presentation Profile — a user interface, not the language |
| `docs/migration.md` | what changed in 1.0 and how to move |
| `docs/implementation.md` | how the reference implementation is built |
