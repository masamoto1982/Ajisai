# ホストプロファイル導出統一 — 実施記録

作成日: 2026-08-14
対象: [`host-profile-derivation-handoff.md`](./host-profile-derivation-handoff.md) §4 の作業項目 1〜3
関連: [`mcp-host-profiles.md`](./mcp-host-profiles.md) /
[`collection-word-billing-2026-08-13.md`](./collection-word-billing-2026-08-13.md) /
[`collection-word-dequadraticization-2026-08-14.md`](./collection-word-dequadraticization-2026-08-14.md)

## 0. この文書の位置づけ

非正典。資源上限は `SPECIFICATION.html` §2.5 によりホストの安全制御であって
言語の意味論ではない。本書はその制御値の決め方についての作業記録であり、
意味論には触れない。

引き継ぎ書 §4 の作業項目 1〜3 に対する実施結果。§1 の着手条件（走査系の
非二次化）は `collection-word-dequadraticization-2026-08-14.md` により満たされて
いたので、本書はその続きとして書く。

## 1. 決めたこと（作業項目 1）

**唯一のパラメータ**: `DEFAULT_HOST_TIME_BUDGET_MS = 30_000`
（`rust/src/interpreter/runtime_limits.rs`）。

これは実測値ではなく**明示的な判断**である。playground 利用者が「計算が
固まった」と判断するまでの時間を実測したデータはどこにも無い。判断の根拠:

- playground の実行は Worker 上で行われ（`src/workers/execution-worker-manager.ts`）、
  明示的な中断ボタンがある（`ExecutionController.abortExecution`）。したがって
  「タブが固まって見える」という Nielsen/Miller の古典的な約 10 秒の閾値
  （フィードバック無しの UI に対する注意持続限界）はそのまま当てはまらない —
  利用者はすでに「実行中」と分かっており、いつでも止められる。
- 10 秒閾値の数倍を採ることで、「教える場」としての playground が意図的に
  大きな計算を試す余地を残す（旧 §2 で退けた「MCP に揃える」案と同じ理由）。
  同時に、誰も到達しない数ではない実際の上限であることも保つ。
- 実データが手に入れば置き換えること。今回は「体感で決めた」と明記する
  （§6 の要請どおり）。

この T から、`numericWork` と `collectionWork` を「T ミリ秒 × そのメーターの
このコンテナでの底値レート（units/ms）」として導出した。底値レートは
`examples/work_meter_calibration.rs` / `examples/collection_word_calibration.rs`
に新設した節で実測し直した（後述 §2）。この 2 つの上限は
`docs/dev/collection-word-billing-2026-08-13.md` §6 が MCP 側で確立した手法
（「メーターの最も遅い経路の units/ms × 目標時間」）をそのまま playground 側に
適用したものである。

`executionSteps` も同じ式で導出した（旧 100,000 → 新 23,190,000）。ただし
これは「上限の外にある」ものだったので §3 でまとめて扱う。

## 2. 実測（このコンテナ、release、rustc 1.94.1、2026-08-14）

`examples/work_meter_calibration.rs` に「executionSteps floor」節、
`examples/collection_word_calibration.rs` に「§6 collectionWork floor」節を
それぞれ新設し、コミットした（使い捨てにしなかった理由: 底値レートは
コンテナ依存であり、次回のデプロイ先で再実測する手順そのものが本書の主張の
一部だから）。

このコンテナはノイズが大きい（共有・仮想化環境）。複数回実行し、**最小値**
（安全な向き — レートが低いほど導出される予算が小さくなる）を採用した。

| メーター | 経路 | 観測レンジ (units/ms または steps/ms) | 採用した底値 |
|---|---|---:|---:|
| `numericWork` | dense tensor lanes (100k) | 4,660 – 6,197 | **4,660** |
| `collectionWork` | REVERSE 100k | 30,176 – 50,915 | **30,176** |
| `executionSteps` | DOWN トランポリン（ユーザー語呼び出し） | 773 – 890 | **773** |

`executionSteps` の底値だけ経路の性質が違う: 他の 2 つはオペランドの
**大きさ**を縛るので「最も遅い経路」を探すが、`executionSteps` は語の
ディスパッチ**回数**を縛るので、内部コストを持たない最も安い語の連続実行が
唯一の候補になる。フラットな `7 +` ループ（1,470–1,838 steps/ms）と、
トランポリン再帰（`cli::step_limit_tests::DOWN_PROBE` と同じ構成、
773–890 steps/ms）の両方を測り、遅い方（トランポリン）を採用した —
辞書引き・フレーム積みのぶん、算術だけのループより実際に高くつくため。

