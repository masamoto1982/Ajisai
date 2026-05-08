# Ajisai 半コンパイル実行系化 — ClaudeCode 引き継ぎ書

最終更新: 2026-05-08

---

## 1. 目的と現状

本ブランチでは、Ajisai 実行系の段階的改修（Epoch → CompiledPlan → QuantizedBlock → 高階関数最適化 → 計測）を進めています。

現状は以下です。

- **第1段階（Temporal Epoch Stack）**: ほぼ導入済み
- **第2段階（CompiledPlan キャッシュ）**: 基本導入済み
- **第3段階（QuantizedBlock / VTU分類）**: arity・purity・dependency推論とVTU hint/metricsまで導入済み
- **第4段階（高階関数最適化）**: MAP/FILTER/ANY/ALL/COUNT/FOLD の quantized + Tensor bulk kernel を導入済み
- **第5段階（計測・安全化）**: メトリクス・trace flag・VTU countersは追加済み、ベンチはまだ簡易テスト相当

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

### QuantizedBlock / VTU（現状）
- `rust/src/interpreter/quantized-block.rs` 追加
- `QuantizedBlock` 型と `quantize_code_block` / `is_quantizable_block`
- arity推論（典型的な 1→1 / 2→1）、purity推論、dependency_words収集を導入済み
- `VtuHint` / `VtuSuitability` / `VtuBackendCandidate` を追加し、実行意味論には影響しない観測用分類として保持
- 高階関数側 `ExecutableCode` に `QuantizedBlock` を追加
- `MAP` / `FILTER` / `ANY` / `ALL` / `COUNT` / `FOLD` で quantized 経路を利用

### VTU Phase II / III（ClaudeCode後続で進展済み）
- Vector生成を dense `Tensor` producer に寄せ、consumer側は `as_vector_view` / `ensure_hydrated` 境界ヘルパで既存意味論を維持
- MAP/FILTER/FOLD/ANY/ALL/COUNT は 1-D dense Tensor + fast kernel の場合、`Tensor.data` を直接走査する bulk fast path を持つ
- SHAPE/RANK/RESHAPE/TRANSPOSE/FILL/JOIN/SORT/COMPARE/LOGIC/CAST/CHARS は dense Tensor 入力互換の parity test を追加済み
- VTU counters: flatten/rebuild/broadcast/unary/simd/candidate/rejected/fusion/bulk を `RuntimeMetrics` に追加済み

### 計測 / trace
- `RuntimeMetrics` 追加（plan build/hit/miss, quantized build/use, VTU counters）
- Cargo features 追加:
  - `trace-compile`
  - `trace-epoch`
  - `trace-quant`

---

## 3. 未完了・要対応（重要）

以下は **仕様書の完了条件に対して未充足** です。

1. **SCAN のVTU/quantized fold parityが未完**
   - `FOLD` は quantized/bulk kernel 済みだが、`SCAN` は fold と同じ binary op を使う設計に留まる
   - accumulator履歴を保持するため、bulk化時の出力形状・エラー復元の parity test がまだ不足

2. **QuantizedBlock 静的解析の精度向上余地**
   - 典型的な builtin arity / purity / dependency_words は入るようになった
   - ただし unknown op を含む複合ブロック、qualified user word、ネストした control-flow の arity は保守的に `Variable` へ落ちる

3. **perf-regression-tests が“ベンチ”として弱い**
   - 現在は `#[test]` ベースのスモーク寄り
   - 所要時間・利用率などの計測レポート未整備

4. **VTU bulk path の網羅性**
   - Phase III は 1-D dense Tensor + fast unary/binary kernel に限定
   - 2-D以上、user word kernel、SCAN、複合predicateのbulk化は未対応

5. **CompiledPlan の最適化粒度**
   - fallback token を含む行は行全体を従来実行へ戻す保守的設計
   - 効果は安全だが性能上の伸び代あり

---

## 4. ClaudeCode で優先して進める実装順

### 優先A（VTU Phase IIIの仕上げ）
1. `SCAN` に quantized fold kernel / bulk parity test を追加
2. MAP/FILTER/FOLD/ANY/ALL/COUNT の bulk fast path について、エラー時のスタック復元 parity test を増やす
3. user word kernel・複合predicateが安全に fallback することを metric込みで維持

### 優先B（第3段階の質向上）
1. `quantize_code_block` の unknown op 周辺の arity 推論をもう一段だけ精密化
2. nested control-flow / qualified word の purity・dependency propagation を拡充
3. `VtuHint` は引き続き guard signature から除外し、意味論に影響しない観測情報として扱う

### 優先C（第5段階の完了）
1. `perf-regression-tests.rs` を bench/計測出力付きに再構成
2. metrics 出力（feature flag有効時）を統一
3. 実行時間・hit率・quantized利用率・VTU bulk利用率を定量可視化

---

## 5. 既存メトリクス項目（利用可能）

`Interpreter.runtime_metrics()` で取得可能:

- `compiled_plan_build_count`
- `compiled_plan_cache_hit_count`
- `compiled_plan_cache_miss_count`
- `quantized_block_build_count`
- `quantized_block_use_count`
- `vtu_tensor_flatten_count` / `vtu_tensor_flattened_elements`
- `vtu_tensor_rebuild_count` / `vtu_tensor_rebuilt_elements`
- `vtu_broadcast_count` / `vtu_unary_flat_count` / `vtu_allocated_elements`
- `vtu_same_shape_elementwise_count` / `vtu_projected_broadcast_count`
- `vtu_simd_kernel_use_count` / `vtu_bulk_kernel_use_count`
- `vtu_candidate_block_count` / `vtu_rejected_block_count` / `vtu_fusion_candidate_count`

---

## 6. 推奨確認コマンド

```bash
cd rust
cargo test -q
cargo test -q compiled_plan_tests
cargo test -q quantized_block_tests
cargo test -q perf_regression_tests
cargo test -q vtu_phase_iii
cargo test perf_regression_tests -- --nocapture
```

trace確認例:

```bash
cd rust
cargo test -q --features trace-compile
cargo test -q --features trace-epoch
cargo test -q --features trace-quant
cargo test -q --features "trace-compile trace-epoch trace-quant"
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
