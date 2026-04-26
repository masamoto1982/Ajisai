# Ajisai Language Specification

Status: **Canonical**
Version: **2026-04-13**

This document is the single design authority for Ajisai. It describes Ajisai as it is. It does not record development history or transitional states. If any other document conflicts with this document, this document takes precedence.

---

## 1. Language Identity

Ajisai is an **AI-first, vector-oriented, fractional-dataflow language**.

Runtime stack:
- Rust interpreter core
- WASM boundary
- TypeScript GUI/runtime shell

Ajisai is designed for mechanical reasoning, automated refactoring, and structurally searchable implementation.

---

## 2. Specification Authority

### 2.1 Canonical

1. This file (`SPECIFICATION.md`)
2. Rust implementation behavior conforming to this file
3. WASM/TypeScript observable contracts derived from this file

### 2.2 Non-canonical

Any roadmap, handover note, TODO note, or design memo is non-canonical unless explicitly promoted here. Secondary documents must not define competing semantics and must not be treated as specification.

---

## 3. Syntax

### 3.1 Token types

| Token | Description |
|-------|-------------|
| Number | Numeric literal (see 3.2) |
| String | Single-quoted text `'...'` |
| Symbol | Word name (all non-whitespace characters excluding reserved chars) |
| `[` `]` | Vector boundaries |
| `{` `}` or `(` `)` | Code block boundaries |
| `==` | Syntactic sugar for `PIPE` (visual pipeline marker, no-op at runtime) |
| `=>` | Syntactic sugar for `OR-NIL` (NIL coalescing) |
| `$` | COND clause separator |
| `~` | Syntactic sugar for `SAFE` (safe mode modifier) |
| `#` | Line comment: all characters from `#` to end of line are ignored |

### 3.2 Numeric literal formats

| Format | Example |
|--------|---------|
| Integer | `42`, `-7` |
| Fraction | `3/4`, `-5/2` |
| Decimal | `3.14`, `.5`, `-1.0` |
| Scientific notation | `1e5`, `1.5e-2`, `-3.0e4` |

All numeric literals are parsed as exact rational numbers.

### 3.3 String literals

A string literal begins with `'` and ends with the last `'` before a token boundary. A token boundary is whitespace, end of input, or any special character other than `'` (such as `[`, `]`, `{`, `}`, `(`, `)`, `#`, `=`, `~`, `$`).

Any `'` that appears before a non-boundary character is a literal quote character in the string content.

Examples:

| Source | String value |
|--------|-------------|
| `'hello'` | `hello` |
| `'it's'` | `it's` |
| `'hel''lo'` | `hel''lo` |
| `'これは'テスト'です'` | `これは'テスト'です` |

### 3.4 Code blocks

A sequence of tokens enclosed in `{...}` or `(...)`. A code block must be written on a single line.

### 3.5 Vectors

A sequence of values enclosed in `[...]`.

### 3.6 COND clauses

Inside a `COND` expression, clauses are separated by `$`. Each clause must occupy exactly one line.

### 3.7 Syntax constraints

- All bracket pairs (`[`, `{`, `(`) must be balanced.
- Code blocks (`{...}`, `(...)`) must be on a single line.
- Each COND clause must occupy exactly one line.

### 3.8 Word name normalization

Word names are normalized to uppercase at runtime. `add` and `ADD` refer to the same word.

---

## 4. Value Model

### 4.1 Value types

| Type | Description |
|------|-------------|
| Scalar | An exact rational number |
| Vector | An ordered, indexable sequence of values (may be nested) |
| Record | An ordered set of named fields (string keys) |
| NIL | The absence of a value |
| CodeBlock | An executable sequence of tokens |
| ProcessHandle | A reference to a running child runtime |
| SupervisorHandle | A reference to a supervisor |

### 4.2 Scalar: exact rational arithmetic

All numeric values are represented as exact fractions.

Internal representation:
- `Small(numerator: i64, denominator: i64)` for values within i64 range
- `Big(numerator: BigInt, denominator: BigInt)` for larger values

Normalization rules:
- Always reduced to lowest terms via GCD.
- Denominator is always positive.

Numeric display is guided by the semantic plane (see Section 12) and does not affect the stored value.

### 4.3 Vector

