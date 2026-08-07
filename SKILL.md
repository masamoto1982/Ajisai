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
- Data lives in vectors: `[ 1 2 3 ]`. Vectors nest for ragged and grouped data. A lone number like `42` is allowed but `[ 42 ]` is the idiomatic scalar.
- Strings: `'single quotes'` (a codepoint vector with text role). Booleans: `TRUE` / `FALSE`. Absence: `NIL`.
- Code blocks: `{ ... }` — quoted programs passed to MAP / FILTER / FOLD / COND / DEF.
- User word: `{ body } 'NAME' DEF` then call `NAME`. Words are case-insensitive (canonicalized to upper case).
- Comments: `#` to end of line.
- One modifier, prefixing the *next word only*: `KEEP` (do not consume operands). Consumption is the default.
- One word does one thing to the stack; there are **no** DUP/SWAP-style shufflers (§8).

## 3. Control and iteration

- Branch: `value { guard } { body } { guard } { body } ... COND`. Guards see the value (it stays for each guard) and must leave TRUE/FALSE; use `{ TRUE }` as the final else-guard. The value remains on the stack after COND.
- Iterate data, not counters: `MAP` / `FILTER` / `FOLD` with `{ }` blocks (examples in §6). `FOLD` requires an explicit `[ init ]`.
- Predicates: `ANY` / `ALL` with a `{ predicate }` block.
- Recursion is allowed in user words (execution-step and depth limits apply; exceeding them is a diagnosed error, not a hang).

## 4. NIL — absence is a value, not an exception

Failed partial operations *bubble*: `[ 1 ] [ 0 ] DIV` succeeds (exit 0) and
pushes `NIL` (reason: `divisionByZero`). The projection is recorded in
`errorFlowTrace` as a `nilProduced` event with a full diagnosis, and the NIL
value itself carries `semantics.absence.reason` on the stack.

- Provide a fallback with `^`: `[ 1 ] [ 0 ] DIV ^ [ 99 ]` → stack `[ 99/1 ]`.
- NIL flows through later operations (bubble rule); check for it where it matters instead of letting it propagate to the end.

## 5. Exactness — comparison always decides

Numbers are exact rationals, closed under `SQRT`. Arithmetic never rounds,
coefficients are arbitrary-precision, and **every comparison of two scalars
decides**: there is no budget, no refinement limit, and no undecided outcome.

```ajisai
8 SQRT 2 SQRT 2 SQRT + =   # √8 vs √2+√2
```

→ stack `TRUE` (exit 0). Values built through different
histories are the same value when they denote the same real. Truth is
two-valued: `TRUE` and `FALSE`, nothing else. An operation that cannot
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
  `[ 1 2 ] [ 3 1 ] <` → stack: `{ TRUE FALSE }`
- Range: one vector [ start end ] (inclusive)
  `[ 0 5 ] RANGE` → stack: `[ 0/1 1/1 2/1 3/1 4/1 5/1 ]`
- Range with step: [ start end step ]
  `[ 0 10 2 ] RANGE` → stack: `[ 0/1 2/1 4/1 6/1 8/1 10/1 ]`
- Fill a tensor: [ shape... value ]
  `[ 2 2 7 ] FILL` → stack: `[ [ 7/1 7/1 ] [ 7/1 7/1 ] ]`
- MAP with a { } code block
  `[ 0 4 ] RANGE { [ 2 ] * } MAP` → stack: `[ [ 0/1 ] [ 2/1 ] [ 4/1 ] [ 6/1 ] [ 8/1 ] ]`
- FILTER keeps matching elements
  `[ 0 10 ] RANGE { 5 > } FILTER` → stack: `[ 6/1 7/1 8/1 9/1 10/1 ]`
- FOLD needs an explicit initial value
  `[ 1 2 3 ] [ 0 ] { + } FOLD` → stack: `[ 6/1 ]`
- ANY / ALL take predicate blocks
  `[ 1 2 3 ] { 1 > } ANY` → stack: `TRUE`
