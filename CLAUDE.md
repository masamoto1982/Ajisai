# Ajisai

A vector-oriented, fractional-dataflow programming language with a Rust (WASM) core and TypeScript web GUI.

## Stack

- **Core interpreter**: Rust → WebAssembly (`rust/` dir, crate `ajisai-core`)
- **Web frontend**: TypeScript + Vite (`js/` dir)
- **Package manager**: npm (no lockfile committed)
- **WASM target**: `wasm32-unknown-unknown` with SIMD128 enabled (`rust/.cargo/config.toml`)
- **Key Rust deps**: `wasm-bindgen`, `num-bigint`, `serde`, `smallvec`, `chrono`
- **Key TS config**: ES2022, strict mode, bundler resolution, `noEmit` (Vite handles bundling)

## Commands

| Command | What it does |
|---------|-------------|
| `npm run dev` | Start Vite dev server on port 3000 |
| `npm run build` | Run `tsc` (type-check only, `noEmit`) |
| `npm run check` | Run `tsc --noEmit` |
| `npm run preview` | Vite preview of built output |
| `cd rust && cargo test --lib` | Run Rust unit/integration tests (rlib mode) |
| `cd rust && cargo check --target wasm32-unknown-unknown` | Verify WASM build compiles |
| `cd rust && cargo bench` | Run interpreter performance benchmarks |

WASM build uses `wasm-pack` (not in package.json scripts). Pre-built WASM artifacts exist at `js/pkg/` and `public/wasm/`.

## Language Spec (current)

Canonical spec: `SPECIFICATION.md` (single source of truth).

### Data model

- All values are `Fraction` (arbitrary-precision `BigInt` numerator/denominator via `num-bigint`).
- `ValueData` enum: `Scalar(Fraction)` | `Vector(Rc<Vec<Value>>)` | `Record { pairs, index }` | `Nil` | `CodeBlock(Vec<Token>)`.
- `Value` struct contains only `data: ValueData` — no display hints in data plane.
- `DisplayHint` (Auto | Number | String | Boolean | DateTime | Nil) lives in `SemanticRegistry`, separate from data plane.
- Strings are vectors of Unicode code points with a String display hint.
- Booleans are `Scalar(1/1)` or `Scalar(0/1)` with Boolean display hint.
- `NIL` is a distinct variant, not a fraction. Empty brackets `[ ]` are errors.

### Two-layer architecture

- **Data plane**: pure `Value`/`ValueData` stack — all computation happens here.
- **Semantic plane**: `SemanticRegistry` holds `DisplayHint` and `ValueExt` metadata, queried only at display/output boundaries (PRINT, STR, GUI).

### Fractional Dataflow

- Each value has a `FlowToken { id, total, remaining, shape }` tracking consumption.
- Operations consume from remaining; `remainder = total - consumed`.
- Conservation law: `initial_total = Σ(consumed) + final_remainder`.
- Bifurcation (`,,`): splits flow mass into child branches, not value copies.

### Execution model

- Post-fix notation, dictionary-based (inherited from FORTH).
- No type system — everything is fractions.
- Call depth limit: max 5 hierarchy levels (main context + 4-step custom word chain = fingers on one hand). `MAX_CALL_DEPTH = 4` counts the chain steps; built-in words don't count.
- Nesting limit: none. Vectors always use `[]`; depth distinguished by color in GUI. `{}` and `()` are code block delimiters only.
- Broadcast: NumPy/APL-style shape broadcasting for arithmetic.

### Modifiers

| Modifier | Effect |
|----------|--------|
| `.` | Target stack top (default) |
| `..` | Target entire stack |
| `,` | Consume operands (default) |
| `,,` | Bifurcation — keep operands + result |
| `~` | Safe mode — errors become NIL |
| `!` | Force flag — allow DEL/DEF of dependent words |
| `==` | Pipeline — visual no-op marker |
| `=>` | Nil coalescing — fallback if NIL |

### Word signature types

