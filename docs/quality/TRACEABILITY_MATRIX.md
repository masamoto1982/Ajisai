# Initial Traceability Matrix

| Requirement ID | Objective | Implementation Reference | Verification Reference |
|---|---|---|---|
| AQ-REQ-001 | Core arithmetic remains exact and deterministic. | `rust/src/types/fraction.rs`, `rust/src/types/fraction-arithmetic.rs` | `cargo test --all-targets --verbose`; AQ-VER-001-A through AQ-VER-001-F |
| AQ-REQ-002 | Parsing/tokenization behavior is stable across regressions. | `rust/src/tokenizer.rs` | `rust/src/tokenizer-regression-tests.rs`, `rust/src/tokenizer-regression-tests-2.rs` |
| AQ-REQ-003 | WASM target build integrity is preserved. | `rust/src/wasm-interpreter-bindings.rs` | CI rust wasm check in `.github/workflows/test.yml` |
| AQ-REQ-004 | TypeScript runtime typing remains sound. | `js/` runtime modules | `npm run check` |
| AQ-REQ-005 | Quality gates block merges on formatting/lint/test failures. | `.github/workflows/test.yml` | CI quality gate job |

## Verification Index

| Verification ID | Requirement | Decision Under Test | Test Location |
|---|---|---|---|
| AQ-VER-001-A | AQ-REQ-001 | `Fraction::eq` NIL guard (`is_nil() \|\| is_nil()` and `is_nil() && is_nil()`) | `rust/src/types/fraction-mcdc-tests.rs::nil_equality_guard` |
| AQ-VER-001-B | AQ-REQ-001 | `Fraction::cmp` Small fast-path same-denominator branch | `rust/src/types/fraction-mcdc-tests.rs::cmp_small_fast_path` |
| AQ-VER-001-C | AQ-REQ-001 | `Fraction::as_usize` Small arm (`d == 1 && n >= 0`) | `rust/src/types/fraction-mcdc-tests.rs::as_usize_small` |
| AQ-VER-001-D | AQ-REQ-001 | `Fraction::add` Small fast paths (`b == 1 && d == 1`, `b == d`) | `rust/src/types/fraction-mcdc-tests.rs::add_small_fast_paths` |
| AQ-VER-001-E | AQ-REQ-001 | `Fraction::floor` (`n < 0 && r != 0`) | `rust/src/types/fraction-mcdc-tests.rs::floor_negative_remainder` |
| AQ-VER-001-F | AQ-REQ-001 | `Fraction::ceil` (`n > 0 && r != 0`) | `rust/src/types/fraction-mcdc-tests.rs::ceil_positive_remainder` |