## 3. 導出された値と、導出されなかった値

| 上限 | 旧値（playground） | 新値（playground） | 導出 |
|---|---:|---:|---|
| `numericWork` | 1,000,000,000 | **139,800,000** | 30,000 × 4,660 |
| `collectionWork` | 2,000,000,000 | **905,280,000** | 30,000 × 30,176 |
| `executionSteps` | 100,000 | **23,190,000** | 30,000 × 773 |
| `sourceBytes` | 67,108,864 (64 MiB) | **16,777,216 (16 MiB)** | 判断（§4） |
| `materializedElements` | 1,000,000 | 1,000,000（不変） | 判断せず、生存性のみ再確認 |
| `bigintBits` | 1,000,000 | 1,000,000（不変） | 同上 |
| `algebraicTerms` | 100,000 | **10,000** | 生存性のため引き下げ（§5） |
| `numericLiteralDigits` | 4,096 | 4,096（不変） | 対象外 |

`numericWork` と `collectionWork` が**小さくなった**ことは意図した後退では
ない。旧値は `docs/dev/host-profile-derivation-handoff.md` §3.2 が
「後付けの解釈であって導出された値ではない」と明記していた数値であり、
このコンテナの底値レート（前回の参照コンテナより明確に遅い —
`collection-word-dequadraticization-2026-08-14.md` §4 が既に指摘していた
傾向と同じ）で正直に計算し直すと、この規模になった。速いコンテナに
デプロイする場合は同じ式がもっと大きな値を出す。**手で引き上げるのでは
なく、再実測すること。**

`DEFAULT_MAX_COLLECTION_WORK = 2 * DEFAULT_MAX_NUMERIC_WORK` という
ハードコードされた関係も削除した。この「2」自体が「同じ時間を買う」ための
係数であり、`collection-word-dequadraticization-2026-08-14.md` §5 が
「動く可能性がある」と予告したとおり、走査系の非二次化で実際に動いた
（旧比 ~2.1 → 新比 ~6.5、このコンテナでの測定）。今は各上限が自分の底値
レートから独立に導出され、時間予算だけを共有する。

## 4. 導出しなかった 3 つ（作業項目 3）

引き継ぎ書 §4 の項目 3 は「全部を一つの倍率に押し込むのは導出ではない」と
明記していた。実際に確認すると:

- **`materializedElements` / `bigintBits`** — 1 つの値の大きさを縛るもので、
  時間予算とは無関係。「画面に出して意味のある大きさ」のような基準が
  本来必要だが、誰も測っていない（引き継ぎ書 §6 が正直に書いていた
  とおり）。今回もその基準は書けなかった。代わりに、より弱い性質
  ——`numericWork`/`collectionWork` の新しい（小さくなった）予算の内側で
  まだ到達可能か——だけを再確認した。両方とも到達可能だった
  （`bigintBits` は 87% の消費で到達、`materializedElements` は
  一回の線形走査が新しい `collectionWork` 予算の 2% 未満）。
- **`sourceBytes`** — 判断は下したが、実測はしていない。64 MiB という
  「大きな丸い数」を、根拠を言える数——実在する最大の正当なプログラム
  （perf-benchmark の ~1.77 MB チェーン）の約 9 倍——に置き換えた。
  人間がテキストエリアに実際に貼る量の実測ではない。
- **`algebraicTerms`** — 唯一、値を動かした。理由は「画面legibility」の
  ような新しい基準を見つけたからではなく、`numericWork` を縮めた結果、
  旧値（100,000）が MCP 側の `algebraicTerms` 4,096 が壊れていたのと
  **全く同じ壊れ方**をしていたから（§5）。10,000 も基準から導出した値
  ではなく、MCP 側の 512 と同じ選び方——生存性に十分な余裕を残す——を
  踏襲しただけである。

## 5. `algebraicTerms` を下げた理由（生存性）

