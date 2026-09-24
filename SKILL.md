<!-- GENERATED FILE — do not edit by hand.
     Regenerate: npm run generate:skill   (verified against the ajisai CLI)
     Source of truth for semantics: SPECIFICATION.html.
     Generator: scripts/generate-skill-md.mjs -->

# Ajisai — Agent Writing Protocol (SKILL.md)

How to *write working Ajisai on the first try*. Every code line below was
executed by the generator against the real interpreter; results shown are
actual outputs. **If a word is not in the §9 table, it does not exist — when
unsure, grep §9 before writing.**

## 1. Run loop

```sh
ajisai run program.ajisai --json     # exit 0 = ok, 1 = language error, 2 = usage
ajisai check program.ajisai --json   # parse + resolve only, no execution
```

Read the JSON in this order (contract: docs/dev/agent-cli-output-contract.md):
1. `status` / exit code. On ok: `stackDisplay` (final stack, bottom→top) and `output` (PRINT lines).
2. On error: `diagnosis.why` + `diagnosis.where` locate the failure; follow `diagnosis.nextChecks` in order; `aiDiagnostic.recoverability` says what kind of change fixes it (`fixProgram` / `fixInput` / `fixHost` ...).
3. Even on ok, scan `errorFlowTrace` for `nilProduced` events if a NIL surprised you.

## 2. Minimal syntax

- Postfix, stack-based. Operands first, word last: `[ 1 ] [ 2 ] +`.
- Numbers are **exact rationals** (`1/3`, `3.14` → 157/50). No floats. Display shows `3/1` for 3.
- Data lives in vectors: `[ 1 2 3 ]`. Vectors nest for ragged and grouped data. A lone number like `42` is allowed but `[ 42 ]` is the idiomatic scalar — **except where a Word takes an *element*** (`PUT`, `GET`, `INDEX-OF`): there `[ 9 ]` is the one-element vector itself, so writing it nests instead of storing 9, and nothing errors (§7).
- Strings: `'single quotes'` (a value domain of its own, not a vector of codepoints). Booleans: `TRUE` / `FALSE`. Absence: `NIL`.
- Code blocks are quoted programs passed to MAP / FILTER / FOLD / DEF, written as an ordinary Vector (§6) — there is no separate block bracket. SELECT is not among them: it takes values, not code.
- Named data is a Record, built by `RECORD` from a Vector of keys and a Vector of values: `[ 'x' 'y' ] [ 1 2 ] RECORD`. It is not a Vector and is never code. It displays as `{ 'x' 1/1 'y' 2/1 }`, which is a display, not source: only `[ ]` delimits.
- Define a user word with a body Vector, then a `'NAME'` string, then `DEF`, then call `NAME`: `[ [ 1 ] [ 2 ] + ] 'MY-SUM' DEF MY-SUM` (§6). Words are case-insensitive (canonicalized to upper case).
- Comments: `#` to end of line.
- Every Word consumes the operands it reads. To use a value more than once, name it with `BIND` and read the name: `5 'N' BIND N N 1 +` leaves `5 6`.
- One word does one thing to the stack; there are **no** DUP/SWAP-style shufflers (§8).

## 3. Control and iteration

- Branch: the two candidates, then the truth that chooses between them, then `SELECT`: `[ 'non-negative' ] [ 'negative' ] [ 4 ] [ 0 ] LT NOT SELECT PRINT` (§6). Both candidates are values the program already built, so neither is skipped and nothing is evaluated by SELECT itself. The choice is made lane by lane, so a Vector of truths branches a whole Vector at once: `[ 0 ] [ -3 5 -1 ] [ -3 5 -1 ] [ 0 ] LT SELECT`. An absent truth chooses neither and answers that same absence.
- Iterate data, not counters: `MAP` / `FILTER` / `FOLD` with block operands (examples in §6). `FOLD` requires an explicit initial-value Vector.
- No recursion: `DEF` refuses a word whose body names itself, directly or through other user words (a diagnosed error at definition time, not at the call). Repetition is expressed only through MAP / FILTER / FOLD / SCAN over an already-finite vector.

## 4. NIL — absence is a value, not an exception

Failed partial operations *bubble*: `1 0 DIV` succeeds (exit 0) and
pushes `NIL` (reason: `divisionByZero`). The projection is recorded in
`errorFlowTrace` as a `nilProduced` event with a full diagnosis, and the NIL
value itself carries `semantics.absence.reason` on the stack.

- Provide a fallback with `BIND`, `NIL?` and `SELECT`: `1 0 DIV 'S' BIND [ 99 ] S S NIL? SELECT` → stack `[ 99/1 ]`. `NIL?` consumes its subject like every Word and answers whether it was absent, which is exactly where `SELECT` wants the truth — so name the subject once and read it twice: the phrase reads "S, or the fallback if S is absent".
- Over a vector the projection is **per lane, not per value**: `[ 6 6 ] [ 1 0 ] DIV` → stack `[ 6/1 NIL ]`. The lane that could not divide is the only one emptied.
- That makes the top a vector, not a NIL, so `NIL?` — which asks about the whole value — answers FALSE and the fallback is not chosen. Recover a lifted result inside the vector, not around it.
- NIL flows through later operations (bubble rule); check for it where it matters instead of letting it propagate to the end.

## 5. Exactness — comparison decides over the algebraic field

Numbers are exact rationals, closed under `SQRT`. Arithmetic never rounds,
coefficients are arbitrary-precision, and **every comparison of two scalars
built from rationals and `SQRT` decides**: there is no budget, no refinement
limit, and no undecided outcome.

