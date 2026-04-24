# Initial Traceability Matrix

| Requirement ID | Objective | Implementation Reference | Verification Reference |
|---|---|---|---|
| AQ-REQ-001 | Core arithmetic remains exact and deterministic. | `rust/src/types/fraction.rs`, `rust/src/types/fraction-arithmetic.rs` | `cargo test --all-targets --verbose`; AQ-VER-001-A through AQ-VER-001-F |
| AQ-REQ-002 | Parsing/tokenization behavior is stable across regressions. | `rust/src/tokenizer.rs` | `rust/src/tokenizer-regression-tests.rs`, `rust/src/tokenizer-regression-tests-2.rs`; AQ-VER-002-A through AQ-VER-002-F |
| AQ-REQ-003 | WASM target build integrity is preserved. | `rust/src/wasm-interpreter-bindings.rs`, `rust/src/wasm-value-conversion.rs` | CI rust wasm check in `.github/workflows/test.yml`; AQ-VER-003-A, AQ-VER-003-B (native pure-helper coverage) |
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
| AQ-VER-002-A | AQ-REQ-002 | `tokenize` `\n` linebreak deduplication (`tokens.last() != Some(LineBreak)`) | `rust/src/tokenizer-mcdc-tests.rs::linebreak_dedup` |
| AQ-VER-002-B | AQ-REQ-002 | `tokenize` comment `had_token_before` (`!is_empty() && last != LineBreak`) | `rust/src/tokenizer-mcdc-tests.rs::comment_had_token_before` |
| AQ-VER-002-C | AQ-REQ-002 | `tokenize` comment newline absorption (`!had_token_before && i < len && c == '\n'`) | `rust/src/tokenizer-mcdc-tests.rs::comment_newline_absorption` |
| AQ-VER-002-D | AQ-REQ-002 | `tokenize` `=` two-char lookahead (`i+1 < len && chars[i+1] == X`) | `rust/src/tokenizer-mcdc-tests.rs::equals_lookahead` |
| AQ-VER-002-E | AQ-REQ-002 | `is_string_close_delimiter` (`is_whitespace \|\| (is_special && != '\'')`) | `rust/src/tokenizer-mcdc-tests.rs::string_close_delimiter` |
| AQ-VER-002-F | AQ-REQ-002 | `parse_number_from_string` sign preamble (length-1 and non-digit guards) | `rust/src/tokenizer-mcdc-tests.rs::number_sign_guards` |
| AQ-VER-003-A | AQ-REQ-003 | `resolve_effective_hint` external/arena precedence | `rust/src/wasm-value-conversion.rs::mcdc_tests::aq_ver_003_a_resolve_effective_hint` |
| AQ-VER-003-B | AQ-REQ-003 | `build_bracket_structure_from_shape` empty/single/multi-dim | `rust/src/wasm-value-conversion.rs::mcdc_tests::aq_ver_003_b_bracket_structure` |

## Coverage Notes

- AQ-REQ-003: the JsValue-bridging entry points (`js_value_to_value`,
  `arena_node_to_js`, `extract_display_hint_from_js`) are not directly
  unit-testable on the native target because they exercise
  `wasm_bindgen` runtime glue. They are verified by the
  `cargo check --target wasm32-unknown-unknown` step in
  `.github/workflows/test.yml` and by downstream WASM smoke runs.
  Native MC/DC coverage targets the pure helpers reachable on both
  targets.
