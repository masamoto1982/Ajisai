# Ajisai Language Specification

Status: **Canonical**
Version: **2026-06-08**

This document is the single design authority for Ajisai. It describes Ajisai as it is. It does not record development history or transitional states. If any other document conflicts with this document, this document takes precedence.

Ajisai is a typed, vector-oriented dataflow language. Its safety story is the conjunction of:

- **Value-shape safety** — operations check that operands have the structural shape they require (Scalar / Vector / Record / NIL / CodeBlock / handles).
- **Encoding safety** — string and code values carry encoding contracts on top of their underlying fraction sequences.
- **Contract safety** — every Coreword has machine-readable `requires` / `ensures` / partiality / NIL policy / effect metadata in the registry.
- **Bubble Rule safety** — a well-formed operation that cannot produce a value projects the failure onto NIL with a structured reason, while malformed use raises an ordinary error (Section 11.2).

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

Machine-readable semantic metadata is part of this firewall. `docs/word-manifest.json` gives every surface word a generated `canonical` spelling and `semantic_role`; `docs/formalization-coverage.json` gives canonical entries their algebraic family, derivation edges, primitive registry, effect boundary, and exploratory debt metadata. Tooling must update the generator and coverage source rather than hand-authoring contradictory manifest semantics.

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
| `<` | Syntactic sugar for `LT` |
| `<=` | Syntactic sugar for `LTE` |
| `<>` | Syntactic sugar for `NEQ` |
| `$` | COND clause separator |
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

A string literal begins with `'` and ends with the last `'` before a token boundary. A token boundary is whitespace, end of input, or any special character other than `'` (such as `[`, `]`, `{`, `}`, `#`, `=`, `$`).

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

`(` and `)` are not Ajisai syntactic characters. They are reserved markers — `(` denotes the concept `RESERVED-BEGIN` and `)` denotes `RESERVED-END` (Section 3.9) — reserved at the lexical level to prevent accidental reuse and to keep the nested continued-fraction serialization form (Section 4.2) unambiguous; encountering `(` or `)` in source text is a tokenizer error.

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

### 3.9 Surface forms

Ajisai source syntax is word-based. Every visible *symbolic* form is an alias or sugar for a named, English-based **canonical concept**.

Not every surface form is a runtime word. A surface form is classified as one of the following kinds:

| Kind | Meaning | Runtime word? |
|------|---------|---------------|
| Word alias | Symbol form of a runtime word (canonicalized at runtime) | yes |
| Modifier sugar | Compound shorthand for stack modifiers | no |
| Delimiter sugar | Parser-level structural delimiter | no |
| Literal sugar | String-literal delimiter | no |
| Source directive | Consumed lexically by the tokenizer | no |
| Control directive | Meaningful only inside a specific construct | no |
| Reserved marker | Never a runtime token; rejected in source | no |
| Conversion word | `>` followed by letters; canonical home is a runtime word | yes |

**Word aliases** are canonicalized to their English name at runtime (Section 7.0); the canonical name is the authoritative identifier. The remaining kinds denote concepts that are *not* runtime words: they are handled at the lexical or parser level and are never produced by `DEF`, never canonicalized to a runtime word, and (for reserved markers) never valid in source at all.

| Surface | Canonical concept | Kind | Runtime word? |
|---------|-------------------|------|---------------|
| `+` `-` `*` `/` `%` | `ADD` `SUB` `MUL` `DIV` `MOD` | Word alias | yes |
| `=` `<>` `<` `<=` `>` `>=` | `EQ` `NEQ` `LT` `LTE` `GT` `GTE` | Word alias | yes |
| `&` `!` `?` | `AND` `FORC` `LOOKUP` | Word alias | yes |
| `.` `..` `,` `,,` | `TOP` `STAK` `EAT` `KEEP` | Word alias | yes |
| `==` `=>` | `PIPE` `OR-NIL` | Word alias | yes |
| `;` | `TOP-EAT` (`. ,`) | Modifier sugar | no |
| `;;` | `STAK-KEEP` (`.. ,,`) | Modifier sugar | no |
| `[` `]` | `BEGIN-VECTOR` `END-VECTOR` | Delimiter sugar | no |
| `{` `}` | `BEGIN-BLOCK` `END-BLOCK` | Delimiter sugar | no |
| `'` | `STRING-QUOTE` | Literal sugar | no |
| `#` | `COMMENT-LINE` | Source directive | no |
| `$` | `COND-CLAUSE` | Control directive | no |
| `(` `)` | `RESERVED-BEGIN` `RESERVED-END` | Reserved marker | no |
| `>CF` | `>CF` (continued-fraction conversion) | Conversion word | yes |

