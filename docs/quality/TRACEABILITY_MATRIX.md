# Initial Traceability Matrix

| Requirement ID | Objective | Implementation Reference | Verification Reference |
|---|---|---|---|
| AQ-REQ-001 | Core arithmetic remains exact and deterministic. | `rust/src/types/fraction.rs`, `rust/src/types/fraction-arithmetic.rs` | `cargo test --all-targets --verbose`; AQ-VER-001-A through AQ-VER-001-J |
| AQ-REQ-002 | Parsing/tokenization behavior is stable across regressions. | `rust/src/tokenizer.rs` | `rust/src/tokenizer-regression-tests.rs`, `rust/src/tokenizer-regression-tests-2.rs`; AQ-VER-002-A through AQ-VER-002-F |
| AQ-REQ-003 | WASM target build integrity is preserved. | `rust/src/wasm-interpreter-bindings.rs`, `rust/src/wasm-value-conversion.rs` | CI rust wasm check in `.github/workflows/test.yml`; AQ-VER-003-A, AQ-VER-003-B (native pure-helper coverage) |
| AQ-REQ-004 | TypeScript runtime typing remains sound and platform/value helpers behave correctly across the supported runtimes. | `js/` runtime modules | `npm run check`; `npm test` (Vitest); AQ-VER-004-A through AQ-VER-004-D |
| AQ-REQ-005 | Quality gates block merges on formatting/lint/test failures. | `.github/workflows/test.yml` | CI quality gate job |
| AQ-REQ-006 | Interpreter execution semantics (mode dispatch, quantization eligibility, epoch-driven plan-cache invalidation) remain stable across regressions. | `rust/src/interpreter/execute-builtin.rs`, `rust/src/interpreter/quantized-block.rs`, `rust/src/interpreter/higher-order-operations.rs`, `rust/src/interpreter/compiled-plan.rs` | `cargo test --all-targets`; AQ-VER-006-A through AQ-VER-006-D |
| AQ-REQ-007 | Built-in word purity classification (`pure` / `observable` / `effectful`) and `safe_preview` gating remain self-consistent so that auto-preview never executes side-effecting words, and module IMPORT/IMPORT-ONLY compatibility for the standard module set is preserved. | `rust/src/coreword_registry.rs`, `rust/src/interpreter/modules/module_builtins.rs`, `rust/src/interpreter/modules/module_word_types.rs` | `cargo test --all-targets`; AQ-VER-007-A through AQ-VER-007-F |

## Verification Index