- **Map** (element-wise transform): STR, NUM, BOOL, CHR, CHARS, JOIN, NOT, FLOOR, CEIL, ROUND, SHAPE, RANK, PRINT
- **Form** (structural): GET, INSERT, REPLACE, REMOVE, LENGTH, TAKE, SPLIT, CONCAT, REVERSE, RANGE, REORDER, SORT, MAP, FILTER, FOLD, COND, RESHAPE, TRANSPOSE, FILL
- **Fold** (reduction): `+` `-` `*` `/` MOD `=` `<` `<=` AND OR
- **None**: TRUE, FALSE, NIL, NOW, CSPRNG, DATETIME, TIMESTAMP, DEF, DEL, `?`, `:` `;` `=>` `==` `.` `..` `,` `,,` `~` `!` COLLECT, IDLE, EXEC, EVAL, HASH, FRAME, IMPORT

### Modules (via IMPORT)

- `music`: SEQ, SIM, PLAY, CHORD, SLOT, GAIN, PAN, ADSR, waveforms (SINE/SQUARE/SAW/TRI)
- `json`: PARSE, STRINGIFY, GET, KEYS, SET
- `io`: INPUT, OUTPUT

### Key principles

- "No change is error": if a word's output equals input, it's an error (e.g., `[1] REVERSE`, `[1 2 3] SORT` when already sorted).
- Default consumption: all words consume operands; use `,,` to bifurcate.
- No stack manipulation words (DUP/SWAP/ROT/OVER are prohibited).
- No `>` or `>=` operators — use `<` and `<=` with operand order.
- No backward compatibility guarantees.

## Architecture

### Rust core (`rust/src/`)

| Path | Responsibility |
|------|---------------|
| `types/mod.rs` | `Value`, `ValueData`, `Token`, `FlowToken`, `SemanticRegistry`, `WordDefinition`, `Stack` |
| `types/fraction.rs` | `Fraction` (BigInt numerator/denominator, auto-reduce, GCD) |
| `types/display.rs` | Display formatting for values |
| `types/json.rs` | JSON serialization/deserialization for types |
| `tokenizer.rs` | Tokenizer: source string → `Vec<Token>` |
| `interpreter/mod.rs` | `Interpreter` struct and main eval loop |
| `interpreter/arithmetic.rs` | `+` `-` `*` `/` MOD FLOOR CEIL ROUND |
| `interpreter/comparison.rs` | `=` `<` `<=` |
| `interpreter/logic.rs` | AND OR NOT (Kleene three-valued logic for NIL) |
| `interpreter/control.rs` | EXEC, EVAL |
| `interpreter/cast.rs` | STR NUM BOOL CHR type conversions |
| `interpreter/dictionary.rs` | DEF DEL `?` word management |
| `interpreter/higher-order-operations.rs` | MAP FILTER FOLD |
| `interpreter/vector-execution-operations.rs` | EXEC EVAL |
| `interpreter/tensor-shape-operations.rs` | SHAPE RANK RESHAPE TRANSPOSE FILL |
| `interpreter/simd-vector-operations.rs` | SIMD-optimized vector ops |
| `interpreter/vector_ops/` | GET INSERT REPLACE REMOVE TAKE SPLIT CONCAT REVERSE RANGE REORDER SORT LENGTH COLLECT |
| `interpreter/modules.rs` | IMPORT, module registration |
| `interpreter/audio.rs` | Music module implementation |
| `interpreter/io.rs` | IO module (INPUT/OUTPUT) |
| `interpreter/json.rs` | JSON module (PARSE/STRINGIFY/GET/KEYS/SET) |
| `interpreter/hash.rs` | HASH word |
| `interpreter/random.rs` | CSPRNG |
| `interpreter/datetime.rs` | NOW DATETIME TIMESTAMP |
| `interpreter/sort.rs` | SORT implementation |
| `builtins/builtin-word-definitions.rs` | Built-in word registry (name, description, syntax, signature type) |
| `builtins/builtin-word-details.rs` | Detailed built-in word info for GUI |
| `error.rs` | `AjisaiError` enum (all error variants) |
| `wasm-interpreter-bindings.rs` | `AjisaiInterpreter` WASM API via wasm-bindgen |
| `lib.rs` | Crate root, module declarations |

### TypeScript frontend (`js/`)

