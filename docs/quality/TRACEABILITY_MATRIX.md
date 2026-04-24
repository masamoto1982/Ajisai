# Initial Traceability Matrix

| Requirement ID | Objective | Implementation Reference | Verification Reference |
|---|---|---|---|
| AQ-REQ-001 | Core arithmetic remains exact and deterministic. | `rust/src/types/fraction.rs`, `rust/src/types/fraction-arithmetic.rs` | `cargo test --all-targets --verbose` |
| AQ-REQ-002 | Parsing/tokenization behavior is stable across regressions. | `rust/src/tokenizer.rs` | `rust/src/tokenizer-regression-tests.rs`, `rust/src/tokenizer-regression-tests-2.rs` |
| AQ-REQ-003 | WASM target build integrity is preserved. | `rust/src/wasm-interpreter-bindings.rs` | CI rust wasm check in `.github/workflows/test.yml` |
| AQ-REQ-004 | TypeScript runtime typing remains sound. | `js/` runtime modules | `npm run check` |
| AQ-REQ-005 | Quality gates block merges on formatting/lint/test failures. | `.github/workflows/test.yml` | CI quality gate job |