`numericWork` を 1,000,000,000 → 139,800,000 に縮めた結果、旧
`max_algebraic_terms = 100,000` は届かなくなった: 100,000 項を最初に超える
倍化（131,072 項、factor=17）は 520,124,416 units 課金する——新しい予算の
約 3.7 倍。`numericWork` が必ず先に答えてしまい、`algebraicTerms` という
名前で拒否されることは二度と無い。これは MCP 側の `algebraicTerms` が
4,096 だったときに壊れていたのと同じ形の不具合であり、
`profile_liveness_tests`（§6）が両方の形で検出する。

10,000 に下げた: 10,000 を最初に超える倍化（16,384 項、factor=14）は
50,356,224 units——新しい 139,800,000 の約 36%。MCP 側の 512
（10,000,000 の約 21%）と同程度の余裕を残す選び方である。

`bigintBits`（不変、1,000,000）は偶然にも生存していた: 1,000,000 ビットを
最初に超える連続乗算（74 回）は 122,321,640 units——新しい予算の約 87.5%。
これは MCP 側の `bigintBits` が既に持っている余裕（86%）とほぼ同じで、
「N-limb の整数は概ね N²/4 の limb 演算を要するので、この対がいつも
一番近い」という `profile_liveness_tests` 内のコメントと整合する。

## 6. `profile_liveness_tests` の両プロファイル化（作業項目 2）

`rust/src/agent/profile_liveness_tests.rs` の `refused_by` を
`Profile`（`limits` + `step_limit` + 各証人プログラムのサイズ）を取る形に
変え、既存の 6 件の主張すべてを MCP プロファイルと playground プロファイルの
両方に対して実行するようにした。証人プログラムのサイズは両プロファイルで
別々（§5 のとおり上限そのものが違うので、当然サイズも違う）:

| 証人 | MCP | playground |
|---|---:|---:|
| algebraic cascade（超過） | factor=10（1,024 項） | factor=14（16,384 項） |
| algebraic cascade（内側） | factor=9（512 項） | factor=13（8,192 項） |
| widening chain（超過） | 20 回 | 74 回 |
| widening chain（内側） | 19 回 | 73 回 |
| 累積 `numericWork`（cascade 8 の反復） | 19 回 | 261 回 |
| 累積 `collectionWork`（UNIQUE の反復、要素数） | 99,999 要素 × 6 回 | 999,998 要素 × 27 回 |

`the_collection_ceiling_is_reachable_inside_the_materialization_one` の
playground 版は 999,998 要素の `UNIQUE` を 27 回連続適用する。理由は MCP 版
と同じ（`collection-word-dequadraticization-2026-08-14.md` §5 が既に書いた
とおり）: 全相異ベクタへの反復 `UNIQUE` は値に対しては no-op なので毎回
ほぼ同額を課金し、安い小整数比較のままで予算に届く。代数的要素を主役にした
証人は debug ビルドで極端に遅くなる罠が既知（同ドキュメント §3.4）なので
避けた。

**副作用**: このテストモジュールの実行時間が 2.7 秒 → 約 53 秒に伸びた
（`cargo test --lib` 全体では 2.7 秒 → 約 51–57 秒）。playground の予算が
MCP よりずっと大きいので、その予算を実際に使い切るには実際にもっと多くの
作業をする必要がある——これは検証の正直な代償であり、証人を小さくして
誤魔化すことはしていない。CI が許容する範囲だが、次にこのテストを触る人は
把握しておくこと。

## 7. `executionSteps` を上限の外から持ち込んだこと（作業項目 3、続き）

引き継ぎ書 §4 の項目 3 が指摘していた「唯一 1x」の原因は
`DEFAULT_MAX_EXECUTION_STEPS`（`rust/src/interpreter/interpreter_core.rs`）
が両ホストで共用されていたこと。実際には MCP 側は
`tools/mcp-server/index.js` の `LIMITS.executionSteps`（100,000）を
`--step-limit` / `stepLimit` として**毎回明示的に**渡しており
（`backend/native-cli.js` / `backend/wasm-worker.js`）、この既定値には
依存していなかった。したがってこの既定値を動かしても MCP には触れない
——実際に `test:mcp-backends` で確認済み。