- Define a user word: { body } then name, then DEF
  `{ [ 1 ] [ 2 ] + } 'MY-SUM' DEF MY-SUM` → stack: `[ 3/1 ]`
- COND: value on stack, then { guard } { body } pairs (use { TRUE } as else-guard)
  `4 { 0 GTE } { 'non-negative' PRINT } { TRUE } { 'negative' PRINT } COND` → prints `non-negative`; stack: `4/1`
- Strings are bare '...' literals; CHARS/JOIN convert
  `'hello' CHARS REVERSE JOIN` → stack: `'olleh'`
- Cast a string to an exact number
  `'42' NUM` → stack: `42/1`
- PRINT pops and emits to output (not the stack)
  `[ 1 2 3 ] PRINT` → prints `[ 1/1 2/1 3/1 ]`
- Sorting is a plain Core word
  `[ 3 1 2 ] SORT` → stack: `[ 1/1 2/1 3/1 ]`
- Exact square root takes a bare scalar
  `2 SQRT` → stack: `( 1 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ( 2 ...) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) ) )`
- The KEEP modifier makes the next word non-consuming
  `[ 5 ] KEEP PRINT` → prints `[ 5/1 ]`; stack: `[ 5/1 ]`

## 7. Common errors — actual CLI output, and the fix

- **Typo / unknown word** — `[ 1 ] ADDD`
  → exit 1, `message: "Unknown word: ADDD"`, `diagnosis: { when: "resolveWord", why: "typoOrUnknownName" }`,
  `aiDiagnostic.recoverability: "fixProgram"`, first nextCheck: "Check spelling".
  Fix: Grep §9 for the word you meant (here: `+` / `ADD`). Word names are upper-cased automatically.
- **Stack underflow: operands must be pushed first** — `+`
  → exit 1, `message: "Stack underflow"`, `diagnosis: { when: "executeWord", why: "stackShape" }`,
  `aiDiagnostic.recoverability: "fixProgram"`, first nextCheck: "Check arity".
  Fix: Push both operands before the operator: `[ 1 ] [ 2 ] +`. Ajisai is postfix; there is no infix form.
- **FOLD without an initial value** — `[ 1 2 3 ] { + } FOLD`
  → exit 1, `message: "Stack underflow"`, `diagnosis: { when: "executeWord", why: "stackShape" }`,
  `aiDiagnostic.recoverability: "fixProgram"`, first nextCheck: "Check arity".
  Fix: FOLD is `vector [ init ] { op } FOLD`: `[ 1 2 3 ] [ 0 ] { + } FOLD`.
- **COND blocks must come in { guard } { body } pairs** — `5 { 3 > } { 'big' PRINT } { 'small' PRINT } COND`
  → exit 1, `message: "COND: expected even number of code blocks (guard/body pairs), got 3"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck: "Check error message".
  Fix: Give every body a guard; the else-branch is `{ TRUE } { ... }`: `5 { 3 > } { 'big' PRINT } { TRUE } { 'small' PRINT } COND`.
- **COND guards must yield a boolean** — `TRUE { [ 1 ] } { [ 2 ] } COND`
  → exit 1, `message: "COND: guard must return TRUE or FALSE, got non-scalar"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck: "Check error message".
  Fix: The first block is a guard, not a value: it must leave TRUE/FALSE. Branch on a stack value with `[ x ] { predicate } { body } ... COND`.
- **Broadcast shape mismatch** — `[ 1 2 ] [ 1 2 3 ] +`
  → exit 1, `message: "Cannot broadcast shapes [2] and [3]"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck: "Check error message".
  Fix: Elementwise ops need equal or broadcastable shapes (scalar `[ 5 ]` broadcasts; `[2]` vs `[3]` does not).
- **NUM casts strings, not booleans** — `TRUE NUM`
  → exit 1, `message: "NUM: expected String, got Boolean"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck: "Check error message".
  Fix: NUM accepts strings: `'42' NUM`. There is no boolean→number cast.
