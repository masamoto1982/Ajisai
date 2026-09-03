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
- Code blocks are quoted programs passed to MAP / FILTER / FOLD / COND / DEF, written as an ordinary Vector (§6) — there is no separate block bracket, and `{` / `}` are not valid Ajisai source characters.
- Define a user word with a body Vector, then a `'NAME'` string, then `DEF`, then call `NAME`: `[ [ 1 ] [ 2 ] + ] 'MY-SUM' DEF MY-SUM` (§6). Words are case-insensitive (canonicalized to upper case).
- Comments: `#` to end of line.
- One modifier, prefixing the *next word only*: `KEEP` (do not consume operands). Consumption is the default.
- One word does one thing to the stack; there are **no** DUP/SWAP-style shufflers (§8).

## 3. Control and iteration

- Branch: a value, then one Vector of guard/body pairs, then `COND`: `4 [ [ 0 GTE ] [ 'non-negative' PRINT ] [ TRUE ] [ 'negative' PRINT ] ] COND` (§6). Guards see the value (it stays for each guard) and must leave TRUE/FALSE; use `[ TRUE ]` as the final else-guard. The value remains on the stack after COND.
- Iterate data, not counters: `MAP` / `FILTER` / `FOLD` with block operands (examples in §6). `FOLD` requires an explicit initial-value Vector.
- Predicates: `ANY` / `ALL` take a predicate block (examples in §6).
- No recursion: `DEF` refuses a word whose body names itself, directly or through other user words (a diagnosed error at definition time, not at the call). Repetition is expressed only through MAP / FILTER / FOLD / ANY / ALL over an already-finite vector.

## 4. NIL — absence is a value, not an exception

Failed partial operations *bubble*: `1 0 DIV` succeeds (exit 0) and
pushes `NIL` (reason: `divisionByZero`). The projection is recorded in
`errorFlowTrace` as a `nilProduced` event with a full diagnosis, and the NIL
value itself carries `semantics.absence.reason` on the stack.

- Provide a fallback with `OR-NIL`: `1 0 DIV OR-NIL [ 99 ]` → stack `[ 99/1 ]`.
- Over a vector the projection is **per lane, not per value**: `[ 6 6 ] [ 1 0 ] DIV` → stack `[ 6/1 NIL ]`. The lane that could not divide is the only one emptied.
- That makes the top a vector, not a NIL, so `OR-NIL` — which inspects the stack top — keeps it as-is. Recover a lifted result inside the vector, not around it.
- NIL flows through later operations (bubble rule); check for it where it matters instead of letting it propagate to the end.

## 5. Exactness — comparison decides over the algebraic field

Numbers are exact rationals, closed under `SQRT`. Arithmetic never rounds,
coefficients are arbitrary-precision, and **every comparison of two scalars
built from rationals and `SQRT` decides**: there is no budget, no refinement
limit, and no undecided outcome over that field.

```ajisai
8 SQRT 2 SQRT 2 SQRT + =   # √8 vs √2+√2
```

→ stack `TRUE` (exit 0). Values built through different
histories are the same value when they denote the same real.

`PI` is the one value outside that field: a general computable real with no
algebraic normal form. Comparing two independently-built `PI` values can
exhaust the comparison's refinement budget without deciding:

```ajisai
PI PI EQ
```

→ stack `NIL` (exit 0, truthValue `unknown`). Truth has
three values: `TRUE`, `FALSE`, and this logical UNKNOWN, which is also what a
NIL operand reads as in a truth position (§4). An operation that cannot
produce a value produces NIL (§4); a malformed one raises an error.

## 6. Canonical examples (all verified by the generator)

- Push a number (always inside a vector)
  `[ 42 ]` → stack: `[ 42/1 ]`
- Exact rational division — no floats, ever
  `[ 1 ] [ 3 ] /` → stack: `[ 1/3 ]`
- Elementwise vector arithmetic
  `[ 1 2 3 ] [ 4 5 6 ] +` → stack: `[ 5/1 7/1 9/1 ]`
- Scalar broadcast over a vector
  `[ 5 ] [ 1 2 3 ] *` → stack: `[ 5/1 10/1 15/1 ]`
- Remainder
  `[ 10 ] [ 3 ] %` → stack: `[ 1/1 ]`
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
- ANY / ALL take predicate blocks
  `[ 1 2 3 ] [ 1 > ] ANY` → stack: `TRUE`