An ordered, indexable sequence of values. Vectors may be nested (tensor-like). Index base is 0. Negative indices count from the end: `-1` is the last element.

### 4.4 Record

A collection of named fields. Each field has a string key and an associated value. Field insertion order is preserved.

### 4.5 NIL

NIL represents the absence of a value. It is pushed by safe mode when an error is absorbed, and produced by operations that yield no meaningful result.

### 4.6 CodeBlock

An executable sequence of tokens. CodeBlocks are first-class values: they can be stored on the stack, passed to higher-order words, and executed with `EXEC`.

### 4.7 ProcessHandle and SupervisorHandle

References to child runtimes and supervisors respectively. Created by `SPAWN` and `SUPERVISE`. Used with the child runtime API (see Section 10).

---

## 5. Stack

### 5.1 Structure

Ajisai maintains a single mutable stack of values. Execution proceeds by pushing and popping values. The stack is ordered; the most recently pushed value is the top.

### 5.2 Two-plane architecture

The runtime is divided into two planes:

**Data plane**: Holds `ValueData` payloads. All arithmetic, comparison, and structural operations execute entirely on the data plane. The data plane contains no display or formatting metadata.

**Semantic plane**: Holds display hints and presentation metadata keyed by stack position. Consulted only at explicit semantic boundaries: rendering, output operations, and module side effects.

These planes are strictly separate. Semantic plane contents do not influence data plane computations.

### 5.3 Execution step limit

Each execution has a step budget. The default limit is 100,000 steps. Exceeding the limit raises `ExecutionLimitExceeded`. This is a runtime safety control, not a language semantic constraint.

---

## 6. Modifiers

Modifiers precede a word and alter its execution behavior. Multiple modifiers may be combined.

### 6.1 Target modifiers

| Modifier | Name | Behavior |
|----------|------|----------|
| `.` | StackTop (default) | The word operates on the top value(s) of the stack |
| `..` | Stack | The entire stack contents are treated as the operand |

### 6.2 Consumption modifiers

| Modifier | Name | Behavior |
|----------|------|----------|
| `,` | Consume (default) | Operands are removed from the stack after the operation |
| `,,` | Keep / Bifurcation | Operands are retained; the result is also pushed |

### 6.3 Safe mode modifier

| Modifier | Name | Behavior |
|----------|------|----------|
| `~` | Safe | If the operation raises an error, NIL is pushed instead of propagating the error |

### 6.4 Modifier combinations

All modifier combinations are explicit and mechanically testable. Combined forms such as `..,,` and `~..` are valid.

### 6.5 Additional syntax forms

All built-in words have English-word-based canonical names (see Section 7). The forms below are syntactic sugar that the tokenizer maps to the corresponding canonical word; either spelling is accepted in source code and behaves identically at runtime.

| Form | Canonical word | Behavior |
|------|----------------|----------|
| `==` | `PIPE` | Visual pipeline separator; no runtime effect |
| `=>` | `OR-NIL` | If the top of the stack is NIL, replace it with the next stack value |
| `!` | `FORC` | Overrides protection checks when redefining or deleting words that have dependents |
| `?` | `LOOKUP` | Display the definition of a word (see Section 7.8) |

---

## 7. Built-in Words

Built-in words are predefined and cannot be redefined or deleted.

### 7.0 English-word-based naming

All built-in words — both Core words and module dictionary words — use English-word-based canonical names. Symbol forms (such as `+`, `-`, `*`, `/`, `%`, `=`, `<`, `<=`, `&`, `==`, `=>`, `?`, `!`, `.`, `..`, `,`, `,,`, `~`) are syntactic sugar that the tokenizer maps to canonical English names. The canonical name is the authoritative identifier; the symbol form is convenience surface syntax. Any new built-in word must be introduced under an English-word-based canonical name.

| Canonical | Sugar | Canonical | Sugar |
|-----------|-------|-----------|-------|
| `ADD` | `+` | `TOP` | `.` |
| `SUB` | `-` | `STAK` | `..` |
| `MUL` | `*` | `EAT` | `,` |
| `DIV` | `/` | `KEEP` | `,,` |
| `MOD` | `%` | `SAFE` | `~` |
| `EQ` | `=` | `FORC` | `!` |
| `LT` | `<` | `PIPE` | `==` |
| `LTE` | `<=` | `OR-NIL` | `=>` |
| `AND` | `&` | `LOOKUP` | `?` |

