![Rust](docs/assets/badges/rust.svg) ![WebAssembly](docs/assets/badges/webassembly.svg) ![TypeScript](docs/assets/badges/typescript.svg) ![Tauri](docs/assets/badges/tauri.svg) [Build and Deploy status](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai QR Code](public/images/Ajisai_QR_Small.png "Ajisai QR Code")

# Ajisai

An AI-first, vector-oriented dataflow language for auditable, exact vector computation with machine-readable contracts.

| | |
|---|---|
| Numbers | Exact — no rounding, ever |
| Vectors | The one way to hold many values; arithmetic and comparison lift element-wise |
| Partial failure | Stays visible and diagnosable — never a silently coerced value |
| Contracts | Every built-in Word carries a machine-readable one, checkable before a program runs |

## Status

| | |
|---|---|
| Release stage | `0.2.0-alpha.1` |
| Specification | Withdrawn from beta and being regenerated from the implementation — see [`spec/README.md`](spec/README.md) |
| Compatibility promise | None while alpha holds |

## The language in one picture

The name *Ajisai* comes from hydrangea, often interpreted as a "water vessel." Ajisai uses water as its main metaphor.

| Water metaphor | Language meaning | Observable idea |
|---|---|---|
| Vessel | the Stack — a Vector holding what has been poured in | `[ ]`, nesting, indexing |
| Water | a value poured into the vessel; exact rationals (extended by `SQRT`) are its purest form | Scalars, booleans, strings, vectors, code blocks |
| Flow | water moving between pouring and the ripple it leaves | Dataflow through operations |
| Ripple | the trace a flow leaves at the surface | `PRINT` output |
| Bubble | a well-formed operation could not produce a value | `NIL`, carrying a machine-readable reason |
| Breach | the vessel or channel gives way — malformed use | raised error, propagates and halts evaluation |

## Ten concepts

Ajisai is built from ten concepts and nothing else.

1. Exact rational arithmetic, closed under square roots, with no rounding.
2. Three outcomes: a value, a reasoned absence, or an error.
3. A stack of values and vectors of values.
4. Code blocks, evaluated only when a Word asks for it.
5. One modifier axis: consume or keep.
6. A two-tier dictionary — sealed Core, user-defined User — with content-addressed identity.
7. A machine-readable contract for every Word.
8. A pre-execution check of user declarations against those contracts.
9. One host protocol, which is the only way anything outside the language observes it.
10. An executable conformance corpus that decides whether an implementation is Ajisai.

## Documentation

| Document | Audience | Rendered at |
|---|---|---|
| Specification | Builders and porters | [SPECIFICATION.html](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html) |
| Reference (Japanese) | Ajisai users | [docs/ja/index.html](https://masamoto1982.github.io/Ajisai/docs/ja/index.html) |
| Reference (English) | Ajisai users | Not yet published — regenerating from the Japanese edition, see [`docs/dev/reference-ja-restructure-handoff.md`](docs/dev/reference-ja-restructure-handoff.md) §3.4/§6.3 |
| Playground | Run it now | [masamoto1982.github.io/Ajisai](https://masamoto1982.github.io/Ajisai/) — its Reference button links to the Japanese edition in the meantime |

## Build and run

| Task | Command |
|---|---|
| Install dependencies | `npm ci` |
| Dev server | `npm run dev` |
| Build the WASM core | `npm run build:wasm` |
| Build for the browser | `npm run build` |
| Build the desktop app (Tauri) | `npm run tauri:build` |
| Run the Rust test suite | `cargo test --all-targets` (in `rust/`) |

The MCP server for AI agents lives in [`tools/mcp-server/`](tools/mcp-server/README.md#install-and-connect).

## License

MIT — see [`LICENSE`](LICENSE).
