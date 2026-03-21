# Migration File Renaming Inventory

## Summary

This inventory records the AI-first filename migration performed in this repository.

- Scope: repository-managed source, docs, styles, workers, Rust modules, tests, and sample programs.
- Goal: replace ambiguous, generic, or context-dependent names with descriptive lowercase kebab-case names.
- Intentional exceptions: framework-constrained files such as `index.html`, Rust `mod.rs` files, and generated WASM artifacts.

## Rename Mapping

| current_path | proposed_path | reason | confidence | notes |
| --- | --- | --- | --- | --- |
| `OPTIMIZATION_REPORT.md` | `performance-optimization-report.md` | `final/report`-style document name replaced with responsibility-explicit report name | high | Updated references |
| `HANDOVER.md` | `project-handover-notes.md` | Generic handover label expanded to project-scoped notes | high | Updated internal references |
| `reference.html` | `language-reference-playground.html` | `reference` alone was weak; new name explains artifact purpose | medium | Updated Vite multi-page input |
| `style.css` | `app-interface.css` | Global stylesheet renamed to domain-specific UI role | high | Updated HTML and service worker |
| `public/docs/css/style.css` | `public/docs/css/docs-reference-styles.css` | Docs stylesheet renamed to explicit docs role | high | Updated docs HTML links |
| `public/docs/script.js` | `public/docs/docs-navigation-script.js` | Generic script name replaced with docs-scoped behavior name | high | Updated docs HTML and service worker |
| `js/main.ts` | `js/web-app-entrypoint.ts` | `main` banned; new name states web entrypoint role | high | Updated HTML entry reference |
| `js/gui/main.ts` | `js/gui/gui-application.ts` | GUI composition root now named by responsibility | high | Updated imports and docs |
| `js/db.ts` | `js/indexeddb-custom-word-store.ts` | `db` too vague; new name states storage technology and subject | high | Updated imports |
| `js/wasm-types.ts` | `js/wasm-interpreter-types.ts` | Types file made subject-specific | high | Updated imports |
| `js/wasm-loader.ts` | `js/wasm-module-loader.ts` | Loader purpose made explicit | high | Updated imports |
| `js/workers/worker-manager.ts` | `js/workers/execution-worker-manager.ts` | Manager subject clarified | high | Updated imports |
| `js/workers/ajisai-worker.ts` | `js/workers/interpreter-execution-worker.ts` | Worker role and responsibility made explicit | high | Updated worker URL |
| `js/gui/fp-utils.ts` | `js/gui/functional-result-helpers.ts` | Abbreviation and `utils`-style vagueness removed | high | Updated imports |
| `js/gui/test.ts` | `js/gui/gui-test-runner.ts` | Generic test filename replaced with actual runner role | high | Updated dynamic import |
| `js/gui/test-cases.ts` | `js/gui/gui-interpreter-test-cases.ts` | Test scope and subject made explicit | high | Updated imports |
| `rust/src/wasm_api.rs` | `rust/src/wasm-interpreter-bindings.rs` | API boundary renamed to explicit WASM binding role | high | Added `#[path]` module binding |
| `rust/src/test_tokenizer.rs` | `rust/src/tokenizer-regression-tests.rs` | Test target and purpose made explicit | high | Added `#[path]` module binding |
| `rust/src/interpreter/helpers.rs` | `rust/src/interpreter/value-extraction-helpers.rs` | Generic `helpers` replaced with actual responsibility | high | Added `#[path]` module binding |
| `rust/src/builtins/details.rs` | `rust/src/builtins/builtin-word-details.rs` | Details file made subject-specific | high | Added `#[path]` module binding |
| `rust/src/builtins/definitions.rs` | `rust/src/builtins/builtin-word-definitions.rs` | Definitions file made subject-specific | high | Added `#[path]` module binding |
| `rust/src/interpreter/higher_order.rs` | `rust/src/interpreter/higher-order-operations.rs` | Snake case normalized; role clarified | medium | Added `#[path]` module binding |
| `rust/src/interpreter/simd_ops.rs` | `rust/src/interpreter/simd-vector-operations.rs` | Abbreviation expanded and subject clarified | high | Added `#[path]` module binding |
| `rust/src/interpreter/tensor_ops.rs` | `rust/src/interpreter/tensor-shape-operations.rs` | Generic ops filename replaced with tensor responsibility | medium | Added `#[path]` module binding |
| `rust/src/interpreter/vector_exec.rs` | `rust/src/interpreter/vector-execution-operations.rs` | Abbreviation removed; role clarified | high | Added `#[path]` module binding |
| `rust/tests/fractional_dataflow_tests.rs` | `rust/tests/fractional-dataflow-behavior-tests.rs` | Snake case normalized; behavior scope clarified | high | Cargo test auto-discovers renamed file |
| `rust/tests/gui_test_cases.rs` | `rust/tests/gui-interpreter-test-cases.rs` | Scope and subject clarified | high | Cargo test auto-discovers renamed file |
| `rust/bench_after_all.txt` | `rust/benchmark-after-all-optimizations.txt` | Report context clarified | high | Text artifact only |
| `rust/bench_after_fraction.txt` | `rust/benchmark-after-fraction-optimizations.txt` | Report context clarified | high | Text artifact only |
| `rust/bench_baseline.txt` | `rust/benchmark-baseline.txt` | Baseline report clarified | high | Text artifact only |
| `test_cast.ajisai` | `cast-operations-test.ajisai` | Test target made explicit | high | Standalone sample test |
| `test_guard_custom_word.ajisai` | `custom-word-guard-test.ajisai` | Subject and assertion focus clarified | high | Standalone sample test |
| `test_nested_vector_brackets.ajisai` | `nested-vector-brackets-test.ajisai` | Test target made explicit | high | Standalone sample test |
| `test_no_change_error.ajisai` | `no-change-error-test.ajisai` | Error scenario named explicitly | high | Standalone sample test |
| `test_range_count.ajisai` | `range-count-test.ajisai` | Test target made explicit | high | Standalone sample test |
| `test_tail_recursion.ajisai` | `tail-recursion-test.ajisai` | Test target made explicit | high | Standalone sample test |
| `examples/test_chars_join.ajisai` | `examples/character-join-sample-test.ajisai` | Example purpose made explicit | medium | Standalone sample |
| `examples/test_input_helper.ajisai` | `examples/editor-input-assist-sample-test.ajisai` | `helper` removed; editor assist role clarified | medium | Standalone sample |
| `examples/test_math_functions.ajisai` | `examples/math-functions-sample-test.ajisai` | Example topic made explicit | high | Standalone sample |
| `examples/test_music.ajisai` | `examples/music-playback-sample-test.ajisai` | Example topic made explicit | high | Standalone sample |
| `examples/test_nested_vector_brackets.ajisai` | `examples/nested-vector-brackets-sample-test.ajisai` | Example topic made explicit | high | Standalone sample |
| `examples/test_tensor.ajisai` | `examples/tensor-operations-sample-test.ajisai` | Example subject made explicit | high | Standalone sample |
| `examples/test_tensor_generation.ajisai` | `examples/tensor-generation-sample-test.ajisai` | Example subject made explicit | high | Standalone sample |
| `public/images/ajisai-logo-min_w40.jpg` | `public/images/ajisai-logo-thumbnail-w40.jpg` | Encoded shorthand replaced with descriptive asset name | medium | Updated HTML reference |

## Deferred Exceptions

| current_path | reason for retaining |
| --- | --- |
| `index.html` | Conventional web entry document; renaming would complicate app hosting and Vite entry configuration for limited value |
| `public/docs/index.html` | Conventional docs landing page and stable static hosting entry |
| `rust/**/mod.rs` | Rust module convention; changing every directory module would add unnecessary `#[path]` indirection across the tree |
| `js/pkg/ajisai_core*` and `public/wasm/ajisai_core*` | Generated WASM artifacts; should be renamed only at generation source |
| `CNAME`, `LICENSE` | External-platform conventional filenames |