`DEFAULT_MAX_EXECUTION_STEPS` を `runtime_limits.rs` に移し
（`interpreter_core.rs` は再エクスポートするだけ）、他の 2 つの work
上限と同じ式で導出した。ただし 1 つ実装上の発見があった:
**大きな `executionSteps` は「実際に実行して使い切ることで証明する」
テストが書けなくなる。** 他の 2 つのメーターは 1 回の広いオペランドで
予算を使い切れるが、`executionSteps` は語のディスパッチ回数を数えるので、
2300 万ステップを実際に使い切るには—遅い debug ビルドでは特に—本物の
壁時計時間がかかる。

`rust/src/cli/step_limit_tests.rs` の
`down_probe_exceeds_default_budget_without_step_limit` はこの理由で壊れた
（旧デフォルト 100,000 を実行で超えることを前提にしていた）。対処:

- 高速な構造チェック（`Interpreter::new().max_execution_steps() ==
  DEFAULT_MAX_EXECUTION_STEPS`）を新設して通常の `cargo test` で回す。
- 実行で本当に超えることを証明するテストは残したが `#[ignore]` にした
  （release で約 28 秒、debug ではもっとかかる）。`cargo test -- --ignored`
  で明示的に回す。
- `agent::resource_usage_tests` の 1 件が同じ理由で壊れていた
  （`step_limit: None` で MCP プロファイルを検証していたのが、既定値の
  変化で「MCP プロファイル」を表さなくなった）。MCP の実際の呼び出し方に
  合わせて `step_limit: Some(100_000)` を明示するよう修正。

## 8. 検証

- `cargo test --lib` / `cargo test --tests`: 全件通過（928 lib + 35
  integration、変更前と同数 + `profile_liveness_tests` の新規アサーション）。
- `cargo clippy --all-targets`: 警告無し。
- `cargo fmt --check`: 整形差分無し。
- `cargo test --lib -- --ignored down_probe_exceeds_the_real_default...`:
  release で 28 秒、通過。
- `scripts/rebuild-wasm.sh` / `scripts/rebuild-mcp-wasm.sh`: 両方再ビルド
  （`wasm-pack` は本セッションでインストール）。`AjisaiInterpreter::
  host_profile()` を Node から直接呼び、新しい値がバイナリに実際に
  反映されていることを確認した:
  `{"algebraicTerms":10000,"bigintBits":1000000,"collectionWork":905280000,
  "executionSteps":23190000,"materializedElements":1000000,
  "numericLiteralDigits":4096,"numericWork":139800000,"sourceBytes":16777216}`
- `npm run test:mcp`（selftest.js、debug ビルド、CI と同じ経路）: 全件通過。
  MCP の宣言値（`LIMITS`）は本作業で一切変えていないので、期待どおり無風。
- `npm run test:mcp-backends`（native/WASM parity）: 全件通過。
- `npm run eval:mcp`: 79/79 通過。
- `npx vitest run`（フロントエンド）: 359/359 通過。

## 9. 測っていないこと・残った作業（§10 以前の時点）

引き継ぎ書 §6 の内容はほぼそのまま残る:

- **playground 利用者が待てる時間は依然として実測されていない。**
  30 秒は本書 §1 のとおり明示的な判断であり、実データが手に入り次第
  `DEFAULT_HOST_TIME_BUDGET_MS` を置き換えること。
- **`materializedElements` / `bigintBits` の playground 値に根拠が無い**
  ことは変わらない。生存性だけを再確認した（§4）。「画面に出して意味の
  ある大きさ」という基準を実際に測る作業は次のセッションへ持ち越し。
- **`ajisai run`（CLI）を独立したホストとして検討していない。** 今回も
  playground と同じ default プロファイルを使い続けている。この論点は
  未決のまま。
- **このセッションの底値レート測定はこのコンテナに閉じている。**
  実際にデプロイされる環境で `work_meter_calibration` /
  `collection_word_calibration` を再実行し、`DEFAULT_MAX_NUMERIC_WORK`
  等を再導出することが望ましい（同じ式、違う定数を入れるだけ）。

**§10 が §9 の最後の項目を訂正する。** 「同じ式、違う定数を入れるだけ」は
不正確だった——入れ替えるべきはコンテナではなくホスト（エンジン）そのもの
だった。

## 10. 訂正 — 較正ホストの誤りと課金の欠陥（第三者レビューによる発見）

本書merge後、独立したレビューセッションが §1〜9 の実施内容を精査し、
2 件の欠陥を発見した。両方とも本セッションの続きとして修正し、この節に
まとめる。関連 PR: #1515（較正ハーネスの区間バグ・恒真テスト・上書きした
記述の3件）、#1516（`Fraction::hash` の gcd 修正）。