```ajisai
8 SQRT 2 SQRT 2 SQRT + =   # √8 vs √2+√2
```

→ stack `TRUE` (exit 0). Values built through different
histories are the same value when they denote the same real.

That field is the whole numeric domain. `POW` answers inside it — an integer
exponent, or `p/2` over a non-negative rational — and projects NIL for any
other exponent rather than leave it:

```ajisai
8 1/3 POW NIL-REASON
```

→ stack `'domainMiss'` (exit 0). Truth has three values: `TRUE`,
`FALSE`, and the logical UNKNOWN, which is what a NIL operand reads as in a
truth position (§4) — no comparison produces it of its own. An operation that
cannot produce a value produces NIL (§4); a malformed one raises an error.

## 6. Canonical examples (all verified by the generator)

- Push a number (always inside a vector)
  `[ 42 ]` → stack: `[ 42/1 ]`
- Exact rational division — no floats, ever
  `[ 1 ] [ 3 ] /` → stack: `[ 1/3 ]`
- Elementwise vector arithmetic
  `[ 1 2 3 ] [ 4 5 6 ] +` → stack: `[ 5/1 7/1 9/1 ]`
- Scalar broadcast over a vector
  `[ 5 ] [ 1 2 3 ] *` → stack: `[ 5/1 10/1 15/1 ]`
- Remainder: name the operands, then a - b * floor(a/b)
  `10 'A' BIND 3 'B' BIND A A B / FLOOR B * -` → stack: `1/1`
- Comparison pushes a boolean
  `1 2 <` → stack: `TRUE`
- Comparison lifts over vectors element-wise
  `[ 1 2 ] [ 3 1 ] <` → stack: `[ TRUE FALSE ]`
- Range: one vector [ start end ] (inclusive)
  `[ 0 5 ] RANGE` → stack: `[ 0/1 1/1 2/1 3/1 4/1 5/1 ]`
- Range with step: [ start end step ]
  `[ 0 10 2 ] RANGE` → stack: `[ 0/1 2/1 4/1 6/1 8/1 10/1 ]`
- Fill a tensor: [ shape... value ]
  `[ 2 2 7 ] FILL` → stack: `[ [ 7/1 7/1 ] [ 7/1 7/1 ] ]`
- MAP with a [ ] code block
  `[ 0 4 ] RANGE [ [ 2 ] * ] MAP` → stack: `[ [ 0/1 ] [ 2/1 ] [ 4/1 ] [ 6/1 ] [ 8/1 ] ]`
- FILTER keeps matching elements
  `[ 0 10 ] RANGE [ 5 > ] FILTER` → stack: `[ 6/1 7/1 8/1 9/1 10/1 ]`
- FOLD needs an explicit initial value
  `[ 1 2 3 ] [ 0 ] [ + ] FOLD` → stack: `[ 6/1 ]`
- A Record from a Vector of keys and a Vector of values
  `[ 'x' 'y' ] [ 1 2 ] RECORD` → stack: `{ 'x' 1/1 'y' 2/1 }`
- Define a user word: [ body ] then name, then DEF
  `[ [ 1 ] [ 2 ] + ] 'MY-SUM' DEF MY-SUM` → stack: `[ 3/1 ]`
- SELECT: the two candidates, then the truth that chooses between them
  `[ 'non-negative' ] [ 'negative' ] [ 4 ] [ 0 ] LT NOT SELECT PRINT` → prints `[ 'non-negative' ]`
- SELECT chooses lane by lane, so a whole vector branches at once
  `[ 0 ] [ -3 5 -1 ] [ -3 5 -1 ] [ 0 ] LT SELECT` → stack: `[ 0/1 5/1 0/1 ]`
- Strings are bare '...' literals; CHARS/JOIN convert
  `'hello' CHARS REVERSE JOIN` → stack: `'olleh'`
- Cast a string to an exact number
  `'42' NUM` → stack: `42/1`
- PRINT pops and emits to output (not the stack)
  `[ 1 2 3 ] PRINT` → prints `[ 1/1 2/1 3/1 ]`
- Sorting is a plain Core word
  `[ 3 1 2 ] SORT` → stack: `[ 1/1 2/1 3/1 ]`
- Exact square root takes a bare scalar
  `2 SQRT` → stack: `sqrt(2)`
- A value used twice is named with BIND
  `5 'N' BIND N N 1 +` → stack: `5/1  6/1`

## 7. Common errors — actual CLI output, and the fix

- **Typo / unknown word** — `[ 1 ] ADDD`
  → exit 1, `message: "Unknown word: ADDD"`, `diagnosis: { when: "resolveWord", why: "typoOrUnknownName" }`,
  `aiDiagnostic.recoverability: "fixProgram"`, first nextCheck code: `checkSpelling`. `diagnosis.candidates: ["ADD","AND"]`.
  Fix: Grep §9 for the word you meant (here: `+` / `ADD`). Word names are upper-cased automatically.
- **Stack underflow: operands must be pushed first** — `+`
  → exit 1, `message: "Stack underflow"`, `diagnosis: { when: "executeWord", why: "stackShape" }`,
  `aiDiagnostic.recoverability: "fixProgram"`, first nextCheck code: `checkDeclaredArity`.
  Fix: Push both operands before the operator: `[ 1 ] [ 2 ] +`. Ajisai is postfix; there is no infix form.
- **FOLD without an initial value** — `[ 1 2 3 ] [ + ] FOLD`
  → exit 1, `message: "Stack underflow"`, `diagnosis: { when: "executeWord", why: "stackShape" }`,
  `aiDiagnostic.recoverability: "fixProgram"`, first nextCheck code: `checkDeclaredArity`.
  Fix: FOLD is `vector [ init ] [ op ] FOLD`: `[ 1 2 3 ] [ 0 ] [ + ] FOLD`.
