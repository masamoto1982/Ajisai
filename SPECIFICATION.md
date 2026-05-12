# Ajisai Language Specification

Status: **Canonical**
Version: **2026-05-12 (Phase 2)**

This document is the single design authority for Ajisai. It supersedes all
prior specifications. Where this file conflicts with any other document,
this file takes precedence.

---

## 1. Identity

Ajisai is an **AI-first, stack-oriented dataflow language** in the Forth
lineage.

* Every numeric value is stored internally as a **finite continued
  fraction** of partial quotients drawn from arbitrary-precision integers.
  Arithmetic goes through exact rational pivots, so there is no rounding
  loss for rational numbers.
* Programs manipulate a **single data stack** plus a **single Register**
  slot. There is no return stack: control flow is expressed entirely by
  data on the stack, and the Register provides a one-slot scratch.
  Avoiding mid-computation memos is a deliberate VTU (Very Thrifty Use)
  energy goal.
* Words are **English-rooted**, with selected symbols (`+`, `-`, `*`,
  `/`, `=`, `<`, `<=`, `>=`, `<>`, `&`, `|`, `!`, `>R`, `R>`, `R@`, `.`)
  acting as syntactic sugar. The standalone `>` symbol and the legacy
  `=>` operator are intentionally **not** provided.
* Ajisai is named after the hydrangea, whose temari (ball-shaped)
  inflorescence visually echoes the human cerebrum. The execution model
  mirrors a coarse model of human memory: **Register** as short-term
  memory, **Stack** as working memory, **Dictionary** as long-term
  memory.
* Internal representation is hidden behind a **semantic plane**: the
  WASM boundary publishes protocol fields, never internal Rust types.

Runtime stack:

* Rust interpreter core (`rust/`)
* WASM boundary (`src/wasm/generated/`)
* TypeScript GUI shell (`src/`)

---

## 2. Specification authority

### 2.1 Canonical

1. This file (`SPECIFICATION.md`).
2. Rust implementation behaviour conforming to this file.
3. The WASM-exposed protocol surface derived from this file.

### 2.2 Non-canonical

Any roadmap, handover note, design memo, or commit message is
non-canonical unless explicitly promoted here.

### 2.3 Semantic plane

The following are **not** part of Ajisai's observable semantics:

* Rust enum variant names and `Debug` output.
* Internal continued-fraction storage layout.
* GUI colours, CSS class names, display strings.
* User-word storage layout.

External consumers must use the machine-readable protocol fields in §6.

---

## 3. Syntax

Source text is a whitespace-separated stream of tokens. Whitespace
includes spaces, tabs, and newlines.

### 3.1 Tokens

| Token form | Description |
|------------|-------------|
| Integer    | `-?[0-9]+`, e.g. `42`, `-7`. |
| Fraction   | `Integer '/' Integer`, e.g. `3/4`, `-5/2`. |
| Decimal    | `-?[0-9]*'.'[0-9]+`, e.g. `3.14`, `.5`. |
| Symbol     | Any other run of non-whitespace characters. |
| Comment    | `#` to end of line is ignored. |

Multi-character symbols (`<=`, `>=`, `<>`, `>R`, `R>`, `R@`) are read as
a single token because the tokenizer is whitespace-delimited; they are
resolved to their English-rooted core word in the dispatch table.

### 3.2 Reserved heads

Two symbols receive special treatment by the parser:

* `DEF NAME body…` — capture the rest of the current execution chunk as
  the body of a new user word `NAME` (see §5.2).
* `DEL NAME` — remove the user word `NAME`.

All other symbols are looked up in the dictionary (core words first,
then user words).

---

## 4. Values

### 4.1 Continued fractions

A continued fraction is the finite list `[a₀, a₁, …, aₙ]` representing

```
a₀ + 1 / (a₁ + 1 / (a₂ + … + 1/aₙ))
```

Canonical nested display:

```
(a₀ (a₁ (a₂ … (aₙ))))
```

| Value     | Nested form         |
|-----------|---------------------|
| `42`      | `(42)`              |
| `3/4`     | `(0 (1 (3)))`       |
| `13/4`    | `(3 (4))`           |
| `355/113` | `(3 (7 (16)))`      |

### 4.2 Canonicalisation

When the coefficient list has more than one element, the last element is
≥ 2: `[…, a, 1]` is rewritten as `[…, a+1]`.

### 4.3 Nil (the bubble)

Nil represents structured absence and propagates:

* Any arithmetic operation with Nil operand returns Nil.
* `1 0 /` returns Nil.
* Comparisons with a Nil operand return Nil.
* Three-valued logic uses Nil for Unknown (see §7.4).

