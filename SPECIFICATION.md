# Ajisai Language Specification

Status: **Canonical**
Version: **2026-05-12 (Phase 1 redesign)**

This document is the single design authority for Ajisai. Earlier versions
described a vector-oriented, fraction-only dialect; this version replaces
that design with the continued-fraction stack model described below.
Where this file conflicts with any other document, this file takes
precedence. The historical `CLAUDE.md` and any older specification
materials are non-canonical.

---

## 1. Identity

Ajisai is an **AI-first, stack-oriented dataflow language** in the Forth
lineage.

* Every numeric value is stored internally as a **finite continued
  fraction** of partial quotients drawn from arbitrary-precision integers
  (Phase 1) and, in later phases, tensors. This gives exact arithmetic on
  rational numbers and a path to exact representation of selected
  irrational numbers.
* Programs manipulate a single **data stack**. There is no return stack;
  control flow is expressed entirely by data on the stack. Avoiding mid-
  computation memos is a deliberate VTU (Very Thrifty Use) energy goal.
* Words are **English-rooted**, with selected symbols (`+`, `-`, `*`,
  `/`, `.`) acting as syntactic sugar for the underlying English-named
  core words.
* Internal representation is hidden behind a **semantic plane** so that
  observable behaviour does not depend on the in-memory layout.

Runtime stack:

* Rust interpreter core (`rust/`)
* WASM boundary (`src/wasm/generated/`)
* TypeScript GUI shell (`src/`)

---

## 2. Specification Authority

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
* GUI colours, CSS class names, and display strings.
* User-word storage layout.

External consumers (the GUI, AI tooling, automated tests) must use the
machine-readable protocol fields described in §6 rather than human-
readable display strings or internal type names.

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

There are no string, vector, or code-block literals in Phase 1. Those
syntactic forms are reserved for later phases.

### 3.2 Reserved heads

Two symbols receive special treatment by the parser:

* `DEF NAME body…` — capture the rest of the current execution chunk as
  the body of a new user word named `NAME` (see §5.2).
* `DEL NAME` — remove the user word named `NAME`.

All other symbols are looked up in the dictionary (core words first,
then user words).

---

## 4. Values

### 4.1 Continued fractions

A continued fraction is the finite list of partial quotients
`[a₀, a₁, …, aₙ]` representing the value

```
a₀ + 1 / (a₁ + 1 / (a₂ + … + 1/aₙ))
```

The canonical display form is the nested parenthesised form

```
(a₀ (a₁ (a₂ … (aₙ))))
```

Examples:

| Value     | Nested form         |
|-----------|---------------------|
| `42`      | `(42)`              |
| `3/4`     | `(0 (1 (3)))`       |
| `13/4`    | `(3 (4))`           |
| `355/113` | `(3 (7 (16)))`      |

Internally, an empty list of partial quotients encodes **Nil**.

### 4.2 Canonicalisation

A continued fraction with more than one partial quotient is canonical
when its last quotient is greater than or equal to `2`. The pair
`[…, a, 1]` is rewritten as `[…, a+1]` because both denote the same
value.

### 4.3 Nil (the bubble)

Nil represents structured absence. Nil propagates:

* Any arithmetic operation that takes Nil as an operand returns Nil.
* `1 0 /` returns Nil (division by zero).

Nil supports a basic three-valued check via `NIL?`, which pushes `1` if
the top of the stack is Nil and `0` otherwise.

---

## 5. Execution model

### 5.1 The data stack

Execution maintains a single LIFO stack of values. Literals push onto
the stack; words consume operands from the stack and push results back.
There is no return stack: user-word calls are inlined by re-executing
the stored body source.

### 5.2 Word definition

```
DEF NAME body…
```

`DEF` consumes the remainder of the current execution chunk as the body
text of a new user word called `NAME` (uppercased). Calling `NAME`
re-executes the body source in the current interpreter.

`DEF` overwrites any existing user word of the same name. Re-defining a
core word is not permitted (the symbol resolves to the core word first).

### 5.3 Word deletion

```
DEL NAME
```

Removes a user word. Deleting an undefined or core word is a no-op.

### 5.4 Errors

