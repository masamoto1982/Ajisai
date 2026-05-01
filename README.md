![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai QR Code](public/images/ajisai-qr.png "Ajisai QR Code")

# Ajisai

Ajisai is an AI-first, vector-oriented, fractional-dataflow language.

It is designed around one strict promise: **value integrity first**. Every numeric value stays exact, structure is explicit, and behavior is mechanically testable through clear contracts.

The name *Ajisai* comes from hydrangea, whose scientific meaning is often interpreted as a “water vessel.” This project uses water as its core metaphor for how values move through code.

- Playground: https://masamoto1982.github.io/Ajisai/
- Desktop (Tauri) build channel is available in the same repository (`src-tauri/`).

---

## Why Ajisai (as a water system)

### 1) Exact fractions: water that never evaporates

In Ajisai, all numeric values are treated as exact rationals internally. No hidden floating-point drift, no silent rounding loss.

Like water that keeps the same volume no matter which channel it passes through, Ajisai values preserve exactness across operations.

Spec links: [§4.2 Scalar: exact rational arithmetic](SPECIFICATION.md#42-scalar-exact-rational-arithmetic), [§3.2 Numeric literal formats](SPECIFICATION.md#32-numeric-literal-formats)

### 2) Vectors with NIL: bubbles inside the vessel

A `Vector` is Ajisai’s vessel: ordered, nestable, and indexable. Unlike many systems that force “all present values,” Ajisai explicitly allows absence via `NIL`, and `NIL` can flow through vector pipelines by rule.

In the water metaphor, `NIL` is a bubble in the flow: not the water itself, but a meaningful part of the stream state.

Spec links: [§4.3 Vector](SPECIFICATION.md#43-vector), [§4.5 NIL](SPECIFICATION.md#45-nil), [§4.5.1 NIL passthrough rule](SPECIFICATION.md#451-nil-passthrough-rule)

### 3) 0-origin and 1-origin by function role: choosing the right measuring scale

Ajisai uses index semantics intentionally, not accidentally. Core vector indexing is 0-origin (including negative index support), while module/runtime words may define their own domain contracts where needed.

Think of it as switching rulers depending on the lock gate you operate: one scale for structural addressing, another for domain-facing conventions.

Spec links: [§4.3 Vector](SPECIFICATION.md#43-vector), [§7 Built-in Words](SPECIFICATION.md#7-built-in-words), [§9 Module System](SPECIFICATION.md#9-module-system)

### 4) Target mode and consumption mode: where water acts, and whether it branches

Ajisai modifiers provide two orthogonal controls:

- **Target mode** (`TOP` / `STAK`): where the operation applies (surface point vs full stream).
- **Consumption mode** (`EAT` / `KEEP`): whether the source flow is consumed or bifurcated.

`KEEP` (``,,``) acts like a branch channel: it preserves source context while emitting a new result path.

Spec links: [§6.1 Target modifiers](SPECIFICATION.md#61-target-modifiers), [§6.2 Consumption modifiers](SPECIFICATION.md#62-consumption-modifiers), [§11.5 KEEP modifier semantics](SPECIFICATION.md#115-keep-modifier-semantics)

### 5) Safe mode (`~`): flood control without losing diagnostics

`SAFE` projects partial operations into total ones by turning runtime errors into `NIL` with structured reason metadata.

So the pipeline does not crash, but the cause is not erased. In water terms: pressure is released into a controlled spillway, and the incident report is still attached.

Spec links: [§6.3 Safe mode modifier](SPECIFICATION.md#63-safe-mode-modifier), [§11.4 Safe mode behavior](SPECIFICATION.md#114-safe-mode-behavior)

---

## Runtime architecture

Rust interpreter core → WASM boundary → TypeScript GUI/runtime shell

- Web Playground channel: Vite build (`npm run build:web`) for GitHub Pages
- Desktop channel: Tauri wrapper (`npm run tauri:build`, frontend via `npm run build:tauri-frontend`)
- Runtime-specific behavior (Persistence / File I/O / Runtime hooks) is abstracted via `src/platform/` adapters

Formal definition: [`SPECIFICATION.md`](SPECIFICATION.md)  
Quality process policy: [`docs/quality/QUALITY_POLICY.md`](docs/quality/QUALITY_POLICY.md)

---

## Development checks

```sh
cd rust && cargo test --lib
cd rust && cargo test --tests
npm run check
```

GUI behavior checks can be run from the in-app `Test` button using cases in `src/gui/gui-interpreter-test-cases.ts`.

---

## License

MIT (`LICENSE`)
