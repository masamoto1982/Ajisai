# DRY リファクタ追加対応 引き継ぎ書（未着手項目対応）

作成日: 2026-04-09  
対象リポジトリ: `masamoto1982/Ajisai`

---

## 0. 背景
前回の DRY リファクタでは、以下は **未着手または見送り** として残した。

1. `TAKE / SPLIT / REVERSE / REORDER` の vector 周辺制御共通化
2. Rust/WASM 側の `js_sys::Reflect::set(...)` 定型の共通化
3. TypeScript/IndexedDB 側の request Promise 化定型の共通化

本書は、上記 3 点を次担当者が安全に改修するための引き継ぎ資料である。

---

## 1. 現在までに完了済み（前提）

- Built-in registry は導入済み（`BuiltinSpec` 正本化）。
- `GET / INSERT / REPLACE / REMOVE` の StackTop 周辺制御は共通化済み。
- TypeScript 側 `InterpreterSnapshot` は共通型/復元ヘルパ化済み。
- Rust GUI integration tests 向け `test_support` は導入済み。

> したがって、今回の作業は「既存共通化の横展開」と「小規模定型の整理」が主目的。

---

## 2. 未着手項目ごとの改修ガイド

## 2-1. vector 操作（`TAKE / SPLIT / REVERSE / REORDER`）の共通化

### 対象ファイル（主）
- `rust/src/interpreter/vector_ops/quantity.rs`
- `rust/src/interpreter/vector_ops/structure.rs`
- 必要に応じて `rust/src/interpreter/vector_ops/mod.rs`

### 目標
`position.rs` で実施した方針（「演算意味論」と「周辺スタック制御」の分離）を、上記 4 operator に適用する。

### 推奨アプローチ
1. まず `TAKE` と `SPLIT`（`quantity.rs`）の重複箇所を抽出。
2. 次に `REVERSE` と `REORDER`（`structure.rs`）へ同型ヘルパを適用。
3. ヘルパ名は、意図が明確な局所名を優先（例: `with_stacktop_vector_target`, `restore_vector_args_on_error` など）。
4. 既存エラー文言・stack 復元順序を変更しない。

### 完了条件
- 少なくとも上記 4 operator で、同一の pop/restore/keep-mode 分岐が減っている。
- 既存テスト（特に mode 系、vector ops 系）が green。

---

## 2-2. Rust/WASM の `Reflect::set` 定型共通化

### 対象ファイル（候補）
- `rust/src/wasm-interpreter-execution.rs`
- `rust/src/wasm-interpreter-state.rs`
- `rust/src/wasm-value-conversion.rs`

### 目標
`js_sys::Reflect::set(...)` の同型処理を小さな builder / helper で統一し、プロパティ追加時の修正箇所を減らす。

### 推奨アプローチ
1. まず `Object` 生成と `Reflect::set` を行う箇所を列挙。
2. `set_prop(obj, key, value)` のような最小ヘルパを導入。
3. 必要であれば `build_ok_result` / `build_error_result` / `build_state_result` 相当の関数を追加。
4. 例外系（set 失敗時）ハンドリング方針は既存と合わせる。

### 完了条件
- `Reflect::set` の重複記述が実質削減される。
- WASM API の返却 shape が不変。

---

## 2-3. TypeScript/IndexedDB request Promise 化共通化

### 対象ファイル（主）
- `js/indexeddb-user-word-store.ts`
- `js/gui/interpreter-state-persistence.ts`

### 目標
`onsuccess/onerror` の定型を `promisifyRequest` 等で共通化し、保存系コードの見通しを改善する。

### 推奨アプローチ
1. 同型の request ハンドリングを抽出。
2. `promisifyRequest<T>(request: IDBRequest<T>)` を先に導入。
3. 必要なら `withObjectStore`（transaction + store 取得）を追加。
4. 呼び出し側は段階的に差し替え、挙動差分が出ないことを確認。

### 完了条件
- IndexedDB アクセス経路の重複が減る。
- 既存の保存/復元動作（GUI 起動時の state 復元含む）が維持される。

---

## 3. 推奨実施順（安全優先）

1. `TAKE / SPLIT / REVERSE / REORDER` の共通化
2. Rust/WASM `Reflect::set` 共通化
3. TypeScript/IndexedDB 共通化

理由:
- 1 は既存パターン（`position.rs`）を横展開しやすい。
- 2 は Rust 側返却 shape に注意が必要だが影響範囲は比較的局所。
- 3 は UI 実行時にしか露呈しない不具合が出やすく、最後にまとめて確認した方が安全。

---

## 4. 変更時の注意事項

- **仕様変更禁止**（エラーメッセージ含む）。
- keep/consume と stack target（`.` / `..`）の挙動維持を最優先。
- 「DRY のための過剰抽象化」は避け、局所ヘルパ中心で進める。
- 1 PR あたりの責務は小さく分割（理想は 2〜3 PR）。

---

## 5. 推奨テストコマンド

### 必須
- `cd rust && cargo test --lib`
- `cd rust && cargo test --test gui-interpreter-test-cases`
- `npm run check`

### 可能なら追加
- `cd rust && cargo test --tests`
- GUI 手動確認（state 保存/復元、worker 実行、step 実行）

---

## 6. 受け入れ基準（Definition of Done）

- [ ] 未着手 3 項目のうち、最低 2 項目は改修完了。
- [ ] 既存テスト green。
- [ ] 新規ヘルパ導入により、同型コードの重複削減を diff で説明可能。
- [ ] 既存 API / GUI 挙動 / テスト期待値を維持。

---

## 7. 次担当者へのメモ

- まず `position.rs` の helper 設計を参照し、同じ抽象度で横展開すること。
- snapshot 系はすでに共通化済みのため、IndexedDB 側の共通化時は重複導入に注意。
- 大規模整形（fmt/lint による全体差分）はレビュー負荷が高いため、対象ファイル限定で実施すること。