| Path | Responsibility |
|------|---------------|
| `web-app-entrypoint.ts` | Application entry point |
| `wasm-module-loader.ts` | WASM module loading |
| `wasm-interpreter-types.ts` | TypeScript types for WASM interop |
| `gui/gui-application.ts` | Main GUI application orchestration |
| `gui/code-input-editor.ts` | Code input editor component |
| `gui/execution-controller.ts` | Execution flow control |
| `gui/step-executor.ts` | Step-by-step execution |
| `gui/output-display-renderer.ts` | Output rendering |
| `gui/value-formatter.ts` | Value display formatting |
| `gui/dictionary-element-builders.ts` | Dictionary UI builders |
| `gui/vocabulary-state-controller.ts` | Word dictionary state |
| `gui/module-selector-sheets.ts` | Module selection UI |
| `gui/interpreter-state-persistence.ts` | State persistence via IndexedDB |
| `gui/gui-test-runner.ts` | GUI-level test runner |
| `gui/gui-interpreter-test-cases.ts` | GUI test case definitions |
| `workers/interpreter-execution-worker.ts` | Web Worker for interpreter execution |
| `workers/execution-worker-manager.ts` | Worker lifecycle management |
| `audio/audio-engine.ts` | Web Audio API engine for music module |

### Tests

- Rust unit tests: inline in source files + `rust/tests/` integration tests
- `rust/tests/gui-interpreter-test-cases.rs`: interpreter behavior tests
- `rust/tests/fractional-dataflow-behavior-tests.rs`: flow conservation tests
- `rust/src/tokenizer-regression-tests.rs`: tokenizer regression tests
- `rust/benches/interpreter-performance-benchmarks.rs`: criterion benchmarks
- `.ajisai` files in repo root and `examples/`: sample programs / manual test cases

### Entry points

- Web app: `index.html` → `js/web-app-entrypoint.ts`
- Language reference: `language-reference-playground.html`
- Public docs: `public/docs/*.html`

## Critical Rules

If adding a new built-in word, then register it in `rust/src/builtins/builtin-word-definitions.rs` AND implement the execution branch in the interpreter eval loop.

If implementing arithmetic/comparison/logic operations, then use the flat tensor pipeline: flatten → shape/stride → broadcast index → rebuild. Do not recurse over nested Value trees.

If a word produces output identical to its input, then it must raise `AjisaiError::NoChange`. This is mandatory, not optional.

If modifying `Value` or `ValueData`, then never add `display_hint` or metadata fields — these belong in `SemanticRegistry`.

If implementing a new Fold-type word with NIL operands, then follow Kleene three-valued logic: absorb NIL when the result is logically determined, propagate NIL otherwise.

If naming a Rust function, then follow the naming-as-index convention: `action_object` pattern, use only approved verbs (`parse`, `resolve`, `build`, `create`, `collect`, `extract`, `lookup`, `register`, `apply`, `execute`, `compute`, `format`, `render`, `emit`). No `handle`, `process`, `do`, `manage`, `helper`, `util`.

If naming a file, then use lowercase kebab-case exposing domain → role → subject (see `docs/guide-file-naming-convention.md`).

If writing Rust code, then add explicit type annotations on `collect()`, `unwrap()`, and function return bindings. Limit iterator chains to 3 stages max. Use guard clauses for early returns.

If tests exist for modified code, then run `cd rust && cargo test --lib` before considering the change complete.

If adding `>` or `>=` comparison operators, then stop — these are intentionally omitted. Use `<` and `<=` with swapped operands.

If tempted to add DUP/SWAP/ROT/OVER or any stack manipulation word, then stop — these are prohibited by design. Use REORDER, `.. GET`, or `,,` bifurcation.

If the custom word call chain exceeds 4 steps (`MAX_CALL_DEPTH`), then it is an error. Main + 4 = 5 total hierarchy levels (fingers on one hand). Do not increase this limit.

If nesting becomes very deep, avoid introducing artificial limits unless explicitly required by specification. Vectors always use `[]`; `{}` and `()` are code block delimiters only.

If implementing backward-compatibility shims, deprecated paths, or feature flags for old behavior, then stop — Ajisai prohibits backward compatibility maintenance.

## Docs

> **Note:** Ajisai is under active development. All documentation other than `SPECIFICATION.md` has been cleared and replaced with placeholder notices. They will be published once the language stabilizes.

| Document | Content |
|----------|---------|
| `SPECIFICATION.md` | Canonical language specification (single source of truth) |
| `examples/*.ajisai` | Sample programs |
