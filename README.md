![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai QR Code](public/images/ajisai-qr.png "Ajisai QR Code")

# Ajisai

Ajisai is an AI-first, stack-oriented dataflow language in the Forth lineage.

Every numeric value is stored internally as a finite continued fraction of
arbitrary-precision integers, giving exact arithmetic on rational numbers
and a path to exact representation of selected irrational numbers. Programs
manipulate a single data stack; there is no return stack, in line with the
project's VTU (Very Thrifty Use) energy goal of avoiding mid-computation
memos.

The name *Ajisai* comes from hydrangea, whose scientific meaning is often
interpreted as a "water vessel." This project keeps water as its core
metaphor for how values flow through code.

- Playground: https://masamoto1982.github.io/Ajisai/
- Desktop (Tauri) build channel is available in the same repository (`src-tauri/`).

---

## Why Ajisai (as a water system)

### 1) Continued fractions: water that never evaporates

In Ajisai, every numeric value lives internally as a finite continued
fraction with arbitrary-precision integer partial quotients. Arithmetic
goes through exact rational pivots, so there is no hidden floating-point
drift and no silent rounding loss. Like water that keeps the same volume
no matter which channel it passes through, Ajisai values preserve
exactness across operations.

The canonical nested display form is `(a0 (a1 (a2 ...)))`, e.g.
`355/113` ≡ `(3 (7 (16)))`.

Spec link: [SPECIFICATION.md §4 Values](SPECIFICATION.md#4-values)

### 2) Bubble / Nil: bubbles that keep the flow inspectable

Ajisai treats absence as a first-class value: `Nil`. In the water
metaphor, `Nil` is a bubble in the flow — not the water itself, but a
meaningful part of the stream state. Nil propagates through arithmetic;
`1 0 /` yields Nil; `NIL?` lets a program inspect the bubble.

Spec link: [SPECIFICATION.md §4.3 Nil (the bubble)](SPECIFICATION.md#43-nil-the-bubble)

### 3) Stack-only control flow

A single data stack carries all state. There is no return stack, so the
runtime never has to "memorise" where to come back to — the VTU goal of
avoiding mid-computation memos for energy efficiency. User-word calls
re-execute the stored body in place, exposing all intermediate state on
the data stack.

Spec link: [SPECIFICATION.md §5 Execution model](SPECIFICATION.md#5-execution-model)

### 4) Three-layer errors

Every error carries three messages: a one-line `summary` for the GUI, a
`detail` explanation for an experienced user, and a `diagnosis` aimed at
AI tooling. The semantic-plane contract keeps human-readable strings out
of the machine-readable surface.

Spec link: [SPECIFICATION.md §5.4 Errors](SPECIFICATION.md#54-errors)

---

## Runtime architecture

Rust interpreter core → WASM boundary → TypeScript GUI/runtime shell

- Web Playground channel: Vite build (`npm run build:web`) for GitHub Pages
- Desktop channel: Tauri wrapper (`npm run tauri:build`, frontend via `npm run build:tauri-frontend`)
- Runtime-specific behaviour (Persistence / File I/O / Runtime hooks) is abstracted via `src/platform/` adapters

Formal definition: [`SPECIFICATION.md`](SPECIFICATION.md)

---

## Development checks

```sh
cd rust && cargo test --lib
npm run check
npm test
```

GUI behaviour checks can be run from the in-app `Test` button using cases in
`src/gui/gui-interpreter-test-cases.ts`.

---

## License

MIT (`LICENSE`)