- **SELECT takes three operands: both candidates, then the truth** — `[ 'big' ] [ 5 ] [ 3 ] GT SELECT`
  → exit 1, `message: "Stack underflow"`, `diagnosis: { when: "executeWord", why: "stackShape" }`,
  `aiDiagnostic.recoverability: "fixProgram"`, first nextCheck code: `checkDeclaredArity`.
  Fix: SELECT is `[ whenTrue ] [ whenFalse ] [ mask ] SELECT` — push both candidates before the test that chooses between them: `[ 'big' ] [ 'small' ] [ 5 ] [ 3 ] GT SELECT`. It chooses between values, never running either one, so an effect goes after it: `... SELECT PRINT`.
- **SELECT needs a truth value, not a number** — `[ 'y' ] [ 'n' ] 1 SELECT`
  → exit 1, `message: "SELECT: expected a truth value, got Scalar"`, `diagnosis: { when: "executeWord", why: "valueShape" }`,
  `aiDiagnostic.recoverability: "fixInput"`, first nextCheck code: `checkFiredCondition`.
  Fix: The third operand must be TRUE, FALSE or an absence — a scalar is not a truth value (§4). Write the test: `[ 1 ] [ 0 ] EQ NOT`.
- **Broadcast shape mismatch** — `[ 1 2 ] [ 1 2 3 ] +`
  → exit 1, `message: "Cannot broadcast shapes [2] and [3]: axis 0 is 2 on the left and 3 on the right, and neither is 1"`, `diagnosis: { when: "executeWord", why: "shapeMismatch" }`,
  `aiDiagnostic.recoverability: "fixInput"`, first nextCheck code: `checkDisagreeingAxis`.
  Fix: Elementwise ops need equal or broadcastable shapes (scalar `[ 5 ]` broadcasts; `[2]` vs `[3]` does not).
- **NUM casts strings, not booleans** — `TRUE NUM`
  → exit 1, `message: "NUM: expected a String, got Boolean"`, `diagnosis: { when: "executeWord", why: "valueShape" }`,
  `aiDiagnostic.recoverability: "fixInput"`, first nextCheck code: `checkFiredCondition`.
  Fix: NUM accepts strings: `'42' NUM`. There is no boolean→number cast.
- **Old two-vector RANGE form** — `[ 0 ] [ 5 ] RANGE`
  → exit 1, `message: "RANGE: expected [ start end ] or [ start end step ], got 1 element(s)"`, `diagnosis: { when: "executeWord", why: "valueShape" }`,
  `aiDiagnostic.recoverability: "fixInput"`, first nextCheck code: `checkFiredCondition`.
  Fix: RANGE takes one vector: `[ 0 5 ] RANGE` (or `[ start end step ]`).

These raise. The next one does not — it succeeds and answers something other
than it looks like it answers, which is the harder kind to notice:

- **A one-element vector where a Word wants an element** — both of these succeed (exit 0):
  `[ 1 2 3 ] [ 1 ] [ 9 ] PUT` → stack `[ [ 1/1 [ 9/1 ] 3/1 ] ]`
  `[ 1 2 3 ] 1 9 PUT` → stack `[ 1/1 9/1 3/1 ]`
  Fix: PUT, GET and INDEX-OF take an *element*, not a one-element vector holding it: `[ 9 ]` is that vector, so it is stored as one. The `[ 42 ]` idiom of §2 is for operands a Word reads as a value; it does not carry here, and no error says so.

## 8. Forbidden patterns (each verified to fail)

- **DUP / SWAP / DROP / OVER / ROT** (`DUP` fails) — Forth-style stack shufflers do not exist. Every Word consumes the operands it reads; name a value with `BIND` to use it more than once.
- **IF / ELSE / THEN / WHILE** (`[ 1 ] IF` fails) — No structured keywords, and no loops. Branch with SELECT over two values; iterate with MAP / FILTER / FOLD / SCAN.
- **A word calling itself** (`[ REC ] 'REC' DEF` fails) — The User dictionary is acyclic: `DEF` refuses a body that names the word being defined, directly or through other user words, so this fails at definition time rather than the call. Repetition is expressed only through MAP / FILTER / FOLD / SCAN over an already-finite vector.
- **Parentheses ( )** (`( 1 2 )` fails) — Reserved; not valid in source. `[ ]` is the sole bracket, for vectors and code alike.
- **Double-quoted strings** (`"hello" PRINT` fails) — Strings use single quotes: 'hello'.
- **// line comments** (`// comment` fails) — Comments start with `#`.

## 9. Word quick reference

Generated from `docs/word-manifest.json` — the complete inventory:
80 canonical Words in one flat Core dictionary, of which
49 form the Semantic Kernel and 31 are Standard Words. Both are
ordinary Core Words called by their plain names; the split is a design
classification, not a namespace. A word absent here does not exist. There is
no module system and nothing to import.

