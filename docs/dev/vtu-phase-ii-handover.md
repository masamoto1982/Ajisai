# VTU Phase II 引き継ぎ書 — Vector / Tensor 並列バリアント導入

最終更新: 2026-05-07
作成セッション: PR #871 (VTU Phase I 観測層) のマージ直後
対象ブランチ: 新規作成 (例: `claude/vtu-phase-ii-tensor-variant-XXX`)
canonical 仕様: `SPECIFICATION.md` (この引き継ぎ書は non-canonical なメモ)

---

## 0. このドキュメントの読み方

このセッションは VTU (Virtual Tensor Unit) Phase II の **設計合意のみ** が完了した状態です。
コード改修はこのドキュメントを起点に新セッションで実施します。

**ユーザーの明示的な制約 (絶対条件)**:
- Fraction exactness は保たれること (近似数値型 f32/f64/BFloat16 を導入してはならない)
- 既存の **混合 Vector** (`[ 1 'a' 3 ]` のような型混在) は引き続き書ける/動くこと
- 後方互換性 (pub API 互換、既存の構造体レイアウト互換) は不要
- 実装コストに糸目をつけず、省電力性 (= データ移動の最小化) を徹底追求する
- すべての値が内部的に Fraction で扱われる原則を徹底する

これらが矛盾するように見えたら、**ユーザーに確認**してください。勝手に判断しないでください。

---

## 1. 完了済みの前提 (PR #871, マージ済み)

### 何が入ったか
`docs/dev/virtual-tensor-unit-design.md` を必ず先に読んでください。要約:

- `RuntimeMetrics` に 13 個の VTU 観測カウンタを追加 (`vtu_*_count`, `vtu_*_elements` 系)
- `quantized_block` に `VtuHint` / `VtuSuitability` / `VtuBackendCandidate` / `DataMovementClass` を追加し、`infer_vtu_hint(kernel_kind, purity)` で `QuantizedBlock` に付与
- `apply_binary_broadcast_with_metrics` / `apply_unary_flat_with_metrics` を新設し、
  `arithmetic.rs` / `logic.rs` / `tensor-shape-commands.rs` の呼び出し元から `&mut RuntimeMetrics` を引き渡している
- SIMD fast path 使用時に `vtu_simd_kernel_use_count` を bump
- `QuantizedBlock` build 時に candidate / rejected / fusion-candidate をカウント
- 9 件の VTU テストを `quantized-block-tests.rs` に追加
- すべて **観測のみ**。意味論・exactness・evaluation order は変更なし
- `vtu_hint` は `GuardSignature` に **意図的に含めていない** (= cache 無効化を起こさない)

### Phase II は何を変えるか
Phase I で「無駄な flatten / rebuild / copy」を **見えるように** した。
Phase II は それを **実際に減らす**。

---

## 2. Phase II 設計合意 (今回確定)

### 値表現の変更
`rust/src/types/mod.rs` の `ValueData` に **新バリアントを追加**:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ValueData {
    Scalar(Fraction),
    Vector(Rc<Vec<Value>>),                                  // 既存・不変。混合用。
    Tensor {                                                 // ★新規。dense numeric 専用
        data: Rc<Vec<Fraction>>,
        shape: Rc<Vec<usize>>,
    },
    Record { pairs: Rc<Vec<Value>>, index: HashMap<String, usize> },
    Nil,
    CodeBlock(Vec<Token>),
    ProcessHandle(u64),
    SupervisorHandle(u64),
}
```

**重要**:
- `Vector` と `Tensor` は **観測上同等の値**として比較・表示される (`PartialEq` / `Display` を両対応に)
- 「同じ論理 Vector が2形態を行き来する」のではなく、構築時点でどちらかに決まる **別バリアント**
- strides は持たない (shape から `compute_strides` で導出。`tensor-shape-operations.rs` 既存関数を流用)

### 昇格ポリシー (重要)
ユーザー合意: **literal parser + 中間生成も Tensor**

- **Literal parser (`rust/src/tokenizer.rs` から `parser` 経由で値を構築する経路)**:
  - `[ N N N ... ]` で **全要素が Number Token のみ** の場合 → `Tensor` として構築
  - 一つでも String / CodeBlock / 混合 / NIL が含まれていたら → 従来通り `Vector`
  - ネストした numeric (例 `[ [ 1 2 ] [ 3 4 ] ]`) も全要素 Fraction なら `Tensor { shape: [2,2] }`
- **算術・MAP・FILTER・FOLD・SCAN などの pure numeric 出力**:
  - `apply_binary_broadcast_with_metrics` / `apply_unary_flat_with_metrics` の出力を `Tensor` に変更
  - 既存の `FlatTensor::to_value()` が `build_nested_value()` で `Vector` を再構成しているのを廃止し、`Tensor` を直接返す
  - HOF (`MAP` / `FILTER` 等) の出力も Fraction-only なら `Tensor`
- **Vector が numeric op に渡された場合**: 現状と同じく `FlatTensor::from_value()` で flatten (Vector → 一時 FlatTensor → 出力 Tensor)。これだけが残る変換コストになる

### Spec 改定文面 (案)
`SPECIFICATION.md` の `## 3. Syntax` または `## 5. Values` 相当の章に追記:

