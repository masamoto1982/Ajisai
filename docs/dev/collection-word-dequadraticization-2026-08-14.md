# 走査系 Word の非二次化

作成日: 2026-08-14
対象: `host-profile-derivation-handoff.md` §1 の着手条件（走査系の非二次化）
関連: [`collection-word-billing-2026-08-13.md`](./collection-word-billing-2026-08-13.md) /
[`mcp-host-profiles.md`](./mcp-host-profiles.md) /
[`host-profile-derivation-handoff.md`](./host-profile-derivation-handoff.md)

## 0. この文書の位置づけ

`host-profile-derivation-handoff.md` は「走査系の非二次化が main に入り、コレクション
メーターが再較正されるまで着手しない」と明記した上で、「非二次化そのものは本書の対象外
であり、そちらの引き継ぎ書はまだ無い」と書いていた。本書がその引き継ぎ書である。

非正典。資源上限は `SPECIFICATION.html` §2.5 によりホストの安全制御であって言語の意味論
ではない。本書はその制御値の決め方についての作業記録であり、意味論には触れない。

## 1. 何を変えたか

`UNIQUE` / `TALLY` / `GROUP`（`rust/src/interpreter/ordering_ops.rs`）は、これまで
「これまでに見つかった相異なり値を線形走査する」実装だった。相異なり数を `d` とすると
実際の計算量は `n × d`（全相異なら `n²`）で、これは `collection-word-billing-2026-08-13.md`
が正しく課金する対象にした量そのものである。課金は正しかったが、アルゴリズム自体は
二次のままだった。

`Value: Hash` を実装し、線形走査を `HashMap` ルックアップに置き換えた。これにより
実際の計算量が `n × d` から平均 `O(n)` になる。課金モデルもこれに合わせて作り直した
（§3）。作業の過程で、新しい `Hash` 実装自体に無視できない実行時コストがあることも
分かり、それも修正した（§3.4）。

## 2. `Value: Hash` の設計

### 2.1 既存の `PartialEq` に一致させる制約

`Value` の等価性（`types/mod.rs`）は素朴ではない:

- `Vector` と `Tensor` は同じ矩形数値データを保持していれば相互に等しい
  （`tensor_eq_vector`、LANG.AUTHORITY.FREEDOM: 表現は意味論に現れない）。
- `Algebraic`（代数的数）の等価性は基底の再構成（rebase）で決まり、構造的な項マップの
  一致ではない: `√12` は単独では基底 `{12}` のまま保持され、`2·√3` として作ると基底
  `{3}` になる。両者は数学的に等しいが構造は異なる。

`Hash` は `a == b ⟹ hash(a) == hash(b)` を満たさなければならない。素朴に内部表現を
ハッシュすると、この二つのケースで違反する。

### 2.2 `Vector`/`Tensor` の相互ハッシュ

`nested_vector_shape` と `nested_flatten_matches`（既存の等価性判定が使っている補助
関数と同じ判定規則）を再利用し、矩形な数値構造を「形状 + 平坦化した `Fraction` 列」に
正準化してからハッシュする。`Tensor` は元々この形なので直接、`Vector` はこの正準化に
成功した場合だけ同じ経路を通り、失敗した場合（`ExactScalar` や `Nil`、`Text` などを含む
場合、既存の `nested_flatten_matches` が一致させない対象）は通常の構造的ハッシュに
フォールバックする。

### 2.3 代数的数の正準ハッシュ — 因数分解なしで

`basis.rs` の GCD-free 基底構築は意図的に完全な素因数分解を避けている（大きな半素数で
コストが無限大になるため）。したがって `(basis, terms)` を正準形として使うことはできない。

