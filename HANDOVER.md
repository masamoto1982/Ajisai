# 引継ぎ指示書: 命名インデックス規約リファクタリング

## 概要

SPECIFICATION.md セクション 9.4「命名インデックス規約（Naming-as-Index Convention）」に基づき、Rust / TypeScript 全体の関数名を統一する作業。

## 進捗: 約 75%

### 完了済み (PR #496 マージ済み)

47ファイル、+954/-784行のリネームが完了:

- **SPECIFICATION.md**: セクション 9.4 (9.4.1〜9.4.12) を新規追加（命名規約の定義）
- **Rust interpreter/**: `mod.rs`, `arithmetic.rs`, `audio.rs`, `cast.rs`, `comparison.rs`, `control.rs`, `datetime.rs`, `dictionary.rs`, `hash.rs`, `helpers.rs`, `higher_order.rs`, `io.rs`, `json.rs`, `logic.rs`, `modules.rs`, `random.rs`, `simd_ops.rs`, `sort.rs`, `tensor_ops.rs`, `vector_exec.rs`, `vector_ops/`
- **Rust types/**: `mod.rs`, `fraction.rs`, `json.rs`, `display.rs`
- **Rust other**: `lib.rs`, `wasm_api.rs`, `tokenizer.rs`, `error.rs`, `builtins/`
- **TypeScript**: `dictionary.ts`, `dictionary-ui.ts`, `display.ts`, `editor.ts`, `execution-controller.ts`, `main.ts`, `mobile.ts`, `module-tabs.ts`, `test.ts`, `js/main.ts`
- **テスト**: `cargo test` 77/77 合格、`npx tsc --noEmit` エラーゼロ

### 残作業 (約26関数)

#### Rust `get_*` → 適切な動詞 (~15関数)

**`rust/src/wasm_api.rs`** (WASM境界 — JS側の呼び出し箇所も同時に更新必要):
| 現在の名前 | 推奨名 |
|---|---|
| `get_stack` | `extract_stack` or keep (Rust慣用) |
| `get_core_words_info` | `collect_core_words_info` |
| `get_module_words_info` | `collect_module_words_info` |
| `get_imported_modules` | `collect_imported_modules` |
| `get_idiolect_words_info` | `collect_idiolect_words_info` |
| `get_word_definition` | `lookup_word_definition` |

**`rust/src/builtins/mod.rs`**:
| 現在の名前 | 推奨名 |
|---|---|
| `get_description` | `lookup_description` |
| `get_hint` | `lookup_hint` |
| `get_builtin` | `lookup_builtin` |
| `get_arity` | `lookup_arity` |
| `get_builtins_info` | `collect_builtins_info` |
| `get_syntax_example` | `lookup_syntax_example` |
| `get_signature_type` | `lookup_signature_type` |
| `get_signature_type_str` | `lookup_signature_type_str` |

**`rust/src/types/mod.rs`**:
| 現在の名前 | 推奨名 |
|---|---|
| `get_display_hint` | `extract_display_hint` |

#### TypeScript `handle*` → 具体的な動詞 (3関数)

| ファイル | 現在の名前 | 推奨名 |
|---|---|---|
| `js/gui/mobile.ts:89` | `handleSwipeGesture` | `resolveSwipeGesture` |
| `js/gui/execution-controller.ts:115` | `handleResult` | `applyExecutionResult` |
| `js/gui/execution-controller.ts:75` | `handleExecutionException` | `resolveExecutionException` |

#### TypeScript `get*` → 適切な動詞 (~8関数)

| ファイル | 現在の名前 | 推奨名 |
|---|---|---|
| `execution-controller.ts` | `getEditorValue` (callback) | `extractEditorValue` |
| `test.ts` | `getOutputElement` | `lookupOutputElement` |
| `display.ts` | `getState` | `extractState` |
| `module-tabs.ts` | `getModuleArea` / `getModuleSheet` | `lookupModuleArea` / `lookupModuleSheet` |
| `module-tabs.ts` | `getSheets` | `collectSheets` |

### 注意点

1. **WASM境界の関数**(`wasm_api.rs`の`#[wasm_bindgen]`関数)をリネームする際は、JS側の全呼び出し箇所も同時に更新すること。`window.ajisaiInterpreter.get_*` を grep して漏れなく修正。
2. **`get_stack`** は Rust の `get()` 慣用（SPECIFICATION 9.4.9 例外規定）に該当する可能性があるため、維持も選択肢。
3. **callback interface のプロパティ名**（`getEditorValue` 等）はリネームすると実装側も全て変更が必要。
4. リネーム後は必ず `cargo test` + `npx tsc --noEmit` で検証。

## ブランチ情報

- マージ済みブランチ: `claude/refactor-naming-system-hKrDS`
- ベースブランチ: `master`
- マージ済みPR: #496