Errors are reported in three layers:

| Layer       | Audience            | Example                                            |
|-------------|---------------------|----------------------------------------------------|
| `summary`   | End user (one line) | `Stack underflow at +`                             |
| `detail`    | Experienced human   | `Word + requires 2 value(s) on the stack but…`     |
| `diagnosis` | AI / tooling        | `Check the operands feeding +: push 1 more value…` |

The GUI surfaces `summary`; deeper layers are exposed via the protocol
surface (see §6) for AI assistance.

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

`value` is the reduced rational view (kept for backwards-compatible
display); `continuedFraction` is the canonical nested form.

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
* `reset()` — clear stack and output buffer.
* `collect_stack()` — return the current stack as protocol values.
* `collect_user_words_info()` — return `[name, definition, description,
  isShadow]` rows.
* `collect_core_words_info()` — return `[name, hoverSummary,
  hoverSyntax]` rows.
* `collect_core_word_aliases_info()` — return aliases (`ADD`, `SUB`,
  `MUL`, `DIV`).
* `collect_input_helper_words_info()` — input-assist entries.
* `lookup_word_definition(name)` — return the body text of a user word,
  or `null`.
* `remove_word(name)` — convenience wrapper around `DEL`.
* `restore_user_words(words)` — bulk register `DEF`-equivalent
  definitions for persistence.

Module-related methods (`collect_imported_modules`,
`collect_module_words_info`, …) return empty results in Phase 1 and
are reserved for Phase 2.

---

## 7. Core words (Phase 1)

| Word    | Stack effect            | Description                                |
|---------|-------------------------|--------------------------------------------|
| `+`     | `a b — (a+b)`           | Add two continued fractions.               |
| `-`     | `a b — (a-b)`           | Subtract `b` from `a`.                     |
| `*`     | `a b — (a*b)`           | Multiply two continued fractions.          |
| `/`     | `a b — (a/b)` or Nil    | Divide, returning Nil on division by zero. |
| `DUP`   | `a — a a`               | Duplicate the top.                         |
| `DROP`  | `a —`                   | Discard the top.                           |
| `SWAP`  | `a b — b a`             | Swap the top two items.                    |
| `OVER`  | `a b — a b a`           | Copy the second item on top.               |
| `NIL`   | `— Nil`                 | Push a Nil bubble.                         |
| `NIL?`  | `x — (1|0)`             | Test for Nil.                              |
| `.`     | `x —`                   | Append `x` to the output buffer.           |
| `DEF`   | reserved head           | Define a user word (§5.2).                 |
| `DEL`   | reserved head           | Delete a user word (§5.3).                 |
| `ADD`/`SUB`/`MUL`/`DIV` | — | English aliases for `+`/`-`/`*`/`/`.       |

---

## 8. Maintained design properties

These properties are preserved across the Phase 1 redesign and form the
acceptance criteria for later phases:

* GUI layout and operability.
* Input assistance (auto-complete of word names and punctuation).
* GUI visualisation of internal state.
* English-rooted words with symbolic sugar.
* Semantic-plane separation of internal representation from display.
* Three-layer error messages.
* The water metaphor for value flow.
* Nil as bubble (propagating absence).
* Module system (Phase 2 target).
* Nil-based three-valued logic.
* DO-178B–style requirement-traceable test design.
* `DEF` and `DEL` user-word management.

---

## 9. Discarded design properties

The following features from the legacy specification are deliberately
removed without a backwards-compatibility shim:

* Operation-target / consumption modes.
* `COND` and other Ajisai-specific control structures inherited from the
  vector dialect.
* The return stack (never introduced).

Source files, examples, and tests targeting these features have been
removed.

---

## 10. Phase plan

| Phase | Scope |
|-------|-------|
| 1 (this file) | Continued-fraction values, stack, four arithmetic ops, DEF/DEL, Nil, three-layer errors, GUI compatibility. |
| 2  | Modules, tensors as continued-fraction coefficients, richer comparison and control words. |
| 3  | Exact irrational numbers, AI-explainable diagnostics, expanded GUI affordances. |

Each later phase must preserve the maintained design properties of §8.