### 7.1 Vector operations

| Word | Description |
|------|-------------|
| `LENGTH` | Number of elements in a vector, or total count of all stack values |
| `GET` | Retrieve element at a given index |
| `INSERT` | Insert element at a given index |
| `REPLACE` | Replace element at a given index |
| `REMOVE` | Remove element at a given index |
| `CONCAT` | Concatenate two or more vectors |
| `REVERSE` | Reverse the order of elements |
| `RANGE` | Generate a sequence of integers from start to end with optional step |
| `TAKE` | Take the first N elements |
| `SPLIT` | Split a vector into sub-vectors by given sizes |
| `REORDER` | Reorder elements according to an index list; supports duplication and negative indices |
| `COLLECT` | Gather all current stack values into a single vector |
| `SORT` | Sort elements in ascending order (exact fraction comparison) |

### 7.2 Tensor operations

Tensor operations operate on nested vectors treated as multi-dimensional arrays.

| Word | Description |
|------|-------------|
| `SHAPE` | Return the size of each dimension as a vector |
| `RANK` | Return the number of dimensions |
| `RESHAPE` | Reshape to new dimension sizes |
| `TRANSPOSE` | Transpose a 2D tensor |
| `FILL` | Create a tensor of given shape filled with a value |
| `FRAME` | Collect stack values into a tensor frame |

Tensor operation implementations must follow the staged pipeline:
1. Flatten input
2. Compute shape, stride, and index metadata
3. Transform indices or selections
4. Rebuild output

Ad hoc recursive shape mutation in intermediate stages is prohibited.

### 7.3 Arithmetic

| Word | Description |
|------|-------------|
| `+` | Addition |
| `-` | Subtraction |
| `*` | Multiplication |
| `/` | Exact rational division |
| `MOD` / `%` | Remainder |
| `FLOOR` | Floor (largest integer ≤ value) |
| `CEIL` | Ceiling (smallest integer ≥ value) |
| `ROUND` | Round to nearest integer |

### 7.4 Comparison

| Word | Description |
|------|-------------|
| `<` | Less than |
| `<=` | Less than or equal |
| `=` | Equal |

Comparisons return a boolean (true/false encoded as Scalar with Boolean display hint).

### 7.5 Logic

| Word | Description |
|------|-------------|
| `AND` / `&` | Logical AND |
| `OR` | Logical OR |
| `NOT` | Logical NOT |
| `TRUE` | Push boolean true |
| `FALSE` | Push boolean false |
| `NIL` | Push NIL |

### 7.6 String and type conversion

| Word | Description |
|------|-------------|
| `STR` | Convert value to its string representation |
| `NUM` | Parse a string as a number |
| `BOOL` | Convert to boolean |
| `CHR` | Convert a number to its Unicode character |
| `CHARS` | Split a string into a vector of individual characters |
| `JOIN` | Join a vector of strings, with optional separator |

### 7.7 Control and higher-order words

| Word | Description |
|------|-------------|
| `MAP` | Apply a code block to each element, collecting results |
| `FILTER` | Keep elements for which a predicate returns true |
| `FOLD` | Reduce a sequence to a single value using an accumulator |
| `UNFOLD` | Generate a sequence by repeatedly applying a generator block |
| `ANY` | True if at least one element satisfies the predicate |
| `ALL` | True if all elements satisfy the predicate |
| `COUNT` | Count elements satisfying the predicate |
| `SCAN` | Like FOLD but returns all intermediate accumulator values |
| `COND` | Evaluate clauses separated by `$`; execute the first whose condition is true |
| `IDLE` | No-op; does nothing |
| `EXEC` | Execute a code block |
| `EVAL` | Parse and execute a string as Ajisai code |

### 7.8 User word dictionary

| Word | Description |
|------|-------------|
| `DEF` | Define a user word (see Section 8) |
| `DEL` | Delete a user word (see Section 8) |
| `LOOKUP` | Look up and display the definition of a word (sugar: `?`) |

### 7.9 IO and utilities

