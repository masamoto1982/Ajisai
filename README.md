![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")

# Ajisai

> **Manifesto: Data is not stored — it flows. Every operation consumes a fraction of the stream and hands the remainder forward. Computation is a chain of consumption, not a pile of copies.**

**A Fractional Dataflow programming language**

Ajisai inherits **postfix notation** and the **dictionary system** from FORTH. Its execution model is **Fractional Dataflow**: every value is a fraction that streams through a pipeline of operations, each consuming what it needs and forwarding the remainder.

**Demo:** [https://masamoto1982.github.io/Ajisai/](https://masamoto1982.github.io/Ajisai/)

---

## Documentation Scope

- This README and `SPECIFICATION.md` describe **only the current Ajisai model**.
- Historical migration notes and old-model comparisons are intentionally excluded from the main specification documents.
- Abstract metaphors are limited to the manifesto; all other sections focus on concrete implementation behavior.

---

## Design Philosophy: Fractional Dataflow

### Core Principles

| Principle | What It Means |
|:----------|:--------------|
| **Fraction as Truth** | `Fraction` is the sole computational substance |
| **Consume/Remainder** | Each operation consumes what it needs; the remainder flows on |
| **No Intermediates** | Pipeline stages fuse — no materialized intermediate collections |
| **Conservation Law** | `initial_total = Sigma(consumed_i) + final_remainder` always holds |
| **Display Hint Separation** | `DisplayHint` is metadata for display only — not a type |
| **NIL Semantics** | `NIL` represents absence of value in evaluation |

### The Consumed/Remainder Model

Every operation in Ajisai follows this pattern:

```
input_flow -> operation -> (consumed, remainder_flow)
```

- The **remainder** is automatically inherited as the next operation's input.
- **Over-consumption** (requesting more than remains) is a hard error.
- At pipeline end, **complete consumption** (remainder = 0) is the goal.
- The interpreter can verify the **conservation law** at any point.

---

## Unified Fraction Architecture

In Ajisai, all computational data exists as a single substance: **fractions**. There is no type system.

```rust
pub struct Value {
    pub data: ValueData, // Data plane: pure computational value
}

pub enum ValueData {
    Scalar(Fraction),
    Vector(Vec<Value>),
    Record { pairs: Vec<Value>, index: HashMap<String, usize> },
    Nil,
    CodeBlock(Vec<Token>),
}

pub struct SemanticRegistry {
    hints: HashMap<u64, DisplayHint>,
    extensions: HashMap<u64, Box<dyn ValueExt>>,
    stack_hints: Vec<Option<DisplayHint>>,
}
```

### FlowToken: Computation Flow Tracker

Each value entering the pipeline is wrapped in a `FlowToken` that tracks its consumption chain:

```rust
pub struct FlowToken {
    pub id: u64,              // Unique chain identifier
    pub total: Fraction,      // Original total entering this chain
    pub remaining: Fraction,  // Fraction still available for consumption
    pub shape: Vec<usize>,    // Logical shape of the flow bundle
    pub parent_flow_id: Option<u64>,  // Bifurcation: parent flow reference
    pub child_flow_ids: Vec<u64>,     // Bifurcation: child branches
    pub mass_ratio: (u64, u64),       // Mass ratio this branch received
}
```

### Bifurcation: Flow Splitting with `,,`

The `,,` modifier is not a copy — it is a **bifurcation** of flow mass. When `,,` is used, the parent flow's mass is split equally among the retained operands and the result:

```
parent_mass = branch_a_mass + branch_b_mass + ...
```

This preserves the conservation law while allowing intermediate values to remain on the stack. The value data is shared (via `Rc`), but each branch carries its own fraction of the original mass.

### Data Representation

User-facing notation is represented internally as fraction-based values with semantic hints:

| Appearance | Internal Representation | Explanation |
|:-----------|:------------------------|:------------|
| `42` | `Scalar(42/1)` | Integers are fractions |
| `TRUE` | `Scalar(1/1)` + Boolean hint | 1 is true, 0 is false |
| `'A'` | `Scalar(65/1)` + String hint | Character code (Unicode) |
| `'Hello'` | `Vector([72/1, 101/1, ...])` + String hint | Array of character codes |
| `NIL` | `Nil` | Absence of value (zero fraction mass) |

---

## Features

### Language Design

- **Fractional Dataflow with Reverse Polish Notation (RPN)**
  - FORTH-inherited postfix notation and dictionary system
  - No stack manipulation words (DUP, SWAP, ROT, OVER do not exist)
  - Operations consume input fractions and pass remainders forward

- **Exact Fraction Arithmetic with Conservation**
  - All numbers internally represented as fractions — no rounding errors
  - Arbitrary precision through `num-bigint`
  - Conservation law verified: `total = consumed + remainder`

- **Stream-First Vector Processing (TPU Analogy)**
  - Vector operations process elements as a stream, not by materializing intermediate arrays
  - Bracket `[ ]` nesting expresses dimensions of the flow bundle
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

### Error Model (Fractional Dataflow)

| Error | Meaning |
|:------|:--------|
| `OverConsumption` | Requested consumption exceeds remaining flow |
| `UnconsumedLeak` | Non-zero remainder at a complete-consumption boundary |
| `FlowBreak` | Flow chain ID discontinuity — remainder cannot be inherited |
| `BifurcationViolation` | Sum of child branch masses does not equal parent mass |

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

### Fraction Flow Through Operations

```ajisai
# Each operation consumes its inputs and produces a remainder-ready output
[ 5 ] [ 3 ] +     # 5/1 and 3/1 consumed -> 8/1 produced
[ 10 ] [ 2 ] /    # 10/1 consumed by 2/1 -> 5/1 remainder flows on
```

### Vector Operations (Stream Processing)

```ajisai
# Creating vectors (flow bundles)
[ 1 2 3 ]               # 1D flow bundle: { 1 2 3 }
[ [ 1 2 ] [ 3 4 ] ]     # Nested: { ( 1 2 ) ( 3 4 ) }

# Broadcasting: fraction flow distributes across the bundle
[ 5 ] [ 1 2 3 ] +       # -> { 6 7 8 }
[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +
# -> { ( 11 22 33 ) ( 14 25 36 ) }
```

### Custom Word Definition

```ajisai
# Define a word that doubles a value (consumes input, produces 2x output)
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

The pipeline operator is a visual marker for the dataflow chain (no-op):

```ajisai
# Readable fraction-flow transformation pipeline
[ 1 2 3 4 5 ]
  == : [ 2 ] * ; MAP           # Consume and double: { 2 4 6 8 10 }
  == : [ 5 ] < NOT ; FILTER    # Consume and keep >= 5: { 6 8 10 }
  == [ 0 ] : + ; FOLD          # Consume and sum: { 24 }
```

### Safe Mode and Nil Coalescing

```ajisai
# Safe mode (~): convert errors to NIL (zero-mass bubble in the flow)
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
