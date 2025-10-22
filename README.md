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
- The system recognizes only words registered in the dictionary, Vectors, booleans, numbers, strings, and Nil
- Vector is the sole data structure
- Vectors can contain Vectors, booleans, strings, and Nil, and support negative indexing for searching from the end
- For Vector operations: position-specifying operations are 0-indexed, quantity-specifying operations are 1-indexed
- Built-in words cannot be deleted or have their meanings overwritten
- Statically typed without requiring type declarations or type inference
- All numbers are internally treated as fractions to avoid rounding errors
- Capable of handling extremely large numbers
- Memory usage and dictionary state are represented in the GUI
- Iteration count and processing time can be specified for each line

(The name "Ajisai" is a metaphor for hydrangea flowers, representing FORTH's characteristic of small words coming together to form functionality. *Note: The flower-like parts of hydrangeas are not actually flowers.)