- Define a user word: [ body ] then name, then DEF
  `[ [ 1 ] [ 2 ] + ] 'MY-SUM' DEF MY-SUM` → stack: `[ 3/1 ]`
- COND: value, then one [ ] of [ guard ] [ body ] pairs (use [ TRUE ] as else-guard)
  `4 [ [ 0 GTE ] [ 'non-negative' PRINT ] [ TRUE ] [ 'negative' PRINT ] ] COND` → prints `non-negative`; stack: `4/1`
- Strings are bare '...' literals; CHARS/JOIN convert
  `'hello' CHARS REVERSE JOIN` → stack: `'olleh'`
- Cast a string to an exact number
  `'42' NUM` → stack: `42/1`
- PRINT pops and emits to output (not the stack)
  `[ 1 2 3 ] PRINT` → prints `[ 1/1 2/1 3/1 ]`
- Sorting is a plain Core word
  `[ 3 1 2 ] SORT` → stack: `[ 1/1 2/1 3/1 ]`
- Exact square root takes a bare scalar
  `2 SQRT` → exact value: `sqrt(2)` (the stack display is its continued fraction)
- The KEEP modifier makes the next word non-consuming
  `[ 5 ] KEEP PRINT` → prints `[ 5/1 ]`; stack: `[ 5/1 ]`

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
- **COND clauses must be wrapped in a single [ ]** — `5 [ 3 > ] [ 'big' PRINT ] COND`
  → exit 1, `message: "Structure error: expected each COND clause to be a [ guard | body ] block, got a non-Vector element"`, `diagnosis: { when: "executeWord", why: "valueShape" }`,
  `aiDiagnostic.recoverability: "fixInput"`, first nextCheck code: `checkExpectedShape`.
  Fix: COND takes its clauses as one Vector, not a run of separate blocks: wrap them together, and give every body a guard — the else-branch is `[ TRUE ] [ ... ]`: `5 [ [ 3 > ] [ 'big' PRINT ] [ TRUE ] [ 'small' PRINT ] ] COND`.
- **COND guards must yield a boolean** — `TRUE [ [ [ 1 ] ] [ [ 2 ] ] ] COND`
  → exit 1, `message: "COND: guard must return TRUE or FALSE, got non-scalar"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck code: `checkDeclaredErrorConditions`.
  Fix: The first block of each pair is a guard, not a value: it must leave TRUE/FALSE. Branch on a stack value with `[ x ] [ [ predicate ] [ body ] ... ] COND`.
- **Broadcast shape mismatch** — `[ 1 2 ] [ 1 2 3 ] +`
  → exit 1, `message: "Cannot broadcast shapes [2] and [3]: axis 0 is 2 on the left and 3 on the right, and neither is 1"`, `diagnosis: { when: "executeWord", why: "shapeMismatch" }`,
  `aiDiagnostic.recoverability: "fixInput"`, first nextCheck code: `checkDisagreeingAxis`.
  Fix: Elementwise ops need equal or broadcastable shapes (scalar `[ 5 ]` broadcasts; `[2]` vs `[3]` does not).
- **NUM casts strings, not booleans** — `TRUE NUM`
  → exit 1, `message: "NUM: expected String, got Boolean"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck code: `checkDeclaredErrorConditions`.
  Fix: NUM accepts strings: `'42' NUM`. There is no boolean→number cast.
- **Old two-vector RANGE form** — `[ 0 ] [ 5 ] RANGE`
  → exit 1, `message: "RANGE requires [start end] or [start end step]"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck code: `checkDeclaredErrorConditions`.
  Fix: RANGE takes one vector: `[ 0 5 ] RANGE` (or `[ start end step ]`).
- **Vector-wrapped string passed to a cast** — `[ '42' ] NUM`
  → exit 1, `message: "NUM: expected String input"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck code: `checkDeclaredErrorConditions`.
  Fix: String casts take the bare string: `'42' NUM`.

These raise. The next one does not — it succeeds and answers something other
than it looks like it answers, which is the harder kind to notice:

