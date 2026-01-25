![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")

# Ajisai

**A stack-based programming language inspired by FORTH**

**Demo:** [https://masamoto1982.github.io/Ajisai/](https://masamoto1982.github.io/Ajisai/)

---

## About AI-Driven Development

> The majority of this project's implementation was done by AI (Claude).
> From design decisions to Rust/TypeScript code implementation, test case creation, and documentation,
> this project is developed through human-AI collaboration.

---

## Overview

Ajisai provides a stack-based interpreter running on WebAssembly and an interactive web-based GUI.

The name "Ajisai" (hydrangea in Japanese) metaphorically represents FORTH's characteristic of small words coming together to form functionality, like how small flowers come together to form a hydrangea cluster. (Note: What appears to be petals are actually sepals.)

---

## Features

### Language Design

- **Stack-based with Reverse Polish Notation (RPN)**
  - FORTH-style stack operations

- **Vector-based Fractal Structure**
  - All container data is represented as nestable Vectors (similar to LISP's list structure)
  - Bracket `[ ]` nesting expresses dimensions, with tensor-like operations (SHAPE, RESHAPE, etc.) supported
  - **Heterogeneous data mixing**: Numbers, strings, booleans, and Vectors can be freely combined, e.g., `[ 1 'hello' TRUE [ 2 3 ] ]`
  - NumPy/APL-style broadcasting

- **The "Rule of 3": Dimension and Call Depth Limits**
  - Dimension 0: Stack (invisible, GUI frame)
  - Dimensions 1-3: Visible nesting (nesting beyond 3 dimensions causes an error)
  - Call depth: Maximum 3 (`A -> B -> C`)

| Dim | Bracket | Visibility | Structure |
|:---:|:---:|:---:|:---|
| 0 | — | Invisible | Stack (implicit outermost shell) |
| 1 | `{ }` | Visible | `{ 1 2 3 }` |
| 2 | `( )` | Visible | `{ ( 1 2 ) ( 3 4 ) }` |
| 3 | `[ ]` | Visible | `{ ( [ 1 ] [ 2 ] ) }` |

- **Exact Fraction Arithmetic**
  - All numbers are internally treated as fractions - no rounding errors
  - Capable of handling extremely large numbers

- **Unified Fraction Architecture (Typeless Design)**
  - All values are internally represented as `Vec<Fraction>`
  - Distinction between numbers, booleans, and strings only at display time (DisplayHint)
  - No type checking - inheriting FORTH's spirit of freedom

- **Built-in Word Protection**
  - Built-in words cannot be deleted or overwritten

### Visualization

- **Depth-based bracket styles**: `[ ]` → `{ }` → `( )` → `[ ]` ... (cycles every 3 levels)

- **Real-time state display**: Stack, dictionary, memory usage visible in GUI

### Technology Stack

| Component | Technology |
|:---|:---|
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
[ [ 1 2 ] [ 3 4 ] ]     # Nested vector (matrix-like): shape [2, 2]

# Heterogeneous data
[ 1 'hello' TRUE [ 2 3 ] ]   # Numbers, strings, booleans, Vectors can be mixed

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

### Control Structure (Guards)

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

## License

[MIT License](LICENSE)