`NIL?` pushes 1 if the top is Nil, else 0.

### 4.4 Truth value of a number

For Boolean tests, a number is **false** iff its rational value is `0`,
otherwise **true**. Negative numbers and proper fractions are true.

---

## 5. Execution model

### 5.1 Data stack

Execution maintains a single LIFO stack of values. Literals push;
words consume operands and push results. There is no return stack.

### 5.2 Register

The interpreter holds a single named slot called the **Register**. It
is initialised to Nil and reset to Nil by `RESET`. Three core words
manipulate it:

| Word     | Sugar | Effect                                                      |
|----------|-------|-------------------------------------------------------------|
| `STORE`  | `>R`  | Pop the top of the stack into the Register.                 |
| `RECALL` | `R>`  | Push the Register onto the stack and reset it to Nil.       |
| `PEEK`   | `R@`  | Push a copy of the Register onto the stack (Register kept). |

#### 5.2.1 Caller-clobbers convention

A word that calls another word **must not** assume the Register is
preserved across the call. A user word that needs to retain a value
across an internal call must push it onto the data stack before the
call and recover it afterwards.

This convention is documented for users and verified by a future linter
(planned for Phase 3 or later); the runtime does not enforce it.

#### 5.2.2 Empty register

The Register's "empty" state is encoded as Nil. `RECALL` on an empty
Register pushes Nil and remains empty.

### 5.3 Word definition

```
DEF NAME body…
```

`DEF` consumes the remainder of the current execution chunk as the body
text of a new user word `NAME` (uppercased). Calling `NAME` re-executes
the body source.

`DEF` overwrites any existing user word of the same name. Re-defining a
core word is not permitted (the symbol resolves to the core word first).

### 5.4 Word deletion

```
DEL NAME
```

Removes a user word. Deleting an undefined or core word is a no-op.

### 5.5 Errors

Errors are reported in three layers:

| Layer       | Audience            | Example                                            |
|-------------|---------------------|----------------------------------------------------|
| `summary`   | End user (one line) | `Stack underflow at +`                             |
| `detail`    | Experienced human   | `Word + requires 2 value(s) on the stack but…`     |
| `diagnosis` | AI / tooling        | `Check the operands feeding +: push 1 more value…` |

The GUI surfaces `summary`; deeper layers are exposed via the protocol
surface (see §6) for AI assistance.

### 5.6 Memory architecture

The execution state forms a three-tier memory architecture inspired by
human cognition:

| Tier              | Ajisai location | Capacity       | Lifetime                   |
|-------------------|-----------------|----------------|----------------------------|
| Short-term memory | **Register**    | 1 slot         | Until the next STORE/RECALL/RESET |
| Working memory    | **Stack**       | Unbounded LIFO | Until consumed              |
| Long-term memory  | **Dictionary**  | Unbounded      | Until DEL or session reset  |

The water and brain metaphors coexist: water describes how values
*flow* through the stack and into the register; the memory architecture
describes *where* state lives.

---

## 6. Protocol surface

The WASM boundary exposes the following protocol-stable fields. They
form the canonical machine-readable contract.

### 6.1 Stack value

```json
{
  "type": "number" | "nil",
  "value": { "numerator": "string", "denominator": "string" } | "Nil",
  "continuedFraction": "(a0 (a1 (a2)))" | "Nil",
  "displayHint": "number" | "nil",
  "semantics": {
    "semanticKind": "number" | "absence",
    "shape": "scalar" | "absence",
    "capabilities": ["..."],
    "origin": "literal"
  }
}
```

The same shape is returned for `collect_register()` (a single value
rather than a list).

### 6.2 Execute result

```json
{
  "status": "OK" | "ERROR",
  "output": "string (optional)",
  "message": "summary (only on ERROR)",
  "detail":  "detail  (only on ERROR)",
  "diagnosis": "diagnosis (only on ERROR)",
  "error": true (only on ERROR)
}
```

### 6.3 Interpreter API

The `AjisaiInterpreter` WASM class exposes:

* `execute(code)` / `execute_step(code)` — run code, returning an
  execute result.
* `reset()` — clear stack, register, and output buffer.
* `collect_stack()` — return the current stack as protocol values.
* `collect_register()` — return the Register as a protocol value.
* `collect_user_words_info()` — return `[name, definition, description,
  isShadow]` rows.
* `collect_core_words_info()` — return `[name, hoverSummary,
  hoverSyntax]` rows.
* `collect_core_word_aliases_info()` — return symbolic-sugar aliases.
* `collect_input_helper_words_info()` — input-assist entries.
* `lookup_word_definition(name)` — return the body text of a user word
  or `null`.
