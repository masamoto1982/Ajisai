# Ajisai Language Specification

Status: **Canonical**
Version: **2026-06-01**

This document is the single design authority for Ajisai. It describes Ajisai as it is. It does not record development history or transitional states. If any other document conflicts with this document, this document takes precedence.

Ajisai is a typed, vector-oriented dataflow language. Its safety story is the conjunction of:

- **Value-shape safety** — operations check that operands have the structural shape they require (Scalar / Vector / Record / NIL / CodeBlock / handles).
- **Encoding safety** — string and code values carry encoding contracts on top of their underlying fraction sequences.
- **Contract safety** — every Coreword has machine-readable `requires` / `ensures` / partiality / NIL policy / effect metadata in the registry.
- **NIL projection safety** — partial operations may project failure onto NIL with a structured reason; `SAFE` is the explicit projection operator.

These layers compose. A change is conformant only if it preserves all of them.

---

## 1. Language Identity

Ajisai is an **AI-first, vector-oriented, fractional-dataflow language**.

Every numeric value is an **exact real**, represented internally as a (possibly lazy) **continued fraction**. Finite continued fractions cover the rationals; lazy infinite continued fractions cover the algebraic and transcendental irrationals admitted by the runtime (e.g. `SQRT`). All arithmetic is performed on the continued-fraction representation directly, without intermediate rounding. Surface numeric literals (Section 3.2) are convenience forms for the same underlying representation.

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

### 2.3 Semantic Firewall

Ajisai separates internal representation from observable semantics. Internal representation may change freely; observable semantics must be accessed through semantic axes and protocol fields. Machine-readable consumers must use protocol fields only. Human-readable strings are non-canonical and may change.

The following are not part of Ajisai's observable semantics:

- Rust enum variant names
- Rust `Debug` output
- internal value representation
- display strings
- GUI colors
- CSS class names
- dictionary storage layout
- module file layout

Ajisai values are observed through independent semantic axes:

| Axis | Protocol field | Initial protocol strings |
|------|----------------|--------------------------|
| semantic kind | `semanticKind` | `number`, `collection`, `record`, `code`, `process`, `supervisor`, `absence`, `unknown` |
| shape | `shape` | `scalar`, `vector`, `tensor`, `record`, `codeBlock`, `handle`, `absence`, `unknown` |
| capabilities | `capabilities` | `numeric`, `exactNumeric`, `iterable`, `indexable`, `callable`, `stackItem`, `nilPassthrough`, `diagnosable`, `serializable`, `displayable`, `userEditable`, `moduleOwned`, `coreOwned`, `aiExplainable`, `truthValued` |
| truth value | `truthValue` | `true`, `false`, `unknown` |
| origin | `origin` | `literal`, `computed`, `coreWord`, `builtinWord`, `moduleWord`, `userWord`, `safeProjection`, `nilPropagation`, `hostEnvironment`, `optimizer`, `unknown` |
| absence metadata | `absence` | structured object |
| diagnostic context | `diagnosis` | structured object |
| display | `display` | human-readable only; non-canonical |
| serialization | `serialization` | explicit format contract only |

External APIs, WASM payloads, GUI logic, AI diagnostics, and user-facing machine-readable tooling must not branch on Rust enum names, `Debug` strings, display text, GUI colors, or storage layout. Protocol strings are lower camel case and are the canonical machine-readable surface.

The `truthValue` axis is present only on values carrying the `TruthValue` interpretation role (Section 12.2); such values also carry the `truthValued` capability. Its three protocol strings — `true`, `false`, `unknown` — are the **only** observable surface for the three-valued logic of Section 7.5. The third value `unknown` (U) is a logical truth value, not an operational absence: how it is represented internally (for example, whether it shares storage with a NIL node) is not observable, and consumers must not infer it from `semanticKind = absence`, from any `absence.reason`, or from display text. A consumer that needs to distinguish true, false, and unknown reads the `truthValue` axis and nothing else.

---

## 3. Syntax

### 3.1 Token types

| Token | Description |
|-------|-------------|
| Number | Numeric literal (see 3.2) |
| String | Single-quoted text `'...'` |
| Symbol | Word name (all non-whitespace characters excluding reserved chars) |
| `[` `]` | Vector boundaries |
| `{` `}` | Code block boundaries |
| `==` | Syntactic sugar for `PIPE` (visual pipeline marker, no-op at runtime) |
| `=>` | Syntactic sugar for `OR-NIL` (NIL coalescing) |
| `>` | Syntactic sugar for `GT` |
| `>=` | Syntactic sugar for `GTE` |
| `<>` | Syntactic sugar for `NEQ` |
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

All numeric literals are parsed as exact real numbers and stored internally as continued fractions (see Section 4.2). The surface literal forms above are convenience syntax: `42`, `42/1`, `42.0`, and `4.2e1` all produce the same internal value. Integer, fraction, decimal, and scientific-notation literals yield finite continued fractions (rationals); irrational continued fractions are produced by words such as `MATH@SQRT`, not by surface literals.

The nested-parentheses form `( a0 ( a1 ( a2 ... )))` is the canonical serialization and AI-readable debug form for continued fractions (Section 4.2). It is not a source-code literal: Ajisai source uses the surface forms above, and the nested form appears only in display and serialization output under the `ContinuedFraction` interpretation role (Section 12.2).

### 3.3 String literals

A string literal begins with `'` and ends with the last `'` before a token boundary. A token boundary is whitespace, end of input, or any special character other than `'` (such as `[`, `]`, `{`, `}`, `#`, `=`, `~`, `$`).

Any `'` that appears before a non-boundary character is a literal quote character in the string content.

Examples:

| Source | String value |
|--------|-------------|
| `'hello'` | `hello` |
| `'it's'` | `it's` |
| `'hel''lo'` | `hel''lo` |
| `'これは'テスト'です'` | `これは'テスト'です` |

### 3.4 Code blocks

A sequence of tokens enclosed in `{...}`. A code block must be written on a single line.

`(` and `)` are not Ajisai syntactic characters. They are reserved at the lexical level to prevent accidental reuse and to keep the nested continued-fraction serialization form (Section 4.2) unambiguous; encountering `(` or `)` in source text is a tokenizer error.

### 3.5 Vectors

A sequence of values enclosed in `[...]`.

### 3.6 COND clauses

Inside a `COND` expression, clauses are separated by `$`. Each clause must occupy exactly one line.

### 3.7 Syntax constraints

- All bracket pairs (`[`, `{`) must be balanced.
- Code blocks (`{...}`) must be on a single line.
- Each COND clause must occupy exactly one line.
- The characters `(` and `)` are not valid in Ajisai source (Section 3.4).

### 3.8 Word name normalization

Word names are normalized to uppercase at runtime. `add` and `ADD` refer to the same word.

---

## 4. Value Model

### 4.1 Value types

| Type | Description |
|------|-------------|
| Scalar | An exact real number, represented internally as a (possibly lazy) continued fraction |
| Vector | An ordered, indexable sequence of values (may be nested) |
| Record | An ordered set of named fields (string keys) |
| NIL | The absence of a value |
| CodeBlock | An executable sequence of tokens |
| ProcessHandle | A reference to a running child runtime |
| SupervisorHandle | A reference to a supervisor |

### 4.2 Scalar: exact-real continued-fraction arithmetic

All numeric values are exact reals represented as **continued fractions** (CF). A scalar is a sequence of integer partial quotients `[a0; a1, a2, ...]`, finite for rationals and lazy (potentially infinite) for irrationals.

#### 4.2.1 Canonical form

A continued fraction is in canonical form when:

1. `a0` is any integer.
2. `a1, a2, ...` are strictly positive integers.
3. If the sequence is finite and has length `n >= 2`, then the last partial quotient `a_{n-1}` is greater than `1` (no trailing `1`).
4. The sequence terminates iff the value is rational.

These rules give every real value a unique canonical CF, so equality of canonical CFs decides equality of values whenever the comparison terminates (Section 7.4).