| word | category | summary |
|---|---|---|
| `TRUE` | constant | Push the boolean TRUE onto the stack. — e.g. `TRUE` |
| `FALSE` | constant | Push the boolean FALSE onto the stack. — e.g. `FALSE` |
| `AND` | logic | Logical AND. FALSE absorbs a NIL operand into FALSE; otherwise a NIL operand yields UNKNOWN. — e.g. `TRUE TRUE &` |
| `NOT` | logic | Logical negation. TRUE and FALSE invert; a NIL operand (UNKNOWN) passes through unchanged. — e.g. `TRUE NOT` |
| `SELECT` | logic | Choose between two already-computed values by a truth value: TRUE answers the first, FALSE answers the second. The choice is element-wise (LANG.COLLECTIONS.LIFT), so a Vector of truths weaves two Vectors lane by lane and a one-lane operand is reused across the other's length. An UNKNOWN lane — a NIL read in truth position, whatever its reason — chooses neither and answers that same absence, so the reason survives the choice. Both operands are values the program already built: SELECT evaluates nothing, and whatever computed them ran before it, exactly once. — e.g. `[ 'yes' ] [ 'no' ] TRUE SELECT` |
| `EQ` | comparison | Test equality of two values. — e.g. `1 1 =` |
| `LT` | comparison | Test less-than comparison. — e.g. `1 2 <` |
| `GT` | comparison | Test greater-than comparison. — e.g. `2 1 >` |
| `ADD` | arithmetic | Add two numeric values, element-wise with broadcasting. — e.g. `1 2 +` |
| `SUB` | arithmetic | Subtract two numeric values, element-wise with broadcasting. — e.g. `5 3 -` |
| `MUL` | arithmetic | Multiply two numeric values, element-wise with broadcasting. — e.g. `2 4 *` |
| `DIV` | arithmetic | Divide two numeric values exactly (fractional result). — e.g. `10 2 /` |
| `FLOOR` | arithmetic | Round toward negative infinity. — e.g. `[ 7/3 ] FLOOR` |
| `ROUND` | arithmetic | Round to nearest integer (half-up). — e.g. `[ 5/2 ] ROUND` |
| `MIN` | math | Smaller of two numbers, element-wise with broadcasting. — e.g. `1 2 MIN` |
| `MAX` | math | Larger of two numbers, element-wise with broadcasting. — e.g. `1 2 MAX` |
| `SQRT` | math | Exact square root of a non-negative rational, element-wise over a vector. — e.g. `2 SQRT` |
| `POW` | math | Exact power `x y POW`, element-wise over Vectors, answered inside the exact field. An integer exponent keeps the result in the base's own tier: `2 10 POW` is `1024`, `2 SQRT 2 POW` is `2`, `2 -1 POW` is `1/2`. An exponent `p/2` over a non-negative rational base stays in the field too — `2 1/2 POW` is exactly what `2 SQRT` answers, and `2 3/2 POW` is `2√2`. `0 y POW` with a negative `y` projects `divisionByZero`; a negative base under `p/2` has no real value and projects `domainMiss`; every other exponent — a denominator other than 1 or 2, `p/2` over an irrational base, an irrational exponent — leaves the field and projects `domainMiss` as well (`8 1/3 POW`, `2 2 SQRT POW`); an exponent past what the machine will materialize projects `spaceExhausted`. `SQRT` remains the Word that builds the field; `POW` is not its sugar. — e.g. `2 10 POW` |
| `GCD` | math | The greatest common divisor of two integers, non-negative, element-wise over Vectors: `12 18 GCD` is `6`, `0 0 GCD` is `0`. Euclid's algorithm is input-dependent repetition, which a definition cannot write in a language that repeats only over a Vector that already exists; the machine already runs it to keep every rational reduced, so the Word only exposes it. A non-integer operand — a fraction or an irrational — projects `domainMiss`. — e.g. `12 18 GCD` |
| `RATIO` | math | A rational opened into its reduced numerator and denominator, as a two-element Vector with the denominator positive: `6/4 RATIO` is `[ 3 2 ]`, `-3 RATIO` is `[ -3 1 ]`, element-wise over Vectors. The language advertises exact rationals; this is the Word that reads their two parts back, and because the answer is a Vector, arithmetic lifts over it as it does over any other. An irrational (`2 SQRT`) has no numerator and projects `domainMiss`. — e.g. `6/4 RATIO` |
| `GET` | vector | Select elements of a vector by index. An index with no element is not an error: `GET` answers what is there, and "nothing" is a complete answer, so an out-of-range index projects to NIL(indexOutOfBounds). The projection is per index — `[ 10 20 30 ] [ 0 9 ] GET` answers `[ 10/1 NIL ]`, keeping every index that did resolve. `TAKE` and `PUT` answer the same condition the same way, so past-the-end is one outcome across the whole vocabulary. — e.g. `[ 10 20 30 ] [ 0 2 ] GET` |
| `LENGTH` | vector | Return the number of elements in a vector. — e.g. `[ 1 2 3 ] LENGTH` |
| `TAKE` | vector | Take the first N or last -N elements of a vector. A count larger than the vector projects to NIL(indexOutOfBounds): asking for more than there is names a position past the end, which is the same question `GET` answers past the end and is answered the same way — well-formed data that did not work out, not a malformed program (LANG.FAILURE.PROJECT). So `[ 1 2 3 ] 9 TAKE` is NIL, and a caller who wants something else writes it: `[ 1 2 3 ] 9 TAKE 'S' BIND [ 1 2 3 ] S S NIL? SELECT` answers the whole vector instead. A count that is not an integer at all is still `invalidCount`, because that is the program being wrong. — e.g. `[ 1 2 3 4 5 ] 3 TAKE` |
| `DROP` | vector | Drop the first N or last -N elements of a vector and answer the rest. TAKE's counterpart: `[ 1 2 3 4 5 ] 2 DROP` is `[ 3 4 5 ]` and `[ 1 2 3 4 5 ] -2 DROP` is `[ 1 2 3 ]`, so `[ n ] TAKE` and `[ n ] DROP` split one vector into two halves that `CONCAT` joins back. A count larger than the vector projects to NIL(indexOutOfBounds), exactly as TAKE's does: it names a position past the end, which is well-formed data that did not work out (LANG.FAILURE.PROJECT). A count that is not an integer at all is still `invalidCount`, because that is the program being wrong. — e.g. `[ 1 2 3 4 5 ] 2 DROP` |
| `CONCAT` | vector | Flatten and concatenate two vectors. — e.g. `[ 1 2 ] [ 3 4 ] CONCAT` |
| `REVERSE` | vector | Reverse the order of vector elements. — e.g. `[ 1 2 3 ] REVERSE` |
| `COLLECT` | vector | Collect N items off the stack into a new vector. — e.g. `1 2 3 3 COLLECT` |
| `RANGE` | vector | Generate a numeric sequence from a [start, end] pair. — e.g. `[ 0 5 ] RANGE` |
| `FILL` | vector | Fill a target shape with a constant value. — e.g. `[ 2 2 0 ] FILL` |
| `SHAPE` | shape | The lengths of a rectangular vector's axes, outermost first: `[ [ 1 2 ] [ 3 4 ] ] SHAPE` is `[ 2 2 ]` and `[ 1 2 3 ] SHAPE` is `[ 3 ]`. This is the shape LANG.COLLECTIONS.LIFT already aligns operands by, made observable. A ragged vector has no shape, so it projects to NIL(domainMiss): `[ [ 1 2 ] [ 3 ] ] SHAPE` is a reasoned absence, not an error, because the vector is well-formed data that the question does not fit. LENGTH answers the outermost axis alone. — e.g. `[ [ 1 2 ] [ 3 4 ] ] SHAPE` |
| `RESHAPE` | shape | Regroup a vector's leaves, in order, under a new shape: `[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE` is `[ [ 1 2 3 ] [ 4 5 6 ] ]`, and `SHAPE RESHAPE` on a rectangular vector gives it back. The leaves are everything FLATTEN would answer, however deeply they were nested. The shape is a vector of positive integers whose product must equal the leaf count; any other shape is ERROR(invalidShape), because nothing is padded or repeated to make it fit. A well-formed shape too large to materialize projects to NIL(spaceExhausted), as RANGE and FILL do (LANG.COLLECTIONS.BUDGET). — e.g. `[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE` |
| `FLATTEN` | shape | Collapse every axis into one: `[ [ 1 [ 2 3 ] ] [ 4 ] ] FLATTEN` is `[ 1 2 3 4 ]`, the leaves in index order however deeply they were nested. CONCAT joins two vectors and flattens one level; FLATTEN takes one vector and flattens all of them. It cannot be written as a user definition: the depth is not known in advance, and a language with no recursion and no unbounded loop cannot walk a structure of unknown depth (LANG.DICTIONARY.ACYCLIC). — e.g. `[ [ 1 [ 2 3 ] ] [ 4 ] ] FLATTEN` |
| `DEPTH` | shape | How deeply a value nests: a leaf — a number, a text, a truth, a NIL — is 0, a flat vector is 1, and a vector is one more than its deepest element, so `[ 1 [ 2 [ 3 ] ] ] DEPTH` is `3` and `[ ] DEPTH` is `1`. Like FLATTEN it cannot be written as a user definition, because the very thing it measures is what a non-recursive program cannot walk. — e.g. `[ 1 [ 2 [ 3 ] ] ] DEPTH` |
| `SORT` | vector | Return a copy of a vector sorted in ascending order. — e.g. `[ 3 1 2 ] SORT` |
| `ORDER` | vector | The indices that would sort a vector ascending; ties keep their original order. — e.g. `[ 30 10 20 ] ORDER` |
| `UNIQUE` | vector | The distinct elements of a vector, in first-occurrence order. — e.g. `[ 'a' 'b' 'a' ] UNIQUE` |
| `TALLY` | record | How many times each distinct element occurs, as a Record from element to count: `[ 'b' 'a' 'b' ] TALLY` is `[ 'b' 'a' ] [ 2/1 1/1 ] RECORD`, keys in order of first appearance. `KEYS` is exactly what `UNIQUE` answers and `VALUES` is the aligned count Vector, so nothing the earlier Vector-of-counts form could do is lost, and the caller no longer has to call `UNIQUE` separately to learn what each count counts. Works for every value, not only numbers. A non-Vector operand is an ERROR. — e.g. `[ 'a' 'b' 'a' ] TALLY` |
| `ZIP` | vector | Bundle equal-length vectors position by position; a matrix transposes. — e.g. `[ [ 1 2 ] [ 3 4 ] ] ZIP` |
| `PUT` | vector | A copy of a vector with the element at one index replaced. An out-of-range index projects to NIL(indexOutOfBounds), exactly as it does for `GET`: a well-formed index over a well-formed vector that names no slot is data that did not work out, not a program that is wrong (LANG.FAILURE.PROJECT). `PUT` used to raise here, on the grounds that it answers with the whole vector and so has no single slot to empty — but what is absent is the *answer*, not a slot, and a reasoned NIL is how this language says an answer is absent. Nothing is lost by saying so: the vector the caller wanted preserved is the one they wrote, and `[ 1 2 3 ] 9 5 PUT 'S' BIND [ 1 2 3 ] S S NIL? SELECT` hands it back. — e.g. `[ 1 2 3 ] 1 9 PUT` |
| `GROUP` | record | Bundle values by the key at the same position, as a Record from key to the Vector of its values: `[ 1 2 3 ] [ 'a' 'b' 'a' ] GROUP` is `[ 'a' 'b' ] [ [ 1/1 3/1 ] [ 2/1 ] ] RECORD`, keys in order of first appearance and every value kept exactly once. The core of a per-class tally, a centroid update or a stratified partition; `R 'a' AT` then reads one group by name where the earlier Vector-of-Vectors form needed `UNIQUE` and `INDEX-OF` to find it. Both operands must be Vectors of the same length. — e.g. `[ 1 2 3 ] [ 'a' 'b' 'a' ] GROUP` |
| `INDEX-OF` | vector | Index of the first element equal to the value; Bubble/NIL if absent. — e.g. `[ 1 2 ] 2 INDEX-OF` |
| `MEMBER?` | vector | Whether the value occurs in the vector: `[ 1 2 3 ] 2 MEMBER?` is `TRUE`. Membership is value equality, the equality UNIQUE and INDEX-OF use, so it works on texts, nested vectors and NILs as well as numbers. The needle is one value, compared rather than read, so a Vector needle is looked for as an element: `[ [ 1 ] 2 ] [ 1 ] MEMBER?` is `TRUE`. It is `INDEX-OF NIL? NOT`. — e.g. `[ 1 2 3 ] 2 MEMBER?` |
| `BSEARCH` | vector | The index of each key in an ascending vector, found by halving: `[ 1 3 5 7 ] [ 5 ] BSEARCH` is `[ 2 ]`, a single key answers a single index, and a key that is not there is a NIL(missingField) lane. The vector must be in ascending order; one that is not raises `unsortedInput`, since a binary search over unordered data would answer something rather than nothing. Checking the order is one pass over the vector, and each key then costs O(log n), so m keys cost O(n + m log n) against INDEX-OF's O(m·n) — and halving a range until it is empty is a loop whose length depends on the data, which a language with no unbounded loop cannot write. — e.g. `[ 1 3 5 7 ] [ 5 ] BSEARCH` |
| `RECORD` | record | Build a Record — a keyed correspondence, the seventh value domain — from a Vector of keys and a Vector of values paired position by position: `[ 'x' 'y' ] [ 1 2 ] RECORD`. Keys keep the order they were given, which KEYS and VALUES read back. Two lengths that differ, or a key that appears twice, is the program being wrong, so both are ERRORs rather than a silent last-one-wins. — e.g. `[ 'x' 'y' ] [ 1 2 ] RECORD` |
| `KEYS` | record | The keys of a Record as a Vector, in the Record's own order, so that `KEYS` and `VALUES` line up position by position: `[ 'x' 'y' ] [ 1 2 ] RECORD KEYS` is `[ 'x' 'y' ]`. Key order is part of a Record's observable structure, so this Vector is one exact thing, not a set in some arbitrary order. A Vector or any other non-Record operand is an ERROR: a Record is not a Vector and nothing converts between them implicitly. — e.g. `[ 'x' 'y' ] [ 1 2 ] RECORD KEYS` |
| `VALUES` | record | The values of a Record as a Vector, aligned with `KEYS`: `[ 'x' 'y' ] [ 1 2 ] RECORD VALUES` is `[ 1/1 2/1 ]`. This is the bridge from the Record domain back to the Vector domain — from here every Vector Word applies — and `RECORD` is the bridge the other way, so `R KEYS R VALUES RECORD` rebuilds `R`. A non-Record operand is an ERROR. — e.g. `[ 'x' 'y' ] [ 1 2 ] RECORD VALUES` |
| `AT` | record | The value under a key: `R 'x' AT`. What `GET` does for a position, `AT` does for a key, and where the parallel-Vector idiom (`INDEX-OF` then `GET`) scans every key, `AT` answers in constant expected time. A key the Record does not hold is a well-formed question with no answer, so it projects the reasoned absence `missingField`, recovered like any other: `R 'x' AT 'S' BIND fallback S S NIL? SELECT`. Ask `HAS?` first when presence itself is the question. A non-Record first operand is an ERROR. — e.g. `[ 'x' 'y' ] [ 1 2 ] RECORD 'x' AT` |
| `WITH` | record | A copy of a Record with one key set: `R 'z' 3 WITH`. A key already present keeps its position and takes the new value; a key not yet present is appended, so the Record's key order records the order in which keys arrived. This is `PUT` for keys, and like `PUT` it never changes the operand it was given — Records are values. The value may be anything, a NIL included, since a NIL under a key is a stored absence; a NIL where the Record or the key should be is an ERROR. — e.g. `[ 'x' ] [ 1 ] RECORD 'y' 2 WITH` |
| `WITHOUT` | record | A copy of a Record with one key removed: `R 'x' WITHOUT`. Removing a key the Record does not hold is not an identity but the absence `missingField` — the same discipline `GET`, `TAKE` and `PUT` keep for a position outside the Vector, so a misspelled key cannot pass silently. The other keys keep their order. A non-Record first operand is an ERROR. — e.g. `[ 'x' 'y' ] [ 1 2 ] RECORD 'x' WITHOUT` |
| `HAS?` | record | Whether a Record holds a key: `R 'x' HAS?` is TRUE or FALSE. It asks about presence without touching the value, so a program can tell a key that is absent from a key whose stored value is NIL — `AT` alone answers NIL for both. Like `NIL?`, it is a predicate and ends in `?`. A non-Record first operand is an ERROR. — e.g. `[ 'x' ] [ 1 ] RECORD 'x' HAS?` |
| `MERGE` | record | The union of two Records, the right one winning: `defaults overrides MERGE`. The left Record's keys keep their order and take the right Record's value wherever both hold the key; keys only the right holds are appended in the right's order. Layering overrides on defaults is the shape this Word is for; swap the operands for the left to win. Either operand not a Record is an ERROR. — e.g. `[ 'x' 'y' ] [ 1 2 ] RECORD [ 'y' 'z' ] [ 9 3 ] RECORD MERGE` |
| `MAP` | higher-order | Apply a code block to each element of a vector. — e.g. `[ 1 2 3 ] [ 2 MUL ] MAP` |
| `FILTER` | higher-order | Keep only the elements for which a predicate block returns TRUE. — e.g. `[ 1 2 3 ] [ 2 = ] FILTER` |
| `FOLD` | higher-order | Reduce a vector to a single value using an initial accumulator and combiner block. — e.g. `[ 1 2 3 ] [ 0 ] [ + ] FOLD` |
| `SCAN` | higher-order | Reduce a vector step by step, answering the accumulator after each element rather than only the last one: `[ 1 2 3 4 ] 0 [ ADD ] SCAN` is `[ 1/1 3/1 6/1 10/1 ]`. The answer has one lane per input lane — the initial accumulator is the seed, not a lane, so it is not among them — which is what lets a scan pair with the Vector it came from. The block sees the accumulator and the current element, exactly as FOLD's does, and what it leaves is both the next accumulator and that lane's answer. An empty Vector answers an empty Vector, and an absent Vector answers that same absence. — e.g. `[ 1 2 3 4 ] 0 [ ADD ] SCAN` |
| `CHARS` | cast | Split a string into a vector of one-character strings. — e.g. `'hi' CHARS` |
| `JOIN` | cast | Join a vector of strings into a single string. — e.g. `[ 'h' 'i' ] JOIN` |
| `TRIM` | cast | Remove whitespace from both ends of a string. — e.g. `'  hi  ' TRIM` |
| `UPPER` | cast | The String with every character mapped to its upper form under Unicode's default, locale-independent case mapping: `'Ajisai' UPPER` is `'AJISAI'`, `'straße' UPPER` is `'STRASSE'`. A character with no upper-case form is kept as it is, so the answer may be longer than the operand but never shorter. A non-String operand is an ERROR (`nonText`). The mapping table is Unicode's, which no definition over `CHARS` and `JOIN` could carry, so the Word is native. — e.g. `'Ajisai' UPPER` |
| `LOWER` | cast | The String with every character mapped to its lower form under Unicode's default, locale-independent case mapping: `'Ajisai' LOWER` is `'ajisai'`, `'ΣΑΣ' LOWER` is `'σασ'`. The final-sigma rule and every other language-specific rule are not applied: the same text lowers the same way wherever it is run. A non-String operand is an ERROR (`nonText`). The mapping table is Unicode's, which no definition over `CHARS` and `JOIN` could carry, so the Word is native. — e.g. `'Ajisai' LOWER` |
| `TOKENIZE` | cast | Split a string into a vector of substrings using a separator; the empty separator splits between every character. — e.g. `'a,b,c' ',' TOKENIZE` |
| `SEARCH` | cast | The position, in characters, at which a text first occurs in another: `'hello world' 'world' SEARCH` is `6`, counted the way CHARS counts, and `'hello' 'z' SEARCH` is NIL(missingField). An empty needle is found at 0. This is INDEX-OF for text: spelled over CHARS it compares a window at every position, and the Word does it in one pass. — e.g. `'hello world' 'world' SEARCH` |
| `REPLACE` | cast | Every occurrence of one text replaced by another: `'a-b-c' '-' '+' REPLACE` is `'a+b+c'`. Occurrences are found left to right and do not overlap, and an empty `from` matches nothing, so the text comes back unchanged rather than growing without bound. Spelled over CHARS and JOIN this is a scan with a window at every position; the Word is the one pass. — e.g. `'a-b-c' '-' '+' REPLACE` |
| `NUM` | cast | Parse text as a number; Bubble/NIL on parse failure. — e.g. `'42' NUM` |
| `STR` | cast | Convert a value to its string representation. — e.g. `42 STR` |
| `FORMAT` | cast | Render an exact scalar as decimal text with a stated number of digits after the point, rounding a tie away from zero exactly as `ROUND` does: `1/3 5 FORMAT` is `'0.33333'`, `5/2 0 FORMAT` is `'3'`, `2 SQRT 3 FORMAT` is `'1.414'`. This is the one place a value is rounded, and it is text that leaves it, never a number: arithmetic performs no rounding and `STR` refuses a number with no exact lexeme, so a program that wants a decimal approximation names its precision here, at the display boundary. The digit count is a non-negative integer (`invalidCount` otherwise) and the value a scalar (`nonNumeric` otherwise). Every digit is decided exactly, an irrational's included, since the field's order never ties. — e.g. `1/3 5 FORMAT` |
| `JSON-DECODE` | cast | Read JSON text into a value: an object becomes a Record keyed by its member names in order, an array a Vector, a string a String, a number the exact rational it spells (`'0.1'` is `1/10`, never a float), `true`/`false` Booleans and `null` a NIL. Text that is not one JSON value — malformed, empty, trailing content, or an object naming one member twice — projects `invalidEncoding`, the reason `NUM` projects for text that spells no number. Nesting is bounded by the text rather than by any Word, so this Word cannot be written in the language, whose repetition is over a Vector that already exists; a value nested past what the machine holds projects `spaceExhausted`, the outcome of every materialization past a ceiling. A non-String operand is an ERROR (`nonText`). — e.g. `'{"a": 1, "b": [true, null]}' JSON-DECODE` |
| `JSON-ENCODE` | cast | Write a value as JSON text, the inverse of `JSON-DECODE`: a Record with String keys becomes an object in key order, a Vector an array, a String a string, a Boolean `true`/`false`, a NIL `null`. A rational with a finite decimal spelling (a denominator of the form 2^a·5^b) is written as a JSON number exactly — `1/4` is `0.25` — and every other rational is written as its Ajisai lexeme inside a string, `1/3` as `"1/3"`, so no digit is ever rounded away: the encoder is not a place a value silently loses precision. A value with no JSON image — a Symbol, an irrational, a Record with a non-String key — projects `domainMiss`. Decoding what this Word writes gives back the value it was given, and a rational written as a lexeme comes back as that String, from which `NUM` recovers the number. — e.g. `[ 'a' ] [ 1 ] RECORD JSON-ENCODE` |
| `EXEC` | control | Evaluate a code block. — e.g. `[ 1 2 ADD ] EXEC` |
| `CONTRACT` | control | The contract of a Word or of a block, as a Record. For a Symbol naming a Core Word it is the registered record of `spec/words.json` (LANG.CONTRACT.REGISTRY), keyed `name` `tier` `inputs` `outputs` `nil` `projection` `errors` `partiality` `purity` `determinism` `cost` `effects`, so `[ DIV ] 0 GET CONTRACT 'cost' AT` asks a Word's cost class before running it. For a Symbol naming a User Word, or for a block of code, it is the contract inferred without running anything — the same inference `ajisai check --contract` runs from outside the language — keyed `inputs` `outputs` `nil` `purity` `determinism` `cost` `effects` `confidence` `gaps`, where `confidence` and `gaps` carry the check's own trichotomy (LANG.CONTRACT.CHECK) as data: an unresolved dependency is a gap in the answer, not an ERROR. A block is never evaluated, so `[ 42 PRINT ] CONTRACT` reports `consoleWrite` under `effects` without printing. A Symbol that names no Word projects `missingField`; an operand that is neither a Symbol nor a block is an ERROR (`notASymbol`). — e.g. `[ ADD ] 0 GET CONTRACT` |
| `FAIL` | control | Raise an ERROR the program states: `'width must be positive' FAIL` halts evaluation with category `declaredFailure` and that text as its message. This is the other half of what ABSENT gives a user Word — the trichotomy's third outcome, for a call that is wrong rather than data that did not work out. Like every ERROR it propagates and cannot be caught; a caller who wants a value to recover from asks for ABSENT instead. A non-text operand is `nonText`. — e.g. `'width must be positive' FAIL` |
| `NIL` | constant | Push the NIL value onto the stack. — e.g. `NIL` |
| `NIL?` | absence | Test whether the top value is an operational NIL (absent). — e.g. `1 0 / NIL?` |
| `NIL-REASON` | absence | Read the direct reason of an operational NIL as a protocol-string Text. — e.g. `1 0 / NIL-REASON` |
| `ABSENT` | absence | A NIL whose reason the program states: `'rate not quoted' ABSENT NIL-REASON` answers `'rate not quoted'`. Its registered reason is `userDeclared`, and the text is the reason NIL-REASON answers, so a user Word can say why it has no answer exactly as a Core Word's contract does — and a caller recovers it the same way, `subject 'S' BIND fallback S S NIL? SELECT`. The text is part of the value (LANG.VALUES.NIL): two absences with different texts are two values. A non-text operand is the program being wrong. — e.g. `'rate not quoted' ABSENT` |
| `BIND` | dictionary | Name a value for the rest of the frame that made it. — e.g. `[ 1 2 3 ] 'XS' BIND` |
| `DEF` | dictionary | Define a user word from a body and a name. — e.g. `[ 2 * ] 'DOUBLE' DEF` |
| `DEL` | dictionary | Delete a user word from the dictionary; ERROR while other words still reference it. — e.g. `[ [ 1 ] ] 'W' DEF 'W' DEL` |
| `DIGEST` | dictionary | The content identity of a Word, or the digest of a value's denotation, as text. A Symbol naming a User Word answers that Word's content identity — the digest over its normalized definition and the identities of the Words it calls that the dictionary already keeps (LANG.DICTIONARY.MUTATION) — and a Symbol naming a Core Word answers the fixed identity of that sealed Word. Any other value, a Symbol naming nothing included, answers the digest of its denotation: two values that `EQ` calls one value digest alike, however each was built, so `8 SQRT DIGEST` equals `2 SQRT 2 SQRT ADD DIGEST`, and a NIL digests by its reason. Equal digests mean one thing; unequal digests mean nothing. — e.g. `[ ADD ] 0 GET DIGEST` |
| `PRINT` | io | Write the top stack value to the output stream, consuming it. A string is written as its raw text, without the quotes the stack shows ('TEST' prints as TEST); nested strings keep their quotes. — e.g. `42 PRINT` |
| `+` | symbol alias | shorthand for `ADD` |
| `-` | symbol alias | shorthand for `SUB` |
| `*` | symbol alias | shorthand for `MUL` |
| `/` | symbol alias | shorthand for `DIV` |
| `=` | symbol alias | shorthand for `EQ` |
| `<` | symbol alias | shorthand for `LT` |
| `>` | symbol alias | shorthand for `GT` |
| `'` | input helper | STRING-QUOTE — editor affordance, not a Word |
| `#` | source directive | COMMENT-LINE — consumed by the lexer, not a Word |
| `[` | delimiter sugar | BEGIN-VECTOR — structural delimiter, not a Word |
| `]` | delimiter sugar | END-VECTOR — structural delimiter, not a Word |
| `'` | literal sugar | STRING-QUOTE — literal delimiter, not a Word |