* `remove_word(name)` — convenience wrapper around `DEL`.
* `restore_user_words(words)` — bulk register `DEF`-equivalent
  definitions for persistence.

Module-related methods (`collect_imported_modules`,
`collect_module_words_info`, …) return empty results pending later
phases.

---

## 7. Core words

### 7.1 Arithmetic

| Word | Sugar | Stack effect            | Notes                    |
|------|-------|-------------------------|--------------------------|
| ADD  | `+`   | `a b — (a+b)`           | Exact addition.          |
| SUB  | `-`   | `a b — (a-b)`           | Exact subtraction.       |
| MUL  | `*`   | `a b — (a*b)`           | Exact multiplication.    |
| DIV  | `/`   | `a b — (a/b)` or Nil    | Division by zero → Nil. |

### 7.2 Stack shuffles

| Word | Stack effect       |
|------|--------------------|
| DUP  | `a — a a`          |
| DROP | `a —`              |
| SWAP | `a b — b a`        |
| OVER | `a b — a b a`      |

### 7.3 Register

| Word   | Sugar | Stack effect                                  |
|--------|-------|-----------------------------------------------|
| STORE  | `>R`  | `a — `   (Register := a)                      |
| RECALL | `R>`  | `— a`   (push Register, Register := Nil)      |
| PEEK   | `R@`  | `— a`   (push copy of Register; Register kept)|

### 7.4 Comparison

All comparisons take two numeric operands and push 1 (true), 0 (false),
or Nil (if either operand is Nil). The standalone `>` symbol is
**not** provided to keep `>R` parseable; use the English `GT` instead.

| Word | Sugar | Meaning            |
|------|-------|--------------------|
| EQ   | `=`   | a equals b         |
| NE   | `<>`  | a does not equal b |
| LT   | `<`   | a is less than b   |
| LE   | `<=`  | a is at most b     |
| GE   | `>=`  | a is at least b    |
| GT   | (none)| a is greater than b|

### 7.5 Three-valued logic (Kleene K3)

Any number is treated as true iff its rational value is non-zero. Nil
acts as Unknown.

| Word | Sugar | Truth table                                           |
|------|-------|-------------------------------------------------------|
| AND  | `&`   | False dominates Nil; Nil dominates True.              |
| OR   | `|`   | True dominates Nil; Nil dominates False.              |
| NOT  | `!`   | `T → F`, `F → T`, `Nil → Nil`.                        |

### 7.6 Nil and output

| Word  | Stack effect            | Description                          |
|-------|-------------------------|--------------------------------------|
| NIL   | `— Nil`                 | Push a Nil bubble.                   |
| NIL?  | `x — (1|0)`             | Test for Nil.                        |
| `.`   | `x —`                   | Append `x` to the output buffer.     |

### 7.7 Definition

| Word  | Form                  | Description                                    |
|-------|-----------------------|------------------------------------------------|
| DEF   | `DEF NAME body…`      | Define a user word from the rest of the chunk. |
| DEL   | `DEL NAME`            | Remove a user word.                            |

---

## 8. Maintained design properties

These properties are preserved across phases and form the acceptance
criteria for later work:

* GUI layout and operability.
* Input assistance.
* GUI visualisation of internal state, including the Register area.
* English-rooted words with symbolic sugar.
* Semantic-plane separation of internal representation from display.
* Three-layer error messages.
* The water metaphor for value flow.
* The brain / memory-architecture metaphor for state location.
* Nil as bubble (propagating absence).
* Module system (Phase 3+ target).
* Three-valued logic via Nil.
* DO-178B–style requirement-traceable test design.
* `DEF` and `DEL` user-word management.

---

## 9. Discarded design properties

The following features from the legacy Ajisai are deliberately removed
without backwards-compatibility shims:

* Operation-target / consumption modes.
* `COND` and other Ajisai-specific control structures inherited from
  the vector dialect.
* The return stack (never introduced).
* The standalone `>` symbol and the legacy `=>` NIL-coalescing operator.

---

## 10. Phase plan

| Phase | Scope |
|-------|-------|
| 1 | Continued-fraction values, stack, four arithmetic ops, DEF/DEL, Nil, three-layer errors, GUI compatibility. |
| **2 (this file)** | **Single Register slot with STORE/RECALL/PEEK, comparison words (EQ/NE/LT/LE/GE/GT), three-valued logic (AND/OR/NOT), GUI Register area, expanded test coverage.** |
| 3 | Caller-clobbers static linter for Register, lexical quotations and IF combinator, modules. |
| 4 | Tensors as continued-fraction coefficients, exact irrational numbers, AI-explainable diagnostics. |

Each later phase must preserve the maintained design properties of §8.