| Word | Description |
|------|-------------|
| `PRINT` | Output the top stack value |
| `NOW` | Push the current timestamp |
| `DATETIME` | Format a timestamp as a datetime string |
| `TIMESTAMP` | Parse a datetime string to a timestamp |
| `CSPRNG` | Push a cryptographically secure random number |
| `HASH` | Compute a hash of the top stack value |

### 7.10 Module loading

| Word | Description |
|------|-------------|
| `IMPORT` | Load all words from a module into the current scope |
| `IMPORT-ONLY` | Load only the specified words from a module |

### 7.11 Child runtime words

| Word | Description |
|------|-------------|
| `SPAWN` | Create and start a child runtime from a code block; push a ProcessHandle |
| `AWAIT` | Wait for a child to finish; push `[status result-stack]` |
| `STATUS` | Return the current state of a child runtime as a string |
| `KILL` | Terminate a child runtime |
| `MONITOR` | Mark a child runtime for monitoring |
| `SUPERVISE` | Create a supervisor over a group of child runtimes |

---

## 8. User Words

### 8.1 Definition syntax

```
{ tokens... } 'NAME' DEF
{ tokens... } 'NAME' 'description' DEF
```

A code block followed by a name string defines a user word in the active dictionary. An optional description string may follow the name. Multiple consecutive code blocks on the stack are merged before definition. A vector may also serve as the definition body.

### 8.2 Rules

- User words are stored per active dictionary (namespace).
- Built-in words cannot be redefined.
- A user word that has active dependents requires the force modifier `!` to be redefined.
- Dependencies are tracked automatically at definition time.

### 8.3 Deletion syntax

```
'NAME' DEL
'DICT@NAME' DEL
```

Deletes a user word. The force modifier `!` is required if other words depend on the word being deleted.

### 8.4 Recursion

User words may call themselves or other user words recursively. There is no hard-coded call-depth limit as a language semantic rule.

### 8.5 Naming conventions

Word names follow action-object convention (e.g., `APPLY-GAIN`, `RESOLVE-PATH`). The runtime emits a warning for:

- Ambiguous prefixes: `DO-`, `HANDLE-`, `PROCESS-`, `MANAGE-`, `UTIL-`, `HELPER-`
- Ambiguous standalone names: `CALC`, `RUN`, `EXEC2`, `TEMP`, `MAIN`, `TEST`, `STUFF`, `THING`

Acceptable forms: `IS-*` and `HAS-*` predicates; hyphen-separated action-object names; short unambiguous names (6 characters or fewer).

---

## 9. Module System

### 9.1 Available modules

| Module | Purpose |
|--------|---------|
| `MUSIC` | Audio sequencing and synthesis |
| `JSON` | JSON parsing, generation, and manipulation |
| `IO` | Standard input/output |

### 9.2 Import syntax

```
'MODULE-NAME' IMPORT
'MODULE-NAME' [ 'WORD1' 'WORD2' ] IMPORT-ONLY
```

`IMPORT` loads all words from a module into the current scope. `IMPORT-ONLY` loads only the specified words.

### 9.3 Module-provided sample words

Modules may provide sample user words for demonstration. Sample words require the force modifier `!` to delete.

---

## 10. Child Runtime

### 10.1 Overview

A child runtime is an isolated interpreter instance spawned from a code block. At spawn time, the child receives a snapshot of the parent dictionary. The child does not share the parent's stack or dictionary during execution. Parent and child are isolated.

### 10.2 Child states

| State | Meaning |
|-------|---------|
| `running` | Currently executing |
| `completed` | Finished normally |
| `failed` | Terminated with an error |
| `killed` | Terminated by `KILL` |
| `timeout` | Step limit exceeded |

### 10.3 SPAWN

Accepts a code block from the stack. Starts the child runtime. Pushes a ProcessHandle.

### 10.4 AWAIT

Accepts a ProcessHandle. Blocks until the child finishes. Pushes `[status result-stack]` where `status` is a string and `result-stack` is a vector of the child's final stack values.

### 10.5 STATUS

Accepts a ProcessHandle. Returns the current state as a string without blocking.

### 10.6 KILL

Accepts a ProcessHandle. Terminates the child runtime immediately.

### 10.7 MONITOR and SUPERVISE

