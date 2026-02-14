![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")

# Ajisai

> **"Ajisai is a vessel of water."**

**A Vector-oriented programming language**

Ajisai inherits **postfix notation** and the **dictionary system** from FORTH. The center of its data structure is not the stack but the **Vector**.

**Demo:** [https://masamoto1982.github.io/Ajisai/](https://masamoto1982.github.io/Ajisai/)

---

## Design Philosophy: The Vessel of Water

The botanical name for hydrangea (Ajisai in Japanese) is *Hydrangea*, derived from the Greek words *hydor* (water) and *angos* (vessel) — literally meaning "vessel of water."

This etymology captures the essence of Ajisai's architecture.

| Concept | Metaphor | Technical Reality |
|:--------|:---------|:------------------|
| **Data** | Water | `Fraction` — the sole truth |
| **Type** | Ripple | `DisplayHint` — interpretation for display only |
| **Shape** | Vessel | Dimensional structure derived from nesting |
| **NIL** | Bubble | `ValueData::Nil` — exists in water but is not water |
| **CodeBlock** | How to pour water | Deferred code that produces water when executed |

---

## Unified Fraction Architecture

In Ajisai, all computational data exists as a single substance: **fractions**. There is no type system.

```rust
pub struct Value {
    pub data: ValueData,            // Water (recursive data structure)
    pub display_hint: DisplayHint,  // Ripple (display interpretation)
    pub audio_hint: Option<AudioHint>, // Music DSL metadata
}

pub enum ValueData {
    Scalar(Fraction),       // A single fraction
    Vector(Vec<Value>),     // Array of Values (recursively nestable)
    Nil,                    // Absence of value (bubble)
    CodeBlock(Vec<Token>),  // Deferred code (not a fraction)
}
```

### Data Duality

What users see as "types" are merely ripples on the surface:

| Appearance (Ripple) | Reality (Water) | Explanation |
|:--------------------|:----------------|:------------|
| `42` | `Scalar(42/1)` | Integers are fractions |
| `TRUE` | `Scalar(1/1)` + Boolean hint | 1 is true, 0 is false |
| `'A'` | `Scalar(65/1)` + String hint | Character code (Unicode) |
| `'Hello'` | `Vector([72/1, 101/1, ...])` + String hint | Array of character codes |
| `NIL` | `Nil` | Absence of value |

---

## Features

### Language Design

- **Vector-oriented with Reverse Polish Notation (RPN)**
  - FORTH-inherited postfix notation and dictionary system
  - No stack manipulation words (DUP, SWAP, ROT, OVER do not exist)

- **Exact Fraction Arithmetic**
  - All numbers internally represented as fractions — no rounding errors
  - Arbitrary precision through `num-bigint`

- **Recursive Vector Structure**
  - All container data represented as nestable Vectors
  - Bracket `[ ]` nesting expresses dimensions
  - Tensor operations (SHAPE, RESHAPE, TRANSPOSE, FILL)
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
[ 1 2 3 ]               # 1D vector: { 1 2 3 }
[ [ 1 2 ] [ 3 4 ] ]     # Nested: { ( 1 2 ) ( 3 4 ) }

# Broadcasting arithmetic
[ 5 ] [ 1 2 3 ] +       # -> { 6 7 8 }
[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +
# -> { ( 11 22 33 ) ( 14 25 36 ) }
```

### Custom Word Definition

```ajisai
# Define a word that doubles a value
: [ 2 ] * ; 'DOUBLE' DEF

# Usage
[ 5 ] DOUBLE    # -> { 10 }

# Combine with higher-order functions
[ 1 2 3 4 5 ] 'DOUBLE' MAP    # -> { 2 4 6 8 10 }
```

### Control Structure (Guards with Chevron Branching)

```ajisai
# Multi-branch guard (>> for conditions, >>> for default)
:
  >> [ 0 ] <
  >> [ -1 ]
  >> [ 0 ] =
  >> [ 0 ]
  >>> [ 1 ]
; 'SIGN' DEF

[ -5 ] SIGN    # -> { -1 }
[ 0 ] SIGN     # -> { 0 }
[ 10 ] SIGN    # -> { 1 }
```

### Pipeline Operator (`==`)

The pipeline operator is a visual marker for data flow (no-op):

```ajisai
# Readable data transformation pipeline
[ 1 2 3 4 5 ]
  == : [ 2 ] * ; MAP           # Double each: { 2 4 6 8 10 }
  == : [ 5 ] < NOT ; FILTER    # Keep >= 5:   { 6 8 10 }
  == [ 0 ] : + ; FOLD          # Sum:         { 24 }
```

### Safe Mode and Nil Coalescing

```ajisai
# Safe mode (~): convert errors to NIL
[ 1 2 3 ] [ 10 ] ~ GET           # -> NIL (index out of bounds)
[ 1 2 3 ] [ 10 ] ~ GET => [ 0 ]  # -> { 0 } (with default)
```

---

## Built-in Words

### Modifiers
`.` `..` `,` `,,` `~` `!` `==` `=>`

### Position Operations (0-indexed)
`GET` `INSERT` `REPLACE` `REMOVE`

### Quantity Operations
`LENGTH` `TAKE`

### Vector Operations
`SPLIT` `CONCAT` `REVERSE` `RANGE` `REORDER` `COLLECT` `SORT`

### Constants
`TRUE` `FALSE` `NIL`

### String Operations
`CHARS` `JOIN`

### Format Conversion
`NUM` `STR` `BOOL` `CHR`

### Arithmetic
`+` `-` `*` `/` `MOD` `FLOOR` `CEIL` `ROUND`

### Comparison
`=` `<` `<=`

### Logic
`AND` `OR` `NOT`

### Higher-Order Functions
`MAP` `FILTER` `FOLD`

### Tensor Operations
`SHAPE` `RANK` `RESHAPE` `TRANSPOSE` `FILL`

### I/O
`PRINT`

### Control Flow
`TIMES` `WAIT` `>>` `>>>` `:` `;`

### Word Management
`DEF` `DEL` `?`

### Meta
`EXEC` `EVAL` `HASH` `CSPRNG`

### DateTime
`NOW` `DATETIME` `TIMESTAMP`

### Music DSL
`SEQ` `SIM` `PLAY` `CHORD` `SLOT` `GAIN` `GAIN-RESET` `PAN` `PAN-RESET` `FX-RESET` `ADSR` `SINE` `SQUARE` `SAW` `TRI`

### Input Helpers
`'` `FRAME`

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

> This project is developed through human-AI collaboration using generative AI.
> From design decisions to Rust/TypeScript code implementation, test case creation, and documentation,
> AI assistance is utilized throughout the development process.

---

## License

[MIT License](LICENSE)