### 10.1 較正したホストと、上限を適用するホストが違っていた

§2〜§7 の底値レート（`NUMERIC_WORK_FLOOR_RATE_UNITS_PER_MS` 等）はすべて
**native release**（`cargo run --release`）で測っていた。しかし
`RuntimeLimits::default()` を実際に消費するのは主に **WASM の playground**
である。同一コンテナで両方を測り直すと、native→WASM の比はメーターごとに
バラバラだった:

| メーター | native | WASM | 比 |
|---|---:|---:|---:|
| `numericWork` の floor 経路 | 7,374 u/ms | 15,345 u/ms | 2.08x 速い |
| `collectionWork` の floor 経路 | 44,015 u/ms | 175,036 u/ms | 3.98x 速い |
| `executionSteps` の floor 経路 | 879 steps/ms | 584 steps/ms | 0.66x 遅い |

「2倍速い」から「3割遅い」までメーターごとに向きも大きさも違うので、
native の 1 回の較正では 3 つの上限を WASM 上で同じ時間に揃えることは
できない。実際、§2〜§7 の値が playground（WASM）でどれだけの時間を買うか
測ると 9.1秒／5.2秒／39.7秒——狙った 30秒に対し最大 7.6 倍のばらつき
だった。「時間を揃える」という本書の目的が、一番大事な場所で成立して
いなかった。

さらに §2 の較正ハーネス自体にも欠陥があった:
`work_meter_calibration::measure` はオペランド構築（`[ 0 99999 ] RANGE`
による 10 万要素の materialize）を計時区間に含めたまま課金単位だけで
割っていたため、floor 経路（dense tensor lanes）のレートが実際より
低く出ていた（6,153 対、構築時間を除いた場合の 7,374 units/ms）。
`collection_word_calibration` は元から setup と計測対象を分離していたので、
**2 つの較正ハーネスが互いに矛盾しており、本書はその間違っている方から
数値を採っていた**。

対処: `measure_after_setup`（`rust/examples/work_meter_calibration.rs`）
を新設して区間を修正し、WASM 用の較正ハーネス
`scripts/wasm-profile-calibration.mjs` を新設した。native/WASM で二重に
較正するのではなく、**playground が実際に動くエンジン（WASM）だけを
`DEFAULT_MAX_*` の根拠にする**——native の `examples/work_meter_calibration`
等は `ajisai run` / native `ajisai agent` 自身の floor を知るための道具
として残すが、共有デフォルトの根拠としては使わない。

### 10.2 `Fraction::hash` の gcd が要素幅に対して超線形だった

WASM 較正の初回実行で `collectionWork` の floor が
`UNIQUE 4k × 4096桁` になり、他経路より**約 300〜470 倍**低いことが
判明した（WASM: 248 対 73,491 u/ms、native: 780 対 45,025 u/ms）。

原因は `num-bigint` の `BigInt::gcd`（`Integer::gcd`）がオペランドの
**幅の差**に対して二次的に遅いことだった:

| `gcd(4096桁, X)` | 時間 |
|---|---:|
| X = 1 | 612 µs |
| X = 7 | 356 µs |
| X = 4096桁 | 1.2 µs |

`Fraction::hash`・`Fraction::new`（正規化）はどちらも `gcd(numerator,
denominator)` を呼ぶ。整数値の Fraction は分母が 1 なので、**最も一般的な
ケースが最悪ケース**になっていた。これは走査系の課金の欠陥ではなく、
`Fraction` の基礎演算そのものの実装コストの欠陥である——非二次化
（`collection-word-dequadraticization-2026-08-14.md`）が `Small` 表現には
同種の修正を施していたが、`Big` 表現は取り残されていた。

修正: `gcd(a, b) == gcd(b, a mod b)` で 1 回除算してから既存の binary GCD
に渡す `balanced_bigint_gcd`（`rust/src/types/bigint_gcd.rs`）を新設し、
`Fraction`・`fraction_arithmetic`・Tier 1 代数的正規形（`exact::basis` /
`exact::algebraic` / `exact::algebraic_field`）の raw `gcd()` 呼び出し
**18 箇所すべて**を置き換えた。実測 494 倍高速化（612µs→1.2µs）、
`UNIQUE 4k×4096桁` は 780→106,051 u/ms（136 倍、native）。正しさは
`Integer::gcd` を oracle にした property test で検証し、`value_hash_tests`
の Hash/Eq 契約（`bc14dc6`、#1513 由来）を含む既存テストは全て無変更で
通過した。