- **A one-element vector where a Word wants an element** — both of these succeed (exit 0):
  `[ 1 2 3 ] [ 1 ] [ 9 ] PUT` → stack `[ 1/1 [ 9/1 ] 3/1 ]`
  `[ 1 2 3 ] 1 9 PUT` → stack `[ 1/1 9/1 3/1 ]`
  Fix: PUT, GET and INDEX-OF take an *element*, not a one-element vector holding it: `[ 9 ]` is that vector, so it is stored as one. The `[ 42 ]` idiom of §2 is for operands a Word reads as a value; it does not carry here, and no error says so.

## 8. Forbidden patterns (each verified to fail)

- **DUP / SWAP / DROP / OVER / ROT** (`DUP` fails) — Forth-style stack shufflers do not exist. Use `KEEP` when the next word must retain its operands; consumption is the default.
- **IF / ELSE / THEN / WHILE** (`[ 1 ] IF` fails) — No structured keywords, and no loops. Branch with COND guard/body pairs; iterate with MAP / FILTER / FOLD / ANY / ALL.
- **A word calling itself** (`[ REC ] 'REC' DEF` fails) — The User dictionary is acyclic: `DEF` refuses a body that names the word being defined, directly or through other user words, so this fails at definition time rather than the call. Repetition is expressed only through MAP / FILTER / FOLD / ANY / ALL over an already-finite vector.
- **Parentheses ( )** (`( 1 2 )` fails) — Reserved; not valid in source. `[ ]` is the sole bracket, for vectors, code, and continued-fraction display alike.
- **Double-quoted strings** (`"hello" PRINT` fails) — Strings use single quotes: 'hello'.
- **// line comments** (`// comment` fails) — Comments start with `#`.

## 9. Word quick reference

Generated from `docs/word-manifest.json` — the complete inventory:
66 canonical Words in one flat Core dictionary, of which
37 form the Semantic Kernel and 29 are Standard Words. Both are
ordinary Core Words called by their plain names; the split is a design
classification, not a namespace. A word absent here does not exist. There is
no module system and nothing to import.