`;` and `;;` are pure shorthand: `; == . ,` and `;; == .. ,,`. The concept names `TOP-EAT` and `STAK-KEEP` name the compound forms; they are not stand-alone runtime words.

`>` followed by an ASCII letter (e.g. `>CF`) is a single **conversion-word** token, not the `>` (`GT`) comparison alias followed by a word. Its canonical home is the runtime conversion word of the same name; `>` and `>=` remain the `GT` / `GTE` aliases.

---

## 4. Value Model

### 4.1 Value types

| Type | Description |
|------|-------------|
| Scalar | An exact real number, represented internally as a (possibly lazy) continued fraction |
| Boolean | A definite logical truth value, `true` or `false` (Section 7.5). A Boolean is a distinct value kind, **not** a number: `TRUE` is not the Scalar `1` and `FALSE` is not the Scalar `0`, so `TRUE 1 EQ` is `false`. The third truth value Unknown (U) is not a Boolean; it is the logical-undecidability value of Section 7.4.1, observed as `truthValue = unknown`. |
| Vector | An ordered, indexable sequence of values (may be nested) |
| Record | An ordered set of named fields (string keys) |
| NIL | The absence of a value |
| CodeBlock | An executable sequence of tokens |
| ProcessHandle | A reference to a running child runtime |
| SupervisorHandle | A reference to a supervisor |

A Boolean carries the `TruthValue` interpretation role (Section 12.2) and the `truthValued` capability (Section 2.3); it renders uniformly as `TRUE` / `FALSE` (display-only, non-canonical). Equality and ordering treat a Boolean as distinct from every Scalar: value identity never conflates a truth value with a number. Inside element-wise numeric operations over vectors, truth lanes are represented numerically as `1` / `0`; the distinctness rule above governs Scalar-level value identity (notably `EQ`), not numeric broadcast over vector lanes.

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

#### 4.2.5 Nearest-integer continued fractions (comparison expansion)

The comparison procedure of Section 7.4.1 expands operands as nearest-integer continued fractions. This is an internal expansion: the **regular** continued fraction of Sections 4.2.1–4.2.4 remains the sole basis for value identity, canonical form, display, and serialization. The nearest-integer expansion changes only how comparison consumes a value and, consequently, the unit of the comparison budget (see the end of this subsection and Section 7.4.1.1).

The **regular** continued fraction (RCF) of Section 4.2.1 takes each partial quotient by *floor*, leaving a remainder in `[0, 1)` and forcing every quotient after `a0` to be a strictly positive integer. The **nearest-integer continued fraction** (NICF) instead takes each partial quotient by *rounding to the nearest integer*, leaving a remainder in `(-1/2, 1/2]`. An NICF is a *semiregular* continued fraction

```
x = b0 + ε1 / (b1 + ε2 / (b2 + ε3 / ( ... )))
```

where each `bi` is the nearest integer to the current tail, each `εi ∈ {+1, -1}` is the sign of the corresponding remainder, and `bi >= 2` for `i >= 1`. Because each step removes more of the value than a floor step can, NICF expansions are never longer than the RCF and are typically shorter: by a classical result the NICF converges at least as fast as the RCF and on average meaningfully faster (larger effective partial quotients per term). Over exact-rational, near-equal, and surd corpora the NICF reduces the agreed-prefix depth by roughly 22–28% on the mean, including ~28% on the surd (lazy-CF) case that actually exhausts the comparison budget.

**Tie-break (normative).** Each partial quotient is `bi = round(t)` of the current tail `t`, where `round` sends a value whose fractional part is exactly `1/2` **down** to the lower integer; equivalently the post-step remainder lies in the half-open interval `(-1/2, 1/2]`, and `bi = ⌈(2·num − den) / (2·den)⌉` for a tail `num/den` with `den > 0`. This makes the NICF digit sequence of every value deterministic, so two conforming implementations agree on it.