代わりに `Algebraic::sign()` が既に使っている「精度を倍々に上げて確定するまで区間を
絞る」手法を転用した。`bounds(bits)` は真の値を含む区間 `[lo, hi]` を返す縮小列で、
無理数（`Algebraic` が保持する値は定義上すべて無理数 — 有理数は即座に `Fraction` へ
降格する）は `2⁻ᴷ` 格子上の境界に厳密には乗らない。よって
`floor(lo · 2ᴷ) == floor(hi · 2ᴷ)` となるまで `bits` を倍にしていく操作は必ず有限回で
終わり、その結果（`floor(値 · 2⁶⁴)`）は基底や項の構造に関係なく、その実数値だけで
決まる。

```rust
// rust/src/types/exact/algebraic.rs
const HASH_KEY_BITS: u64 = 64;
fn floor_scaled(f: &Fraction, target_bits: u64) -> BigInt { ... }
impl Hash for Algebraic {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut bits = HASH_KEY_BITS + 8;
        loop {
            let (lo, hi) = self.bounds(bits);
            let (lo_key, hi_key) = (floor_scaled(&lo, HASH_KEY_BITS), floor_scaled(&hi, HASH_KEY_BITS));
            if lo_key == hi_key { lo_key.hash(state); return; }
            bits *= 2;
        }
    }
}
```

因数分解を一切行わない。ハッシュの衝突（異なる値が同じキーになること）は許容される
（`HashMap` 内で `==` によるフォールバック比較が起きるだけ）。許容されないのは
「等しい値が異なるキーになること」であり、それをこの手法が防ぐ。この区間精緻化は
実測で 1 回の `cmp` と同程度に高くつく（§3.1）。

### 2.4 `Computable`（Tier 2）

`Computable::eq` はジェネレータ関数の `Arc` ポインタ同一性（`Arc::ptr_eq`）で決まる
（値の極限としての等価性は決定不能なため）。`Hash` もポインタの生アドレス
（`Arc::as_ptr` のメタデータを落とした部分。`ptr_eq` 自身がメタデータを無視すると
文書化されているのに合わせた）で一致させた。現在どの Word もこの階層を構築しないため
到達不能だが、`ExactReal: Hash` を全域にするために必要。

## 3. 課金モデルの作り直しと実装コスト

### 3.1 課金モデル: 何が変わったか

旧モデル（`collection_meter.rs` の `ScanMeter::charge_scan_of`）は要素ごとに
「これまでの相異なり数 `candidates`」に比例した額を課金していた
（`probe_units × candidates`）。これは実装が実際に行う線形走査の上界だったので、
非二次化前は正確な値付けだった。

新モデルは要素ごとに固定額を課金する: `probe_units`（その要素をハッシュする費用。
既存の `ElementCost::probe()` をそのまま転用 — 代数的要素の場合は §2.3 の区間精緻化の
費用を、`ALGEBRAIC_ELEMENT_UNITS` という既存の較正値で近似する。1 回の `cmp` と
同程度の作業だからである）に加え、`COLLECTION_HASH_UNITS`（新設、値は 16 =
`COLLECTION_COPY_UNITS` と同じ）を足す。新しい値を保持する場合は従来通り
`copy_units` を追加で課金する。

```rust
// charge_scan_of: 旧 probe_units * candidates → 新 probe_units + COLLECTION_HASH_UNITS
```

`COLLECTION_HASH_UNITS` を追加した理由（`HashMap` の再ハッシュ・キャッシュ局所性を
`probe_units` だけでは捕捉できないという実測）、および `COLLECTION_COPY_UNITS` /
`ALGEBRAIC_ELEMENT_UNITS` / 予算値そのものを変えなかった理由は §3.3 に記す。

### 3.2 実装コスト: `Hash` 自体が高くついた

課金モデルとは別に、実装した `HashMap<Value, _>` ベースの走査そのものが当初
遅すぎた。原因は二つ、どちらも「declared cost（課金額）は正しいが、その額に見合わない
実行時コストを払っていた」という形の不具合で、修正した:

