![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai QR Code](public/images/ajisai-qr.png "Ajisai QR Code")

# Ajisai

Ajisai is an AI-first, vector-oriented, continued-fraction-dataflow language.

It is designed around one strict promise: **value integrity first**. Every numeric value is an exact real backed by a finite or lazy continued fraction, structure is explicit, and behavior is mechanically testable through clear contracts.

The name *Ajisai* comes from hydrangea, whose scientific meaning is often interpreted as a “water vessel.” This project uses water as its core metaphor for how values move through code.

- Playground: https://masamoto1982.github.io/Ajisai/
- Desktop (Tauri) build channel is available in the same repository (`src-tauri/`).

---

## Why Ajisai (as a water system)

### 1) Continued fractions: water with an exact flow history

Ajisai no longer models its numeric world as “everything is internally a fraction.” Every numeric value is now an **exact real represented internally as a continued fraction**. Finite continued fractions cover the rational values Ajisai has always handled exactly; lazy infinite continued fractions let the runtime represent admitted irrationals such as `MATH@SQRT` results without collapsing them into approximate floats.

Surface numeric literals still look familiar — integers, fractions, decimals, and scientific notation — but they are convenience forms for the same continued-fraction representation. The canonical AI-readable serialization is the nested continued-fraction display form, not a source literal.

Like water that keeps both its volume and its channel history, Ajisai values preserve exactness while exposing a representation that can keep flowing beyond the rational sub-domain. Arithmetic operates on partial quotients directly, so operations do not detour through hidden floating-point approximations or truncated rationals.

Spec links: [§4.2 Scalar: exact-real continued-fraction arithmetic](SPECIFICATION.md#42-scalar-exact-real-continued-fraction-arithmetic), [§3.2 Numeric literal formats](SPECIFICATION.md#32-numeric-literal-formats), [§12.2 Display hints](SPECIFICATION.md#122-display-hints)

### 2) Bubble/NIL: bubbles that keep the flow inspectable

Ajisai treats absence as a first-class value: `NIL`. In the water metaphor, `NIL` is a bubble in the flow: not the water itself, but a meaningful part of the stream state.

The current failure model is the **Bubble Rule**:

> If the operation was well-formed but could not produce a value, it produces Bubble/NIL with a reason. If the operation was malformed, it raises an error.

For example, division by zero, an out-of-range `GET` on a valid vector, `NUM` parse failure, and invalid `CHR` code points produce reasoned Bubble/NIL values. Misusing a word — such as dividing by text or calling `GET` on a non-vector target — remains an error. Existing NIL-passthrough words preserve the reason as the bubble flows onward, and `=>` supplies a fallback when a bubble reaches a point where the program wants an ordinary value.

Spec links: [§4.5 NIL](SPECIFICATION.md#45-nil), [§4.5.1 NIL passthrough rule](SPECIFICATION.md#451-nil-passthrough-rule), [§11.2 Bubble Rule](SPECIFICATION.md#112-bubble-rule)

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

### 5) Safe mode (`~`): flood control for raised errors

`SAFE` is a boundary for errors that still raise, not the main way to make ordinary partial operations recoverable. If the next word raises an error — for example stack underflow, an unknown word, or a malformed input shape — `SAFE` converts that error to `NIL` with `safeCaught` diagnostic metadata. If the next word already produced a direct Bubble/NIL by the Bubble Rule, `SAFE` leaves that bubble and the word’s normal stack effect unchanged.

So the pipeline does not crash on raised errors that you explicitly guard, while ordinary “could not produce a value” cases can flow as reasoned bubbles without `SAFE`. In water terms: `SAFE` is still a spillway for incidents that would otherwise break the channel; bubbles already in the stream do not get relabeled at the gate.

Spec links: [§6.3 Safe mode modifier](SPECIFICATION.md#63-safe-mode-modifier), [§11.3 Safe mode behavior](SPECIFICATION.md#113-safe-mode-behavior)

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