NICF changes which expansion the **comparison procedure** consumes (Section 7.4.1.1). It does **not** change the following observable surfaces, which remain defined by the RCF:

- **Value identity and canonical form.** The canonical CF of a value remains its RCF (Section 4.2.1); two values are equal iff their RCFs agree (Section 4.2.4). NICF is never the canonical form and is never the basis of equality.
- **Display and serialization.** The nested form of Section 4.2.3 always renders the RCF. NICF digits, signs, and the `εi` are never serialized and must not be parsed.
- **The internal representation tag** (Section 4.2.2 / 4.2.4) remains non-observable: which stored representation backs a value must not be branched on.

What NICF *does* make observable, indirectly, is the **unit of the comparison budget** (Sections 7.4.1.1, 7.4.2): a budget term, and hence an `agreedPrefix` count, is a *semiregular (NICF) term*. NICF introduces no new numeric type, no new Coreword, and no new protocol field.

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

NIL represents the absence of a value. It is produced by well-formed operations that yield no meaningful result (Section 11.2).

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

A Bubble/NIL result carries its own direct reason (Section 11.2). Literal NIL has `origin = literal` and no `reason` unless a future protocol explicitly adds one.

#### 4.5.1 NIL passthrough

Operations classified as **NIL-passthrough** in Section 7 do not raise `StructureError` when a NIL operand is encountered. Instead, they produce NIL. The rule is uniform across consumption modes and target modes: if any operand consumed by the operation is NIL, the operation consumes its operands as it normally would and pushes a single NIL result.

NIL-passthrough applies to arithmetic, comparison, and the unary numeric rounding words (see Section 7.13). It does not apply to control-flow words, type-conversion words, IO words, or to `OR-NIL` (`=>`) itself, whose entire purpose is to react to NIL.

The intent is that pipelines propagate a Bubble/NIL through subsequent computation without crashing, so that a single `=>` at the end of the pipeline can supply a fallback value.

When a NIL-passthrough operation receives one or more NIL operands, the resulting NIL inherits the reason of the leftmost NIL operand that carried a reason. This makes the cause traceable through long pipelines.

#### 4.5.2 NIL versus Unknown

NIL and the logical truth value `Unknown` (U, Section 7.5) are distinct and must not be conflated. **NIL is an operational absence**: a diagnostic bubble that records *why* a value is missing (division by zero, out-of-range `GET`, parse failure). **U is a logical undecidability**: a definite member of the three-valued truth domain that records that a proposition could not be settled true or false (notably a continued-fraction comparison that did not decide within its budget, Section 7.4.1). U carries the `TruthValue` role and is observed as `truthValue = unknown`; NIL is observed as `semanticKind = absence`.

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

### 6.3 Modifier combinations

All modifier combinations are explicit and mechanically testable. Combined forms such as `.,,` and `..,,` are valid.

### 6.4 Additional syntax forms

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

All built-in words — both Core words and module dictionary words — use English-word-based canonical names. Symbol forms (such as `+`, `-`, `*`, `/`, `%`, `=`, `<`, `<=`, `&`, `==`, `=>`, `?`, `!`, `.`, `..`, `,`, `,,`) are syntactic sugar that the tokenizer maps to canonical English names. The canonical name is the authoritative identifier; the symbol form is convenience surface syntax. Any new built-in word must be introduced under an English-word-based canonical name.

| Canonical | Sugar | Canonical | Sugar |
|-----------|-------|-----------|-------|
| `ADD` | `+` | `TOP` | `.` |
| `SUB` | `-` | `STAK` | `..` |
| `MUL` | `*` | `EAT` | `,` |
| `DIV` | `/` | `KEEP` | `,,` |
| `MOD` | `%` | `FORC` | `!` |
| `EQ` | `=` | `PIPE` | `==` |
| `NEQ` | `<>` | `OR-NIL` | `=>` |
| `LT` | `<` | `LOOKUP` | `?` |
| `LTE` | `<=` | | |
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

`SORT` is a **Canonical Module + core-listed boundary word**: its canonical home is the `ALGO` module (Section 9.1) and it is additionally surfaced in the Core listing view, so a bare `SORT` resolves and `ALGO@SORT` names the same word. It is listed here for its vector role.

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