#### 4.2.2 Internal representation

A scalar is internally one of the following representations. Which representation is used is not observable; only the canonical CF sequence and its value are.

- **Rational** — a finite CF, stored equivalently as either a small `(i64, i64)` reduced fraction or a `(BigInt, BigInt)` reduced fraction. Partial quotients are generated on demand by floor-division Euclidean algorithm.
- **AlgebraicSqrt** — `SQRT` of a non-negative rational. The CF expansion is eventually periodic (Lagrange's theorem) and produced lazily.
- **Gosper** — an unevaluated bihomographic transform `(a x y + b x + c y + d) / (e x y + f x + g y + h)` of one or two operand CFs, used by arithmetic (Section 7.3). Partial quotients of the result are emitted as soon as the next quotient is unambiguously determined by the current Möbius coefficients.
- **LazyCf** — any other lazy CF stream (reserved for future words; not produced by the Coreword set defined in this document).

Möbius coefficients used by Gosper transforms are stored as arbitrary-precision integers (BigInt) at all times. Implementations must not use bounded-width coefficient storage that can overflow during normal evaluation.

#### 4.2.3 Display and serialization

The canonical AI-readable serialization of a scalar is the **nested right-associative** form:

```
( a0 ( a1 ( a2 ... ( a_{n-1} ) ... )))
```

with one integer per nesting level and one closing `)` per opening `(`. A lazy infinite CF is serialized by emitting partial quotients up to an implementation-defined display budget and terminating with the marker `...)` before the unproduced quotients' closing parens; the truncated display is non-canonical and must not be parsed back as an exact value.

This nested form is **not** Ajisai source syntax (Section 3.4). It appears only in display and serialization output under the `ContinuedFraction` interpretation role (Section 12.2) and in AI-targeted diagnostics. Under the `RawNumber` role a rational scalar renders as a reduced `numerator/denominator` (Section 12.2); the surface literal style of Section 3.2 is convenience input syntax only and is not retained for display.

#### 4.2.4 Equivalence of representations

Two scalars are equal as values iff they produce the same canonical CF sequence. A `Rational` scalar and a `Gosper`/`AlgebraicSqrt` scalar may compare equal whenever their generated partial quotients agree at every position. The internal representation tag is not part of value identity and must not be branched on by Corewords or external consumers.

### 4.3 Vector

An ordered, indexable sequence of values. Vectors may be nested (tensor-like). Index base is 0. Negative indices count from the end: `-1` is the last element.

#### 4.3.1 Internal representation classes

A Vector value is internally represented in one of two classes:

- **nested** — a tree of `Value` elements (`Vec<Value>`). Any element type may appear, including mixed types (Scalars, Vectors, Strings, NIL, etc.).
- **dense** — a SIMD-oriented `DenseTensor` backed by Structure-of-Arrays numerator and denominator buffers, a shape, and a validity mask. Every valid lane is an exact small Fraction; an invalid lane represents NIL occupancy without rebuilding the dense representation into nested `Vec<Value>` form.

The class is chosen at construction time. Observable semantics — `Display`, ordering, equality, NIL-ness, `SHAPE`, `LENGTH`, indexing, iteration order — are identical between the two classes. Operations are free to take a fast path when the input is dense.

Dense Tensor exactness rule:
- Lanes admit only scalars whose canonical CF (Section 4.2) is finite and whose equivalent reduced rational fits within `(i64 numerator, i64 denominator)`. These are stored in normalized form.
- Scalars whose CF is infinite (irrational), or whose rational-equivalent requires BigInt storage, are exact values but are not admitted to the small-lane `DenseTensor` representation. They live in the nested `Vec<Value>` class until a CF-capable SoA representation is introduced.
- Implementations must not truncate, round, or otherwise approximate a scalar to fit a dense lane. Any value that does not satisfy the lane admission rule causes the construction to fall back to the nested class.

**No-Rebuild Principle:** a dense Vector never degrades to a nested Vector solely because a lane becomes NIL. NIL occupancy is represented by clearing the corresponding validity-mask bit. Internal diagnostic reasons for invalid lanes are stored outside the dense payload in an execution-context sparse registry keyed by tensor identity and lane index; they are not embedded in `DenseTensor`.

Equality across classes: a dense Vector and a nested Vector compare equal when (1) flattening the nested Vector into its leaf Fractions/NIL lanes yields the same lane sequence and validity as the dense buffer, and (2) the shape inferred from the nested Vector matches the dense Vector's shape. No language-visible word distinguishes the two classes; they are interchangeable from the user's perspective.

This dual representation exists for the Virtual Tensor Unit (VTU) data-movement optimizations (see `docs/dev/virtual-tensor-unit-design.md`). It does not introduce approximate numeric types: all numeric leaves remain exact-real continued fractions (Section 4.2). The small-lane `DenseTensor` is an optimization for the rational sub-domain only; SIMD vectorization of CF arithmetic over lazy lanes is out of scope for this specification.

### 4.4 Record

A collection of named fields. Each field has a string key and an associated value. Field insertion order is preserved.

### 4.5 NIL

NIL represents the absence of a value. It is pushed by safe mode when an error is absorbed, and produced by operations that yield no meaningful result.

#### 4.5.0 Diagnostic absence metadata

NIL is a diagnostic absence value. NIL identity is separate from its reason, origin, recoverability, and diagnostic context. Surface display, equality, hashing, and serialization treat all NIL values uniformly unless a protocol explicitly asks for diagnostic metadata.

For any NIL value:

- `semanticKind` is `absence`
- `shape` is `absence`
- `capabilities` includes `diagnosable` and `nilPassthrough`
- the human-readable display text `NIL` is non-canonical and must not be used for machine decisions

NIL metadata is exposed as an optional structured `absence` object with these fields:

| Field | Meaning | Machine-readable contract |
|-------|---------|---------------------------|
| `reason` | Direct reason the value became NIL | Optional lower camel case protocol string |
| `origin` | Path by which the NIL was produced | Required lower camel case protocol string |
| `recoverability` | UI/AI hint for next action | Required lower camel case protocol string |
| `caughtCategory` | Original error category caught by `SAFE`, when applicable | Optional lower camel case protocol string |
| `diagnosis` | Three-layer debug diagnosis | Optional structured object |

`NIL?` checks only whether a value is absent. It must not branch on `absence.reason`. Reason-specific code must use explicit diagnostic accessors such as `NIL-REASON`, `NIL-ORIGIN`, `NIL-RECOVERABLE?`, or `NIL-DIAGNOSIS` when such words are available.

The diagnostic object uses the existing three-layer model:

| Diagnostic field | Meaning | Contract |
|------------------|---------|----------|
| `when` | phase where the event occurred | lower camel case protocol string |
| `where.kind` | locus kind | lower camel case protocol string |
| `where.word` / `where.module` / `where.dictionary` | optional locus details | strings, not enum names |
| `why` | cause class | lower camel case protocol string |
| `summary` | human-readable explanation | non-canonical; do not parse |
| `evidence` | human-readable evidence list | non-canonical; do not parse |
| `nextChecks` | suggested checks for UI/AI display | structured label/detail strings |
| `agreedPrefix` | for a continued-fraction comparison that produced `Unknown` (Section 7.4.1): the number of leading partial quotients that matched before the budget was exhausted | optional non-negative integer; machine-readable |

SAFE-caught errors use `reason = safeCaught` and preserve the original error category in `caughtCategory`, for example `caughtCategory = structureError`. Direct Bubble/NIL results use their own reason and are not wrapped as `safeCaught`. Literal NIL has `origin = literal` and no `reason` unless a future protocol explicitly adds one.

#### 4.5.1 NIL passthrough

Operations classified as **NIL-passthrough** in Section 7 do not raise `StructureError` when a NIL operand is encountered. Instead, they produce NIL. The rule is uniform across consumption modes and target modes: if any operand consumed by the operation is NIL, the operation consumes its operands as it normally would and pushes a single NIL result.

NIL-passthrough applies to arithmetic, comparison, and the unary numeric rounding words (see Section 7.13). It does not apply to control-flow words, type-conversion words, IO words, or to `OR-NIL` (`=>`) itself, whose entire purpose is to react to NIL.

The intent is that pipelines built with safe mode (`~`) propagate NIL through subsequent computation without crashing, so that a single `=>` at the end of the pipeline can supply a fallback value.

When a NIL-passthrough operation receives one or more NIL operands, the resulting NIL inherits the reason of the leftmost NIL operand that carried a reason. This makes the cause traceable through long pipelines.

#### 4.5.2 NIL versus Unknown

NIL and the logical truth value `Unknown` (U, Section 7.5) are distinct and must not be conflated. **NIL is an operational absence**: a diagnostic bubble that records *why* a value is missing (division by zero, out-of-range `GET`, parse failure, a `SAFE`-caught error). **U is a logical undecidability**: a definite member of the three-valued truth domain that records that a proposition could not be settled true or false (notably a continued-fraction comparison that did not decide within its budget, Section 7.4.1). U carries the `TruthValue` role and is observed as `truthValue = unknown`; NIL is observed as `semanticKind = absence`.

When NIL and U meet in the same operation, **NIL takes priority**. NIL carries a diagnostic `reason` that must be preserved (`PreservesReason`, Section 7.14), so the stronger, reason-bearing operational information is never erased by the logical value. Concretely, a logic word (Section 7.5) that receives both a NIL operand and a U operand applies its NIL handling and produces NIL, not U, unless an absorbing definite operand (`false` for `AND`, `true` for `OR`) settles the result first.

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

**Semantic plane**: Holds interpretation roles and presentation metadata keyed by stack position. Consulted only at explicit semantic boundaries: rendering, output operations, and module side effects.

These planes are strictly separate. Semantic plane contents do not influence data plane computations.

### 5.3 Execution step limit

Each execution has a step budget. The default limit is 100,000 steps. Exceeding the limit raises `ExecutionLimitExceeded`. This is a runtime safety control, not a language semantic constraint.

---

## 6. Modifiers

Modifiers precede a word and alter its execution behavior. Multiple modifiers may be combined.

### 6.1 Target modifiers

| Canonical | Sugar | Behavior |
|-----------|-------|----------|
| `TOP` | `.` | The word operates on the top value(s) of the stack (default) |
| `STAK` | `..` | The entire stack contents are treated as the operand |

### 6.2 Consumption modifiers

| Canonical | Sugar | Behavior |
|-----------|-------|----------|
| `EAT` | `,` | Operands are removed from the stack after the operation (default) |
| `KEEP` | `,,` | Operands are retained; the result is also pushed |

### 6.3 Safe mode modifier

| Canonical | Sugar | Behavior |
|-----------|-------|----------|
| `SAFE` | `~` | If the operation raises an error, NIL is pushed instead of propagating the error |

### 6.4 Modifier combinations

All modifier combinations are explicit and mechanically testable. Combined forms such as `..,,` and `~..` are valid.

### 6.5 Additional syntax forms

All built-in words have English-word-based canonical names (see Section 7). The forms below are syntactic sugar that the tokenizer maps to the corresponding canonical word; either spelling is accepted in source code and behaves identically at runtime.

| Canonical | Sugar | Behavior |
|-----------|-------|----------|
| `PIPE` | `==` | Visual pipeline separator; no runtime effect |
| `OR-NIL` | `=>` | If the top of the stack is NIL, replace it with the next stack value |
| `FORC` | `!` | Overrides protection checks when redefining or deleting words that have dependents |
| `LOOKUP` | `?` | Display the definition of a word (see Section 7.8) |

---

## 7. Built-in Words

Ajisai vocabulary follows the rule: **Core is permanent, Module is detachable, User is editable.**

Built-in words are predefined and cannot be redefined or deleted.

- **Core words** belong to the Ajisai runtime itself. They are always available and cannot be deleted, hidden, unimported, or redefined.
- **Module words** belong to a module dictionary. Their definitions are built in and cannot be redefined or destructively deleted, but their visibility in the current vocabulary is controlled with `IMPORT`, `IMPORT-ONLY`, `UNIMPORT`, and `UNIMPORT-ONLY`.
- **User words** belong to a user dictionary. They are editable, and deletion or redefinition is controlled by dependency checks and the force modifier where applicable.

Every built-in word has exactly one **canonical home**, either Core or a specific module. The canonical home determines where the implementation lives and, for module-canonical words, which module name `IMPORT` activates them under.

Independent of canonical home, every built-in word has a set of **listings** identifying which dictionary views surface the word for browsing or documentation. The Core listing view and any module listing view are not necessarily disjoint: a word may be listed in both. Such a word is called a **boundary word**, recognising that it has both a semantic role (Core) and a capability role (module).

Listings are presentation-only. A listing does not change name resolution, IMPORT semantics, or qualified-name resolution: bare names resolve through the Core vocabulary first and then through imported modules; `MODULE@WORD` resolves only to that module's canonical entries.

Boundary classes:

- **Core-only words** — canonical home Core; listed only in Core.
- **Canonical Core + module-listed boundary words** — canonical home Core; additionally surfaced in one or more module or category listing views (e.g. `PRINT` in `IO`, `STR`/`NUM`/`BOOL` in the `CAST` category, `SHAPE`/`RANK` in the `TENSOR` category, `SPAWN`/`AWAIT` in the `RUNTIME` category).
- **Canonical Module + core-listed boundary words** — canonical home in a module, additionally surfaced in the Core listing view (e.g. `SORT` whose canonical home is `ALGO`).
- **Module-only words** — canonical home in a module; listed only under that module.

`IMPORT`, `IMPORT-ONLY`, `UNIMPORT`, and `UNIMPORT-ONLY` are Canonical Core words and remain Core-only — they are not module-listed and are not affected by listings.

Categories such as `CAST`, `TEXT`, `TENSOR`, and `RUNTIME` are documentation-only labels used to group Core words by capability surface. They are **not** modules, are not registered in `MODULE_SPECS`, and cannot be supplied to `IMPORT`.

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
| `NEQ` | `<>` | `PIPE` | `==` |
| `LT` | `<` | `OR-NIL` | `=>` |
| `LTE` | `<=` | `LOOKUP` | `?` |
| `GT` | `>` | | |
| `GTE` | `>=` | | |
| `AND` | `&` | | |

### 7.1 Vector operations

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `LENGTH` | — | Number of elements in a vector, or total count of all stack values |
| `GET` | — | Retrieve element at a given index |
| `INSERT` | — | Insert element at a given index |
| `REPLACE` | — | Replace element at a given index |
| `REMOVE` | — | Remove element at a given index |
| `CONCAT` | — | Concatenate two or more vectors |
| `REVERSE` | — | Reverse the order of elements |
| `RANGE` | — | Generate a sequence of integers from start to end with optional step |
| `TAKE` | — | Take the first N elements |
| `SPLIT` | — | Split a vector into sub-vectors by given sizes |
| `REORDER` | — | Reorder elements according to an index list; supports duplication and negative indices |
| `COLLECT` | — | Gather all current stack values into a single vector |
| `SORT` | — | Sort elements in ascending order; yields `Unknown` if any required comparison is undecidable (Section 7.4.3) |

### 7.2 Tensor operations

Tensor operations operate on nested vectors treated as multi-dimensional arrays.

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `SHAPE` | — | Return the size of each dimension as a vector |
| `RANK` | — | Return the number of dimensions |
| `RESHAPE` | — | Reshape to new dimension sizes |
| `TRANSPOSE` | — | Transpose a 2D tensor |
| `FILL` | — | Create a tensor of given shape filled with a value |

Tensor operation implementations must follow the staged pipeline:
1. Flatten input
2. Compute shape, stride, and index metadata
3. Transform indices or selections
4. Rebuild output

Ad hoc recursive shape mutation in intermediate stages is prohibited.

### 7.3 Arithmetic

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `ADD` | `+` | Addition |
| `SUB` | `-` | Subtraction |
| `MUL` | `*` | Multiplication |
| `DIV` | `/` | Exact-real division |
| `MOD` | `%` | Remainder |
| `FLOOR` | — | Floor (largest integer ≤ value) |
| `CEIL` | — | Ceiling (smallest integer ≥ value) |
| `ROUND` | — | Round to nearest integer |

Arithmetic on scalars is performed directly on the continued-fraction representation by **Gosper's bihomographic algorithm**: each operation forms a Möbius (for unary) or bihomographic (for binary) transform of its operands and emits partial quotients of the result as soon as the next quotient is unambiguously determined by the current coefficients. No intermediate value is materialized as an approximate real or as a truncated rational. Coefficients are BigInt at all times (Section 4.2.2).

`FLOOR`, `CEIL`, and `ROUND` pull partial quotients from their operand until the integer part of the result is determined. For finite (rational) operands this terminates after a bounded number of steps; for lazy (irrational) operands it terminates after enough partial quotients have been emitted to fix the integer part, which is always finite for irrationals strictly between consecutive integers. A value that is exactly an integer requires no pull beyond `a0`.

`MOD` is defined as `x - FLOOR(x / y) * y` and is computed by composing the underlying Gosper transforms. `DIV` by a value whose CF reduces to zero produces NIL with `reason = divisionByZero` (Section 11.2 Bubble Rule); division by an irrational that cannot be distinguished from zero within the comparison budget produces NIL with `reason = undecidable` (Section 7.4). This is an *operational* absence — `DIV` could not produce a number — and remains a NIL; it is distinct from the *logical* `unknown` (U) that the comparison words themselves return on budget exhaustion (Section 7.4.1, Section 4.5.2).

### 7.4 Comparison

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `LT` | `<` | Less than |
| `LTE` | `<=` | Less than or equal |
| `GT` | `>` | Greater than |
| `GTE` | `>=` | Greater than or equal |
| `EQ` | `=` | Equal |
| `NEQ` | `<>` | Not equal |

Comparisons return a truth value with the `TruthValue` interpretation role: `true`, `false`, or — when the comparison is not decidable within the comparison budget (see below) — `unknown` (U, the third value of the three-valued logic of Section 7.5).

The set of comparison primitives is intentionally complete (all six standard ordering relations), so that an automated producer can emit the relation that matches its intent directly rather than rewriting it as a negation or operand swap. `GT` and `GTE` are the strict mirrors of `LT` and `LTE`; `NEQ` is the negation of `EQ`. Every relation is independently registered with its own Coreword contract metadata (Section 7.14), is NIL-passthrough (Section 7.12), and supports the same modifier combinations (`TOP` / `STAK`, `EAT` / `KEEP`, `SAFE`).

Under `STAK` mode, ordering comparisons describe a sequence property of the consumed values: `LT` true iff strictly increasing, `LTE` non-decreasing, `GT` strictly decreasing, `GTE` non-increasing, `EQ` all equal, `NEQ` all adjacent pairs unequal.

#### 7.4.1 Decidability and comparison budget

Comparison on continued fractions (Section 4.2) proceeds by emitting partial quotients of the two operands in parallel and stopping at the first index where they differ; the sign of the difference at that index determines the order (with the usual alternating-parity rule of CF comparison). For two finite CFs this always terminates. For lazy CFs, two distinct irrationals always differ at some finite index, but two equal irrationals never differ — the procedure does not terminate by itself.

To preserve totality, every comparison operation runs under an implementation-defined **partial-quotient budget**. If the budget is exhausted before a difference is found:

- the comparison produces the truth value `unknown` (U), observed as `truthValue = unknown`, with the `TruthValue` interpretation role;
- U is a logical truth value, not an operational absence: the comparison does **not** produce a `reason = undecidable` NIL, and the Bubble Rule (Section 11.2) does not apply to it;
- U flows into the three-valued logic of Section 7.5 directly. It is not a NIL, so the NIL-passthrough machinery of Section 7.12 does not carry it; instead `AND`/`OR`/`NOT` settle it according to the Kleene truth tables.

The U outcome is required for `EQ`, `LT`, `LTE`, `GT`, `GTE`, `NEQ`, and for any downstream Coreword whose result depends on a comparison (notably `MIN`, `MAX`, `SORT`, and `COND` clauses whose head reduces to such a comparison); the propagation rule for those words is fixed in Section 7.4.3. The budget value itself is not part of observable semantics; it must be high enough that distinct rationals always decide.

The agreed-prefix length — the number of leading partial quotients that matched before the budget was exhausted — is carried on the comparison's `Unknown` result in the machine-readable `diagnosis.agreedPrefix` field (Section 4.5.0): a non-negative integer. It means "the two values are equal to at least this depth of continued-fraction precision" and is the CF-specific evidence behind the U result. It is diagnostic context only and does not change the observable `truthValue`.

`STAK`-mode ordering comparisons short-circuit on the first pair that yields U: the entire stack-mode result is U, regardless of how many subsequent pairs would have decided. (A NIL operand still short-circuits to NIL per Section 7.12; when both a NIL operand and a U-producing pair are present, NIL takes priority per Section 4.5.2.)

#### 7.4.2 Explicit-budget comparison: `COMPARE-WITHIN`

The six relations of Section 7.4 run under an implementation-defined budget that is not observable. `COMPARE-WITHIN` makes the half-decidability threshold a first-class, user-controlled parameter: the program names the depth at which an undecided comparison becomes `Unknown`.

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `COMPARE-WITHIN` | — | Three-way compare two values within an explicit partial-quotient budget |

Stack effect: `[ a ] [ b ] [ budget ] -> [ -1 | 0 | 1 | UNKNOWN ]`.

`COMPARE-WITHIN` consumes two numeric values `a` and `b` and a positive integer `budget`, and emits partial quotients of `a` and `b` in parallel (Section 7.4.1) for at most `budget` steps. It produces:

- the scalar `-1` (role `RawNumber`) if `a < b`,
- the scalar `0` if `a = b`,
- the scalar `1` if `a > b`,
- the logical `Unknown` (U, Section 7.5) if the order is not settled within `budget` partial quotients; the U result carries `diagnosis.agreedPrefix` (Section 4.5.0), which for this word equals the consumed `budget`.

The decided outcome is the exact sign of `a − b`; the six relations of Section 7.4 are recoverable from it (`a < b` iff `-1`, `a <= b` iff not `1`, and so on). For two finite (rational) operands the comparison always decides regardless of `budget`, because finite CFs differ at a bounded index; `budget` only governs lazy irrationals.

`COMPARE-WITHIN` is `Projecting` (Section 7.14): it is total over well-shaped input because it projects the undecided case onto U. It is NIL-passthrough (Section 7.12) for its `a`/`b` operands. A non-positive or non-integer `budget`, or non-numeric `a`/`b`, is malformed use and raises an error (Section 11.2), not U. The implicit budget of the bare relations is an implementation-defined constant; `COMPARE-WITHIN` does not change it and is the only way to observe or override the depth.

#### 7.4.3 Propagation of U through comparison-dependent words

The U outcome named in Section 7.4.1 is required not only of the comparison primitives themselves but of every Coreword whose result depends on a comparison. This section fixes how U flows through the four such words — `MIN`, `MAX`, `SORT`, and `COND` — so that an undecidable comparison never silently produces a wrong order, a spurious error, or a definite truth value it has not earned.

The common rule is **U-honesty**: when the comparison a word relies on is undecidable, the word must surface that undecidability (as U, or, for `COND`, as the absence of a satisfied clause) rather than fabricate a decision. None of these words may treat U as `true`, as `false`, or as a malformed-input error.

**`MIN` / `MAX`.** These select one of two (or, in sequence form, several) numeric operands by the order relation. They accept the full numeric domain, including the lazy continued-fraction operands of Section 4.2, and decide the order through the same budgeted comparison as the relations (Section 7.4.1). When the governing comparison decides, the selected operand is returned unchanged. When it does not decide within the budget, the result is the logical `Unknown` (U), observed as `truthValue = unknown` and carrying `diagnosis.agreedPrefix` (Section 4.5.0) — because the program cannot be told *which* operand is the minimum/maximum when their order is unknown. `MIN` and `MAX` remain NIL-passthrough (Section 7.12): a NIL operand yields NIL, and NIL takes priority over a U-producing comparison per Section 4.5.2. In sequence form they short-circuit on the first undecidable pair, matching the `STAK`-mode rule of Section 7.4.1.

**`SORT`.** Sorting is a transitive cascade of pairwise order comparisons; a single undecidable pair makes the position of those elements relative to each other unknown, and the sorted order as a whole is therefore not established. When every pairwise comparison the sort requires decides, `SORT` returns the elements in ascending order as before. When any required comparison is undecidable within the budget, `SORT` produces the logical `Unknown` (U) for the whole result, carrying `diagnosis.agreedPrefix` for the first undecidable pair encountered. `SORT` does not return a partially-sorted vector, and it does not fall back to a tie-break: a partial order is not a sort. (The earlier exact-fraction-only behavior is the decided case of this rule: finite rationals always decide, so a vector of rationals always sorts.) A NIL element is handled by `SORT`'s existing NIL policy and takes priority over a U-producing comparison per Section 4.5.2.

**`COND`.** A `COND` clause fires when its guard evaluates to a definite `true`. Under three-valued logic a guard may now reduce to U (for example, a guard that is itself an undecidable comparison). A guard that yields U is **not** a definite `true`, so its clause does **not** fire; evaluation falls through to the next clause exactly as it would for a `false` guard, and ultimately to the `IDLE` / else clause if no guard yields a definite `true`. A U guard is therefore neither an error nor a match: it is the K3-faithful reading of "this clause's condition could not be established." If no clause fires and there is no else clause, the existing `CondExhausted` outcome (Section 11) applies unchanged. This makes a U guard behave, for clause-selection purposes, like `false` — but the distinction is observable while the guard value is on the stack (it reads `truthValue = unknown`, not `false`), and it is only the *clause-firing decision* that treats "not definitely true" uniformly.

In all four words the budget is the same implementation-defined constant used by the bare relations (Section 7.4.1); none of them expose or override it. `COMPARE-WITHIN` (Section 7.4.2) remains the only word that names the budget explicitly.

### 7.5 Logic

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `AND` | `&` | Logical AND |
| `OR` | — | Logical OR |
| `NOT` | — | Logical NOT |
| `TRUE` | — | Push truth value `true` |
| `FALSE` | — | Push truth value `false` |
| `NIL` | — | Push NIL |

`AND`, `OR`, and `NOT` implement **strong Kleene three-valued logic (K3)** over the truth domain {`true` (T), `false` (F), `unknown` (U)}. U is a first-class logical truth value, observed as `truthValue = unknown` (Section 2.3), and arises in particular from undecidable continued-fraction comparisons (Section 7.4.1). The truth tables are:

```
AND:  T∧T=T   T∧U=U   T∧F=F   U∧U=U   U∧F=F   F∧anything=F
OR :  F∨F=F   F∨U=U   F∨T=T   U∨U=U   U∨T=T   T∨anything=T
NOT:  ¬T=F    ¬U=U    ¬F=T
```

The absorbing elements are exact: `AND` returns F whenever either operand is F (even if the other is U), and `OR` returns T whenever either operand is T (even if the other is U). U propagates in every other case where it appears.

**Interaction with NIL.** NIL (operational absence, Section 4.5) and U (logical undecidability) are distinct values; see Section 4.5.2. The absorbing rule that previously collapsed NIL — `AND` with a definite F yields F, `OR` with a definite T yields T — is unchanged and continues to apply to NIL operands. In all other cases involving a NIL operand the logic word produces NIL (preserving the leftmost reason, Section 4.5.1), and `NOT` of NIL is NIL. When an operation receives both a NIL operand and a U operand and no absorbing definite operand settles the result, **NIL takes priority** and the result is NIL, not U (Section 4.5.2). This keeps the reason-bearing diagnostic value from being erased by the logical value.

### 7.6 String and type conversion

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `STR` | — | Convert value to its string representation |
| `NUM` | — | Parse a string as a number |
| `BOOL` | — | Convert to boolean |
| `CHR` | — | Convert a number to its Unicode character |
| `CHARS` | — | Split a string into a vector of individual characters |
| `JOIN` | — | Join a vector of strings, with optional separator |
| `TRIM` | — | Strip whitespace from both ends of a string |
| `TRIM-LEFT` | — | Strip leading whitespace |
| `TRIM-RIGHT` | — | Strip trailing whitespace |
| `TOKENIZE` | — | Split a string by a separator into a vector of substrings |
| `SUBSTITUTE` | — | Replace every occurrence of a substring with another |
| `STARTS-WITH?` | — | True if the string begins with the given prefix |
| `ENDS-WITH?` | — | True if the string ends with the given suffix |

`TRIM` / `TRIM-LEFT` / `TRIM-RIGHT` / `TOKENIZE` / `SUBSTITUTE` /
`STARTS-WITH?` / `ENDS-WITH?` are Canonical Core words also listed in the
`TEXT` documentation category alongside `CHR` / `CHARS` / `JOIN`. The
listing is presentation-only and does not introduce a `TEXT` module.

### 7.7 Control and higher-order words

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `MAP` | — | Apply a code block to each element, collecting results |
| `FILTER` | — | Keep elements for which a predicate returns true |
| `FOLD` | — | Reduce a sequence to a single value using an accumulator |
| `UNFOLD` | — | Generate a sequence by repeatedly applying a generator block |
| `ANY` | — | True if at least one element satisfies the predicate |
| `ALL` | — | True if all elements satisfy the predicate |
| `COUNT` | — | Count elements satisfying the predicate |
| `SCAN` | — | Like FOLD but returns all intermediate accumulator values |
| `COND` | — | Evaluate clauses separated by `$`; execute the first whose guard is definitely true (a U guard does not fire, Section 7.4.3) |
| `IDLE` | — | No-op; does nothing |
| `EXEC` | — | Execute a code block |
| `EVAL` | — | Parse and execute a string as Ajisai code |

### 7.8 User word dictionary

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `DEF` | — | Define a user word (see Section 8) |
| `DEL` | — | Delete a user word (see Section 8) |
| `LOOKUP` | `?` | Look up and display the definition of a word |

### 7.9 IO and utilities

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `PRINT` | — | Output the top stack value |
| `NOW` | — | Push the current instant (exact seconds since the Unix epoch) |
| `DATETIME` | — | Render an instant as a timezone-free civil datetime at a UTC offset |
| `TIMESTAMP` | — | Resolve a timezone-free civil datetime to an instant at a UTC offset |
| `CSPRNG` | — | Push a cryptographically secure random number |
| `HASH` | — | Compute a hash of the top stack value |

### 7.10 Module loading

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `IMPORT` | — | Load all words from a module into the current scope |
| `IMPORT-ONLY` | — | Load only the specified words from a module |

### 7.11 Child runtime words

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `SPAWN` | — | Create and start a child runtime from a code block; push a ProcessHandle |
| `AWAIT` | — | Wait for a child to finish; push `[status result-stack]` |
| `STATUS` | — | Return the current state of a child runtime as a string |
| `KILL` | — | Terminate a child runtime |
| `MONITOR` | — | Mark a child runtime for monitoring |
| `SUPERVISE` | — | Create a supervisor over a group of child runtimes |

### 7.12 NIL-passthrough words

The following built-in words follow the NIL passthrough rule defined in Section 4.5.1. If any operand they consume is NIL, the result is NIL and no error is raised.

| Category | Words |
|----------|-------|
| Arithmetic | `ADD`, `SUB`, `MUL`, `DIV`, `MOD`, `FLOOR`, `CEIL`, `ROUND` |
| Comparison | `LT`, `LTE`, `GT`, `GTE`, `EQ`, `NEQ` |
| Logic | `AND`, `OR`, `NOT` (three-valued; see below) |

`AND`, `OR`, and `NOT` implement strong Kleene three-valued logic (K3) over {`true`, `false`, `unknown`}; the truth tables and the U value are defined in Section 7.5. The three-valued behavior listed here concerns NIL operands specifically: NIL combined with a definite false (for `AND`) or definite true (for `OR`) collapses to that definite value; in all other cases involving NIL the result is NIL, and `NOT` of NIL is NIL. The logical third value U is separate from NIL (Section 4.5.2); when both appear, NIL takes priority.

The comparison words are NIL-passthrough for NIL operands as listed above. They do **not** produce a `reason = undecidable` NIL on budget exhaustion; that case now yields the truth value `unknown` (Section 7.4.1), which is a `TruthValue` result rather than an absence.

Words not listed here retain their existing handling of NIL. In particular, control-flow words (`COND`, `EXEC`, `MAP`, `FILTER`, `FOLD`, `UNFOLD`, `ANY`, `ALL`, `COUNT`, `SCAN`), most conversion words (`STR`, `BOOL`, `CHARS`, `JOIN`), IO words (`PRINT`, `NOW`, `DATETIME`, `TIMESTAMP`, `CSPRNG`, `HASH`), child-runtime words, and `OR-NIL` (`=>`) itself are not NIL-passthrough. `NUM` and `CHR` can create reasoned Bubble/NIL values for well-formed conversion failures as described by the Bubble Rule.

---

## 7.14 Coreword contract metadata

Every Coreword (built-in or module-provided) has a machine-readable contract entry in the Coreword registry. The contract is the authoritative description of the word's input requirements, output guarantees, and runtime classification. Tests, documentation, and tooling consume this metadata; narrative text is non-canonical.

A contract entry has the following fields, in addition to the existing identification (`name`, `category`), effect-classification fields (`purity`, `effects`, `deterministic`, `safe_preview`), and listing fields (`canonical_home`, `listed_in_core`, `listed_in_modules`, `listed_in_categories`):

| Field | Domain | Meaning |
|-------|--------|---------|
| `partiality` | `Total` / `Partial` / `Projecting` | `Total`: the operation is defined on every well-shaped input. `Partial`: the operation has well-shaped inputs for which it raises an error. `Projecting`: the operation is total because it projects all failures onto NIL with reason. |
| `nil_policy` | `Passthrough` / `CreatesNil` / `RejectsNil` / `ConsumesNil` / `PreservesReason` | How the word reacts to and produces NIL. `Passthrough` words follow Section 4.5.1; `CreatesNil` words project domain failures (e.g. division by zero); `RejectsNil` words raise `StructureError` on NIL; `ConsumesNil` words inspect or branch on NIL (e.g. `OR-NIL`); `PreservesReason` words must not erase a reason that is already attached to a propagated NIL. |
| `safety_level` | `A` / `B` / `C` / `D` / `Quarantined` | Increasing strength of safety guarantees. `A`: total, pure, deterministic; `B`: partial but with explicit error categories; `C`: observable or has external state read; `D`: effectful; `Quarantined`: not eligible for self-host execution. |

`partiality` and `nil_policy` are independent axes. For example, `~/` (safe-mode division) is `Projecting` with `nil_policy = CreatesNil`. Under the Bubble Rule, bare `/`, `GET`, `NUM`, and `CHR` can also be `Projecting`/`CreatesNil` for well-formed domain misses while malformed inputs remain ordinary errors.

Comparison words (`EQ`, `NEQ`, `LT`, `LTE`, `GT`, `GTE`) are `Projecting`: they are total because every well-shaped input yields a `TruthValue` result, projecting the undecidable case onto the truth value `unknown` (U) rather than raising. Their `nil_policy` is `Passthrough` (they pass NIL operands through per Section 7.12); on budget exhaustion they produce U, which is a `TruthValue` result and **not** a `reason`-bearing NIL, so they are no longer classified `CreatesNil` for that case. (The `reason = undecidable` NIL of earlier revisions is retired from the comparison path; see Section 7.4.1.)

Logic words (`AND`, `OR`, `NOT`) are `Total` over the three-valued domain {`true`, `false`, `unknown`} (Section 7.5) and `Passthrough` for NIL operands, with `PreservesReason` so that the leftmost NIL reason survives and a NIL operand is never silently replaced by U (Section 4.5.2).

`COMPARE-WITHIN` (Section 7.4.2) is `Projecting`: it is total over well-shaped input because it projects the budget-undecided case onto the logical `Unknown` (a result, not a reasoned NIL). Its `nil_policy` is `Passthrough` for the `a`/`b` operands. A non-positive or non-integer `budget` or non-numeric operands are malformed use and raise an error, so it is not `CreatesNil`.

`MIN`, `MAX`, and `SORT` (Section 7.4.3) are `Projecting`: each is total over well-shaped numeric input because it projects an undecidable governing comparison onto the logical `Unknown` (a result, not a reasoned NIL). Their `nil_policy` is `Passthrough`, with NIL taking priority over a U-producing comparison (Section 4.5.2). `COND` (Section 7.7) treats a U guard as not-firing rather than producing U as a value, so its existing partiality and `CondExhausted` behavior are unchanged by Section 7.4.3.

Contract metadata is reachable from both the Rust runtime and the WASM boundary. Adding a Coreword without a contract entry is a conformance violation.

The listing fields work as follows:

| Field | Domain | Meaning |
|-------|--------|---------|
| `canonical_home` | `Core` / `Module(name)` | Where the canonical implementation lives. For module-canonical words this is also the only module that can resolve the word as `MODULE@WORD` and the only module whose `IMPORT` brings the word into bare scope. |
| `listed_in_core` | `bool` | Whether the word appears in the Core listing view. Does not affect resolution. |
| `listed_in_modules` | `[module_name]` | Module listing views in which the word appears. A canonical module word always has its own module here. Boundary listings (e.g. `PRINT` in `IO`) are presentation-only. |
| `listed_in_categories` | `[category_label]` | Documentation-only category labels (e.g. `CAST`, `TEXT`, `TENSOR`, `RUNTIME`). Categories are not modules and cannot be `IMPORT`ed. |

`IMPORT-ONLY` of a selector that is core-listed in the target module's view but not canonically owned by that module is a no-op: the word is already available as a Canonical Core word, so the selector is silently skipped with a single warning line. Selectors that match neither a canonical word nor a listing remain an error.

A bare name may legitimately appear under more than one canonical home (for example core list `GET` and `JSON@GET`). In that case bare-name lookup resolves to the Canonical Core entry — matching the runtime's resolution order — while `MODULE@WORD` always reaches the module entry.

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

`DEL` never destroys module dictionaries or module words. To remove module words from the current vocabulary, use `UNIMPORT` or `UNIMPORT-ONLY`; the module dictionary remains cached as the definition source.

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
| `TIME` | Exact, timezone-free date/time values (instant / datetime / date / time); timezone is supplied only at instant↔civil conversion as a UTC offset in hours |
| `CRYPTO` | Cryptographically secure random and hash |
| `ALGO` | Sorting and other deterministic algorithms |
| `MATH` | Square root, exact-rational interval arithmetic, and scalar utilities (`ABS`, `NEG`, `SIGN`, `MIN`, `MAX`, `POW`, `GCD`, `LCM`) |
| `SERIAL` | Host-mediated serial port output |

### 9.2 Import and unimport syntax

```
'MODULE-NAME' IMPORT
'MODULE-NAME' [ 'WORD1' 'WORD2' ] IMPORT-ONLY
'MODULE-NAME' UNIMPORT
'MODULE-NAME' [ 'WORD1' 'WORD2' ] UNIMPORT-ONLY
```

`IMPORT` loads all public words from a module into the current vocabulary. `IMPORT-ONLY` loads only the specified module words or sample words.

`UNIMPORT` hides unreferenced imported words from the current vocabulary without deleting the module dictionary. If a user word references a module word or module sample word, `UNIMPORT` keeps that referenced item visible and shrinks the module import to an explicit partial-import state.

`UNIMPORT-ONLY` hides only the specified module words or sample words. It fails if a selected item is referenced by a user word; use dictionary-level `UNIMPORT` when the desired operation is to clean up unused module imports while preserving referenced items.

Selectors that name Core words merely listed in a module view are no-ops for `IMPORT-ONLY` and `UNIMPORT-ONLY`, because Core words are always available and cannot be imported or unimported through a module.

### 9.3 Module-provided sample words

Modules may provide sample words for demonstration. Sample words are part of the module dictionary for import visibility: they can be introduced with `IMPORT` / `IMPORT-ONLY` and hidden with `UNIMPORT` / `UNIMPORT-ONLY`, but they are not destructively deleted with `DEL`.

### 9.4 SERIAL module (host-mediated serial output)

The `SERIAL` module exposes a serial port to Ajisai programs. Serial access is a property of the host environment, not of the runtime: the runtime never opens, writes, reads, or closes a port itself. Outbound words are effectful and produce a single host command at the IO/semantic boundary (Section 5.2); the host environment consumes that command and performs the actual port operation. The serial transport itself (a browser Web Serial implementation, a native serial backend, or none) is a host capability outside this specification. The absence of a serial-capable host is an environment condition, not a language semantic error.

A serial connection is identified by an **opaque port-id text value** that the host environment assigns when the user grants access. Programs treat the port id as a connection handle and thread it along the stack. The port id is `Text`; the runtime does not interpret its contents.

**Receive model.** Inbound data is delivered to a program through a per-run *receive buffer* (inbox). The host environment injects the bytes received on each open port before a run begins; `READ` drains the inbox for a port. A run therefore observes exactly the bytes that arrived since the previous run — an event-poll model, not a blocking read. Within a single run the inbox is fixed, so a run is deterministic with respect to its injected input. Each `READ` consumes the buffered bytes for its port; a subsequent `READ` in the same run with no further data projects `noData` (Section 11.2).

The module provides the following words:

| Word | Stack effect | Description |
|------|--------------|-------------|
| `LIST-PORTS` | `--` | Request host enumeration of available ports |
| `OPEN` | `port-id -- port-id` | Open the named port; the port id remains on the stack as the connection handle |
| `CONFIGURE` | `port-id baud-rate -- port-id` | Set the baud rate of an open port |
| `WRITE` | `port-id bytes -- port-id` | Send a byte vector (each element an integer `0`–`255`) to an open port |
| `READ` | `port-id -- bytes` | Drain the port's receive buffer, returning a byte vector; Bubble/NIL when none is available |
| `FLUSH` | `port-id -- port-id` | Flush the port's outgoing data |
| `CLOSE` | `port-id --` | Close the port and release the connection |

Contract classification (Section 7.14): every `SERIAL` word has `purity = Effectful`, `deterministic = false`, `safe_preview = false`, and `safety_level = D`. The outbound words (`LIST-PORTS`, `OPEN`, `CONFIGURE`, `WRITE`, `FLUSH`, `CLOSE`) are `partiality = Partial`, `nil_policy = RejectsNil`. `READ` is `partiality = Projecting`, `nil_policy = CreatesNil`: it projects the no-data and disconnected conditions onto Bubble/NIL. Because they drive or observe external hardware, `SERIAL` words are never eligible for speculative reordering, caching, or `safe_preview` execution.

Misuse raises an error rather than producing Bubble/NIL (Section 11.2 "malformed use → error"): a non-text port id, a byte outside `0`–`255`, a non-positive baud rate, or a missing operand raises `StructureError` or `StackUnderflow`. For `READ`, the absence of data is a well-formed outcome, not misuse: with no buffered bytes it projects Bubble/NIL with `reason = noData`, or `reason = portDisconnected` when the host has reported the port gone. The host-side outcome of a well-formed outbound command (for example a device that is physically disconnected) is reported through the host environment and does not change the runtime stack effect of the word.

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

### 11.2 Bubble Rule

The Bubble Rule is the user-level failure model for well-formed partial operations:

> If an operation is well-formed but cannot produce a value, it produces a Bubble/NIL with a reason. If the operation is malformed, it raises an error.

In Japanese user-facing guidance this is summarized as: "できなかった -> 泡 / そもそも使い方が違う -> エラー". Internally, Bubble/NIL is represented by `Value::Nil` with `AbsenceMetadata` and a direct `NilReason`. `NilReason::SafeCaught` is reserved for actual errors caught by the `SAFE` (`~`) boundary; `SAFE` does not rewrap a direct Bubble/NIL produced by a well-formed operation.

Initial Core words following this rule include:

| Word | Bubble/NIL case | Error case |
|------|-----------------|------------|
| `DIV` / `/` | Division by zero (`NilReason::DivisionByZero`); divisor indistinguishable from zero within the comparison budget (`NilReason::Undecidable`) | Non-numeric operands or malformed shapes |
| `GET` | Valid vector target with an out-of-range index (`NilReason::IndexOutOfBounds`) | Non-vector target or non-numeric index |
| `NUM` | Text cannot be parsed as a number (`NilReason::InvalidEncoding`) | Input shape is not convertible text |
| `CHR` | Numeric code point is outside the valid Unicode scalar range (`NilReason::InvalidEncoding`) | Operand is not numeric, or numeric operand is not an integer |
| `EQ` / `NEQ` / `LT` / `LTE` / `GT` / `GTE` | None: budget exhaustion on lazy continued fractions yields the truth value `unknown` (U), a `TruthValue` result, not a Bubble/NIL (Section 7.4.1). These words still pass NIL operands through (Section 7.12). | Non-numeric operands or malformed shapes |
| `SERIAL@READ` | Receive buffer empty (`NilReason::NoData`); host reported the port disconnected with no remaining data (`NilReason::PortDisconnected`); both with `absence.origin = hostEnvironment` (Section 9.4) | Non-text port id, or a missing operand |

`OR-NIL` (`=>`) replaces Bubble/NIL with a fallback value. Existing NIL passthrough behavior preserves the reason as Bubble/NIL flows through later operations.

### 11.3 Equal-value output

Operations that produce a value equal to their input are successful. Equal-value output is not an error.

### 11.3 Safe mode behavior

`~` (`SAFE`) is the explicit projection operator for malformed-use errors that have not already become Bubble/NIL values. If the guarded word raises an error, the projected NIL carries `absence.reason = safeCaught` and preserves the original error category in `absence.caughtCategory` (for example `stackUnderflow`, `unknownWord`, or `structureError`). The error itself does not propagate.

Stack discipline: when `~`-guarded execution raises an error, the stack is restored to the snapshot taken before the guarded word ran, then a single NIL with `absence.reason = safeCaught` is pushed. When the guarded word succeeds by producing a direct Bubble/NIL, `SAFE` leaves that Bubble/NIL and the word's normal stack effect unchanged. The semantic plane is normalized to the new stack length.

The NIL passthrough rule (Section 4.5.1) means a NIL produced by a `~`-guarded operation continues to flow through subsequent NIL-passthrough words (Section 7.12) without raising `StructureError`. A pipeline can therefore use `OR-NIL` (`=>`) once at the end to supply a fallback value.

`~` is **not** a generic exception swallower: the original error category and three-layer diagnosis are preserved on the resulting NIL for debugging, testing, and proof logging.

---

## 12. Semantic Plane

### 12.1 Purpose

The semantic plane holds an **interpretation role** for each stack position. A role is the meaning the runtime assigned to a value, not a formatting switch: rendering for both humans and AI is a pure function of `(data, role)`. The semantic plane is separate from the data plane and does not influence computation.

### 12.2 Interpretation roles

| Role | Meaning |
|------|---------|
| `Unassigned` | No role has been assigned. The value is rendered in its raw structural form. The runtime never infers a richer meaning (such as "string-like") at render time; interpretation is decided once, at construction. |
| `RawNumber` | A plain number. A rational scalar renders as a reduced `numerator/denominator`, integers included (`3` renders as `3/1`). There is no decimal surface form and no per-value style: the display is uniform and matches the exact-real internal model. |
| `ContinuedFraction` | Display a numeric scalar as the nested right-associative continued-fraction form `( a0 ( a1 ( a2 ... )))` (Section 4.2.3); lazy CFs render with a `...)` truncation marker |
| `Interval` | A 2-element vector interpreted as the closed interval `[lo, hi]` |
| `Text` | A codepoint sequence interpreted as text |
| `TruthValue` | A three-state scalar-like truth value drawn from {`true`, `false`, `unknown`} (Section 7.5). It is observed through the `truthValue` axis (`true` / `false` / `unknown`, Section 2.3) and carries the `truthValued` capability. Displays as `TRUE`, `FALSE`, or `UNKNOWN` (display-only, non-canonical). The third state `unknown` (U) is a logical truth value, not an operational absence; how it is represented internally is not observable. |
| `Timestamp` | An integer interpreted as a formatted datetime |
| `Nil` | A diagnostic absence value, displayed as `NIL` |

Interpretation roles are applied only at explicit semantic boundaries: rendering, `PRINT`, and module-level output operations. A value's role and the surface style of any literal that produced it are independent: surface literal forms (Section 3.2) are convenience input syntax and are never retained as display state. Two values are displayed identically whenever their data and role are equal.

The `ContinuedFraction` role is the canonical AI-readable numeric serialization form. Machine-readable tooling that needs to round-trip exact values across the WASM boundary or diagnostic logs must request this role; the `RawNumber` role may lose information for lazy irrationals.

---

## 13. Fractional-Dataflow Internal Invariants

### 13.1 Static Mass Conservation

Ajisai treats flow mass conservation as a compile/JIT/load-time property. A Coreword Contract declares arity, consumption, production, bifurcation, and NIL-projection behavior. Optimized execution paths may be entered only after those contracts have been validated for the surrounding flow.

The ordinary runtime must not maintain per-value `FlowToken` objects or perform step-by-step mass accounting. Flow-accounting failures such as over-consumption, unconsumed leaks, flow breaks, and bifurcation-ratio violations are contract-validation failures and must be reported by the compiler/JIT, loader, or developer diagnostics before the optimized path executes.

### 13.2 Bifurcation

The `,,` (keep/bifurcation) modifier retains source context while also pushing the result. Its mass relationship is specified by the relevant Coreword Contract and is statically checked with the surrounding flow. Runtime execution only performs the value-level stack effects that the contract has already proven.

---

## 14. AI-first Implementation Rules

### 14.1 Mandatory

- Prefer explicit, structurally searchable function and module names.
- Keep Rust source files under 500 lines.
- Keep control flow shallow and phase-separated.
- Separate semantic changes from structural cleanup in change management.
- Maintain single canonical implementations; do not allow dual-mode drift.
- Source code comments are allowed when they clarify intent, invariants, traceability, or non-obvious behavior. When source code is changed, nearby comments must be reviewed and updated so they remain accurate. Comments that merely restate obvious code should be avoided.

### 14.2 Advisory

- Prefer small helper extraction for duplicated control scaffolding.
- Prefer deterministic, low-ambiguity error classification.
- Prefer mechanically enforceable tests over narrative documentation.

---

## 15. Test Discipline

### 15.1 Per-Coreword contract coverage

For each Coreword, the test suite must exercise:

- inputs that satisfy `requires` (success path)
- inputs that violate `requires` (failure path)
- the documented `nil_policy` (NIL passthrough or NIL creation, as appropriate)
- the documented `partiality` (every error category in the partial case; the projection target in the projecting case)
- effect-boundary expectations from `purity` (e.g. effectful words must not be reachable in `safe_preview`)

### 15.2 NIL reason coverage

Every NIL-producing path must have at least one test that asserts both the surface NIL and the structured `absence` metadata. `SAFE`-projected NILs must additionally assert `absence.reason = safeCaught` and that the original error category survives in `absence.caughtCategory`. Tests must verify protocol strings, not Rust `Debug` output.

### 15.3 MC/DC-style coverage for compound decisions

Words whose behavior depends on more than one independent condition (e.g. `~ /` decides on left-operand validity, right-operand validity, right-operand zero-ness, NIL-passthrough applicability, and SAFE engagement) must have tests that vary each condition independently with all others held fixed. Each condition must be shown to flip the outcome on its own.

The three-valued logic words (`AND`, `OR`, `NOT`, Section 7.5) must cover every cell of the K3 truth tables — `AND` and `OR` over the nine {T, F, U}² combinations, `NOT` over the three {T, F, U} inputs — plus the NIL-interaction cases (absorbing collapse, NIL propagation, and the NIL-over-U priority rule of Section 4.5.2). The comparison words must cover at least: finite CFs always deciding to a definite truth value, equal irrationals yielding `unknown` (U), and distinct irrationals deciding at a finite prefix.

`COMPARE-WITHIN` (Section 7.4.2) must cover: each decided sign (`-1` / `0` / `1`); the budget-undecided case yielding `unknown` with `diagnosis.agreedPrefix` equal to the consumed `budget`; finite operands deciding regardless of `budget`; the same lazy-irrational pair deciding at a large `budget` but yielding U at a small `budget`; NIL operand passthrough; and the malformed-`budget` / non-numeric-operand error paths.

The propagation of U through comparison-dependent words (Section 7.4.3) must cover, for `MIN` and `MAX`: a decided comparison selecting the correct operand; an undecidable comparison yielding `unknown` with `diagnosis.agreedPrefix`; NIL-operand passthrough with NIL taking priority over a U-producing comparison; and the sequence-form short-circuit on the first undecidable pair. For `SORT`: a fully-decidable input (notably any vector of rationals) sorting to ascending order; an input with at least one undecidable pair yielding `unknown` for the whole result (never a partially-sorted vector) carrying `diagnosis.agreedPrefix`; and NIL handling. For `COND`: a definite-`true` guard firing its clause; a U guard **not** firing and falling through to the next clause; a U guard before a later definite-`true` guard selecting the later clause; and a U guard with no other match reaching the `IDLE`/else clause, or `CondExhausted` when none exists.

### 15.4 Stack discipline under projection

`SAFE`-guarded failures must restore the stack to the pre-call snapshot before pushing the projected NIL. Tests must verify the stack length, the semantic-plane length, and the absence of leaked partial intermediates.

---

## 16. Conformance Checklist

A change is conformant only if all of the following hold:

1. It does not introduce a second design authority.
2. It does not treat equal-value output as a runtime error.
3. It does not impose hard-coded call-depth limits as language semantics.
4. It preserves data-plane/semantic-plane separation.
5. It keeps vector/tensor staged pipeline boundaries explicit.
6. It improves or preserves AI-first structural clarity.
7. Every built-in word (Core or module) introduced or renamed has an English-word-based canonical name; any symbolic form is registered as syntactic sugar that maps to that canonical name.
8. Every introduced or modified Coreword has a contract entry covering `partiality`, `nil_policy`, and `safety_level` (Section 7.14).
9. Every NIL-producing path attaches appropriate structured `absence` metadata (Section 4.5.0); `SAFE` projection preserves the original error category as `absence.caughtCategory` (Section 11.3).
10. Per-Coreword contract tests, NIL reason tests, MC/DC-style tests, and stack-discipline tests exist as required by Section 15.
11. Scalar arithmetic and comparison operate on the continued-fraction representation (Section 4.2) without intermediate rounding or truncation; Möbius coefficients in Gosper transforms are BigInt; comparison-budget exhaustion in the comparison words produces the truth value `unknown` (U) rather than an error or a non-deterministic answer (Section 7.4.1).
12. Source text contains no `(` or `)` outside of string literals; the nested continued-fraction form is a display/serialization artifact only (Sections 3.4, 4.2.3).