`MONITOR` marks a child runtime for observation. `SUPERVISE` creates a SupervisorHandle that manages a group of child runtimes.

---

## 11. Error Model

### 11.1 User-level error categories

| Error | Trigger condition |
|-------|-------------------|
| `StackUnderflow` | Insufficient values on the stack for the operation |
| `StructureError` | Value type does not match what the operation requires |
| `UnknownWord` | Referenced word is not defined in any accessible dictionary |
| `UnknownModule` | Referenced module is not available |
| `DivisionByZero` | Divisor is zero |
| `IndexOutOfBounds` | Index is outside the valid range of the vector |
| `VectorLengthMismatch` | Operation requires equal-length vectors but lengths differ |
| `ExecutionLimitExceeded` | Step budget exhausted |
| `ModeUnsupported` | The modifier combination is not supported for this word |
| `BuiltinProtection` | Attempt to redefine or delete a built-in word |
| `CondExhausted` | COND expression has no matching clause |
| `Custom` | Explicit error raised by user code |

### 11.2 Internal flow-level error categories

The following errors are raised by the internal flow conservation mechanism. They are not user-level language constructs and are not observable in normal execution.

| Error | Trigger condition |
|-------|-------------------|
| `OverConsumption` | More flow mass consumed than available |
| `UnconsumedLeak` | Flow mass not fully consumed at an operation boundary |
| `FlowBreak` | Flow conservation invariant violated |
| `BifurcationViolation` | Sum of child flow masses does not equal parent flow mass |

### 11.3 Equal-value output

Operations that produce a value equal to their input are successful. Equal-value output is not an error.

### 11.4 Safe mode behavior

Any operation preceded by `~` absorbs its own errors locally. If an error is raised, NIL is pushed in place of the result. The error does not propagate.

---

## 12. Semantic Plane

### 12.1 Purpose

The semantic plane holds display hints for each stack position. It is separate from the data plane and does not influence computation.

### 12.2 Display hints

| Hint | Meaning |
|------|---------|
| `Auto` | Determine display from value type at render time |
| `Number` | Display as a number |
| `String` | Display as a string |
| `Boolean` | Display as a boolean |
| `DateTime` | Display as a formatted datetime |
| `Nil` | Display as NIL |

Display hints are applied only at explicit semantic boundaries: rendering, `PRINT`, and module-level output operations.

---

## 13. Fractional-Dataflow Internal Invariants

### 13.1 Flow tokens

The runtime optionally tracks flow mass via internal FlowToken objects. Each FlowToken holds:
- A unique runtime-assigned ID
- Total mass and remaining mass
- Value shape information
- Parent and child flow relationships
- Mass ratio for bifurcations

FlowToken state is an **internal runtime invariant**. It is not a user-visible language construct and must not appear as default runtime output.

### 13.2 Bifurcation

The `,,` (keep/bifurcation) modifier retains source context while also pushing the result. Mass ratio and branch conservation details are internal. Optional diagnostics may surface FlowToken information, but only when diagnostic visibility is explicitly enabled.

---

## 14. AI-first Implementation Rules

### 14.1 Mandatory

- Prefer explicit, structurally searchable function and module names.
- Keep Rust source files under 500 lines.
- Keep control flow shallow and phase-separated.
- Separate semantic changes from structural cleanup in change management.
- Maintain single canonical implementations; do not allow dual-mode drift.
- Source code files must contain no inline comments or block comments. All explanatory text must reside in external specification and documentation files.

### 14.2 Advisory

- Prefer small helper extraction for duplicated control scaffolding.
- Prefer deterministic, low-ambiguity error classification.
- Prefer mechanically enforceable tests over narrative documentation.

---

## 15. Conformance Checklist

A change is conformant only if all of the following hold:

1. It does not introduce a second design authority.
2. It does not treat equal-value output as a runtime error.
3. It does not impose hard-coded call-depth limits as language semantics.
4. It preserves data-plane/semantic-plane separation.
5. It keeps vector/tensor staged pipeline boundaries explicit.
6. It improves or preserves AI-first structural clarity.
7. Every built-in word (Core or module) introduced or renamed has an English-word-based canonical name; any symbolic form is registered as syntactic sugar that maps to that canonical name.