- **Old two-vector RANGE form** — `[ 0 ] [ 5 ] RANGE`
  → exit 1, `message: "RANGE requires [start end] or [start end step]"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck: "Check error message".
  Fix: RANGE takes one vector: `[ 0 5 ] RANGE` (or `[ start end step ]`).
- **Vector-wrapped string passed to a cast** — `[ '42' ] NUM`
  → exit 1, `message: "NUM: expected String input"`, `diagnosis: { when: "executeWord", why: "unknown" }`,
  `aiDiagnostic.recoverability: "inspectContext"`, first nextCheck: "Check error message".
  Fix: String casts take the bare string: `'42' NUM`.

## 8. Forbidden patterns (each verified to fail)

- **DUP / SWAP / DROP / OVER / ROT** (`DUP` fails) — Forth-style stack shufflers do not exist. Use `KEEP` when the next word must retain its operands; consumption is the default.
- **IF / ELSE / THEN / WHILE** (`[ 1 ] IF` fails) — No structured keywords. Branch with COND guard/body pairs; iterate with MAP / FILTER / FOLD or recursive user words.
- **Parentheses ( )** (`( 1 2 )` fails) — Reserved for the continued-fraction *display* form only. Vectors are `[ ]`, code blocks are `{ }`.
- **Double-quoted strings** (`"hello" PRINT` fails) — Strings use single quotes: 'hello'.
- **// line comments** (`// comment` fails) — Comments start with `#`.

## 9. Word quick reference

Generated from `docs/word-manifest.json` — the complete inventory:
59 canonical Words in one flat Core dictionary, of which
36 form the Semantic Kernel and 23 are Standard Words. Both are
ordinary Core Words called by their plain names; the split is a design
classification, not a namespace. A word absent here does not exist. There is
no module system and nothing to import.

