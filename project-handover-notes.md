# 引継ぎ指示書: 命名インデックス規約リファクタリング

## 概要

SPECIFICATION.md セクション 9.4「命名インデックス規約（Naming-as-Index Convention）」に基づき、Rust / TypeScript 全体の関数名を統一する作業。

## 進捗: 約 80%

残り違反数: **48関数** (Rust 22 + TypeScript 26)

### 完了済み (PR #496 マージ済み)

47ファイル、+954/-784行のリネームが完了:

- **SPECIFICATION.md**: セクション 9.4 (9.4.1〜9.4.12) を新規追加（命名規約の定義）
- **Rust interpreter/**: `mod.rs`, `arithmetic.rs`, `audio.rs`, `cast.rs`, `comparison.rs`, `control.rs`, `datetime.rs`, `dictionary.rs`, `hash.rs`, `value-extraction-helpers.rs`, `higher-order-operations.rs`, `io.rs`, `json.rs`, `logic.rs`, `modules.rs`, `random.rs`, `simd-vector-operations.rs`, `sort.rs`, `tensor-shape-operations.rs`, `vector-execution-operations.rs`, `vector_ops/`
- **Rust types/**: `mod.rs`, `fraction.rs`, `json.rs`, `display.rs`
- **Rust other**: `lib.rs`, `wasm-interpreter-bindings.rs`, `tokenizer.rs`, `error.rs`, `builtins/`
- **TypeScript**: `dictionary.ts`, `dictionary-ui.ts`, `display.ts`, `editor.ts`, `execution-controller.ts`, `gui-application.ts`, `mobile.ts`, `module-tabs.ts`, `gui-test-runner.ts`, `js/web-app-entrypoint.ts`
- **テスト**: `cargo test` 77/77 合格、`npx tsc --noEmit` エラーゼロ
- **完全にクリーン**: `fetch*`, `retrieve*`, `process*`, `handle*`(Rust) は違反ゼロ

### 除外対象（リネーム不要）

- `rust/src/interpreter/datetime.rs` の `get_*` FFI バインディング (9件) — `extern "C"` + `#[wasm_bindgen(js_name = getXxx)]` で JS Date API に束縛。§9.4.9 例外規定に準拠
- `js/pkg/` — wasm-bindgen 自動生成コード

---

## 残作業

### 1. Rust `get_*` → 適切な動詞 (13関数)

**`rust/src/wasm-interpreter-bindings.rs`** (WASM境界 — **最優先**。JS側呼び出し箇所も同時更新必要):

| 行 | 現在の名前 | 推奨名 |
|---|---|---|
| 357 | `get_stack` | `collect_stack` |
| 366 | `get_idiolect_words_info` | `collect_idiolect_words_info` |
| 392 | `get_imported_modules_array` | `collect_imported_modules_array` |
| 400 | `get_custom_words_for_state` | `collect_custom_words_for_state` |
| 415 | `get_core_words_info` | `collect_core_words_info` |
| 422 | `get_imported_modules` | `collect_imported_modules` |
| 433 | `get_module_sample_words_info` | `collect_module_sample_words_info` |
| 455 | `get_module_words_info` | `collect_module_words_info` |
| 488 | `get_word_definition` | `lookup_word_definition` |
| 545 | `get_io_output_buffer` | `extract_io_output_buffer` |

**`rust/src/interpreter/mod.rs`**:

| 行 | 現在の名前 | 推奨名 |
|---|---|---|
| 1044 | `get_stack` | `extract_stack` (§9.4.9 Rust慣用`get`として維持も可) |

**`rust/src/types/mod.rs`**:

| 行 | 現在の名前 | 推奨名 |
|---|---|---|
| 286 | `get_child` | `extract_child` |
| 294 | `get_child_mut` | `extract_child_mut` |

### 2. Rust `set_*` → `update_*` (2関数)

| ファイル | 現在の名前 | 推奨名 |
|---|---|---|
| `interpreter/mod.rs:127` | `set_flow_tracking` | `update_flow_tracking` |
| `wasm-interpreter-bindings.rs:540` | `set_input_buffer` | `update_input_buffer` |

### 3. Rust `make_*` / `generate_*` → `build_*` / `create_*` (7関数)

| ファイル | 現在の名前 | 推奨名 | 備考 |
|---|---|---|---|
| `wasm-interpreter-bindings.rs:60` | `generate_bracket_structure_from_shape` | `build_bracket_structure_from_shape` | §9.4.12 の良い例そのもの |
| `audio.rs:696` | `make_number` | `create_number` | `#[cfg(test)]` |
| `audio.rs:700` | `make_fraction` | `create_fraction` | `#[cfg(test)]` |
| `audio.rs:707` | `make_nil` | `create_nil` | `#[cfg(test)]` |
| `audio.rs:711` | `make_vector` | `create_vector` | `#[cfg(test)]` |
| `simd-vector-operations.rs:262` | `make_int_vector` | `create_int_vector` | `#[cfg(test)]` |
| `sort.rs:152` | `make_fraction` | `create_fraction` | `#[cfg(test)]` |

### 4. TypeScript `get*` → 適切な動詞 (13関数)

| ファイル | 現在の名前 | 推奨名 |
|---|---|---|
| `js/wasm-module-loader.ts:48` | `getCompiledWasmModule` | `extractCompiledWasmModule` |
| `js/gui/gui-application.ts:135` | `getAutocompleteWords` | `collectAutocompleteWords` |
| `js/gui/gui-application.ts:183` | `getTabButtons` | `collectTabButtons` |
| `js/gui/gui-application.ts:534-540` | `getElements`, `getDisplay`, `getEditor`, `getVocabulary`, `getMobile`, `getPersistence`, `getExecutionController` | `extractElements`, `extractDisplay` 等 |
| `js/gui/step-executor.ts:63` | `getCustomWords` | `collectCustomWords` |
| `js/gui/step-executor.ts:135` | `getState` | `extractState` |
| `js/gui/display.ts:445` | `getState` | `extractState` |
| `js/gui/module-tabs.ts:194` | `getModuleSheet` | `lookupModuleSheet` |
| `js/gui/module-tabs.ts:199` | `getSheets` | `collectSheets` |
| `js/gui/mobile.ts:56` | `getStylesForMode` | `lookupStylesForMode` |
| `js/gui/gui-test-runner.ts:94` | `getOutputElement` | `lookupOutputElement` |
| `js/gui/editor.ts:231` | `getValue` | `extractValue` |
| `js/gui/persistence.ts:46` | `getCurrentState` | `collectCurrentState` |

### 5. TypeScript `set*` → `update*` (7関数)

| ファイル | 現在の名前 | 推奨名 |
|---|---|---|
| `js/gui/editor.ts:46` | `setElementValue` | `updateElementValue` |
| `js/gui/editor.ts:54` | `setSelectionRange` | `updateSelectionRange` |
| `js/gui/editor.ts:233` | `setValue` | `updateValue` |
| `js/gui/editor.ts:308` | `setOnContentChange` | `registerContentChangeCallback` |
| `js/gui/gui-application.ts:230` | `setDesktopModes` | `updateDesktopModes` |
| `js/gui/dictionary.ts:337` | `setSearchFilter` | `updateSearchFilter` |
| `js/gui/module-tabs.ts:201` | `setSearchFilter` | `updateSearchFilter` |

### 6. TypeScript `handle*` → 具体的な動詞 (5関数)

| ファイル | 現在の名前 | 推奨名 |
|---|---|---|
| `js/gui/execution-controller.ts:75` | `handleExecutionException` | `resolveExecutionException` |
| `js/gui/execution-controller.ts:115` | `handleResult` | `applyExecutionResult` |
| `js/gui/step-executor.ts:79` | `handleStepExecutionException` | `resolveStepExecutionException` |
| `js/gui/mobile.ts:89` | `handleSwipeGesture` | `resolveSwipeGesture` |
| `js/gui/gui-application.ts:345` | `handleSearchInput` | `applySearchInput` |

### 7. TypeScript `generate*` → `build*` (1関数)

| ファイル | 現在の名前 | 推奨名 |
|---|---|---|
| `js/gui/persistence.ts:68` | `generateExportFilename` | `buildExportFilename` |

---

## 作業上の注意点

1. **WASM境界が最優先かつ最高リスク**: `wasm-interpreter-bindings.rs` の `#[wasm_bindgen]` 関数をリネームすると、JS側の `window.ajisaiInterpreter.get_*` 呼び出し箇所も**すべて**同時更新が必要。grep で `ajisaiInterpreter.get_` を検索して漏れなく修正すること。
2. **`setSearchFilter`**: dictionary.ts と module-tabs.ts の両方にあり、さらに `VocabularyManager` / `ModuleTabManager` インターフェースの定義と `main.ts` の呼び出し箇所も更新が必要。
3. **callback interface のプロパティ名**（`getEditorValue` 等）はインターフェース定義・実装・呼び出しの3箇所を同時に変更。
4. **`get_stack` (Rust)**: §9.4.9 例外規定（Rust慣用`get()`）に該当する可能性あり。維持するか `extract_stack` にするかは判断が必要。
5. リネーム後は必ず `cargo test` + `npx tsc --noEmit` で検証。

## ブランチ情報

- マージ済みブランチ: `claude/refactor-naming-system-hKrDS`
- ベースブランチ: `master`
- マージ済みPR: #496
