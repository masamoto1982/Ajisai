# 並列対応・最適化 実装指示書（セッション引き継ぎ）

本書は「Ajisai 全体を並列処理の仕組みに合わせて最適化する」フェーズの**実装引き継ぎ
ドキュメント**である。前セッションでソース分析と方針合意まで完了した。次セッションは
本書を起点にコールドスタートで実装に着手できる。

## Authority（権威モデル）
- **Non-canonical.** 本書は意味論・互換性を定義しない。Canonical source は
  `SPECIFICATION.html` のみ。観測可能な挙動を変える変更は仕様改訂を先行させること。
- 上位指針: `docs/dev/implicit-parallelism-roadmap.md`（暗黙並列ロードマップ）。本書は
  その Phase 1〜4 を「普段使いの低速回帰を直す」観点で具体化した作業指示である。

## 不可侵の契約（全作業に優先）
1. **Same Result** — 最適化後の結果は逐次実行とビット単位で同一。
2. **Never Slower** — どの規模でも逐次より遅くしない。
3. **Zero Syntax** — 並列/最適化のためのユーザ向け語・注釈・スレッド数指定を増やさない。
- 副作用（I/O・時刻・乱数・`SERIAL@*`）の発火順序は逐次と同一。

---

## 0. 背景（なぜやるか）

並列要素を後付けした結果、巨大要素数では速くなったが**普段使いで低速化**した。原因は
並列カーネルそのものではなく、**カーネルへ食わせるための値表現変換が、並列が発火しない
通常サイズでも常時課金されている**こと。しかもその変換は SoA(`DenseTensor`) を
AoS(`Vector`) へ逆方向に劣化させている（原理I「机を片付けよ」に反する）。

### 診断サマリ（コード根拠）
- 整数テンソル `+ - *` は SIMD 経路（`arithmetic.rs` `simd_schema_candidate` 付近 →
  `simd_ops.rs`）を通る。
- 入力抽出 `extract_integer_vector`（`rust/src/interpreter/simd_ops.rs:486`）は `Tensor`
  を受けても `DenseTensor.numerators: Vec<i64>` を**借用せず**、要素ごとに `Fraction` を
  生成して `to_i64()` し、新しい `Vec<i64>` を確保する。
- 出力 `create_value_from_integer_vector`（`simd_ops.rs:541`）は `from_children`
  （`value_operations.rs:289`）を呼び、結果を **`ValueData::Vector(Arc<Vec<Value>>)`（AoS・
  箱入り `Value` n個）** にする。`Tensor` には戻らない＝表現が毎回劣化する。
- 1回の `+` の往復:
  `DenseTensor(i64) → Vec<Fraction> → Vec<i64> → [演算] → Vec<i64> → Vec<Value>(AoS)`
  → 次演算で再抽出。O(n) 確保・解放が複数回、すべて逐次。
- 対照: 非SIMD broadcast 経路 `apply_binary_broadcast_with_metrics`
  （`rust/src/interpreter/tensor_ops.rs:369`）は `FlatTensor → Value::from_tensor` で
  **SoA を維持**する。つまり SIMD 導入が表現上は退化を持ち込んでいる。
- 並列発火閾値 `PARALLEL_DISPATCH_MIN = 900_000`（`rust/src/interpreter/parallel.rs:65`）。
  普段サイズでは並列は発火せず、変換税だけが残る。
- wasm は `parallel.rs` 全体が `#[cfg(not(target_arch = "wasm32"))]`、SIMD128 も既定外
  （`simd_ops.rs:588`）。**主戦場のブラウザでは並列もSIMDも無いのに変換税は丸ごと払う。**
- 既定モードは `ElasticMode::Greedy`（`rust/src/elastic/execution_mode.rs:14`）。並列の
  便益はオフ、変換税はオン、という最悪の組み合わせ。

### 既に実装済みで「やらなくてよい」こと（重要）
- **有理数/無理数の分離は実装済み**。`ValueData::Scalar(Fraction)`（有理数）と
  `ValueData::ExactScalar(ExactReal)`（無理数）。普段の数値は連分数に触れない。
- **連分数の LFT/双線形遅延合成（Gosper）も実装済み**。`types/continued_fraction.rs` の
  `Gosper::Mobius` / `Gosper::Bihomographic`、両オペランド有理なら `Fraction` へ短絡。
- → よって「連分数まわりの再設計」は本フェーズの対象外。あくまで**整数/有理数テンソルの
  ホットパス**を直す。

---

## 合意済み実装順序

```
正攻法 手1（ゼロコピー）        ← まず出血を止める。単独で普段使いは戻る
  ├─ 奇策本命（投機ロワリング）  ← 手1の上に乗せると native 整数速度に届く
  └─ 奇策その2（融合）           ← 手2/手4の前にやると中間確保を消せる
正攻法 手3 → 手4 → 手5          ← 地盤が固まってから本来の戦場で並列化・既定化
```