The set of comparison primitives is intentionally complete (all six standard ordering relations), so that an automated producer can emit the relation that matches its intent directly rather than rewriting it as a negation or operand swap. `GT` and `GTE` are the strict mirrors of `LT` and `LTE`; `NEQ` is the negation of `EQ`. Every relation is independently registered with its own Coreword contract metadata (Section 7.14), is NIL-passthrough (Section 7.12), and supports the same modifier combinations (`TOP` / `STAK`, `EAT` / `KEEP`).

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

##### 7.4.1.1 NICF-accelerated comparison

The comparison procedure of Section 7.4.1 decides the order of two operands by emitting their **nearest-integer** continued fractions (NICF, Section 4.2.5) in parallel and stopping at the first index where the semiregular expansions differ; the sign of the difference there determines the order, with the sign correction induced by the `εi` partial-numerator signs in place of the plain alternating parity. Because NICF converges at least as fast as RCF, two distinct values reveal their order at an NICF index no later — and usually earlier — than the RCF index, so a budget of fixed size decides at least as many comparisons and in practice meaningfully more (Section 4.2.5). The budget mechanism, the U outcome, and the totality guarantee of Section 7.4.1 are otherwise unchanged.

The order computed over NICF is the true order of the values: it is **identical** to the order RCF would compute whenever RCF decides within any budget. NICF never flips a decided order; it only moves the budget *boundary* at which a pair of distinct values crosses from U to decided, in the favorable direction. Equal values never differ in either expansion, so they still yield U.

**Budget unit and `agreedPrefix`.** One budget term is one **semiregular (NICF) term**, uniformly for the six relations (Section 7.4) and for `COMPARE-WITHIN` (Section 7.4.2). The `agreedPrefix` of a U result (Sections 4.5.0, 7.4.1) therefore counts matching NICF terms. Its contract is otherwise unchanged: a non-negative integer, monotone (a larger budget never decreases it), `0` for operands that differ at the first term, diagnostic-only (it never affects `truthValue`), and for `COMPARE-WITHIN` equal to the consumed `budget` on U. Because the unit is uniformly NICF, there is no RCF-versus-NICF unit ambiguity to reconcile: a caller of `COMPARE-WITHIN` who names `budget = n` buys `n` semiregular terms, and that is the documented meaning. The correspondence between an NICF term count and an RCF term count is **not** observable and must not be relied upon.

**Conformance requirement.** Because the budget unit is user-observable through `COMPARE-WITHIN`, NICF expansion is not an optional per-implementation choice: a conforming implementation must emit NICF terms for **every** comparable scalar representation (Section 4.2.2) — `Rational`, `AlgebraicSqrt`, and `Gosper` (which backs the results of arithmetic). For the homographic and bihomographic (`Gosper`) representations this is a local change to the partial-quotient emitter: a term is emitted when the *nearest integer* (rather than the floor) agrees across the transform's value-range endpoints, and the post-emit coefficient update is unchanged from the floor case — a negative semiregular remainder propagates its sign through the reciprocal continuation, so no separate `εi` bookkeeping is carried in the transform coefficients.

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

Of these four, `COND` is a Canonical Core word; `MIN` and `MAX` are canonically owned by the `MATH` module and `SORT` by the `ALGO` module (Section 9.1). `SORT` is additionally Core-listed (Section 7.1); `MIN` / `MAX` are reached as bare names only after `IMPORT 'MATH'`, and always as `MATH@MIN` / `MATH@MAX`. The U-propagation contract below is a property of each word wherever it is invoked, independent of its canonical home.

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
| `>CF` | `>CF` | Tag a numeric scalar so it displays and serializes under the `ContinuedFraction` interpretation role (Section 12.2); value-preserving (`[ x ] -> [ x ]`) |

`>CF` is the conversion-word surface form of Section 3.9: it changes only the requested display/serialization role of its operand (the nested-parentheses continued-fraction form of Section 3.2 / Section 4.2), never the value. It is a Canonical Core word.

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
| `PRECOMPUTE` | — | Definition-time staging marker: evaluate a code block when a word is defined and splice the resulting values into the definition |

`PRECOMPUTE` is a **definition-time-only** Canonical Core word, not a macro. It consumes a code block (`[ { body } ] -> [ value... ]`) and is meaningful only while a `DEF` body is being compiled: the block is evaluated once at definition time and its result values are staged into the compiled definition, so the cost is not paid on each later call. It is `Partial` (it raises on malformed use such as a non-block operand) with `nil_policy = RejectsNil`; using it outside a definition-time context is an error.

