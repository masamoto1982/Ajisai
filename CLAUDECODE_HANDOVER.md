# Ajisai 半コンパイル実行系化 — ClaudeCode 引き継ぎ書

最終更新: 2026-04-13

---

## 1. 目的と現状

本ブランチでは、Ajisai 実行系の段階的改修（Epoch → CompiledPlan → QuantizedBlock → 高階関数最適化 → 計測）を進めています。

現状は以下です。

- **第1段階（Temporal Epoch Stack）**: ほぼ導入済み
- **第2段階（CompiledPlan キャッシュ）**: 基本導入済み
- **第3段階（QuantizedBlock）**: 骨組み実装のみ
- **第4段階（高階関数最適化）**: MAP中心の部分対応
- **第5段階（計測・安全化）**: メトリクス・trace flagは追加済み、ベンチはまだ簡易テスト相当

---

## 2. 主要変更済みポイント

### Epoch / 無効化基盤
- `EpochSnapshot` 追加: `rust/src/interpreter/epoch.rs`
- `Interpreter` に epoch群追加: `global_epoch`, `dictionary_epoch`, `module_epoch`, `execution_epoch`, `epoch_stack`
- `DEF` / `DEL` / `IMPORT` 周辺で epoch bump を実施
- child runtime に `spawn_epoch` を保持

### CompiledPlan
- `rust/src/interpreter/compiled-plan.rs` 追加
- `CompiledPlan`, `CompiledLine`, `CompiledOp`、`compile_word_definition`, `execute_compiled_plan`, `is_plan_valid`
- `WordDefinition.compiled_plan` を追加してキャッシュ保持
- `execute_word_core` でキャッシュ hit/miss と再コンパイル分岐

### QuantizedBlock（現状は最小限）
- `rust/src/interpreter/quantized-block.rs` 追加
- `QuantizedBlock` 型と `quantize_code_block` / `is_quantizable_block`
- 高階関数側 `ExecutableCode` に `QuantizedBlock` を追加
- `MAP` 経路で quantized 実行を一部利用

### 計測 / trace
- `RuntimeMetrics` 追加（plan build/hit/miss, quantized build/use）
- Cargo features 追加:
  - `trace-compile`
  - `trace-epoch`
  - `trace-quant`

---

## 3. 未完了・要対応（重要）

以下は **仕様書の完了条件に対して未充足** です。

1. **QuantizedBlock の静的解析不足**
   - `input_arity` / `output_arity` がほぼ `Variable`
   - `purity` が `Unknown` 中心
   - `dependency_words` 未活用

2. **高階関数の最適化不足**
   - MAP 以外（`FILTER`, `ANY`, `ALL`, `COUNT`, `FOLD`, `SCAN`）の専用 kernel が未整備
   - エラー時のスタック復元ポリシーを quantized 経路で統一検証できていない

3. **perf-regression-tests が“ベンチ”として弱い**
   - 現在は `#[test]` ベースのスモーク寄り
   - 所要時間・利用率などの計測レポート未整備

4. **CompiledPlan の最適化粒度**
   - fallback token を含む行は行全体を従来実行へ戻す保守的設計
   - 効果は安全だが性能上の伸び代あり

---

## 4. ClaudeCode で優先して進める実装順

### 優先A（第4段階の実質完了）
1. `FILTER` / `ANY` / `ALL` / `COUNT` に quantized predicate kernel を導入
2. `FOLD` / `SCAN` に quantized fold kernel を導入
3. quantized経路と従来経路の一致テスト（結果・エラー動作）を拡充

### 優先B（第3段階の質向上）
1. `quantize_code_block` で arity 推論（最低でも典型1→1, 2→1）
2. purity 推論（副作用語検出）
3. dependency_words の収集

### 優先C（第5段階の完了）
1. `perf-regression-tests.rs` を bench/計測出力付きに再構成
2. metrics 出力（feature flag有効時）を統一
3. 実行時間・hit率・quantized利用率を定量可視化

---

## 5. 既存メトリクス項目（利用可能）

`Interpreter.runtime_metrics()` で取得可能:

- `compiled_plan_build_count`
- `compiled_plan_cache_hit_count`
- `compiled_plan_cache_miss_count`
- `quantized_block_build_count`
- `quantized_block_use_count`

---

## 6. 推奨確認コマンド

```bash
cd rust
cargo test -q
cargo test -q compiled_plan_tests
cargo test -q quantized_block_tests
cargo test -q perf_regression_tests
```

trace確認例:

```bash
cd rust
cargo test -q --features trace-compile
cargo test -q --features trace-epoch
cargo test -q --features trace-quant
```

---

## 7. 注意点

- 意味論保全が最優先（未対応は必ず fallback）
- import/再定義/削除後の plan 無効化を壊さない
- semantic_registry 整合を崩さない
- child runtime の `spawn_epoch` は将来拡張の前提情報なので削除しない

---

## 8. 受け渡しメモ

このブランチは「基盤を先に固めた状態」です。最終完了には、

- quantized 経路の網羅化
- 推論精度（arity/purity/dependencies）の強化
- 計測の本格化

が必要です。

