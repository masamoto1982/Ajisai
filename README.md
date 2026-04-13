![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")
![Ajisai QR Code](public/images/ajisai-qr.png "Ajisai QR Code")

# Ajisai

Ajisai is an **AI-first, vector-oriented, fractional-dataflow language**.  
Runtime stack: Rust interpreter core → WASM boundary → TypeScript GUI/runtime shell.

- Playground: https://masamoto1982.github.io/Ajisai/
- Canonical specification: `SPECIFICATION.md`

---

## The Water Model

In Ajisai, computation is **flow**.

Values stream through a single stack like water through a channel. Each word draws from the surface and deposits its result — the current always moves forward.

**Two planes, not one.**  
Below the surface lies the *data plane*: exact rational arithmetic, pure and lossless, untouched by display concerns.  
On the surface sits the *semantic plane*: the reflection — display hints consulted only at render time.  
What you see does not affect what computes.

**Flow is conserved.**  
Numbers are exact fractions; division never rounds, precision never leaks.  
The `,,` bifurcation modifier splits a stream without loss: both branches retain the source, like a river dividing around an island and rejoining intact.

**Errors are absorbed, not propagated.**  
Prefix any word with `~` to open a safe channel.  
If that operation fails, NIL settles in place of the missing value — a still pool where turbulence would otherwise flood upstream.

**Tributaries run in isolation.**  
`SPAWN` forks a child runtime from a code block, a separate stream carrying a snapshot of the parent's knowledge.  
`AWAIT` collects the tributary's final state when it rejoins.  
Parent and child never share a current.

---

## Modifiers

Modifiers precede a word and shape how flow passes through it.

| Modifier | Name | Behavior |
|----------|------|----------|
| `.` | StackTop (default) | Operate on the top value(s) |
| `..` | Stack | Operate on the entire stack |
| `,` | Consume (default) | Remove operands after the operation |
| `,,` | Bifurcation | Retain operands; also push result |
| `~` | Safe | Absorb errors; push NIL on failure |
| `==` | Pipeline | Visual separator; no runtime effect |
| `=>` | NIL coalescing | Replace NIL at top with the next stack value |

---

## Syntax at a Glance

```
# Define a word
{ 2 * } 'DOUBLE' DEF

# Map over a vector
[ 1 2 3 4 ] { DOUBLE } MAP     # → [ 2 4 6 8 ]

# Exact fractions — no rounding
1 3 /                          # → 1/3

# Bifurcation: retain source, also push result
3 ,,DOUBLE                     # stack: 3  6

# Safe channel: absorb the error, push NIL
[ 1 2 3 ] 9 ~GET               # → NIL

# NIL coalescing: replace NIL with a fallback
[ 1 2 3 ] 9 ~GET => 0          # → 0

# Spawn a child runtime
{ 100 RANGE { * } FOLD } SPAWN AWAIT
```

---

## Development Checks

```sh
cd rust && cargo test --lib
cd rust && cargo test --tests
cd rust && cargo test --test gui-interpreter-test-cases
npm run check
```

---

## License

MIT (`LICENSE`)