| Verification ID | Requirement | Decision Under Test | Test Location |
|---|---|---|---|
| AQ-VER-001-A | AQ-REQ-001 | `Fraction::eq` NIL guard (`is_nil() \|\| is_nil()` and `is_nil() && is_nil()`) | `rust/src/types/fraction-mcdc-tests.rs::nil_equality_guard` |
| AQ-VER-001-B | AQ-REQ-001 | `Fraction::cmp` Small fast-path same-denominator branch | `rust/src/types/fraction-mcdc-tests.rs::cmp_small_fast_path` |
| AQ-VER-001-C | AQ-REQ-001 | `Fraction::as_usize` Small arm (`d == 1 && n >= 0`) | `rust/src/types/fraction-mcdc-tests.rs::as_usize_small` |
| AQ-VER-001-D | AQ-REQ-001 | `Fraction::add` Small fast paths (`b == 1 && d == 1`, `b == d`) | `rust/src/types/fraction-mcdc-tests.rs::add_small_fast_paths` |
| AQ-VER-001-E | AQ-REQ-001 | `Fraction::floor` (`n < 0 && r != 0`) | `rust/src/types/fraction-mcdc-tests.rs::floor_negative_remainder` |
| AQ-VER-001-F | AQ-REQ-001 | `Fraction::ceil` (`n > 0 && r != 0`) | `rust/src/types/fraction-mcdc-tests.rs::ceil_positive_remainder` |
| AQ-VER-001-G | AQ-REQ-001 | `Fraction::create_from_i128` Small-vs-Big boundary (`n >= i64::MIN as i128 && n <= i64::MAX as i128 && d >= 0 && d <= i64::MAX as i128`) | `rust/src/types/fraction-mcdc-tests.rs::create_from_i128_small_big_boundary` |
| AQ-VER-001-H | AQ-REQ-001 | `Fraction::add` Small fast-path entry guard (tuple destructuring of `extract_i64_pair` Some/Some) | `rust/src/types/fraction-mcdc-tests.rs::add_small_fast_path_entry_guard` |
| AQ-VER-001-I | AQ-REQ-001 | `Fraction::add` checked-mul/checked-add chain (defensive; None-arm structurally unreachable for i64 operands, with arithmetic proof) | `rust/src/types/fraction-mcdc-tests.rs::add_checked_chain_defensive` |
| AQ-VER-001-J | AQ-REQ-001 | `Fraction::modulo` Small fast-path remainder sign normalization (`rem < 0 && c > 0`) | `rust/src/types/fraction-mcdc-tests.rs::modulo_remainder_sign_normalization` |
| AQ-VER-002-A | AQ-REQ-002 | `tokenize` `\n` linebreak deduplication (`tokens.last() != Some(LineBreak)`) | `rust/src/tokenizer-mcdc-tests.rs::linebreak_dedup` |
| AQ-VER-002-B | AQ-REQ-002 | `tokenize` comment `had_token_before` (`!is_empty() && last != LineBreak`) | `rust/src/tokenizer-mcdc-tests.rs::comment_had_token_before` |
| AQ-VER-002-C | AQ-REQ-002 | `tokenize` comment newline absorption (`!had_token_before && i < len && c == '\n'`) | `rust/src/tokenizer-mcdc-tests.rs::comment_newline_absorption` |
| AQ-VER-002-D | AQ-REQ-002 | `tokenize` `=` two-char lookahead (`i+1 < len && chars[i+1] == X`) | `rust/src/tokenizer-mcdc-tests.rs::equals_lookahead` |
| AQ-VER-002-E | AQ-REQ-002 | `is_string_close_delimiter` (`is_whitespace \|\| (is_special && != '\'')`) | `rust/src/tokenizer-mcdc-tests.rs::string_close_delimiter` |
| AQ-VER-002-F | AQ-REQ-002 | `parse_number_from_string` sign preamble (length-1 and non-digit guards) | `rust/src/tokenizer-mcdc-tests.rs::number_sign_guards` |
| AQ-VER-003-A | AQ-REQ-003 | `resolve_effective_hint` external/arena precedence | `rust/src/wasm-value-conversion.rs::mcdc_tests::aq_ver_003_a_resolve_effective_hint` |
| AQ-VER-003-B | AQ-REQ-003 | `build_bracket_structure_from_shape` empty/single/multi-dim | `rust/src/wasm-value-conversion.rs::mcdc_tests::aq_ver_003_b_bracket_structure` |
| AQ-VER-006-A | AQ-REQ-006 | `Interpreter::is_hedged_mode` ElasticMode disjunction (`HedgedSafe \| HedgedTrace`) | `rust/src/interpreter/higher-order-operations-mcdc-tests.rs::hedged_mode_classifier` |
| AQ-VER-006-B | AQ-REQ-006 | `is_quantizable_block` outer conjunction (`!empty && !any(LineBreak \| SafeMode)`) and inner token-variant disjunction | `rust/src/interpreter/higher-order-operations-mcdc-tests.rs::is_quantizable_block_outer`, `::is_quantizable_block_inner_match` |
| AQ-VER-006-C | AQ-REQ-006 | `get_execution_plan_set` quantized-cache guard (`dictionary_epoch == && module_epoch ==`) observed via `compiled_plan_cache_miss_count` deltas around `bump_dictionary_epoch` / `bump_module_epoch` | `rust/src/interpreter/higher-order-operations-mcdc-tests.rs::compiled_plan_cache_guard` |
| AQ-VER-006-D | AQ-REQ-006 | `is_quantizable_block` Phase 1-C purity gate (third conjunct `!any(token_is_impure_builtin)`) and inner predicate truth table over (Symbol-Variant × PurityKnown × PurityImpure) | `rust/src/interpreter/higher-order-operations-mcdc-tests.rs::is_quantizable_block_purity_gate` |
| AQ-VER-004-A | AQ-REQ-004 | `detectRuntimeKind` build-time injection conjunction (`typeof __AJISAI_TARGET__ !== 'undefined' && __AJISAI_TARGET__ === 'tauri'`) and runtime DOM-detection conjunction (`typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window`) | `js/platform/runtime-kind.test.ts` |
| AQ-VER-004-B | AQ-REQ-004 | `compareValue` number-arm equality conjunction (`numerator === && denominator ===`) and vector-arm array-guard disjunction (`!Array.isArray(actual) \|\| !Array.isArray(expected)`) | `js/gui/value-formatter.test.ts::compareValue *` |
| AQ-VER-004-C | AQ-REQ-004 | `compareStack` per-index 3-disjunct loop guard (`!a \|\| !e \|\| !compareValue(a, e)`) | `js/gui/value-formatter.test.ts::compareStack *` |
| AQ-VER-004-D | AQ-REQ-004 | `formatFractionScientific` scientific-form conjunction (`numSci.includes('e') && denSci.includes('e')`) | `js/gui/value-formatter.test.ts::formatFractionScientific *` |
| AQ-VER-007-A | AQ-REQ-007 | Metadata completeness: every entry in `get_builtin_word_registry()` has non-empty `name`, non-empty `category`, and a `purity` ∈ {Pure, Observable, Effectful}. | `rust/src/coreword_registry.rs::tests::aq_ver_007_a_metadata_exists_for_all_builtin_words` |
| AQ-VER-007-B | AQ-REQ-007 | Pure-word integrity conjunction (`effects.is_empty() && deterministic && safe_preview`) holds for every `WordPurity::Pure` entry. | `rust/src/coreword_registry.rs::tests::aq_ver_007_b_pure_words_must_be_safe_and_deterministic_without_effects` |
| AQ-VER-007-C | AQ-REQ-007 | Effectful-word safety conjunction (`!safe_preview && !effects.is_empty()`) holds for every `WordPurity::Effectful` entry. | `rust/src/coreword_registry.rs::tests::aq_ver_007_c_effectful_words_must_not_be_safe_preview` |
| AQ-VER-007-D | AQ-REQ-007 | Observable-word safety conjunction (`!effects.is_empty() && !safe_preview` and `!deterministic` by default, with documented LOOKUP exception) holds for every `WordPurity::Observable` entry. | `rust/src/coreword_registry.rs::tests::aq_ver_007_d_observable_words_are_nondeterministic_and_not_safe_preview_by_default` |
| AQ-VER-007-E | AQ-REQ-007 | `is_safe_preview_word` decision (`metadata.is_some() && metadata.safe_preview`) — independent-effect MC/DC truth table over (metadata-present × safe_preview) including the default `unwrap_or(false)` short-circuit for unknown names. | `rust/src/coreword_registry.rs::tests::aq_ver_007_e_is_safe_preview_word_decision_truth_table` |
| AQ-VER-007-F | AQ-REQ-007 | IMPORT / IMPORT-ONLY compatibility for the standard module set (`MATH`, `JSON`, `IO`, `TIME`, `CRYPTO`, `ALGO`, `MUSIC`) is preserved, including selective `'MATH' [ 'SQRT' ] IMPORT-ONLY`. | `rust/src/interpreter/coreword-registry-import-compat-tests.rs::tests::aq_ver_007_f_import_and_import_only_remain_compatible_for_standard_modules` |

## Coverage Notes

- AQ-REQ-003: the JsValue-bridging entry points (`js_value_to_value`,
  `arena_node_to_js`, `extract_display_hint_from_js`) are not directly
  unit-testable on the native target because they exercise
  `wasm_bindgen` runtime glue. They are verified by the
  `cargo check --target wasm32-unknown-unknown` step in
  `.github/workflows/test.yml` and by downstream WASM smoke runs.
  Native MC/DC coverage targets the pure helpers reachable on both
  targets.