> Vector 値の内部表現は2クラス:
> - `nested`: 任意の Value を要素に持つツリー構造 (`Vec<Value>`)
> - `dense`: 全要素が Fraction で、`shape` を持つ密表現 (`Vec<Fraction>` + `shape`)
>
> 構築時点でどちらかが選ばれ、観測可能な意味論 (Display / 順序 / 等価性 / NIL 判定 / shape) は2クラス間で完全に一致する。
> 操作は dense を fast path として扱ってよい。dense と nested の混在比較は、
> nested 側を flatten したときの Fraction 列および shape が一致するときに等しい。

正確な章番号や周辺文章はリポジトリの最新 spec を確認すること。

---

## 3. 推奨 PR シーケンス

各 PR は独立 commit / 独立 draft PR として作る。順序依存があるため、原則直列。

### PR #1 (規模: 中) — Spec 改定 + バリアント追加 + read-side 対応
**コードを動かす producer はゼロのままにする**。これが守れていれば既存テスト 817 件は不変。

着手チェックリスト:
- [ ] `SPECIFICATION.md` に Vector/Tensor 二クラス規定を追記 (上記文面ベース)
- [ ] `docs/dev/virtual-tensor-unit-design.md` の "Future scope" を更新し、Phase II が始まったことを反映
- [ ] `rust/src/types/mod.rs` の `ValueData` に `Tensor { data, shape }` バリアントを追加
- [ ] `rust/src/types/value-operations.rs` の以下メソッドを `Tensor` 対応にする (`fn shape:411`, `fn count_fractions:398`, `fn collect_fractions_flat_into:383`, `fn is_vector:154`, `fn len:185`, `fn get_child:200`, `fn get_child_mut:212`, `fn as_scalar:318`, `fn from_vector:95` の周辺)
- [ ] `Display`, `PartialEq`, `Hash` を `Tensor` 対応 (Vector 等価性ルール: shape と flatten 列が一致)
- [ ] `rust/src/types/flow-token.rs:46` の `match` 分岐を `Tensor` 対応
- [ ] `rust/src/types/arena.rs:103` の `match` 分岐を `Tensor` 対応 (alloc/round-trip)
- [ ] `rust/src/wasm-value-conversion.rs` の Value↔JS 変換を Tensor 対応 (TS 側に渡す瞬間に hydrate しても可)
- [ ] `cargo build --tests` が通る (= 全 `match ValueData` の網羅性が満たされる)
- [ ] `cargo test --lib` が 817 件 pass (Tensor 生成者ゼロなので変化なし)
- [ ] draft PR 作成

