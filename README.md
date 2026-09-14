![Rust](docs/assets/badges/rust.svg) ![WebAssembly](docs/assets/badges/webassembly.svg) ![TypeScript](docs/assets/badges/typescript.svg) ![Tauri](docs/assets/badges/tauri.svg) [Build and Deploy status](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai QR Code](public/images/Ajisai_QR_Small.png "Ajisai QR Code")

# Ajisai

Ajisai is an AI-first, vector-oriented dataflow language for auditable, exact vector computation with machine-readable contracts. Fractions and the Vector data structure carry the central role.

## Why Ajisai

I built Ajisai for two reasons. The first is that I had given up partway on learning every programming language I ever tried. The second is that I did not want to build something with a language — I wanted to build the tool itself.

What changed things for me was **Forth**: stack-oriented minimalism, pared back as far as it goes. Stripped of syntactic noise, it left the act of computation exposed, and it taught me that a language is allowed to be that small.

But I work in a world where manuals and established procedure matter, and I could not bring myself to give up the rigor of types. That left me with a sharp contradiction: I wanted types to matter, and I did not want to write them.

The answer was to carry that rigor in **machine-readable contracts on the words** rather than in annotations on values. Every Core Word states what it takes and how many, whether it is pure, and what it returns when it cannot produce a value; a user definition can carry the same kind of declaration, checked against those contracts before anything runs. There is nowhere in the language for a programmer to write a type — and the checking stays, on the side of the words.

Numbers follow the same principle. Write an integer or a decimal and the value is an exact rational either way, with `SQRT` extending the field out to algebraic irrationals. Nothing is rounded. The question of float versus double versus decimal does not exist here.

What made this uncompromising design practical was **AI**. Now that AI can read intent and act as a real partner in development, there is no need to dress a language up in syntactic sugar for human convenience. Taking an AI-first premise — a new kind of intelligence writing the code alongside me — I became convinced that a language could hold nothing but strict machine-readable rules and leave everything else to plain dataflow.

Exact numbers, and a stack to carry them. When that shape settled, the picture in my mind was water poured into a vessel. What fills it is water alone, never rounded; yet there is no limit to the shapes of the ripples AI and I spread across its surface.

As it happens, a flower takes its scientific name from the Greek for "water vessel": the hydrangea — *ajisai*. I began the work in June, in the middle of Japan's rainy season. Ajisai is the language I built with that picture in mind.

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