### 7.8 User word dictionary

| Canonical | Sugar | Description |
|-----------|-------|-------------|
| `DEF` | — | Define a user word (see Section 8) |
| `DEL` | — | Delete a user word (see Section 8) |
| `LOOKUP` | `?` | Look up and display the definition of a word |

### 7.9 IO and utilities

| Canonical | Canonical home | Sugar | Description |
|-----------|----------------|-------|-------------|
| `PRINT` | Core (listed in `IO`) | — | Output the top stack value |
| `NOW` | `TIME` | — | Push the current instant (exact seconds since the Unix epoch) |
| `DATETIME` | `TIME` | — | Render an instant as a timezone-free civil datetime at a UTC offset |
| `TIMESTAMP` | `TIME` | — | Resolve a timezone-free civil datetime to an instant at a UTC offset |
| `CSPRNG` | `CRYPTO` | — | Push a cryptographically secure random number |
| `HASH` | `CRYPTO` | — | Compute a hash of the top stack value |

Only `PRINT` is a Canonical Core word here; it is additionally boundary-listed in the `IO` view (Section 7). `NOW` / `DATETIME` / `TIMESTAMP` are canonically owned by the `TIME` module and `CSPRNG` / `HASH` by the `CRYPTO` module (Section 9.1): they are **not** Core-listed, so a bare name resolves only after `IMPORT`, and they are always reachable as `TIME@NOW`, `CRYPTO@HASH`, and so on. They are grouped here by utility role, not by canonical home.

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
| `safety_level` | `A` / `B` / `C` / `D` / `Quarantined` | Increasing strength of safety guarantees. `A`: total, pure, deterministic; `B`: partial but with explicit error categories; `C`: observable or has external state read; `D`: effectful; `Quarantined`: not eligible for self-host execution. A `Partial` word is therefore at least `B` (it is not total); `Projecting` words are total by projection and may be `A`. |
| `mass` | `Fixed { consumes, produces }` / `Dynamic` | The static flow-mass contract of Section 13.1: under the default `TOP`/`EAT` mode the word reads `consumes` operands and pushes `produces` results; under `KEEP` the `consumes` operands are additionally retained (bifurcation, Section 13.2). `Dynamic` marks a word whose arity is not a statically pinned `Fixed` value — either because it is genuinely data-dependent (for example the `STAK` count-fold or runtime-shaped vector operations) or because it has not been probe-verified into the `Fixed` set. `Dynamic` is sound in both cases: the static mass-conservation validator simply abstains on `Dynamic` words, so marking a fixed-arity word `Dynamic` only forgoes static checking, it never asserts a wrong arity. |

`partiality` and `nil_policy` are independent axes. For example, under the Bubble Rule `/` (division) is `Projecting` with `nil_policy = CreatesNil`: `/`, `GET`, `NUM`, and `CHR` are `Projecting`/`CreatesNil` for well-formed domain misses while malformed inputs remain ordinary errors.

Comparison words (`EQ`, `NEQ`, `LT`, `LTE`, `GT`, `GTE`) are `Projecting`: they are total because every well-shaped input yields a `TruthValue` result, projecting the undecidable case onto the truth value `unknown` (U) rather than raising. Their `nil_policy` is `Passthrough` (they pass NIL operands through per Section 7.12); on budget exhaustion they produce U, which is a `TruthValue` result and **not** a `reason`-bearing NIL, so they are no longer classified `CreatesNil` for that case. (The `reason = undecidable` NIL of earlier revisions is retired from the comparison path; see Section 7.4.1.)

Logic words (`AND`, `OR`, `NOT`) are `Total` over the three-valued domain {`true`, `false`, `unknown`} (Section 7.5). Their registry `nil_policy` is `Passthrough`: they pass NIL operands through per Section 7.12. The `nil_policy` field holds a single value, so reason-preservation is **not** a second simultaneous policy here but an additional behavioral requirement layered on `Passthrough` — the leftmost NIL reason must survive and a NIL operand is never silently replaced by U (Section 4.5.2). The distinct `PreservesReason` value is reserved for words whose *only* NIL contract is to carry a reason through unchanged: the pure no-op / marker words (`TOP`, `STAK`, `EAT`, `KEEP`, `TRUE`, `FALSE`, `NIL`, `IDLE`, `PIPE`, `OR-NIL`).