1. **`Fraction::hash` が小さい値にも `BigInt` 変換を強制していた。** `Small(i64, i64)`
   表現の値でも `to_bigint_pair()` を経由して `BigInt::gcd` を呼んでいたため、
   毎回ヒープ確保が発生していた。`compute_gcd_i64`（同ファイル既存）で `i64` のまま
   約分してからハッシュする高速経路を追加し、`Big` 表現側は「約分した結果が `i64` に
   収まるなら同じ経路に合流する」ことでどちらの表現から来ても同じ値は同じバイト列を
   ハッシュに渡すようにした（表現ではなく、約分後の値で経路が決まる）。
2. **`HashMap::get` の後に `HashMap::insert` を呼び、新規要素ごとにハッシュを 2 回
   計算していた。** `Entry` API（`entry().or_insert_with` 相当）に書き換えて 1 回に
   した。同時に、`UNIQUE`/`TALLY` の走査中は `items` を借用したキー（`&Value`）だけを
   使い、実際に生き残った要素だけを最後に 1 回だけ `clone()` するように変えた
   （走査中に要素を複製する必要がなくなった）。

効果（このコンテナ、debug ビルド、`[ 0 99999 ] RANGE UNIQUE LENGTH`、`ajisai agent
compute`）:

| | 実測時間 | 換算 units/ms |
|---|---:|---:|
| 修正前 | 1.485 s | 3,434 |
| 修正後 | 0.274 s | **18,613** |

5.4 倍速くなった。release ビルドでも改善は残る（`n=2,000` の `UNIQUE` が 2.1ms →
0.5ms、`examples/collection_word_calibration` §1 参照）。この修正が要った理由は
§3.4 に書く: debug ビルドでの実行速度が golden テストの前提条件そのものだったから
である。

### 3.3 `COLLECTION_HASH_UNITS` の導出、変えなかったもの

`materializedElements` の実際の上限（playground: 1,000,000）まで `[ 0 999999 ]
RANGE UNIQUE`（全相異）を走らせ、実際に課金された単位と実測ミリ秒から units/ms を
計算すると（§3.2 の修正**前**、`probe_units` のみでの計測）:

| n | ms | 課金単位（`probe_units` のみ） | units/ms |
|---:|---:|---:|---:|
| 200,000 | 528 | 3,600,000 | 7,644 |
| 500,000 | 1,604 | 9,000,000 | 5,610 |
| 1,000,000 | 7,905 | 18,000,000 | **2,277** |

`examples/work_meter_calibration` がこのコンテナで測った、上限のない算術経路の下限
（`dense tensor lanes`）は **2,814 units/ms**。1,000,000 要素での 2,277 はこれを
**下回る** — 危険な向きである。`COLLECTION_HASH_UNITS = 16` を毎要素（保持するか
どうかに関係なく）追加したところ 4,564 units/ms まで上がり、下限を上回った
（`COLLECTION_COPY_UNITS` と同じ値を再利用したのは「`HashMap` への挿入もキーの
コピーである」という `collection-word-billing-2026-08-13.md` §4 と同じ理由による）。

§3.2 の実行時コスト修正後、同じ計測をやり直すと 1,000,000 要素で **16,087 units/ms**
まで上がった（`examples/big_probe`、非コミットの一時測定スクリプト、再現は
`RuntimeLimits` を無制限にして `[ 0 999999 ] RANGE` の後に `UNIQUE` を計測すれば
よい）。`COLLECTION_HASH_UNITS` はこの時点で数値上は不要なほどの余裕があるが、
「`HashMap` の挿入もコピーである」という導出そのものは実行時コストの実装品質に
依存しない主張なので、外さずに残した。

`COLLECTION_COPY_UNITS`（16）、`ALGEBRAIC_ELEMENT_UNITS`（512）、
`DEFAULT_MAX_COLLECTION_WORK` / `LOCAL_AGENT_RUNTIME_LIMITS.max_collection_work`
（それぞれ 20,000,000 / 2,000,000,000、既存の値）は変えていない。理由: これらは
「1 要素の複製」「1 回の代数的比較」という**原始操作**の値段であり、その原始操作
自体は今回変更していない（変わったのは、UNIQUE/TALLY/GROUP が何回その原始操作を
呼ぶかという走査戦略と、その呼び出し自体の実装コストだけ）。下限チェックが通った
以上、既存の予算値を動かす実測上の根拠はない。

