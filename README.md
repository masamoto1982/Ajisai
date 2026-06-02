![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai QR Code](public/images/QR_341201.png "Ajisai QR Code")

# Ajisai

Ajisai is an AI-first, vector-oriented, continued-fraction dataflow language.

Its central promise is **value integrity first**: numbers stay exact, structure stays visible, partial failure stays diagnosable, and every built-in word is expected to have a machine-readable contract.

The name *Ajisai* comes from hydrangea, often interpreted as a “water vessel.” Ajisai uses water as its main metaphor: values flow through channels, operations shape those channels, and exceptional situations remain visible instead of disappearing into hidden runtime state.

- Playground: https://masamoto1982.github.io/Ajisai/
- Desktop build channel: Tauri wrapper in [`src-tauri/`](src-tauri/)
- Canonical language definition: [`SPECIFICATION.md`](SPECIFICATION.md)

---

## The language in one picture

| Water metaphor | Language meaning | Observable idea |
| --- | --- | --- |
| Flow | ordinary values moving through the stack | Scalars, vectors, records, code blocks, handles |
| Bubble | a well-formed operation could not produce a value | `NIL` with structured absence metadata |
| Stagnation | a value exists, but the current observation cannot decide the next direction | logical `UNKNOWN` in Kleene three-valued logic |
| Channel error | the operation or input shape is malformed | raised error, optionally projected by `SAFE` |

Ajisai keeps these cases separate. A bubble is absence. Stagnation is undecidability. An error is not a value in the stream.