| word | category | summary |
|---|---|---|
| `TRUE` | constant | Push the boolean TRUE onto the stack. — e.g. `TRUE` |
| `FALSE` | constant | Push the boolean FALSE onto the stack. — e.g. `FALSE` |
| `AND` | logic | Logical AND. A NIL operand passes through. — e.g. `TRUE TRUE &` |
| `OR` | logic | Logical OR. A NIL operand passes through. — e.g. `TRUE FALSE OR` |
| `NOT` | logic | Logical negation. A NIL operand passes through. — e.g. `TRUE NOT` |
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
| `MIN` | math | Smaller of two numbers. — e.g. `1 2 MIN` |
| `MAX` | math | Larger of two numbers. — e.g. `1 2 MAX` |
| `SQRT` | math | Exact square root of a non-negative rational. — e.g. `2 SQRT` |
| `GET` | vector | Select elements of a vector by index. — e.g. `[ 10 20 30 ] [ 0 2 ] GET` |
| `LENGTH` | vector | Return the number of elements in a vector. — e.g. `[ 1 2 3 ] LENGTH` |
| `TAKE` | vector | Take the first N or last -N elements of a vector. — e.g. `[ 1 2 3 4 5 ] [ 3 ] TAKE` |
| `CONCAT` | vector | Flatten and concatenate two vectors. — e.g. `[ 1 2 ] [ 3 4 ] CONCAT` |
| `REVERSE` | vector | Reverse the order of vector elements. — e.g. `[ 1 2 3 ] REVERSE` |
| `COLLECT` | vector | Collect N items off the stack into a new vector. — e.g. `1 2 3 3 COLLECT` |
| `RANGE` | vector | Generate a numeric sequence from a [start, end] pair. — e.g. `[ 0 5 ] RANGE` |
| `FILL` | tensor | Fill a target shape with a constant value. — e.g. `[ 2 2 0 ] FILL` |
| `SORT` | vector | Return a copy of a vector sorted in ascending order. — e.g. `[ 3 1 2 ] SORT` |
| `INDEX-OF` | vector | Index of the first element equal to the value; Bubble/NIL if absent. — e.g. `[ 1 2 ] 2 INDEX-OF` |
| `MAP` | higher-order | Apply a code block to each element of a vector. — e.g. `[ 1 2 3 ] { 2 MUL } MAP` |
| `FILTER` | higher-order | Keep only the elements for which a predicate block returns TRUE. — e.g. `[ 1 2 3 ] { 2 = } FILTER` |
| `FOLD` | higher-order | Reduce a vector to a single value using an initial accumulator and combiner block. — e.g. `[ 1 2 3 ] [ 0 ] { + } FOLD` |
| `ANY` | higher-order | TRUE if at least one element satisfies the predicate. — e.g. `[ 1 2 3 ] { 2 = } ANY` |
| `ALL` | higher-order | TRUE if every element satisfies the predicate. — e.g. `[ 2 4 ] { 2 MOD 0 = } ALL` |
| `CHARS` | cast | Split a string into a vector of one-character strings. — e.g. `'hi' CHARS` |
| `JOIN` | cast | Join a vector of strings into a single string. — e.g. `[ 'h' 'i' ] JOIN` |
| `TRIM` | cast | Remove whitespace from both ends of a string. — e.g. `'  hi  ' TRIM` |
| `TOKENIZE` | cast | Split a string into a vector of substrings using a separator. — e.g. `'a,b,c' ',' TOKENIZE` |
| `SUBSTITUTE` | cast | Replace every occurrence of a substring with another. — e.g. `'hello' 'l' 'L' SUBSTITUTE` |
| `NUM` | cast | Parse text as a number; Bubble/NIL on parse failure. — e.g. `'42' NUM` |
| `STR` | cast | Convert a value to its string representation. — e.g. `42 STR` |
| `COND` | control | Evaluate guard/body clauses in order, executing the first match. Each guard and the winning body run in an isolated frame that holds exactly the target value, and exactly one value comes back: whatever the body leaves on top. A body that leaves nothing is an error; extra values below the top are discarded with the frame. — e.g. `1 { TRUE } { 'y' } { IDLE } { 'n' } COND` |
| `EXEC` | control | Evaluate a code block. — e.g. `{ 1 2 ADD } EXEC` |
| `NIL` | constant | Push the NIL value onto the stack. — e.g. `NIL` |
| `NIL?` | absence | Test whether the top value is an operational NIL (absent). — e.g. `1 0 / NIL?` |
| `NIL-REASON` | absence | Read the direct reason of an operational NIL as a protocol-string Text. — e.g. `1 0 / NIL-REASON` |
| `VENT` | control-directive | Lazy NIL-coalescing control directive: keep a non-NIL top and skip the following source unit; on a NIL top, discard it and evaluate the following source unit as the fallback. — e.g. `NIL ^ [ 0 ]` |
| `KEEP` | modifier | Set the consumption mode to keep operands. — e.g. `KEEP +` |
| `BIND` | dictionary | Name a value for the rest of the frame that made it. — e.g. `[ 1 2 3 ] 'XS' BIND` |
| `DEF` | dictionary | Define a user word from a body and a name. — e.g. `{ 2 * } 'DOUBLE' DEF` |
| `DEL` | dictionary | Delete a user word from the dictionary. — e.g. `{ [ 1 ] } 'W' DEF 'W' DEL` |
| `LOOKUP` | dictionary | Display the documentation for a named word. — e.g. `'ADD' ?` |
| `PRINT` | io | Write the top stack value to the output stream, consuming it. A string is written as its raw text, without the quotes the stack shows ('TEST' prints as TEST); nested strings keep their quotes. — e.g. `42 PRINT` |
| `REFLECT` | reflection | Reflect a CodeBlock into canonical code data, or canonical code data into a CodeBlock. — e.g. `CodeBlock REFLECT
code-data REFLECT` |
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
| `?` | symbol alias | shorthand for `LOOKUP` |
| `^` | syntax sugar | shorthand for `VENT` |
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