この判断は**このコンテナ限定**であることを明記する。§6 に測っていないことを残す。

### 3.4 なぜ実装コストが golden テストを壊しかけたか

`tools/mcp-server` の golden / eval スイート（`npm run test:mcp` が CI で叩く経路）は
**最適化なしの `cargo build`** で作った `ajisai` バイナリに対して走る
(`package.json` の `test:mcp` スクリプトに `--release` が無い)。`wallTimeMs` の
ホストゲートは 5,000 ms で、これは release ビルドでの実測（§3.1 が期待する
0.7 秒規模）を前提にした値である。

非二次化前の境界ケース（`[ 0 6291 ] RANGE UNIQUE LENGTH`、小さい整数の線形走査）が
debug ビルドでも 0.615 秒で済んでいたのは、比較 1 回が「機械語 1 語を比較するだけ」の
安い操作だったからである。非二次化後、境界に届かせるために代数的要素や幅広い
BigInt を使う証人プログラムを最初に選んだところ、debug ビルドで 16〜18 秒かかり
`wallTimeMs` を超えて `timeout` になった。§3.2 の実行時コスト修正後も、代数的要素の
`Algebraic::hash`（区間精緻化）自体は本質的に重い操作であり、これは変えていない。

解決したのは証人の選び方である。§5 に書くとおり、同じ全相異ベクタに対して
`UNIQUE` を複数回連続で適用する構成に切り替えた: 1 回あたりの操作は§3.2の
高速化後の「安い小整数比較」のままで、呼び出し回数で予算に届かせる。

## 4. 実測（このコンテナ、release、`examples/collection_word_calibration`、§3.2 の
修正後）

非二次化後の全文出力は `cargo run --release --example collection_word_calibration` で
再現できる。要点:

- **`UNIQUE` の距離依存性がほぼ消えた**（n=16,000）。旧実装は `d=1` で 0.52ms、
  `d=16,000`（全相異）で 682ms（1,300 倍）だった。新実装は §3.2 の実行時コスト
  修正後、小さい `n` では 1ms 未満で収まり、距離への依存はもはや支配的でない。
- **`[ 0 99999 ] RANGE UNIQUE` は 45.5 秒（無課金時代）→ 164ms（課金はあるが二次の
  まま、`collectionWork` に拒否される）→ 完走するようになった（非二次化後）。**
- このコンテナは `docs/dev/collection-word-billing-2026-08-13.md` の「参照コンテナ」
  より明確に遅い（例: `work_meter_calibration` の下限が 14,465 units/ms → 2,814
  units/ms）。絶対値の直接比較はできないため、§3.3 のとおり本書の判断は
  このコンテナ自身の内部整合性チェックに基づく。

## 5. 波及した修正

非二次化前の証人プログラムは「小さい `n` で `d` を故意に大きくする」ことで安く
上限に届かせる設計だったため、非二次化後は届かなくなった。以下を新しい証人・境界値に
差し替えた。証人の選定は §3.4 のとおり二段階だった: まず代数的要素で届かせたものが
debug ビルドで `wallTimeMs` を超え、最終的に「安い小整数走査を複数回繰り返す」形に
落ち着いた。

- `rust/src/agent/profile_liveness_tests.rs`
  `the_collection_ceiling_is_reachable_inside_the_materialization_one`:
  `[ 0 99999 ] RANGE UNIQUE` →
  `[ 0 99998 ] RANGE UNIQUE UNIQUE UNIQUE UNIQUE UNIQUE UNIQUE`
  （99,999 個の全相異な小整数に対し `UNIQUE` を 6 回連続適用。1 回目は
  `materializedElements` の内側で構築し、2 回目以降は同じ全相異ベクタを
  再走査するだけなので毎回ほぼ同額を課金し、6 回目の途中で 20,000,000 を
  超える）。