| word | category | summary |
|---|---|---|
| `TRUE` | constant | Push the boolean TRUE onto the stack. — e.g. `TRUE` |
| `FALSE` | constant | Push the boolean FALSE onto the stack. — e.g. `FALSE` |
| `AND` | logic | Logical AND. FALSE absorbs a NIL operand into FALSE; otherwise a NIL operand yields UNKNOWN. — e.g. `TRUE TRUE &` |
| `OR` | logic | Logical OR. TRUE absorbs a NIL operand into TRUE; otherwise a NIL operand yields UNKNOWN. — e.g. `TRUE FALSE OR` |
| `NOT` | logic | Logical negation. TRUE and FALSE invert; a NIL operand (UNKNOWN) passes through unchanged. — e.g. `TRUE NOT` |
| `EQ` | comparison | Test equality of two values. — e.g. `1 1 =` |
| `NEQ` | comparison | Test inequality of two values. — e.g. `1 2 NEQ` |
| `LT` | comparison | Test less-than comparison. — e.g. `1 2 <` |
| `LTE` | comparison | Test less-than-or-equal comparison. — e.g. `1 1 LTE` |
| `GT` | comparison | Test greater-than comparison. — e.g. `2 1 >` |
| `GTE` | comparison | Test greater-than-or-equal comparison. — e.g. `1 1 GTE` |
| `ADD` | arithmetic | Add two numeric values, element-wise with broadcasting. — e.g. `1 2 +` |
| `SUB` | arithmetic | Subtract two numeric values, element-wise with broadcasting. — e.g. `5 3 -` |
| `MUL` | arithmetic | Multiply two numeric values, element-wise with broadcasting. — e.g. `2 4 *` |
| `DIV` | arithmetic | Divide two numeric values exactly (fractional result). — e.g. `10 2 /` |
| `MOD` | arithmetic | Modulo (remainder) of two numeric values. — e.g. `7 3 %` |
| `FLOOR` | arithmetic | Round toward negative infinity. — e.g. `[ 7/3 ] FLOOR` |
| `ROUND` | arithmetic | Round to nearest integer (half-up). — e.g. `[ 5/2 ] ROUND` |
| `QUANTIZE` | arithmetic | Round to the nearest multiple of 1/d, bounding the denominator by d. — e.g. `[ 119/125 32/125 ] 10 QUANTIZE` |
| `ABS` | math | Absolute value of a number. — e.g. `-2 ABS` |
| `NEG` | math | Numeric negation. — e.g. `2 NEG` |
| `MIN` | math | Smaller of two numbers, element-wise with broadcasting. — e.g. `1 2 MIN` |
| `MAX` | math | Larger of two numbers, element-wise with broadcasting. — e.g. `1 2 MAX` |
| `SQRT` | math | Exact square root of a non-negative rational, element-wise over a vector. — e.g. `2 SQRT` |
| `PI` | constant | The Tier 2 computable real π. — e.g. `PI` |
| `RANDOM` | math | Count exact rationals in [0,1), determined entirely by the seed. — e.g. `7 3 RANDOM` |
| `GET` | vector | Select elements of a vector by index. — e.g. `[ 10 20 30 ] [ 0 2 ] GET` |
| `LENGTH` | vector | Return the number of elements in a vector. — e.g. `[ 1 2 3 ] LENGTH` |
| `TAKE` | vector | Take the first N or last -N elements of a vector. — e.g. `[ 1 2 3 4 5 ] [ 3 ] TAKE` |
| `CONCAT` | vector | Flatten and concatenate two vectors. — e.g. `[ 1 2 ] [ 3 4 ] CONCAT` |
| `REVERSE` | vector | Reverse the order of vector elements. — e.g. `[ 1 2 3 ] REVERSE` |
| `COLLECT` | vector | Collect N items off the stack into a new vector. — e.g. `1 2 3 3 COLLECT` |
| `RANGE` | vector | Generate a numeric sequence from a [start, end] pair. — e.g. `[ 0 5 ] RANGE` |
| `FILL` | tensor | Fill a target shape with a constant value. — e.g. `[ 2 2 0 ] FILL` |
| `SORT` | vector | Return a copy of a vector sorted in ascending order. — e.g. `[ 3 1 2 ] SORT` |
| `ORDER` | vector | The indices that would sort a vector ascending; ties keep their original order. — e.g. `[ 30 10 20 ] ORDER` |
| `UNIQUE` | vector | The distinct elements of a vector, in first-occurrence order. — e.g. `[ 'a' 'b' 'a' ] UNIQUE` |
| `TALLY` | vector | How many times each distinct element occurs, in UNIQUE order. — e.g. `[ 'a' 'b' 'a' ] TALLY` |
| `ZIP` | vector | Bundle equal-length vectors position by position; a matrix transposes. — e.g. `[ [ 1 2 ] [ 3 4 ] ] ZIP` |
| `SUM` | arithmetic | Fold the outermost axis with ADD; the empty vector sums to zero. — e.g. `[ 1 2 3 ] SUM` |
| `PUT` | vector | A copy of a vector with the element at one index replaced. — e.g. `[ 1 2 3 ] 1 9 PUT` |
| `GROUP` | vector | Bundle values by the key at the same position, in UNIQUE key order. — e.g. `[ 1 2 3 ] [ 'a' 'b' 'a' ] GROUP` |
| `INDEX-OF` | vector | Index of the first element equal to the value; Bubble/NIL if absent. — e.g. `[ 1 2 ] 2 INDEX-OF` |
| `MAP` | higher-order | Apply a code block to each element of a vector. — e.g. `[ 1 2 3 ] [ 2 MUL ] MAP` |
| `FILTER` | higher-order | Keep only the elements for which a predicate block returns TRUE. — e.g. `[ 1 2 3 ] [ 2 = ] FILTER` |
| `FOLD` | higher-order | Reduce a vector to a single value using an initial accumulator and combiner block. — e.g. `[ 1 2 3 ] [ 0 ] [ + ] FOLD` |
| `ANY` | higher-order | TRUE if at least one element satisfies the predicate. — e.g. `[ 1 2 3 ] [ 2 = ] ANY` |
| `ALL` | higher-order | TRUE if every element satisfies the predicate. — e.g. `[ 2 4 ] [ 2 MOD 0 = ] ALL` |
| `CHARS` | cast | Split a string into a vector of one-character strings. — e.g. `'hi' CHARS` |
| `JOIN` | cast | Join a vector of strings into a single string. — e.g. `[ 'h' 'i' ] JOIN` |
| `TRIM` | cast | Remove whitespace from both ends of a string. — e.g. `'  hi  ' TRIM` |
| `TOKENIZE` | cast | Split a string into a vector of substrings using a separator. — e.g. `'a,b,c' ',' TOKENIZE` |
| `NUM` | cast | Parse text as a number; Bubble/NIL on parse failure. — e.g. `'42' NUM` |
| `STR` | cast | Convert a value to its string representation. — e.g. `42 STR` |
| `COND` | control | Evaluate guard/body clauses in order, executing the first match. The clauses are a single Vector, each element itself a [ guard | body ] (or paired [ guard ] [ body ]) clause block. Each guard and the winning body run in an isolated frame that holds exactly the target value, and exactly one value comes back: whatever the body leaves on top. A body that leaves nothing is an error; extra values below the top are discarded with the frame. An absent target is the logical Unknown, not a rejection: every guard that reads it answers Unknown and so does not fire, and the clauses that do not read it decide as they always do — so `NIL` reaches the `[ TRUE ]` else-clause like any other unmatched value. — e.g. `1 [ [ TRUE ] [ 'y' ] [ IDLE ] [ 'n' ] ] COND` |
| `EXEC` | control | Evaluate a code block. — e.g. `[ 1 2 ADD ] EXEC` |
| `PROBE` | control | Infer a code block's contract against the current dictionary, without evaluating it. — e.g. `[ 1 2 ADD ] PROBE` |
| `NIL` | constant | Push the NIL value onto the stack. — e.g. `NIL` |
| `NIL?` | absence | Test whether the top value is an operational NIL (absent). — e.g. `1 0 / NIL?` |
| `NIL-REASON` | absence | Read the direct reason of an operational NIL as a protocol-string Text. — e.g. `1 0 / NIL-REASON` |
| `OR-NIL` | control-directive | Lazy NIL-coalescing control directive: keep a non-NIL top and skip the following source unit; on a NIL top, discard it and evaluate the following source unit as the fallback. — e.g. `NIL OR-NIL [ 0 ]` |
| `KEEP` | modifier | Set the consumption mode to keep operands. — e.g. `KEEP +` |
| `BIND` | dictionary | Name a value for the rest of the frame that made it. — e.g. `[ 1 2 3 ] 'XS' BIND` |
| `DEF` | dictionary | Define a user word from a body and a name. — e.g. `[ 2 * ] 'DOUBLE' DEF` |
| `DEL` | dictionary | Delete a user word from the dictionary. — e.g. `[ [ 1 ] ] 'W' DEF 'W' DEL` |
| `PRINT` | io | Write the top stack value to the output stream, consuming it. A string is written as its raw text, without the quotes the stack shows ('TEST' prints as TEST); nested strings keep their quotes. — e.g. `42 PRINT` |
| `+` | symbol alias | shorthand for `ADD` |
| `-` | symbol alias | shorthand for `SUB` |
| `*` | symbol alias | shorthand for `MUL` |
| `/` | symbol alias | shorthand for `DIV` |
| `%` | symbol alias | shorthand for `MOD` |
| `=` | symbol alias | shorthand for `EQ` |
| `<` | symbol alias | shorthand for `LT` |
| `>` | symbol alias | shorthand for `GT` |
| `<=` | symbol alias | shorthand for `LTE` |
| `>=` | symbol alias | shorthand for `GTE` |
| `!=` | symbol alias | shorthand for `NEQ` |
| `'` | input helper | STRING-QUOTE — editor affordance, not a Word |
| `#` | source directive | COMMENT-LINE — consumed by the lexer, not a Word |
| `\|` | control directive | COND-CLAUSE — only inside the construct that defines it |
| `IDLE` | control directive | COND-ELSE-GUARD — only inside the construct that defines it |
| `[` | delimiter sugar | BEGIN-VECTOR — structural delimiter, not a Word |
| `]` | delimiter sugar | END-VECTOR — structural delimiter, not a Word |
| `{` | delimiter sugar | BEGIN-BLOCK — structural delimiter, not a Word |
| `}` | delimiter sugar | END-BLOCK — structural delimiter, not a Word |
| `'` | literal sugar | STRING-QUOTE — literal delimiter, not a Word |
| `(` | reserved marker | RESERVED-BEGIN — reserved, never valid in source |
| `)` | reserved marker | RESERVED-END — reserved, never valid in source |
