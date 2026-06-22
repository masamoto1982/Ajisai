![Rust](docs/assets/badges/rust.svg) ![WebAssembly](docs/assets/badges/webassembly.svg) ![TypeScript](docs/assets/badges/typescript.svg) ![Tauri](docs/assets/badges/tauri.svg) [Build and Deploy status](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai QR Code](public/images/QR_ajisai.png "Ajisai QR Code")

# Ajisai

Ajisai is an AI-first, vector-oriented, continued-fraction dataflow language.

Its central promise is **value integrity first**: numbers stay exact, structure stays visible, partial failure stays diagnosable, and every built-in word is expected to have a machine-readable contract.

The name *Ajisai* comes from hydrangea, often interpreted as a “water vessel.” Ajisai uses water as its main metaphor: values flow through channels, operations shape those channels, and exceptional situations remain visible instead of disappearing into hidden runtime state.

## Documentation

The specification and the Reference are authored in HTML (see [`docs/dev/ajisai-authoring-style.md`](docs/dev/ajisai-authoring-style.md)) and are served rendered on the project site:

| Document | Rendered at | Role |
| --- | --- | --- |
| **Specification** | https://masamoto1982.github.io/Ajisai/SPECIFICATION.html | Canonical language definition — the single design authority |
| **Reference** | https://masamoto1982.github.io/Ajisai/docs/index.html | Verified examples, each openable in the Playground |
| **Playground** | https://masamoto1982.github.io/Ajisai/ | Run Ajisai in the browser |

The HTML source of the specification lives at [`SPECIFICATION.html`](SPECIFICATION.html) in this repository; the rendered URL above is the reading surface. The desktop build channel is the Tauri wrapper in [`src-tauri/`](src-tauri/).

---

## The language in one picture

| Water metaphor | Language meaning | Observable idea |
| --- | --- | --- |
| Flow | ordinary values moving through the stack | Scalars, vectors, records, code blocks, handles |
| Bubble | a well-formed operation could not produce a value | `NIL` with structured absence metadata |
| Stagnation | a value exists, but the current observation cannot decide the next direction | logical `UNKNOWN` in Kleene three-valued logic |
| Channel error | the operation or input shape is malformed | raised error that propagates and halts evaluation |

Ajisai keeps these cases separate. A bubble is absence. Stagnation is undecidability. An error is not a value in the stream.

