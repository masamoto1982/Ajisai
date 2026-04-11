![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")
![Ajisai QR Code](public/images/ajisai-qr.png "Ajisai QR Code")

# Ajisai

Ajisai is an AI-first, vector-oriented, fractional-dataflow language with a Rust (WASM) core and TypeScript GUI/runtime shell.

- Playground: https://masamoto1982.github.io/Ajisai/
- Runtime model includes local safe-mode (`~`) plus isolated child-runtime lifecycle words (`SPAWN`, `AWAIT`, `STATUS`, `KILL`, `MONITOR`, `SUPERVISE`) for Ajisai-style let-it-crash execution.

## Documentation Authority

- **Canonical language/runtime semantics**: `SPECIFICATION.md`
- **Secondary docs**: informational only, non-canonical unless explicitly promoted by `SPECIFICATION.md`

## Development Checks

- `cd rust && cargo test --lib`
- `cd rust && cargo test --tests`
- `cd rust && cargo test --test gui-interpreter-test-cases`
- `npm run check`

## License

MIT (`LICENSE`)
