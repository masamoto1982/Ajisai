![Rust](docs/assets/badges/rust.svg) ![WebAssembly](docs/assets/badges/webassembly.svg) ![TypeScript](docs/assets/badges/typescript.svg) ![Tauri](docs/assets/badges/tauri.svg) [Build and Deploy status](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai QR Code](public/images/Ajisai_QR_Small.png "Ajisai QR Code")

# Ajisai

Ajisai is an AI-first, vector-oriented dataflow language for **auditable, exact vector computation with machine-readable contracts** — built so that both people and agents can check a computation *before* it runs.

Its central promise is **value integrity first**: numbers stay exact, structure stays visible, partial failure stays diagnosable, and every built-in Word carries a machine-readable contract that a user word's declaration can be checked against ahead of execution (`ajisai check --contract`). That check is deliberately conservative — it verifies declarations within the syntactic fragment its inference can analyze, and reports anything it cannot prove as "cannot verify".

**Ten concepts.** Ajisai is built from ten concepts and nothing else: exact rationals closed under `SQRT`; three outcomes (a value, a reasoned absence, an error); a stack and vectors; code blocks evaluated only on request; one modifier axis; a sealed-Core / User dictionary with content-addressed identity; a machine-readable contract per Word; a pre-execution check of user declarations against those contracts; one host protocol; and an executable conformance corpus. The vocabulary is **69 Words** in one flat dictionary.

**Numeric scope.** Numbers are *exact by default*: exact rationals and the multiquadratic field they generate under `SQRT` — square roots of non-negative rationals, closed under field arithmetic, in a normal form \(\sum_d c_d\sqrt d\). Arithmetic never rounds, coefficients are arbitrary-precision, and **comparison is total**: every comparison of two scalars decides in finite time. That field is the whole numeric domain, so π, e, and logarithms are not Ajisai values.

The name *Ajisai* comes from hydrangea, often interpreted as a “water vessel.” Ajisai uses water as its main metaphor: values flow through channels, operations shape those channels, and exceptional situations remain visible instead of disappearing into hidden runtime state.

## Documentation

The specification and the Reference are authored in HTML (see [`docs/dev/ajisai-authoring-style.md`](docs/dev/ajisai-authoring-style.md)) and are served rendered on the project site:

| Document | Audience | Rendered at | Role |
| --- | --- | --- | --- |
| **Specification** | Builders and porters | https://masamoto1982.github.io/Ajisai/SPECIFICATION.html | Canonical language definition — the single design authority |
| **Reference** | Ajisai users | https://masamoto1982.github.io/Ajisai/docs/index.html | Verified examples, each openable in the Playground |
| **Playground** | Run it now | https://masamoto1982.github.io/Ajisai/ | Run Ajisai in the browser |

The HTML source of the specification lives at [`SPECIFICATION.html`](SPECIFICATION.html) in this repository; the rendered URL above is the reading surface. The desktop build channel is the Tauri wrapper in [`src-tauri/`](src-tauri/).

---

## The language in one picture

| Water metaphor | Language meaning | Observable idea |
| --- | --- | --- |
| Flow | ordinary values moving through the stack | Scalars, booleans, strings, vectors, code blocks |
| Bubble | a well-formed operation could not produce a value | `NIL`, carrying a machine-readable reason |
| Channel error | the operation or input shape is malformed | raised error that propagates and halts evaluation |

Ajisai keeps these three cases separate. A bubble is absence, not falsehood; an error is not a value in the stream. Truth is two-valued — `TRUE` and `FALSE` — and every comparison decides.

Spec links: [Value Domains](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-values), [Diagnostic absence](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-values-nil), [Partiality and Failure](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-failure)

---

## Why Ajisai exists

### 1) Exact numbers

Every numeric value is **exact**. Integer, fraction, decimal, and scientific-notation literals are convenient source forms for exact rationals, and `SQRT` produces exact algebraic irrationals carried in a multiquadratic normal form \(\sum_d c_d\sqrt d\). That is the whole numeric domain: rationals plus the `SQRT` closure over them.

Arithmetic works directly on those values, so a result is either the exact answer or a reasoned absence. Coefficients are arbitrary-precision, so a number grows as large as the value requires. Canonical AI-readable display uses a nested continued-fraction form derived from the value, rather than remembering the original source literal.

**Comparison is total.** Every comparison of two scalars answers `TRUE` or `FALSE` in finite time. Values built through different histories are the same value when they denote the same real: `8 SQRT` equals `2 SQRT 2 SQRT +`.

Spec links: [Exact scalars](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-values-exact), [Two-valued truth](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-values-truth), [Work limits](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-machine-limits)

### 2) Bubble: partial failure stays visible

Ajisai distinguishes three outcomes. A **value** is ordinary success. A **bubble** is `NIL`: a well-formed operation that could not produce a value — division by zero, a failed `NUM` parse, an invalid `CHR` code point, an out-of-range `GET`. A **channel error** is malformed use, which propagates and halts evaluation.

A bubble carries a machine-readable **reason** and flows downstream, so absence stays diagnosable. `NIL` is not `FALSE`, and an error is not a value.

- `NIL` means "the value is absent", and `^` (`VENT`) turns it into a fallback at the end of a pipeline.
- `AND` / `OR` / `NOT` pass a `NIL` operand through, so a bubble stays visible even when the other operand would settle the answer.
- A channel error is the one outcome that ends the program: it propagates to the top and halts evaluation, so misuse surfaces where it happened rather than turning into a value.

Spec links: [Diagnostic absence](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-values-nil), [Value, absence, misuse](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-failure-trichotomy), [NIL passthrough](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-failure-passthrough), [Recovery](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-failure-recovery)

### 3) Vectors: the one way to hold many values

