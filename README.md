![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)
![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")
# Ajisai

Ajisai is a stack-based programming language inspired by FORTH.
It provides an interpreter running on WebAssembly and a web-based GUI.

## Development Concept
- Stack-based with Reverse Polish Notation (RPN), inspired by FORTH
- The system recognizes only words registered in the dictionary, Tensors, booleans, numbers, strings, and Nil
- **Tensor-based dimension model**: All numeric data is represented as N-dimensional tensors
  - Dimension 0: Scalar (single value)
  - Dimension 1: Vector (sequence of values)
  - Dimension 2: Matrix (2D array)
  - Dimension N: N-dimensional tensor
- **NumPy/APL-style broadcasting**: Automatic shape adjustment for operations between tensors of different shapes
- Rectangular constraint: All nested structures must be rectangular (same shape at each level)
- Tensor display uses depth-based bracket styles for visual clarity: `[ ]` at depth 0, `{ }` at depth 1, `( )` at depth 2, cycling every 3 levels
- Tensor operations: SHAPE, RANK, RESHAPE, TRANSPOSE for shape manipulation
- Built-in words cannot be deleted or have their meanings overwritten
- Statically typed without requiring type declarations or type inference
- All numbers are internally treated as fractions to avoid rounding errors
- Capable of handling extremely large numbers
- Memory usage and dictionary state are represented in the GUI
- Iteration count and processing time can be specified for each line

(The name "Ajisai" is a metaphor for hydrangea flowers, representing FORTH's characteristic of small words coming together to form functionality. *Note: The flower-like parts of hydrangeas are not actually flowers.)

## Tensor Operations Examples

### Basic Tensor Creation
```ajisai
[ 1 2 3 ]           # 1D tensor (vector): shape [3]
[ [ 1 2 ] [ 3 4 ] ] # 2D tensor (matrix): shape [2, 2]
```

### Broadcasting Arithmetic
```ajisai
# Scalar + Vector
[ 5 ] [ 1 2 3 ] +
# → [ 6 7 8 ]

# Vector + Matrix (broadcast along rows)
[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +
# → [ [ 11 22 33 ] [ 14 25 36 ] ]

# Column vector + Matrix (broadcast along columns)
[ [ 1 2 3 ] [ 4 5 6 ] ] [ [ 100 ] [ 200 ] ] +
# → [ [ 101 102 103 ] [ 204 205 206 ] ]
```

### Shape Manipulation
```ajisai
# Get shape
[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE
# → [ [ 1 2 3 ] [ 4 5 6 ] ] [ 2 3 ]

# Get rank (number of dimensions)
[ [ 1 2 3 ] [ 4 5 6 ] ] RANK
# → [ [ 1 2 3 ] [ 4 5 6 ] ] [ 2 ]

# Reshape
[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE
# → [ [ 1 2 3 ] [ 4 5 6 ] ]

# Transpose
[ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE
# → [ [ 1 4 ] [ 2 5 ] [ 3 6 ] ]
```