`COMPARE-WITHIN` (Section 7.4.2) is `Projecting`: it is total over well-shaped input because it projects the budget-undecided case onto the logical `Unknown` (a result, not a reasoned NIL). Its `nil_policy` is `Passthrough` for the `a`/`b` operands. A non-positive or non-integer `budget` or non-numeric operands are malformed use and raise an error, so it is not `CreatesNil`.

`MIN`, `MAX`, and `SORT` (Section 7.4.3) are `Projecting`: each is total over well-shaped numeric input because it projects an undecidable governing comparison onto the logical `Unknown` (a result, not a reasoned NIL). Their `nil_policy` is `Passthrough`, with NIL taking priority over a U-producing comparison (Section 4.5.2). `MIN` and `MAX` are canonically `MATH` words and `SORT` is canonically an `ALGO` word (Sections 7.4.3, 9.1); their contracts are registry entries on the same footing as Core contracts. `COND` (Section 7.7) treats a U guard as not-firing rather than producing U as a value, so its existing partiality and `CondExhausted` behavior are unchanged by Section 7.4.3.

Other module-canonical words carry registry contracts on the same footing. `MATH@POW` is `Projecting` / `CreatesNil`: it projects `0` raised to a negative exponent onto Bubble/NIL (`reason = divisionByZero`) while malformed use raises an error. `MATH@GCD` and `MATH@LCM` are `Partial` / `Passthrough`: a non-integer numeric operand is malformed use and raises an error (cf. `CHR`), while NIL operands pass through. `ALGO@INDEX-OF` and `TIME@PARSE-ISO` are `Projecting` / `CreatesNil`, projecting a well-formed miss (value absent / unparseable text) onto Bubble/NIL with `reason = missingField` and `reason = invalidEncoding` respectively. Adding any Coreword — Core or module — without a contract entry is a conformance violation.

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

### 9.5 IO module (host-mediated standard input/output)

The `IO` module (Section 9.1) provides host-mediated textual standard input and output. Like `SERIAL`, these words act only at the IO/semantic boundary (Section 5.2): the runtime does not perform the I/O itself; the host environment supplies the input buffer and consumes the output buffer.

| Word | Stack effect | Description |
|------|--------------|-------------|
| `INPUT` | `-> [ text ]` | Read text from the host input buffer (observable host ingress) |
| `OUTPUT` | `[ value ] ->` | Write a value to the host output buffer (effectful host egress) |

`IO@INPUT` is `purity = Observable` (it reads external host state); `IO@OUTPUT` is `purity = Effectful`. Both are imported and resolved like any other module word (`IMPORT 'IO'`, or `IO@INPUT` / `IO@OUTPUT`). The Canonical Core word `PRINT` (Section 7.9) is boundary-listed in the `IO` view but is not the same word as `IO@OUTPUT`.

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

In Japanese user-facing guidance this is summarized as: "できなかった -> 泡 / そもそも使い方が違う -> エラー". Internally, Bubble/NIL is represented by `Value::Nil` with `AbsenceMetadata` and a direct `NilReason`. A malformed operation raises an ordinary error, which propagates rather than becoming a value.

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

### 11.4 Error propagation

Ajisai has no modifier or mode that converts a raised error into a value. A malformed operation (Section 11.2) raises an error that propagates to the top level and halts the current evaluation; it is never projected onto NIL. Partial failure of a *well-formed* operation is handled entirely by the Bubble Rule (Section 11.2), which produces a reasoned Bubble/NIL that downstream NIL-passthrough words (Section 7.12) carry without raising, so a pipeline can end with a single `OR-NIL` (`=>`) fallback. The distinction is deliberate: "could not produce a value" becomes a bubble, while "used incorrectly" stays an error.

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
| `TruthValue` | A three-state truth value drawn from {`true`, `false`, `unknown`} (Section 7.5). The definite states `true` / `false` are carried by a Boolean value (Section 4.1) — a distinct value kind, not a number; the third state `unknown` (U) is the logical-undecidability value of Section 7.4.1. Observed through the `truthValue` axis (`true` / `false` / `unknown`, Section 2.3); carries the `truthValued` capability. Displays as `TRUE`, `FALSE`, or `UNKNOWN` (display-only, non-canonical). How U is represented internally is not observable. |
| `Timestamp` | An integer interpreted as a formatted datetime |
| `Nil` | A diagnostic absence value, displayed as `NIL` |