Spec links: [§4 Value Model](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#4-value-model), [§4.5 NIL](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#45-nil), [§4.5.2 NIL versus Unknown](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#452-nil-versus-unknown), [§11 Error Model](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#11-error-model)

---

## Why Ajisai exists

### 1) Exact numbers: water with a traceable flow history

Every numeric value in Ajisai is an **exact real represented internally as a continued fraction**. Integer, fraction, decimal, and scientific-notation literals are just convenient source forms for that same exact representation. Runtime words such as `SQRT` may produce lazy infinite continued fractions for admitted irrational values.

Ajisai therefore avoids the usual hidden detour through approximate floating-point values. Arithmetic operates on the continued-fraction representation directly, and canonical AI-readable display uses a nested continued-fraction form rather than remembering the original source literal.

For comparison, Ajisai uses a faster internal observation method: **nearest-integer continued fractions**. This does not change value identity, display, or serialization; it only changes how comparison consumes observation budget.

Spec links: [§3.2 Numeric literal formats](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#32-numeric-literal-formats), [§4.2 Scalar: exact-real continued-fraction arithmetic](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#42-scalar-exact-real-continued-fraction-arithmetic), [§4.2.5 Nearest-integer continued fractions](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#425-nearest-integer-continued-fractions-comparison-expansion), [§12.2 Interpretation roles](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#122-interpretation-roles)

### 2) Bubble and Stagnation: failure and undecidability stay visible

Ajisai uses the **Bubble and Stagnation Model** to explain partial computation.

A **Bubble** is `NIL`. It appears when an operation is well-formed but cannot produce a meaningful value: division by zero, a failed `NUM` parse, an invalid `CHR` code point, or an out-of-range `GET` on a valid vector. The bubble carries a reason and diagnostic metadata as it flows downstream.

A **Stagnation** is `UNKNOWN`. It appears when a value is present but the current observation budget cannot decide a truth value. The main example is exact continued-fraction comparison: two values may agree for every observed term, exhaust the comparison budget, and still not settle `TRUE` or `FALSE`. Ajisai reports this as logical `UNKNOWN`, often with comparison diagnosis such as `agreedPrefix`.

The distinction matters:

- `NIL` means “the value is absent.”
- `UNKNOWN` means “the value exists, but this question is not decided yet.”
- Generic NIL passthrough uses operational NIL only, so logical `UNKNOWN` is not silently absorbed as a bubble.
- Logic words use Strong Kleene three-valued logic: for example, `FALSE AND UNKNOWN` is `FALSE`, while `TRUE OR UNKNOWN` is `TRUE`.

Spec links: [§4.5.0 Diagnostic absence metadata](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#450-diagnostic-absence-metadata), [§4.5.1 NIL passthrough](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#451-nil-passthrough), [§4.5.2 NIL versus Unknown](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#452-nil-versus-unknown), [§7.4.1 Decidability and comparison budget](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#741-decidability-and-comparison-budget), [§7.4.2 `COMPARE-WITHIN`](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#742-explicit-budget-comparison-compare-within), [§7.5 Logic](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#75-logic), [§11.2 Bubble Rule](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#112-bubble-rule)

### 3) Vectors and tensors: channels can be nested or dense

Ajisai is vector-oriented. A vector is an ordered, indexable sequence, and nested vectors naturally express tensor-like structures. Indexing is 0-origin, and negative indices count from the end.

Internally, vectors may be represented either as nested `Value` trees or as dense tensor buffers. Dense tensors use exact small rational lanes plus a validity mask for NIL occupancy. They do not approximate irrational or BigInt-scale exact values just to fit a fast path, and they do not rebuild into nested vectors merely because a lane becomes NIL.

This is part of Ajisai's Virtual Tensor Unit direction: optimize movement and shape operations without weakening exactness.

Spec links: [§4.3 Vector](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#43-vector), [§4.3.1 Internal representation classes](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#431-internal-representation-classes), [§7.1 Vector operations](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#71-vector-operations), [§7.2 Tensor operations](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#72-tensor-operations), [VTU design note](docs/dev/virtual-tensor-unit-design.md)

### 4) Modifiers: how a word touches the stream

Ajisai modifiers control how a word touches the stream.

- **Target mode** chooses where the word acts: `TOP` for the surface point, `STAK` for the whole stack.
- **Consumption mode** chooses whether the input is consumed or preserved: `EAT` consumes, `KEEP` branches.

There is no error-swallowing modifier. Partial failure of a well-formed operation is handled by the Bubble Rule, which produces a reasoned `NIL` directly; a malformed operation raises an error that propagates rather than becoming a value.

Spec links: [§6 Modifiers](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#6-modifiers), [§6.1 Target modifiers](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#61-target-modifiers), [§6.2 Consumption modifiers](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#62-consumption-modifiers), [§11.2 Bubble Rule](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#112-bubble-rule), [§11.4 Error propagation](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#114-error-propagation)

### 5) Words, modules, and contracts: searchable channels for humans and AI

Ajisai treats built-in words as documented, searchable units. The registry records canonical names, aliases, categories, purity, stack effects, canonical home, and module listings. Module words can live in importable dictionaries while still appearing in documentation views where they make sense.

The design goal is not only to make the language usable by people, but also mechanically inspectable by AI tools: every Coreword should expose contracts for requirements, guarantees, partiality, NIL policy, safety level, and effects.

Spec links: [§7.0 English-word-based naming](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#70-english-word-based-naming), [§7.14 Coreword contract metadata](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#714-coreword-contract-metadata), [§8 User Words](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#8-user-words), [§9 Module System](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#9-module-system), [§14 AI-first Implementation Rules](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#14-ai-first-implementation-rules)

---

## Safety model: safe by design, with gates and water levels

Ajisai does **not** rely on a broad "safe mode" that wraps evaluation. There is no
global safe/unsafe switch. Ordinary value flow is safe by design:

- a well-formed operation that cannot produce a value becomes a **bubble** (`NIL`),
- an observation that cannot decide a truth value becomes **stagnation** (`UNKNOWN`),
- a malformed use raises a **channel error**.

There is no mode or modifier that converts an error into a value: a channel error
propagates and halts evaluation, while well-formed partial failure already flows
as a reasoned bubble that a single `^` (`VENT`) can turn into a fallback at the
end of a pipeline.

Two further controls complete the water metaphor — both are names for mechanisms
Ajisai already has, not new subsystems:

- **Gates** control *where* flow may cross a boundary. Outward gates guard host
  effects (such as serial and future IO), where effects are emitted as host
  commands and the host performs them. Inward gates guard module imports crossing
  the Core / Module / User trust boundary (`IMPORT` / `UNIMPORT`).
- **Water levels** control *how much* flow may run. The evaluation step budget
  bounds total work and raises `ExecutionLimitExceeded` when reached. The
  comparison budget bounds observation depth; when it is reached the result is
  the logical `UNKNOWN` (stagnation), never a bubble — keeping operational
  absence and logical undecidability distinct.

Spec links: [§4.5.2 NIL versus Unknown](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#452-nil-versus-unknown), [§5.2 Two-plane architecture](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#52-two-plane-architecture), [§5.3 Execution step limit](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#53-execution-step-limit), [§7.4.1 Decidability and comparison budget](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#741-decidability-and-comparison-budget), [§7.4.2 `COMPARE-WITHIN`](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#742-explicit-budget-comparison-compare-within), [§9 Module System](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#9-module-system), [§11.2 Bubble Rule](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#112-bubble-rule), [§11.4 Error propagation](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#114-error-propagation), [Appendix A Gates and Water Levels](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#appendix-a-gates-and-water-levels-non-normative-index)

### Supply-chain integrity: content-addressed source provenance

The same content-addressing that gives each word a stable identity (§8.6) is
lifted to the whole trust-critical source surface. `npm run provenance:attest`
records a SHA-256 digest of every tracked source file and a Merkle-style **root
identity** in [`docs/provenance/`](docs/provenance/); `npm run provenance:check`
(run in CI) fails the moment any tracked source drifts from that recorded
identity. This is the defensible form of "detect a backdoor the instant it is
injected": rather than embedding something that constantly phones home (which
the pure core has no capability to do), the injection shows up as a content-hash
mismatch, with no network involved. See [the design
note](docs/dev/source-provenance-attestation-design.md) for the threat model and
how to anchor the root pin externally.

---

## A small taste

The **Expected value** column shows the final stack exactly as the language renders it (numbers display as exact fractions in `numerator/denominator` form).

| Sample code | Expected value | Notes |
| --- | --- | --- |
| `2 3 / 1 3 / +` | `1/1` | Exact rational arithmetic: two thirds plus one third is exactly one. |
| `[ 1 2 3 ] [ 4 5 6 ] +` | `[ 5/1 7/1 9/1 ]` | Vectorized arithmetic: equal-length vectors combine element-wise. |
| `1 0 / ^ 99` | `99/1` | Division by zero produces a Bubble (`NIL`); `^` (`VENT`) replaces it with the fallback value. |
| `'math' IMPORT 2 SQRT 2 LT` | `TRUE` | `SQRT` yields a lazy exact continued fraction for √2 and compares it without rounding. |

More examples are available in [`examples/`](examples/) and in the [Reference](https://masamoto1982.github.io/Ajisai/docs/index.html), where every sample opens in the Playground.

Spec links: [§3 Syntax](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#3-syntax), [§7 Built-in Words](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#7-built-in-words), [§9.2 Import and unimport syntax](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#92-import-and-unimport-syntax)

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

Spec links: [§1 Language Identity](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#1-language-identity), [§12 Semantic Plane](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#12-semantic-plane), [§13 Fractional-Dataflow Internal Invariants](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#13-fractional-dataflow-internal-invariants)

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
| [`SPECIFICATION.html`](SPECIFICATION.html) | Canonical language specification (HTML source; [rendered here](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html)) |
| [`rust/src/`](rust/src/) | Rust interpreter core and value model |
| [`src/`](src/) | TypeScript GUI/runtime shell |
| [`src-tauri/`](src-tauri/) | Desktop wrapper |
| [`examples/`](examples/) | Ajisai sample programs |
| [`public/docs/`](public/docs/) | Hand-authored HTML Reference ([rendered here](https://masamoto1982.github.io/Ajisai/docs/index.html)) |
| [`docs/dev/`](docs/dev/) | Non-canonical design notes and implementation guidance |
| [`docs/quality/`](docs/quality/) | Quality, traceability, and verification policy |

---

## License

MIT ([`LICENSE`](LICENSE))
