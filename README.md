![Rust](docs/assets/badges/rust.svg) ![WebAssembly](docs/assets/badges/webassembly.svg) ![TypeScript](docs/assets/badges/typescript.svg) ![Tauri](docs/assets/badges/tauri.svg) [Build and Deploy status](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai QR Code](public/images/Ajisai_QR_Small.png "Ajisai QR Code")

# Ajisai

Ajisai is an AI-first, vector-oriented dataflow language for auditable, exact vector computation with machine-readable contracts. Fractions and the Vector data structure carry the central role.

## Why Ajisai

Ajisai wasn't built to build something else with — it came from the pull of building a language itself, after years of not sticking with any other one.

The turning point was **Forth**: a stack-oriented minimalism that strips away syntactic noise and leaves computation exposed as a bare vessel. But giving up the rigor of types felt wrong too, which left a real contradiction: wanting types to matter without wanting to write them down.

The resolution was to unify every number around one exact representation — the **fraction**, closed under `SQRT`, free of rounding error — so a value's numeric shape is never something a program has to declare.

That uncompromising, unsweetened design became realistic once **AI** could act as a genuine collaborator in reading intent: with an AI-first premise, syntax doesn't need to be dressed up for human convenience — only machine-readable rules, and otherwise plain dataflow through a stack, need to exist.

The name comes from *hydrangea* (紫陽花), whose scientific name is often read as "water vessel" — a fitting namesake for a design that took shape, fittingly, during Japan's early-summer rainy season.

## Status

<table>
<tr><td>Release stage</td><td>Alpha</td></tr>
<tr><td>Specification</td><td>Regenerated from the implementation — see <a href="spec/README.md"><code>spec/README.md</code></a></td></tr>
<tr><td>Compatibility promise</td><td>None while alpha holds</td></tr>
</table>

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