### 10.3 再導出した値

10.1・10.2 の修正後、WASM で底値レートを測り直した（複数回実行の最小値、
「コンテナのノイズに対して安全な向き」という既存の方針を踏襲）:

| メーター | WASM 底値レート | 経路 |
|---|---:|---|
| `numericWork` | 10,373 units/ms | dense tensor lanes (100k) |
| `collectionWork` | 47,162 units/ms | UNIQUE 4k × 4096桁（もう「穴」ではなく、通常の意味での floor） |
| `executionSteps` | 406 steps/ms | DOWN トランポリン |

`DEFAULT_HOST_TIME_BUDGET_MS`（30秒）は変更していない——今回の訂正は
どのホストを測るかの誤りであって、時間予算の判断そのものは再検討して
いない。

| 上限 | §2〜§7 の値（native 較正、誤り） | 訂正後（WASM 較正） |
|---|---:|---:|
| `numericWork` | 139,800,000 | **311,190,000** |
| `collectionWork` | 905,280,000 | **1,414,860,000** |
| `executionSteps` | 23,190,000 | **12,180,000** |

`algebraicTerms`（10,000）・`bigintBits`（1,000,000）は値そのものは
変えていない——生存に必要な閾値は課金式から決まる固定値であり、
どのホストで底値レートを測ったかに依存しないため。ただし新しい
`numericWork` 予算のもとで余裕（マージン）を再確認した:
`bigintBits` の到達コストは新予算の約 39%（旧: 87%）、`algebraicTerms`
は約 16%（旧: 36%）——どちらも以前よりむしろ安全になった。
`algebraicTerms` を予算の余裕に合わせて 100,000 側へ戻すことはしていない
（`host_profile_defaults.rs` の同定数の doc comment に理由を書いた:
「予算が動くたびに値を追いかけるのは原則の無いchurnである」）。

`rust/src/agent/profile_liveness_tests.rs` の playground 側証人も
更新した: `cumulative_cascade_reps` 261→580、`collection_reps_to_cross`
27→42（`algebraic_terms_over/under`・`bigint_bits_over/under` は閾値が
固定費なので不変）。

### 10.4 検証（10.1〜10.3 適用後）

- `cargo test --lib` / `--tests`・clippy(`-D warnings`)・fmt・
  file-size budget: 全て通過。
- `cargo test --lib bigint_gcd`: property test（`Integer::gcd` を oracle
  に random pair 比較）・固定ケーステスト、両方通過。引数を入れ替える
  変異で両方落ちることを確認済み（牙があることの検証）。
- `npm run test:mcp` / `test:mcp-backends`（native/WASM parity）/
  `eval:mcp` / `npx vitest run`: 全て通過。
- WASM バンドル再ビルド済み。gcd 修正のみでは再ビルド後のバイナリに
  差分が出ない（既に反映済みの状態からの再ビルドだったため）ことを
  `git status` で確認、`host_profile_defaults.rs` の値変更後は差分あり
  （このコミットに含む）。

### 10.5 残った作業（更新）

§9 の 4 項目のうち、以下は本節で状況が変わった:

- **底値レートの計測ホストが container-specific なのは変わらないが、
  今後は native ではなく WASM を測ること。** 別のデプロイ環境・別の
  ブラウザエンジンでは `scripts/wasm-profile-calibration.mjs` を
  再実行すること。
- **`ajisai run`（CLI）を独立したホストとして検討していない、という
  論点は今回さらに具体的な根拠を得た。** native と WASM の底値レート比が
  メーターごとに 0.66x〜4.0x とバラバラだったという実測は、
  `ajisai run` が playground と同じデフォルトを共有し続けることの
  コストを定量化している——native CLI は今、自分自身のエンジンではなく
  WASM のために較正された上限の下で動いている。分離するかどうかの判断
  は本節でも下していない。

他の 3 項目（playground 利用者の待てる時間、materializedElements/
bigintBits の基準、いずれも未測定）は §9 のまま変わらない。