Ajisai is vector-oriented. A vector is an ordered, indexable sequence; indexing is 0-origin and negative indices count from the end. Vectors nest, which is how ragged and grouped data is expressed.

Arithmetic and comparison **lift element-wise**: two vectors of equal length combine pairwise, and a scalar combines with every element. Any other pairing raises an error, so a length mismatch is reported at the point it occurs.

A vector is a sequence: values in order, nested as deeply as the data needs. Executable code is a separate kind of value that lives in `{ }` code blocks, so code and data never occupy the same shape. Internal storage (nested trees or dense buffers) is unobservable.

Spec links: [Vectors](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-values-vector), [Element lifting](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-collections-lift), [Higher-order evaluation](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-collections-higher)

### 4) One modifier axis: consume or keep

A modifier prefixes the next word only, and the single axis is **consumption**. `EAT` (`,`, the default) consumes the operands a word reads; `KEEP` (`,,`) leaves them on the stack beneath the result.

That is the whole modifier system: two words, one decision. Everything else about how a Word behaves is in the Word itself, so reading a call means reading a name and at most one prefix.

Spec links: [The consumption axis](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-modifiers-consumption), [Stack observation](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-stack-order)

### 5) Words and contracts: searchable channels for humans and AI

The dictionary has two tiers: **Core** holds the 69 canonical Words and is sealed, and **User** holds definitions made by `DEF`. Every Core Word is reachable by its plain name, so a program starts with the full vocabulary already in scope.

Each Core Word's contract is a machine-readable record in [`spec/words.json`](spec/words.json): arity, consumption, NIL policy, projection reason, error conditions, purity, effects, and documentation. That record is the single place the Word is defined; prose is a projection of it.

Every Word also has a **content identity** — a digest over its normalized definition and the identities of the Words it calls — so a change to a dependency changes the identity of everything downstream.

Spec links: [Word contracts](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-machine-word-contract), [Deterministic lookup](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-dictionary-resolution), [User Words](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-dictionary-mutation), [Contracts and Static Checking](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-contract)

## Safety model: safe by design

Safety in Ajisai is a property of ordinary value flow. Every operation lands in
one of two places:

- a well-formed operation that cannot produce a value becomes a **bubble** (`NIL`),
- a malformed use raises a **channel error**.

The two are kept apart on purpose. A bubble carries its reason downstream, and a
single `^` (`VENT`) turns it into a fallback at the end of a pipeline. A channel
error propagates to the top and halts evaluation, so misuse is reported rather
than absorbed into the result.

Two limits bound how much a program may do, and they mean different things:

- The **execution-step limit** bounds total work and raises its registered error
  when reached.
- The **materialization ceiling** bounds how large a single generated collection
  may become; exceeding it projects to `NIL` with reason `spaceExhausted` rather
  than aborting.

Both are host safety controls, not language-semantic constraints: their numeric
values are implementation freedom, but the outcome category each produces is
normative.

Beyond runtime, `ajisai check --contract` verifies a user word's `#:contract`
declaration against the Core contracts of the words it calls **without running
the program**. The check is deliberately conservative: it reports *verified*,
*violated*, or *cannot verify*, and anything outside the fragment it can analyze
is reported as *cannot verify*, never silently passed.

Spec links: [Work limits](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-machine-limits), [Value, absence, misuse](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-failure-trichotomy), [Errors](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-failure-error), [Pre-execution check](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-contract-check)

## A small taste

The **Expected value** column shows the final stack exactly as the language renders it (numbers display as exact fractions in `numerator/denominator` form).

| Sample code | Expected value | Notes |
| --- | --- | --- |
| `2 3 / 1 3 / +` | `1/1` | Exact rational arithmetic: two thirds plus one third is exactly one. |
| `[ 1 2 3 ] [ 4 5 6 ] +` | `[ 5/1 7/1 9/1 ]` | Element-wise arithmetic: equal-length vectors combine pairwise. |
| `1 0 / ^ 99` | `99/1` | Division by zero produces a bubble (`NIL`); `^` (`VENT`) replaces it with the fallback value. |
| `2 SQRT 2 LT` | `TRUE` | `SQRT` yields the exact algebraic √2 and compares it without rounding. |
| `8 SQRT 2 SQRT 2 SQRT + =` | `TRUE` | Values built through different histories are one value when they denote the same real. |

More examples are available in [`examples/`](examples/) and in the [Reference](https://masamoto1982.github.io/Ajisai/docs/index.html), where every sample opens in the Playground.

Spec links: [Source and Desugaring](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-source), [Value Domains](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-values), [Dictionary and Effects](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-dictionary)

## Runtime architecture

```text
Rust interpreter core → WASM boundary → TypeScript GUI/runtime shell
                              └──────→ Tauri desktop shell
```

- Rust core: tokenizer, value model, interpreter, Core words, tests
- WASM boundary: protocol conversion between Rust values and the TypeScript runtime
- TypeScript GUI: editor, dictionary sheets, execution controller, output rendering, platform adapters
- Tauri shell: desktop integration and host capabilities

Runtime-specific behavior such as persistence, file I/O, and host hooks is abstracted through [`src/platform/`](src/platform/).

Spec links: [Language identity](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-authority-identity), [Implementation freedom](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-authority-freedom), [Observation and Host Protocol](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html#lang-observation)

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
| [`spec/`](spec/) | Canonical sources: the semantic kernel, the 69-Word registry, the shared laws, presentation, and the host protocol |
| [`SPECIFICATION.html`](SPECIFICATION.html) | Generated from `spec/` ([rendered here](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html)) |
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

Ajisai redistributes third-party components (KaTeX, Tauri, and various Rust
crates). Their copyright and license notices are collected in
[`THIRD-PARTY-LICENSES.md`](THIRD-PARTY-LICENSES.md).