各ステップは独立にレビュー可能な単位（できれば別コミット）とする。

---

## 手1【最優先】整数レーンのゼロコピー化

**目的**: 並列の有無に関係なく、整数テンソル算術の表現往復を撤廃し普段使いを回復する。

**作業**
1. `extract_integer_vector`（`simd_ops.rs:486`）に「借用」高速路を追加。
   - `DenseTensor` が `is_pure_integer == true` かつ全 `valid_mask` 有効なら、
     `&data.numerators`（`&[i64]`）を**そのまま借りる**。要素ごとの `Fraction` 生成・
     `to_i64()` を撤廃。所有が必要な箇所は `Cow<[i64]>` 等で借用/所有を選べる形に。
   - `Vector(Arc<Vec<Value>>)` 入力は従来通り（ただし手1の主眼は Tensor 経路）。
2. 出力を SoA で返す。`create_value_from_integer_vector`（`simd_ops.rs:541`）を、
   `from_children`(AoS) ではなく **`Value::from_tensor` 相当で `DenseTensor` を直接構築**
   して `Tensor` を返すよう変更（`value_operations.rs:1044` `from_tensor`、または
   `DenseTensor` を直に組んで `ValueData::Tensor` を生成）。
   - 純粋整数なら `numerators = result`, `denominators = vec![1; n]`,
     `valid_mask = 全有効`, `is_pure_integer = true`, `shape = [n]` を直接セットして
     `Fraction` 経由の再densifyを避ける（`DenseTensor::from_fractions` のコストも回避）。
3. `apply_simd_*`（`simd_ops.rs:787` 以降）と `arithmetic.rs` の呼び出し側の型を、
   借用スライス＋SoA出力に合わせて調整。

**契約**
- Same Result: 既存差分テストが緑。出力が `Tensor`(SoA) になっても、表示/比較/等価は
  `ValueData::PartialEq` の Vector↔Tensor 相互比較（`types/mod.rs:360`）でカバーされる
  が、**hint と absence の扱いが従来 `from_children`(Unassigned) と変わらないこと**を確認。
- Never Slower: 中規模ベクトル `+`/`*` のベンチで改善（最低でも非劣化）を実測。

**検証**
- `cargo test`（特に `simd_ops` テスト、`interpreter::differential_tests`、
  `tensor_operation_tests`、`arithmetic_operation_tests`）。
- `rust/benches/interpreter-performance-benchmarks.rs` に中規模(例 1K/10K/100K)整数
  `+`/`*` を追加し、変更前後を比較。

**注意**
- 結果表現が `Vector` → `Tensor` に変わることで、後段の語（`GET`/`LENGTH`/`MAP` 等）が
  Tensor を正しく扱うか回帰確認。`as_dense_tensor`（`value_operations.rs:520`）経路がある
  ので大半は問題ないはずだが、Text hint や NIL レーンの伝播に注意。

---

## 奇策本命【手1の上に乗せる】検証付き投機的ロワリング

**目的**: 厳密性を安全装置に転用し、普段の整数演算を native i64/i128 速度へ。

**発想**: 普段の整数配列はほぼ `i64` に収まる。最速路を投機的に走らせ、**オーバーフロー
が一度も立たなければ結果は厳密値とビット同一**（証明済み）→ そのまま採用。立った要素が
あった場合のみ BigInt 厳密路で再計算。「並列化したら答えが変わる」が原理的に起きない。

**作業**
1. 整数レーン演算を `checked_*` / `overflowing_*` で実装し、レーン全体で overflow フラグ
   を OR 集約（SIMD/並列でも集約可能）。
2. overflow == 0 → SoA i64 結果を確定（手1の出力経路に直結）。
3. overflow != 0 → 当該演算を既存の厳密路（`apply_binary_broadcast_with_metrics`、
   `Fraction`/BigInt）にフォールバック。
4. `i128` 中間で広げてから範囲チェックする版も検討（乗算のオーバーフロー頻度低減）。

**既存資産との接続**
- 検証思想は `higher_order/hedged.rs` / `shadow_validation.rs` と同型だが、**二重フル実行に
  しない**。安い overflow チェックで勝ちを確定するのが要点（hedged の正しい使い方）。
- 観測カウンタは `RuntimeMetrics`（`interpreter_core.rs:188` 以降）に
  `vtu_*` と並べて投機採用/フォールバック回数を追加してよい（観測専用・意味論不変）。

**契約**
- Same Result: overflow フォールバックの結果が厳密路と一致する差分 proptest を追加
  （`differential_tests.rs` 拡張）。境界値（`i64::MAX` 近傍、乗算オーバーフロー）を必ず掃く。
- Never Slower: フォールバックは稀かつチェック自体は安い。オーバーフロー多発の合成入力で
  退化しないことをベンチで確認。