Interpretation roles are applied only at explicit semantic boundaries: rendering, `PRINT`, and module-level output operations. A value's role and the surface style of any literal that produced it are independent: surface literal forms (Section 3.2) are convenience input syntax and are never retained as display state. Two values are displayed identically whenever their data and role are equal.

The `ContinuedFraction` role is the canonical AI-readable numeric serialization form. Machine-readable tooling that needs to round-trip exact values across the WASM boundary or diagnostic logs must request this role; the `RawNumber` role may lose information for lazy irrationals.

---

## 13. Fractional-Dataflow Internal Invariants

### 13.1 Static Mass Conservation

Ajisai treats flow mass conservation as a compile/JIT/load-time property. A Coreword Contract declares arity, consumption, production, bifurcation, and NIL-projection behavior: arity/consumption/production are carried by the `mass` contract field (Section 7.14), bifurcation by the `KEEP` modifier (Section 13.2), and NIL-projection by `nil_policy`. Optimized execution paths may be entered only after those contracts have been validated for the surrounding flow.

The ordinary runtime must not maintain per-value `FlowToken` objects or perform step-by-step mass accounting. Flow-accounting failures such as over-consumption, unconsumed leaks, flow breaks, and bifurcation-ratio violations are contract-validation failures and must be reported by the compiler/JIT, loader, or developer diagnostics before the optimized path executes. A word whose `mass` is `Dynamic` has a data-dependent arity that the static validator does not certify; flows through such words are validated only up to that point.

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

Every NIL-producing path must have at least one test that asserts both the surface NIL and the structured `absence` metadata. Tests must verify protocol strings, not Rust `Debug` output.

### 15.3 MC/DC-style coverage for compound decisions

Words whose behavior depends on more than one independent condition (e.g. `/` decides on left-operand validity, right-operand validity, right-operand zero-ness, and NIL-passthrough applicability) must have tests that vary each condition independently with all others held fixed. Each condition must be shown to flip the outcome on its own.

The three-valued logic words (`AND`, `OR`, `NOT`, Section 7.5) must cover every cell of the K3 truth tables — `AND` and `OR` over the nine {T, F, U}² combinations, `NOT` over the three {T, F, U} inputs — plus the NIL-interaction cases (absorbing collapse, NIL propagation, and the NIL-over-U priority rule of Section 4.5.2). The comparison words must cover at least: finite CFs always deciding to a definite truth value, equal irrationals yielding `unknown` (U), and distinct irrationals deciding at a finite prefix.

`COMPARE-WITHIN` (Section 7.4.2) must cover: each decided sign (`-1` / `0` / `1`); the budget-undecided case yielding `unknown` with `diagnosis.agreedPrefix` equal to the consumed `budget`; finite operands deciding regardless of `budget`; the same lazy-irrational pair deciding at a large `budget` but yielding U at a small `budget`; NIL operand passthrough; and the malformed-`budget` / non-numeric-operand error paths.

NICF-accelerated comparison (Section 7.4.1.1) must cover: (i) a reference set of decided comparisons producing an identical `truthValue` whether reasoned about via the RCF or computed via the NICF expansion — across `Rational`, `AlgebraicSqrt`, and `Gosper` operands — so the acceleration never changes a decided order; (ii) `agreedPrefix` being monotone non-decreasing in the budget under NICF; and (iii) the normative tie-break of Section 4.2.5 yielding the specified semiregular digit on the singular `1/2`-remainder cases, including the round-half-down boundary.

