![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")

# Ajisai

> **"Ajisai is a vessel of water."**

**A stack-based programming language inspired by FORTH**

**Demo:** [https://masamoto1982.github.io/Ajisai/](https://masamoto1982.github.io/Ajisai/)

---

## Design Philosophy: The Vessel of Water

The botanical name for hydrangea (Ajisai in Japanese) is *Hydrangea*, derived from the Greek words *hydor* (water) and *angos* (vessel) — literally meaning "vessel of water."

This etymology perfectly captures the essence of Ajisai's architecture.

### The Metaphor

Imagine a vessel filled with water. The substance within is singular — just water. Yet upon its surface, countless ripples can form, each creating different patterns of light and shadow.

In Ajisai:

| Concept | Metaphor | Technical Reality |
|:--------|:---------|:------------------|
| **Data** | Water | `Vec<Fraction>` — the sole truth |
| **Type** | Ripple | `DisplayHint` — interpretation for display only |
| **Shape** | Vessel | `shape: Vec<usize>` — dimensional structure |

Just as water conforms to its container while remaining fundamentally unchanged, Ajisai's data adapts its interpretation to context while maintaining a unified internal representation.

---

## Unified Fraction Architecture

Ajisai abolishes traditional type systems. All values exist as a single substance: **fractions**.

```rust
pub struct Value {
    pub data: Vec<Fraction>,       // The water (sole truth)
    pub display_hint: DisplayHint, // The ripple (display interpretation)
    pub shape: Vec<usize>,         // The vessel (dimensional shape)
}
```

### Data Duality

What users see as "types" are merely ripples on the surface:

| Appearance (Ripple) | Reality (Water) | Explanation |
|:--------------------|:----------------|:------------|
| `42` | `[42/1]` | Integers are fractions |
| `TRUE` | `[1/1]` | 1 is true, 0 is false |
| `'A'` | `[65/1]` | Character code (ASCII/Unicode) |
| `'Hello'` | `[72/1, 101/1, ...]` | Array of character codes |
| `NIL` | `[0/0]` | Sentinel value |

### Design Implications

- **No type errors**: Any data can mix with any other — they are all water
- **Structural validation**: Shape and length mismatches are caught, not "types"
- **Display freedom**: Words like `STR` and `NUM` only change the ripple pattern, not the water itself

This inherits FORTH's spirit: **trust the programmer**.

---

## Features

### Language Design

- **Stack-based with Reverse Polish Notation (RPN)**
  - FORTH-style stack operations

- **Exact Fraction Arithmetic**
  - All numbers internally represented as fractions — no rounding errors
  - Arbitrary precision through `num-bigint`

- **Vector-based Fractal Structure**
  - All container data represented as nestable Vectors
  - Bracket `[ ]` nesting expresses dimensions
  - Tensor-like operations (SHAPE, RESHAPE, etc.)
  - **Heterogeneous mixing**: `[ 1 'hello' TRUE [ 2 3 ] ]`
  - NumPy/APL-style broadcasting

- **The Rule of 3: Dimension and Call Depth Limits**

| Dim | Bracket | Visibility | Example |
|:---:|:-------:|:----------:|:--------|
| 0 | — | Invisible | Stack (implicit frame) |
| 1 | `{ }` | Visible | `{ 1 2 3 }` |
| 2 | `( )` | Visible | `{ ( 1 2 ) ( 3 4 ) }` |
| 3 | `[ ]` | Visible | `{ ( [ 1 ] [ 2 ] ) }` |

- **Built-in Word Protection**
  - Built-in words cannot be deleted or overwritten

### Visualization

- **Depth-based bracket styles**: `[ ]` → `{ }` → `( )` → `[ ]` (cycles every 3 levels)
- **Real-time state display**: Stack, dictionary, memory visible in GUI

### Technology Stack

| Component | Technology |
|:----------|:-----------|
| Core Interpreter | Rust |
| Runtime | WebAssembly |
| Frontend | TypeScript |
| Build Tool | Vite |
| CI/CD | GitHub Actions |

---

## Code Examples

### Vector Operations

```ajisai
# Creating vectors
[ 1 2 3 ]               # 1D vector: shape [3]
[ [ 1 2 ] [ 3 4 ] ]     # Nested vector: shape [2, 2]

# Heterogeneous data
[ 1 'hello' TRUE [ 2 3 ] ]

# Broadcasting arithmetic
[ 5 ] [ 1 2 3 ] +       # → [ 6 7 8 ]
[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +
# → [ [ 11 22 33 ] [ 14 25 36 ] ]
```

### Custom Word Definition

```ajisai
# Define a word that doubles a value
[ '[ 2 ] *' ] 'DOUBLE' DEF

# Usage
[ 5 ] DOUBLE    # → [ 10 ]

# Combine with higher-order functions
[ 1 2 3 4 5 ] 'DOUBLE' MAP    # → [ 2 4 6 8 10 ]
```

### Control Structure

```ajisai
# Conditional: TRUE if even, FALSE if odd
[ '[ 2 ] MOD [ 0 ] =' ] 'EVEN?' DEF

[ 4 ] EVEN?    # → [ TRUE ]
[ 7 ] EVEN?    # → [ FALSE ]
```

---

## Built-in Words

### Target Specification
`.` `..`

### Input Helpers
`'` `FRAME`

### Position Operations (0-indexed)
`GET` `INSERT` `REPLACE` `REMOVE`

### Quantity Operations
`LENGTH` `TAKE`

### Vector Operations
`SPLIT` `CONCAT` `REVERSE` `RANGE` `SORT`

### Constants
`TRUE` `FALSE` `NIL`

### String Operations
`CHARS` `JOIN`

### Parse/Convert
`NUM` `STR` `BOOL` `CHR`

### DateTime
`NOW` `DATETIME` `TIMESTAMP`

### Arithmetic
`+` `-` `*` `/` `MOD` `FLOOR` `CEIL` `ROUND`

### Comparison
`=` `<` `<=` `>` `>=`

### Logic
`AND` `OR` `NOT`

### Higher-Order Functions
`MAP` `FILTER` `FOLD`

### I/O
`PRINT`

### Music
`SEQ` `SIM` `PLAY`

### Word Management
`DEF` `DEL` `?`

### Control Flow
`TIMES` `WAIT` `:` `!`

### Random
`CSPRNG`

### Hash
`HASH`

---

## Local Development

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/) (v20+ recommended)

### Setup

```bash
# Clone the repository
git clone https://github.com/masamoto1982/Ajisai.git
cd Ajisai

# Install dependencies
npm install

# Build WASM
cd rust
wasm-pack build --target web --out-dir ../js/pkg
cd ..

# Build TypeScript
npm run build

# Start development server
npx vite
```

### Build

```bash
# Production build
npx vite build
```

---

## About AI-Driven Development

> The majority of this project's implementation was done by AI (Claude).
> From design decisions to Rust/TypeScript code implementation, test case creation, and documentation,
> this project is developed through human-AI collaboration.

---

## License

[MIT License](LICENSE)