- §8 非目標に注意: f64 を**値の計算**に使わない（整数で完結）。f64 は将来、比較の
  「予測器」としてのみ（確定は厳密区間）検討。

---

## 奇策その2【手2/手4の前に】パイプライン融合（deforestation）

**目的**: `MAP … MAP … FILTER` 連鎖の**中間配列確保**を消す。スレッド化より普段サイズで
効くことが多い。

**作業**
1. 隣接する純粋演算（`purity_table` で判定可能）を、`quantized_block`
   （`rust/src/interpreter/quantized_block.rs`）の段階で**1つの列指向カーネルに融合**し、
   SoA バッファを一掃きで走査する。
2. 融合の安全境界は純粋性・順序非依存（`elastic_eligible`、
   `evaluation_unit.rs`）で引く。order-sensitive/eager は融合しない（逐次）。
3. まずは `MAP→MAP`、`MAP→FILTER` の2段融合から。FOLD は結合的と判定できた時のみ。

**契約**
- Same Result: 融合前後の差分テスト。
- Never Slower: 中間配列が消えることでの改善を実測。
- Zero Syntax: 融合は完全自動。

**注意**: これは「並列化の依頼に対し『並列化しない』が最適解」になりうるステップ
（原理IV 過剰分散の禁止）。スレッドを足す前に中間確保を消すのが先。

---

## 手3 ディスパッチの脱アロケーション

**作業**
- `canonicalize_core_word_name`（`rust/src/core_word_aliases.rs:149`）が毎回 `String` を
  確保している。`Cow<'static, str>` 返却に変更（別名ヒット時は `&'static str`、それ以外も
  大文字化が不要なら借用）。
- `lookup_core_word_alias`（`core_word_aliases.rs:145`）の線形スキャンを静的マップ
  （`phf` か、`match`/ソート済み二分探索など依存追加なしの手段）へ。
- 二重正規化の解消: `execute_word_core_inner`（`execute_builtin.rs:68`）と
  `execute_builtin`（`:170`）、`resolve_word_entry`（`resolve_word.rs:268`）で重複して
  canonicalize している。一度だけにする。
- 二項算術のオペランド clone（`arithmetic.rs` `stacktop_pair`、約 :155-164）の削減を検討
  （借用で済む経路を増やす）。

**契約/検証**: 機能不変。FOLD 等多反復ベンチで改善を実測。`perf_regression_tests.rs`。

---

## 手4 正しい戦場で並列化

**作業**
- 階層A の並列対象を、メモリ律速の `i64 +` から **compute-bound カーネル**へ:
  厳密有理数の要素演算（num/den 2レーン + 後段 gcd 正規化）、行列積・畳み込みのブロック化。
- 演算種別ごとに閾値を分離（帯域律速は高め、compute-bound は低め）。`PARALLEL_DISPATCH_MIN`
  を演算特性別の関数/定数群に拡張。
- 既存プール（`parallel.rs` の `for_each_chunk`、disjoint 所有）を再利用。

**契約**: 対コア数で準線形スケール（compute-bound）、差分テスト緑、Never-Slower ベンチ。

---

## 手5 既定化（Phase 4）

**作業**
- Never-Slower ベンチが緑になってから、`execution_mode.rs` の既定を自動並列モードへ昇格。
  `greedy` は明示オプトアウトとして残す。WASM/CLI のモード文字列既定も更新。
- 差分テスト・Never-Slower ベンチを CI ゲート化（Phase 7 と並走）。

**契約**: 既定で並列が効き、`greedy` 指定で逐次に戻ることをテスト。

---

## 検証ハーネス（全手で使う）
- 差分（並列==逐次, 投機==厳密）: `rust/src/interpreter/differential_tests.rs` を拡張。
- 並列カーネル単体: `rust/src/interpreter/parallel.rs` の proptest（policy-free な
  `run_parallel_binary` を直接駆動）。
- 回帰: `rust/src/interpreter/perf_regression_tests.rs`、
  `rust/benches/interpreter-performance-benchmarks.rs`、`bench-baselines/`。
- ビルド: native (`cargo test`) と wasm の両方を必ず確認（`parallel.rs` の cfg 分岐、
  `Cargo.toml` の `wasm` feature）。

## やってはいけないこと（非目標 / ロードマップ §8）
- 並列/最適化のためのユーザ向け新語・新構文・注釈の追加。
- f64 浮動小数点での値の計算（生 FLOPS 競争）。
- BigInt(`Big`)レーンの SIMD 化。
- 連分数まわりの再設計（既に妥当。本フェーズ対象外）。
- `SPECIFICATION.html` の観測可能挙動の変更（必要なら仕様改訂を先行）。

## 着手推奨
**手1 + 奇策本命（投機ロワリング）を一括実装 → ベンチ検証**から。普段使いの整数演算に
最も効く。地盤（往復除去）の上に最速路を乗せる順序を厳守すること。
</content>
</invoke>
