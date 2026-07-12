# Implementation Quickstart (Non-Canonical)

This quickstart provides build/test commands and workflow hints.
It does not define Ajisai semantics.

## Canonical Source
- `SPECIFICATION.md`

## Recommended Verification Commands
- `cd rust && cargo test --lib`
- `cd rust && cargo test --tests`
- `npm run check`

GUI behavior cases live in `src/gui/gui-interpreter-test-cases.ts` and are executed from the in-app `Test` button.

WASM generated bindings are located in `src/wasm/generated/`.