Spec links: [§4 Value Model](SPECIFICATION.md#4-value-model), [§4.5 NIL](SPECIFICATION.md#45-nil), [§4.5.2 NIL versus Unknown](SPECIFICATION.md#452-nil-versus-unknown), [§11 Error Model](SPECIFICATION.md#11-error-model)

---

## Why Ajisai exists

### 1) Exact numbers: water with a traceable flow history

Every numeric value in Ajisai is an **exact real represented internally as a continued fraction**. Integer, fraction, decimal, and scientific-notation literals are just convenient source forms for that same exact representation. Runtime words such as `SQRT` may produce lazy infinite continued fractions for admitted irrational values.

Ajisai therefore avoids the usual hidden detour through approximate floating-point values. Arithmetic operates on the continued-fraction representation directly, and canonical AI-readable display uses a nested continued-fraction form rather than remembering the original source literal.

For comparison, Ajisai uses a faster internal observation method: **nearest-integer continued fractions**. This does not change value identity, display, or serialization; it only changes how comparison consumes observation budget.

Spec links: [§3.2 Numeric literal formats](SPECIFICATION.md#32-numeric-literal-formats), [§4.2 Scalar: exact-real continued-fraction arithmetic](SPECIFICATION.md#42-scalar-exact-real-continued-fraction-arithmetic), [§4.2.5 Nearest-integer continued fractions](SPECIFICATION.md#425-nearest-integer-continued-fractions-comparison-expansion), [§12.2 Interpretation roles](SPECIFICATION.md#122-interpretation-roles)

### 2) Bubble and Stagnation: failure and undecidability stay visible

Ajisai uses the **Bubble and Stagnation Model** to explain partial computation.

A **Bubble** is `NIL`. It appears when an operation is well-formed but cannot produce a meaningful value: division by zero, a failed `NUM` parse, an invalid `CHR` code point, or an out-of-range `GET` on a valid vector. The bubble carries a reason and diagnostic metadata as it flows downstream.

A **Stagnation** is `UNKNOWN`. It appears when a value is present but the current observation budget cannot decide a truth value. The main example is exact continued-fraction comparison: two values may agree for every observed term, exhaust the comparison budget, and still not settle `TRUE` or `FALSE`. Ajisai reports this as logical `UNKNOWN`, often with comparison diagnosis such as `agreedPrefix`.

The distinction matters:

- `NIL` means “the value is absent.”
- `UNKNOWN` means “the value exists, but this question is not decided yet.”
- Generic NIL passthrough uses operational NIL only, so logical `UNKNOWN` is not silently absorbed as a bubble.
- Logic words use Strong Kleene three-valued logic: for example, `FALSE AND UNKNOWN` is `FALSE`, while `TRUE OR UNKNOWN` is `TRUE`.

Spec links: [§4.5.0 Diagnostic absence metadata](SPECIFICATION.md#450-diagnostic-absence-metadata), [§4.5.1 NIL passthrough](SPECIFICATION.md#451-nil-passthrough), [§4.5.2 NIL versus Unknown](SPECIFICATION.md#452-nil-versus-unknown), [§7.4.1 Decidability and comparison budget](SPECIFICATION.md#741-decidability-and-comparison-budget), [§7.4.2 `COMPARE-WITHIN`](SPECIFICATION.md#742-explicit-budget-comparison-compare-within), [§7.5 Logic](SPECIFICATION.md#75-logic), [§11.2 Bubble Rule](SPECIFICATION.md#112-bubble-rule)

### 3) Vectors and tensors: channels can be nested or dense

Ajisai is vector-oriented. A vector is an ordered, indexable sequence, and nested vectors naturally express tensor-like structures. Indexing is 0-origin, and negative indices count from the end.

Internally, vectors may be represented either as nested `Value` trees or as dense tensor buffers. Dense tensors use exact small rational lanes plus a validity mask for NIL occupancy. They do not approximate irrational or BigInt-scale exact values just to fit a fast path, and they do not rebuild into nested vectors merely because a lane becomes NIL.

This is part of Ajisai's Virtual Tensor Unit direction: optimize movement and shape operations without weakening exactness.

Spec links: [§4.3 Vector](SPECIFICATION.md#43-vector), [§4.3.1 Internal representation classes](SPECIFICATION.md#431-internal-representation-classes), [§7.1 Vector operations](SPECIFICATION.md#71-vector-operations), [§7.2 Tensor operations](SPECIFICATION.md#72-tensor-operations), [VTU design note](docs/dev/virtual-tensor-unit-design.md)

### 4) Modifiers: gates, branches, and spillways

Ajisai modifiers control how a word touches the stream.

- **Target mode** chooses where the word acts: `TOP` for the surface point, `STAK` for the whole stack.
- **Consumption mode** chooses whether the input is consumed or preserved: `EAT` consumes, `KEEP` branches.
- **Safe mode** (`~`) catches raised errors from the next word and projects them to `NIL` with `safeCaught` metadata.

`SAFE` is not the main mechanism for ordinary partial operations. Under the Bubble Rule, many well-formed “could not produce a value” cases already produce reasoned `NIL` directly. `SAFE` is the explicit spillway for errors that would otherwise break the channel.

Spec links: [§6 Modifiers](SPECIFICATION.md#6-modifiers), [§6.1 Target modifiers](SPECIFICATION.md#61-target-modifiers), [§6.2 Consumption modifiers](SPECIFICATION.md#62-consumption-modifiers), [§6.3 Safe mode modifier](SPECIFICATION.md#63-safe-mode-modifier), [§11.3 Safe mode behavior](SPECIFICATION.md#113-safe-mode-behavior)

### 5) Words, modules, and contracts: searchable channels for humans and AI

Ajisai treats built-in words as documented, searchable units. The registry records canonical names, aliases, categories, purity, stack effects, canonical home, and module listings. Module words can live in importable dictionaries while still appearing in documentation views where they make sense.

The design goal is not only to make the language usable by people, but also mechanically inspectable by AI tools: every Coreword should expose contracts for requirements, guarantees, partiality, NIL policy, safety level, and effects.

Spec links: [§7.0 English-word-based naming](SPECIFICATION.md#70-english-word-based-naming), [§7.14 Coreword contract metadata](SPECIFICATION.md#714-coreword-contract-metadata), [§8 User Words](SPECIFICATION.md#8-user-words), [§9 Module System](SPECIFICATION.md#9-module-system), [§14 AI-first Implementation Rules](SPECIFICATION.md#14-ai-first-implementation-rules)

---

## A small taste

```ajisai
# Exact rational arithmetic
[ 1 3 ] / [ 1 2 ] /        # => [ 2/3 ]

# Vectorized arithmetic
[ 1 2 3 ] [ 4 5 6 ] +      # => [ 5 7 9 ]

# Bubble/NIL fallback
1 0 DIV => 99              # division-by-zero Bubble becomes fallback value

# Importing a module word
'math' IMPORT 2 SQRT       # lazy exact continued fraction for √2
```

More examples are available in [`examples/`](examples/).

Spec links: [§3 Syntax](SPECIFICATION.md#3-syntax), [§7 Built-in Words](SPECIFICATION.md#7-built-in-words), [§9.2 Import and unimport syntax](SPECIFICATION.md#92-import-and-unimport-syntax)

---

## Runtime architecture

```text
Rust interpreter core → WASM boundary → TypeScript GUI/runtime shell
                              └──────→ Tauri desktop shell
```

- Rust core: tokenizer, value model, interpreter, built-in words, modules, tests
- WASM boundary: protocol conversion between Rust values and the TypeScript runtime
- TypeScript GUI: editor, dictionary sheets, execution controller, output rendering, platform adapters
- Tauri shell: desktop integration and host capabilities

Runtime-specific behavior such as persistence, file I/O, and host hooks is abstracted through [`src/platform/`](src/platform/).

Spec links: [§1 Language Identity](SPECIFICATION.md#1-language-identity), [§12 Semantic Plane](SPECIFICATION.md#12-semantic-plane), [§13 Fractional-Dataflow Internal Invariants](SPECIFICATION.md#13-fractional-dataflow-internal-invariants)

---

## Development checks

```sh
# Rust interpreter and integration tests
cd rust && cargo test --lib
cd rust && cargo test --tests

# TypeScript type check and frontend tests
npm run check
npm run test

# Semantic firewall check
npm run check:semantic-firewall
```

Build commands:

```sh
# Web playground build
npm run build:web

# Rebuild Rust/WASM bridge
npm run build:wasm

# Tauri desktop build
npm run tauri:build
```

Quality process documents live in [`docs/quality/`](docs/quality/), including the [quality policy](docs/quality/QUALITY_POLICY.md), [verification plan](docs/quality/VERIFICATION_PLAN.md), and [release verification checklist](docs/quality/RELEASE_VERIFICATION_CHECKLIST.md).

---

## Repository map

| Path | Purpose |
| --- | --- |
| [`SPECIFICATION.md`](SPECIFICATION.md) | Canonical language specification |
| [`rust/src/`](rust/src/) | Rust interpreter core and value model |
| [`src/`](src/) | TypeScript GUI/runtime shell |
| [`src-tauri/`](src-tauri/) | Desktop wrapper |
| [`examples/`](examples/) | Ajisai sample programs |
| [`docs/dev/`](docs/dev/) | Non-canonical design notes and implementation guidance |
| [`docs/quality/`](docs/quality/) | Quality, traceability, and verification policy |

---

## License

MIT ([`LICENSE`](LICENSE))