**意図的に省く**:
- producer の追加 (PR #2)
- HOF / parser の Tensor 化 (PR #2 以降)

### PR #2 (規模: 中) — Producer 切替
- [ ] literal parser (`rust/src/parser.rs` または相当) で全要素 Fraction の `[ ... ]` を `Tensor` 構築
- [ ] `tensor-shape-operations.rs` の `apply_binary_broadcast_with_metrics` / `apply_unary_flat_with_metrics` の最終 `to_value()` を `Tensor` 直接返却に変更
- [ ] `quantize_code_block` 経由の HOF kernel 出力 (`MAP` / `FILTER` 等) を `Tensor` 化
- [ ] `FlatTensor::to_value()` の挙動変更 or `to_tensor_value()` 追加
- [ ] **計測**: PR #871 で入れた `vtu_tensor_rebuild_count` が劇的に減ることを `bench-baselines/` に記録
- [ ] 既存テスト全件 pass

### PR #3 (規模: 大) — Consumer 完全対応
PR #1 で `match` 網羅性は通っているが、**意味論的に正しい** consumer 対応はここで仕上げる。

- [ ] HOF (`MAP` / `FILTER` / `FOLD` / `SCAN` / `ANY` / `ALL`) を `Tensor` 入力で fast path 動作させる (現状 `Vector` 前提のループを `Tensor.data` 直接イテレートに)
- [ ] CAST / JSON / PRINT / SHAPE / RESHAPE / TRANSPOSE / FILL / COMPARE / LOGIC 各 op の `Tensor` 対応
- [ ] **user-visible 境界 (PRINT / JSON-EXPORT / GUI 出力 / error メッセージ)** で必ず hydrate するか、Tensor 形のまま正しく出力できるかを精査
- [ ] `Value::ensure_hydrated()` または `Value::as_vector_view()` のヘルパーを設けて hot path 以外の互換性を担保
- [ ] examples 全件が変わらない出力であることを CI で確認

### PR #4 (規模: 中) — Plan-level Fusion 実行 (Phase B)
- [ ] `CompiledPlan` プリパスで `MapUnaryPure → MapUnaryPure`, `MapUnaryPure → PredicateUnaryPure` 連鎖を検出
- [ ] 検出された連鎖を単一クロージャ kernel に lowering
- [ ] `vtu_fusion_executed_count` を `RuntimeMetrics` に追加
- [ ] PR #871 で入れた `vtu_fusion_candidate_count` ↔ 新規 `vtu_fusion_executed_count` で「検出 vs 実行」が比較可能に

### PR #5 (規模: 中) — In-place 変異 (Phase C)
- [ ] `Tensor.data` が `Rc::get_mut` で取れる場合、出力バッファを再利用 (新 alloc しない)
- [ ] 既存の `optimization_hooks::check_in_place_candidate` が「候補までは検出」しているのを実消費
- [ ] `vtu_allocated_elements` が unary/同形 binary で 0 になることを確認

### PR #6 (規模: 中) — 純粋全プラン結果メモ化 (Phase D)
- [ ] `QuantizedBlock.eligible_for_cache && purity == Pure` で、入力 `Tensor` の Fraction digest をキーに出力 `Tensor` をキャッシュ
- [ ] `elastic_cache` (`rust/src/elastic/cache_manager.rs`) と統合 or 隣接モジュール化
- [ ] ループ内の同入力・同 plan を完全スキップ

### PR #7 (規模: 小〜中) — 整数 Fraction SIMD 拡張 (Phase E)
- [ ] `rust/src/interpreter/simd-vector-operations.rs` の `extract_integer_vector` を **分母 == 1 の Fraction batch** にも拡張
- [ ] `MAP` / `FILTER` の inner kernel として `apply_simd_*` を呼べるよう接続
- [ ] 分母が 2 のべき乗のときの除算 SIMD など、exactness を壊さない範囲での専用 kernel

### PR #8 (規模: 小) — `explain --vtu <word>` (Phase F)
- [ ] fusion 後の kernel 列・各 kernel の suitability・推定 allocated_elements を textual dump
- [ ] StableHLO 風の安定書式 (将来 backend 接続の前提)

---

## 4. PR 依存関係

```
PR1 ──► PR2 ──► PR3 ──┬──► PR4 ──► PR8
                       ├──► PR5
                       ├──► PR6
                       └──► PR7
```

PR4〜7 は PR3 完了後は並行可能。

---

## 5. 重要な「触るな」リスト

以下は Phase II の対象外。触るとスコープが破裂する:

- `EXACT` / `APPROX` 境界の言語拡張 (将来別フェーズ)
- 実 backend (WebGPU / NPU / TPU) の lowering
- `f32` / `BFloat16` のような近似数値型の導入
- StableHLO / MLIR テキストダンプ (`explain --vtu` の拡張として将来実装)
- `vtu_hint` を `GuardSignature` に含めること (絶対 NG。cache 無効化を起こす)
- 並列 reduction (FoldBinaryPure を StrongCandidate に格上げすること) — exactness 議論が未決
- VTU メトリクスを「実測電力」と表現すること (常に「省電力の代理指標」)

---

## 6. 確認・調査が必要な未解決事項

新セッションで PR #1 着手前に確認/調査すべき項目:

1. **混合 Vector の現状使用箇所**: examples / built-ins / user dictionaries で `[ 1 'a' 3 ]` 系の混合 Vector が使われていないか grep。Phase II では混合 Vector は引き続き動くが、影響範囲の把握のため。
2. **literal parser の場所と入口**: `rust/src/parser.rs` か別ファイルか。「Token 列 → Value」変換が一箇所でできているか、複数経路あるか。
3. **arena (`rust/src/types/arena.rs`)**: `Tensor` バリアント追加時に SoA 化や ID 変更が必要か。`alloc_vector` 相当の `alloc_tensor` を生やす設計か。
4. **`DisplayHint`**: Tensor の hint は Vector と同じで良いか、専用の `DisplayHint::Tensor` を作るか (現状の hint 値の使われ方を確認)。
5. **GUI/TS 側**: `wasm-value-conversion.rs` 経由で TS に渡される Value 形は変わるか。GUI が Tensor を理解する必要があるか、それとも WASM 境界で必ず Vector に hydrate するか。後者推奨。

これらは PR #1 着手前に user に確認するか、調査結果を PR description に明記すること。

---

## 7. 参考: 主要ファイルの場所

| 役割 | パス |
|------|------|
| Spec (canonical) | `SPECIFICATION.md` |
| Phase I 設計メモ | `docs/dev/virtual-tensor-unit-design.md` |
| `ValueData` 定義 | `rust/src/types/mod.rs:48` |
| Value メソッド群 | `rust/src/types/value-operations.rs` |
| FlatTensor / 既存 tensor ops | `rust/src/interpreter/tensor-shape-operations.rs` |
| QuantizedBlock + VtuHint | `rust/src/interpreter/quantized-block.rs` |
| RuntimeMetrics 定義 | `rust/src/interpreter/interpreter-core.rs:120` 周辺 |
| SIMD fast paths | `rust/src/interpreter/simd-vector-operations.rs` |
| 既存 VTU テスト | `rust/src/interpreter/quantized-block-tests.rs` (末尾の `vtu_*` 群) |
| arena (alloc 経路) | `rust/src/types/arena.rs` |
| WASM 境界 | `rust/src/wasm-value-conversion.rs` |
| flow-token (match を増やすべき場所の例) | `rust/src/types/flow-token.rs:46` |

---

## 8. 受け入れ基準 (各 PR 共通)

- `cargo test --manifest-path rust/Cargo.toml --lib` が全件 pass
- `cargo clippy --tests --no-deps` が新規 warning を増やさない (現状 baseline 100 件)
- 既存 examples の出力が不変
- VTU メトリクスのテストが pass し、PR 効果を `bench-baselines/` に記録
- draft PR で開き、CI が green になってからレビュー依頼

---

## 9. このセッションでの会話履歴サマリ

- ユーザーが PR #871 (Phase I 観測層) のマージ後、次フェーズの方針提示を依頼
- 4 案 (可視化 / fusion 検出 / TensorView / EXACT-APPROX 境界) を提示
- ユーザー回答: 「実装コストに糸目をつけず省電力徹底、Fraction 内部処理の徹底さえ守れば後方互換不要」
- 8 PR 構成の Phase II プランを提示
- A: ExecutionValue::FlatTensor 全面置換 / B: Spec 更新 で合意
- 「dual-representation 規定」より単純な解として **Vector + Tensor の並列バリアント** を提案 → 受諾
- 昇格タイミング: literal parser + 中間生成も Tensor で合意
- 混合 Vector は引き続き書ける必要あり (確認済み)

新セッションでは **PR #1 から着手**。最初に Section 6 の未解決事項を確認してから動くこと。