- `rust/src/agent/resource_usage_tests.rs`
  `collection_work_is_reported_and_tracks_the_data_not_the_length`: 「16,000
  distinct は拒否される」という旧来の主張を「distinct は uniform よりわずかに
  高いが 3 倍未満」という新しい主張に置き換えた。
- `rust/src/interpreter/collection_meter_tests.rs`: 4 件（距離依存性の逆転、
  代数的要素の比率、二つの拒否テストの入力サイズ／予算）。すべて実測で確認した
  実数値に基づく。
- `tools/mcp-server/golden/limits.json` の `collectionWork` 境界:
  `[ 0 6290 ]` / `[ 0 6291 ] RANGE UNIQUE LENGTH` →
  `[ 0 99998 ] RANGE UNIQUE UNIQUE UNIQUE UNIQUE UNIQUE LENGTH`（5 回、成功）/
  同じ操作を 6 回（拒否）。実測: under は 18,699,813 / 20,000,000、over は
  6 回目のパスの 38,240 要素目で 20,000,007 / 20,000,000、
  `progress.total = 99999`。debug ビルドでそれぞれ 1.26 秒・1.46 秒
  （`wallTimeMs` 5,000ms の内側）。
- `tools/mcp-server/eval/cases.json`・`repair-cases.json` の
  `ceiling-collection-work` / `repair-collection-ceiling`: 同じ理由で
  ソースを差し替え、`reference-traces.json` は
  `npm run eval:reference-traces` で再生成、`reference-repair-traces.json`
  は対応する `firstAttempt`/`repairedAttempt`（6 回 / 5 回）を手で合わせた。
- `rust/src/types/fraction.rs`・`rust/src/interpreter/ordering_ops.rs`:
  §3.2 の実行時コスト修正そのもの。

検証: `node validate-evaluation.js` / `capture-traces.test.js` /
`capture-repairs.test.js` / `selftest.js` / `backend/parity-test.js`
（ビルドしたネイティブ `ajisai` バイナリと再ビルドした WASM バンドルの両方に対して）/
`npm run test:mcp`（CI と同じ debug ビルド）/ `cargo test --lib` /
`cargo test --tests` / `cargo clippy --all-targets` / `cargo fmt --check` /
`wasm-pack test --node`（`rust/wasm-tests`）ですべて確認済み。

## 6. 測っていないこと・残った作業

- **このコンテナの絶対値を他のホストに一般化していない。** §3.3 の下限チェックは
  このコンテナに閉じている。実際にデプロイされる環境で
  `examples/work_meter_calibration` と `examples/collection_word_calibration`
  を再実行し、`COLLECTION_HASH_UNITS` の下限チェックをやり直すことが望ましい
  （`collection-word-billing-2026-08-13.md` 自身が「再測定してから信頼する」
  原則を掲げている）。
- **代数的要素・幅広い BigInt 要素の debug ビルドでの実行時コストは、この作業では
  下げていない。** §3.2 で直したのは `Fraction`（有理数）のハッシュだけである。
  `Algebraic::hash` の区間精緻化や `Big` 表現の `Fraction` の比較コストが
  debug ビルドで重いこと自体は変わっていない。golden の証人を「安い小整数の
  繰り返し」に変えることで問題を避けたが、代数的要素を主役にした境界ケースが
  将来必要になった場合、同じ壁に当たる。
- **`host-profile-derivation-handoff.md` の作業本体は未着手。** 本書はその
  §1 着手条件を満たすためだけの作業であり、プロファイル導出の統一（同文書 §4）
  には着手していない。次のセッションは §5 の「非二次化が動かすもの」を
  本書で置き換えた上で、§4 のタスク 1〜3 に進むこと。
