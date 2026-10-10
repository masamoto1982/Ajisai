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

Division by zero follows from the same two choices, and it is the part of the design I like best. Every number is a reduced pair of integers over a non-negative denominator, and division is total: it is multiplication by the reciprocal, and every pair has one. `1 0 DIV` is neither a trap nor an exception nor an absence but a number, `1/0`. Because the gcd of `n` and `0` is `|n|`, every pair over zero reduces to one of exactly three — `1/0`, `-1/0` and `0/0` — so `100 0 DIV` is `1/0` for the same reason `4/2` is `2/1`, and `1/0` is a literal as `1/2` is. The three points compute by the same pair formulas as everything else: `1/0 5 ADD` is `1/0`, `1 1/0 DIV` is `0`, `1/0 1/0 ADD` is `0/0`. Element-wise, a zero divisor marks exactly its own element: `[ 1 2 3 ] [ 1 0 2 ] DIV` is `[ 1 1/0 3/2 ]`, and every Word after it computes with the point by the same formulas. The price any arithmetic pays for letting division be total is paid here too, in the open: at the three points, and nowhere else, the laws of a field that mention zero give way — `0 1/0 MUL` is `0/0`, not `0`; `1/0 1/0 SUB` is `0/0`, not `0`; and distributivity fails there — and the machine does not restore them by special case. `0/0` absorbs everything it meets and has no place in the order, so an order asked of it is the one question arithmetic cannot answer, and the one place the two sides of the design meet: `0/0 1 LT` projects a reasoned absence, which reads as *unknown* in three-valued logic (`NIL-REASON` says `'domainMiss'`), while `0/0 0/0 EQ` is `TRUE` because identity is denotation. Absence itself stays what it was — the answer of a partial Word such as `-1 SQRT` — and the same two Words recover it: `-1 SQRT 'S' BIND 0 S S NIL? SELECT` is `0`. What all this buys is that the failure of one element costs one element: one zero among a million divisors costs one lane of storage and one lane of work, never the vector, and nothing is ever stored that is not a number.

None of that arithmetic is new, and it is worth saying whose it is. Reduced pairs of integers over a non-negative denominator, with `1/0`, `-1/0` and `0/0` as the three pairs over zero, are Anderson's *transrational* numbers, and the rules above — `0/0` absorbing every operation and lying outside the order, `1/0 1/0 SUB` and `0 1/0 MUL` both `0/0`, `1 1/0 DIV` zero — are transreal arithmetic restricted to the field Ajisai computes in; its signed infinities and nullity are the transreal ∞, −∞ and Φ under other names. Carlström's wheels totalize division the same way but with one unsigned point at infinity (`1/0` and `-1/0` collapse) beside a bottom element `⊥`, their `0/0`; Ajisai keeps the sign, which is what lets order stay decidable everywhere but at `0/0`. What Ajisai adds is not the arithmetic but where it puts it: the three points are literals a program can write, identity is denotation (so `0/0 0/0 EQ` is `TRUE`), and an order asked of `0/0` is a reasoned absence rather than a comparison defined to be false, so it reads as *unknown* downstream instead of silently choosing a branch.

Branching followed the same logic. `SELECT` takes the two candidate values and the truth that chooses between them, and nothing else: the candidates are values the program has already built, so a branch evaluates nothing and skips nothing. That is only affordable because Ajisai has no recursion and no unbounded loop — every arm is finite by construction — and because a failure that depends on data is a reasoned absence rather than a raise, so computing the arm that loses costs a value, never a crash. The choice is made element by element, so one `SELECT` branches a whole vector without a loop, and its cost is the sum of what it was handed rather than something only running can discover.

What made this uncompromising design practical was **AI**. Now that AI can read intent and act as a real partner in development, there is no need to dress a language up in syntactic sugar for human convenience. Taking an AI-first premise — a new kind of intelligence writing the code alongside me — I became convinced that a language could hold nothing but strict machine-readable rules and leave everything else to plain dataflow.

Exact numbers, and a stack to carry them. When that shape settled, the picture in my mind was water poured into a vessel. What fills it is water alone, never rounded; yet there is no limit to the shapes of the ripples AI and I spread across its surface.

As it happens, a flower takes its scientific name from the Greek for "water vessel": the hydrangea — *ajisai*. I began the work in June, in the middle of Japan's rainy season. Ajisai is the language I built with that picture in mind.

## Status

<table>
<tr><td>Release stage</td><td>Beta</td></tr>
<tr><td>Version</td><td>1.0.0-beta.1 (implementation and specification)</td></tr>
<tr><td>Specification</td><td>Regenerated from the implementation — see <a href="spec/README.md"><code>spec/README.md</code></a></td></tr>
<tr><td>Compatibility promise</td><td>From 1.0.0. Until then a breaking change raises the specification version and is named in the release that ships it</td></tr>
</table>

## Ten concepts

Ajisai is built from ten concepts and nothing else.

1. Exact arithmetic with no rounding over reduced pairs of integers, closed under division: `1/0`, `-1/0` and `0/0` are the three numbers over zero, every pair over zero reduces to one of them, and `SQRT` extends the field to algebraic irrationals. Equality decides everywhere; order decides everywhere but at `0/0`.
2. Three outcomes: a value, a reasoned absence, or an error — with three-valued truth, and a program may declare either failing outcome itself.
3. A stack of values, and vectors of values, text included.
4. Shape and rank: element-wise lifting follows a vector's shape, which a program can read and rewrite.
5. Keyed correspondence: Records, and tables as Records of columns.
6. Code is a vector, evaluated only when a Word asks for it — and branching is not one of those Words.
7. One modifier axis: consume or keep.
8. A two-tier dictionary — sealed Core, user-defined User — with content-addressed identity that a program can ask for.
9. A machine-readable contract for every Word, and a pre-execution check of user declarations (`#:contract DOUBLE inputs=1 outputs=1 …`) against those contracts, run by `check` and by `compute` before anything executes — both readable from inside the language.
10. One host protocol, the only way anything outside the language observes it, and an executable conformance corpus that decides whether an implementation is Ajisai.

## Documentation

| Document | Audience | Rendered at |
|---|---|---|
| Specification | Builders and porters | [SPECIFICATION.html](https://masamoto1982.github.io/Ajisai/SPECIFICATION.html) |
| Reference (English) | Ajisai users | [docs/en/index.html](https://masamoto1982.github.io/Ajisai/docs/en/index.html) — from a program down to its elements and every built-in Word with its contract (generated from `spec/words.json`), with runnable samples |
| Reference (Japanese) | Ajisai users | [docs/ja/index.html](https://masamoto1982.github.io/Ajisai/docs/ja/index.html) — the same content in Japanese |
| Playground | Run it now | [masamoto1982.github.io/Ajisai](https://masamoto1982.github.io/Ajisai/) — its Reference button opens the Reference in the UI language, English or Japanese |

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