The propagation of U through comparison-dependent words (Section 7.4.3) must cover, for `MIN` and `MAX`: a decided comparison selecting the correct operand; an undecidable comparison yielding `unknown` with `diagnosis.agreedPrefix`; NIL-operand passthrough with NIL taking priority over a U-producing comparison; and the sequence-form short-circuit on the first undecidable pair. For `SORT`: a fully-decidable input (notably any vector of rationals) sorting to ascending order; an input with at least one undecidable pair yielding `unknown` for the whole result (never a partially-sorted vector) carrying `diagnosis.agreedPrefix`; and NIL handling. For `COND`: a definite-`true` guard firing its clause; a U guard **not** firing and falling through to the next clause; a U guard before a later definite-`true` guard selecting the later clause; and a U guard with no other match reaching the `IDLE`/else clause, or `CondExhausted` when none exists.

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
9. Every NIL-producing path attaches appropriate structured `absence` metadata (Section 4.5.0).
10. Per-Coreword contract tests, NIL reason tests, MC/DC-style tests, and stack-discipline tests exist as required by Section 15.
11. Scalar arithmetic and comparison operate on the continued-fraction representation (Section 4.2) without intermediate rounding or truncation; Möbius coefficients in Gosper transforms are BigInt; comparison-budget exhaustion in the comparison words produces the truth value `unknown` (U) rather than an error or a non-deterministic answer (Section 7.4.1).
12. Source text contains no `(` or `)` outside of string literals; the nested continued-fraction form is a display/serialization artifact only (Sections 3.4, 4.2.3).

---

## Appendix A. Gates and Water Levels (non-normative index)

This appendix is **non-normative**. It introduces no new rules, types, words, or
protocol fields. It is a vocabulary index that maps the water metaphor used in
user-facing material onto the normative mechanisms already defined above. Where
this appendix and any numbered section disagree, the numbered section governs.

Ajisai does not have a global "safe mode" that wraps evaluation. Ordinary value
flow is **safe by design**: a well-formed operation that cannot produce a value
yields a Bubble/NIL (Section 11.2), an observation that cannot decide a truth
value yields the logical `Unknown` / Stagnation (Sections 4.5.2, 7.4.1), and a
malformed use raises a channel error (Section 11.1). A raised channel error
propagates and halts the current evaluation; Ajisai has no modifier or mode that
converts an error into a value (Section 11.4).

Two further families of controls complete the metaphor:

**Gates — where flow may cross a boundary.** A gate controls whether flow may
cross a trust or effect boundary. Gates are not a new subsystem; they are the
existing boundaries:

| Gate (vocabulary) | Direction | Normative mechanism |
|-------------------|-----------|---------------------|
| Host effects (serial, future IO) | outward | IO/semantic boundary; effects are emitted as host commands and the host performs them; absence of a capable host is an environment condition, not a semantic error (Sections 5.2, 9.4) |
| Module dictionary import | inward | `IMPORT` / `IMPORT-ONLY` / `UNIMPORT` visibility control and dependency tracking across the Core / Module / User boundary (Sections 7, 9) |

**Water Levels — how much flow may run.** A water level bounds the amount of
work or expansion. These are the existing budgets and limits:

| Water Level (vocabulary) | Normative mechanism | Outcome on reaching the level |
|--------------------------|---------------------|-------------------------------|
| Evaluation step budget | step limit, default 100,000 (Section 5.3) | raises `ExecutionLimitExceeded` (Section 11.1) |
| Comparison / observation depth | comparison budget (Section 7.4.1); explicitly via `COMPARE-WITHIN` (Section 7.4.2) | yields the logical `Unknown` (U) / Stagnation, **not** a Bubble/NIL (Sections 4.5.2, 7.4.3) |

The defining distinction of Section 4.5.2 is preserved by this vocabulary: a gate
refusal or a step-budget exhaustion is an operational or environment condition,
whereas an exhausted comparison budget is a *logical* `Unknown`, never an
operational absence.


## Portability Profiles

### Core Profile
Host-independent semantics only: tokenization, vector evaluation, exact
arithmetic, vectors, blocks, map/form/fold, NIL/UNKNOWN, user definitions.

### Hosted Profile
Words requiring host capabilities: NOW, CSPRNG, SERIAL, AUDIO, JSONEXPORT,
persistence, file I/O.

### Platform Profile
Concrete runtime bindings: Web/WASM, Tauri, CLI, WASI, Native desktop.

## Conformance and Identity
An implementation is an Ajisai implementation if and only if it passes the
conformance suite (tests/conformance/). The suite defines, language-
independently, the correspondence between Ajisai source programs and their
observable results, including the ordered sequence of host effects. This
correspondence is the phenomenon of Ajisai; everything not fixed by the
suite is implementation freedom.
