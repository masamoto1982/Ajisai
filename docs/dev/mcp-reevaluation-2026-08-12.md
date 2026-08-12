# Ajisai MCP 再評価・改善提案書（改訂版）

改訂日: 2026-08-12
原案: 2026-08-12 付「Ajisai MCP 再評価・改善提案書」
対象: `944c5cb8ca7f94353acc8100d556f208d333e494`

> **進捗（2026-08-12 追記）**
>
> - **P0-1'（§3.4）実装済み**。課金とサイズ検査は
>   `rust/src/interpreter/arithmetic_meter.rs` に集約され、レーン数 × オペランド幅で
>   課金する。`[ 1 21000 ] RANGE [ 1 ] { * } FOLD` は scalar 版と同一の 10,000,573
>   単位で `numericWork` に拒否される。受け入れ条件 1〜5 達成。
> - **P0-3（§4）実装済み**。`rust/src/agent/error_stack.rs`。エラー報告のみ、
>   64 KiB 予算を超えるスロットの値を位置を保ったまま省略し、`elided` /
>   `stackElided` で明示する。上記の拒否は 5,773,682 bytes → **7,470 bytes**、
>   `diagnosis.resourceLimit.resource: "numericWork"` は無傷。受け入れ条件 1〜4 達成。
> - **条件 6（`golden/limits.json` の boundary 化）は依然未達だが、理由が変わった**。
>   §4.3 の想定では P0-3 で達成できるはずだったが、実測の結果それは誤りだった。
>   到達可能な source は存在する（`[ 0 99999 ] RANGE` + 101 回の加算 = 10,100,000
>   単位、応答約 7 KB）が、debug ビルドで 5.2 秒かかり `wallTimeMs` 5,000 に先に
>   当たる。レーン課金は 1 レーン 1 単位だが、boxed な要素演算の実時間は limb
>   乗算よりはるかに高く、この経路は約 7,700 単位/ms — scalar 連鎖で較正した
>   57,000〜86,000 単位/ms と 8 倍ずれている。逆に幅を広げる構成では
>   `bigintBits` が先に発火する（10,000,000 単位に乗算で到達するには構造上
>   4,096 limb 超のオペランドが要る）。**残っているのは課金の穴ではなく、
>   3 つの上限の順序づけ（較正）である。** §4.3 の該当記述は本追記で訂正する。
> - **較正問題に着手し、原因を実測で特定した**（`rust/examples/work_meter_calibration.rs`）。
>   前回「レーン課金が約 7,700 単位/ms」と書いたのは debug ビルドの数字を release の
>   較正値と比べた誤りで、真の原因は別だった。**有理数の加算が線形価格で課金されて
>   いた。** Ajisai の `+` は有理数加算 `(ad+cb)/(bd)` + gcd、すなわち乗算 3 回と
>   Euclid であって線形ではない。リテラルを 1 回だけ解析して測ると、4096 桁の加算は
>   **745 µs**（同幅の乗算は **366 µs**）かかるのに **213 単位**しか課金されず、
>   同幅の乗算は 45,369 単位だった。**4,500 倍の過小課金**で、しかも他に上限のない
>   経路。修正後は同経路が約 66,000 単位/ms。
> - **さらに、代数的値の支配的コストはどの上限も課金していないことが判明した。**
>   計算ではなく**表示**である。項数 2/4/8/16/32 で実行は一貫して 0.1 ms、
>   `stackDisplay`（連分数展開）は 5 ms / 57 ms / 951 ms / 9.1 s / **147 s** と
>   1 段あたり約 12 倍。`wallTimeMs` 5,000 は 16 項前後で尽きるので、宣言値
>   `algebraicTerms: 4096` は**到達可能点より 8 段上**にある。golden の wallTimeMs
>   over ケースが 14 秒かかるのも、0.1 ms で計算できた 16 項の値の描画にほぼ全部を
>   使っているからである。表示コストの課金または上限化が次の課題。
> - **表示展開の上限化を実施した。** `CF_OBSERVATION_WORK_BUDGET`。連分数展開を
>   項数ではなく**作業量**で予算化し、1 ステップを `terms³ × limbs` で実行前に課金する。
>   項数 2/4/8/16/32 の描画は 5 ms / 57 ms / 951 ms / 9.1 s / **147 s** から
>   4.7 / 18.9 / 4.9 / 5.8 / **0.0 ms** へ。`2 SQRT` は 32 項の完全表示を維持し、
>   高価な値は `...)`（既存の切り詰め記法）または `( ...)`（既存の未確定記法）になる。
>   `exactTerms` / `exactDisplay` は無傷 —— 代数的値にとって連分数は表示であって値ではない。
> - **これにより宣言済み上限の boundary 被覆が 7 → 9 に増えた。** 表示コストに
>   隠されていた 2 つが宣言値で到達可能になった。`numericWork`（12 因子 = 4,096 項が
>   約 8.4M 単位で成功、13 段目が 16,799,744 で名前つき拒否）と `bigintBits`
>   （4096 桁リテラル 19 回で 77,824 桁が成功、20 回目が 272,133 bit で拒否、約 16 ms）。
> - **残る 1 つ `algebraicTerms` の理由は 1 文になった。** 4,096 項を最初に超える
>   13 段目が 16.8M 単位かかり、`numericWork` の 10,000,000 が先に名乗る。
>   `max_numeric_work` を約 17M 超にすれば露出するが、これはメーターの問題ではなく
>   ホストプロファイルの方針判断なので記録に留めた。
> - **新規発見**: `wallTimeMs` の over ケースだった 4 因子の積は、描画が速くなった
>   結果 18 ms で成功するようになった。代替として採用した `UNIQUE` は **O(n²) かつ
>   どの上限も課金していない**（5k/10k/20k/40k で 97 ms / 388 ms / 1.86 s / 7.0 s、
>   材料化上限の 100,000 要素で約 48 秒）。work meter は算術を課金するが、
>   コレクション系 Word の実作業はどの上限も数えていない。これがこの系統の次の課題。
> - **P0-2 は未着手。**

## 0. この文書の位置づけ

原案を実機で検証したうえで改訂したもの。原案の記述を「実測で追認できたもの」「実測に
反するもの」「実測できなかったもの」に分けて示し、反するものは根拠つきで差し替える。

原案の価値判断 —— 何を守り、何を先に直し、何をまだ増やさないか —— はほぼ正しい。
維持すべき設計の一覧（原案 §4）と、npm 公開を実装バグではなくリリース判断として扱う
姿勢（原案 P1-3）は、そのまま引き継ぐ。

差し替えるのは一点、ただし最優先の一点である。

## 1. 改訂後の結論

**原案 P0-1 の原因診断は事実に反する。** 症状（`[ 1 21000 ] RANGE [ 1 ] { * } FOLD`
が宣言済み上限を通り抜けて 271,233 bit の値を返す）は再現する。しかし原因は
「高階 Word（`MAP` / `FOLD`）の block 内の算術が work meter に届かない」ことではない。

- `FOLD` の block 内の算術は課金される。初期値を `1`（scalar）にすると同じ源が
  `numericWork` で拒否される。
- User Word 経由・`EXEC` 経由でも、1 単位まで同一に課金される。**経路不変性は既に
  成立している。**
- 逆に、高階 Word を一切使わない直列 source でも、オペランドが Vector なら課金は
  ゼロになる。**「直列 source は課金される」という原案の前提も成り立たない。**

破れているのは実行経路に対する不変性ではなく、**値の形（scalar か vector/tensor か）
に対する不変性**である。原案の受け入れ条件（direct / User Word / EXEC / MAP / FOLD の
経路差分テスト）をそのまま実装すると、scalar 形で書けば修正前から全て PASS し、
vector 形で書けば一様に FAIL する。どちらでも測っているのは経路差ではないので、
**合格しても穴は塞がらない。**

**加えて、原案が触れていない P0 級の問題がある。** 資源上限で正しく拒否された結果が、
MCP 境界で `responseTooLarge` にすり替わる。エンジンが `numericWork of 10000573
exceeds the limit of 10000000` と言っているのに、エージェントに届くのは「応答が
大きすぎた」だけである。原案は P0-1 の done 条件に「`golden/limits.json` の
`numericWork` と `bigintBits` を実 source の boundary へ移す」を含めているが、
これを直さない限り達成できない。

判定は次のとおり改める。

| 判定対象 | 原案 | 改訂後 |
|---|---|---|
| ローカル stdio・WASM ベータ | 条件付き合格 | 条件付き合格（変更なし） |
| 意味論・診断境界 | 合格 | 合格（変更なし） |
| オンボーディング | 大幅改善 | 大幅改善（変更なし） |
| 資源上限の実効性 | 要改善 | 要改善（**原因が別。範囲は原案より広い**） |
| 資源上限の**観測可能性** | （項目なし） | **要改善（新規 P0）** |
| 実モデルでの有用性証明 | 未完了 | 未完了（ただし着手順を後ろへ） |
| npm による一般配布 | 未実施 | 未実施（変更なし） |

## 2. 検証環境と再現手順

| 項目 | 値 |
|---|---|
| コミット | `944c5cb` |
| Rust | 1.94.1 |
| Node | 22.22.2 |
| backend | native（`cargo build --release --bin ajisai`） |

観測はすべて次で取った。

```
echo '<source>' | ./rust/target/release/ajisai agent compute -
```

この CLI 経路は MCP と同じ `agent::api::compute` を通り、同じ
`LOCAL_AGENT_RUNTIME_LIMITS`（`numericWork` 10,000,000 / `bigintBits` 262,144 /
`sourceBytes` 65,536）が効く。効いていることは §3.1 の A ケースが
`numericWork of 10000573 exceeds the limit of 10000000` を返すことで確認済み。

**検証できなかったこと**（原案の該当記述を否定も追認もしない）:

- native / WASM parity。WASM を再ビルドしていない。
- MCP over stdio の実接続。`tools/mcp-server/node_modules` が未取得。ただし MCP の
  応答は `agent::api` の報告をそのまま運ぶので、§4 の envelope 実測はそのまま
  MCP に効く。
- npm registry の状態。

## 3. P0-1 の原因診断は誤り —— 実測による反証

### 3.1 反証実験

同じ乗算を五つの書き方で実行した。`fastpath` は `runtimeMetrics.scalarFastpathCount`。

| # | source | 結果 | fastpath |
|---|---|---|---|
| A | `[ 1 21000 ] RANGE 1 { * } FOLD` | **error**: `numericWork of 10000573 exceeds the limit of 10000000` | 10,678 |
| B | `[ 1 21000 ] RANGE [ 1 ] { * } FOLD` | ok: 81,649 桁 / 271,233 bit | 0 |
| C | `{ * } 'MULW' DEF [ 1 21000 ] RANGE 1 'MULW' FOLD` | **error**: A と同一の数値まで一致 | 10,678 |
| D | `[ 1 21000 ] RANGE 1 { { * } EXEC } FOLD` | **error**: A と同一の数値まで一致 | 10,678 |
| E | `[ 2 ] 2 * 2 * 2 * 2 *` | ok | 0 |

読み方:

- **A と B** は同じ `FOLD`、同じ block、同じ 21,000 回の乗算。違うのは初期値が
  `1` か `[ 1 ]` かだけ。A は課金され上限で拒否され、B は課金がゼロ。
  block の中か外かでは説明がつかない。
- **C と D** は「User Word 経由」「`EXEC` 経由」でも A と `10000573` まで完全一致
  することを示す。原案が新規に保証したがっている経路不変性は、既にある。
- **E** は高階 Word を一切含まない直列 source である。それでもオペランドが
  Vector なら課金はゼロ。原案の「source へ直列に書く経路は課金され」という前提は
  成り立たない。

B の値は 81,649 桁 = 271,233 bit で、宣言値 262,144 bit を確かに超える。原案の
症状の記述と数値は正確である。誤っているのは原因の帰属だけである。

### 3.2 コード上の根拠

エンジン全体で work meter に課金する地点は**二つしかない**。

- `rust/src/interpreter/arithmetic.rs:271` — `push_scalar_fastpath_result`
  （scalar 形の `+ - * /`）
- `rust/src/interpreter/arithmetic.rs:145` — `push_exact_real_schema_result`
  （scalar 形の代数的数）

サイズ検査も同じ二箇所（`:284` の `check_bigint_bits`、`:155` の
`check_algebraic_size`）にしかない。`grep -rn "charge_numeric_work\|check_bigint_bits\|
check_algebraic_size"` が返すのはこれで全部である。

`apply_exact_arithmetic_schema` のディスパッチ順と課金の有無:

| 順 | 経路 | 課金 | サイズ検査 |
|---|---|---|---|
| 1 | `push_scalar_fastpath_result` | **あり** | **あり** |
| 2 | `push_simd_schema_result` | なし | なし |
| 3 | `sparse_mul_candidate` | なし | なし |
| 4 | `push_exact_real_schema_result` | **あり** | **あり** |
| 5 | `push_exact_real_broadcast_result` | なし | なし |
| 6 | `apply_binary_arithmetic` → `apply_binary_broadcast_with_metrics` | なし | なし |

課金するのは 1 と 4 だけで、どちらも scalar 形にしか適用されない。

B が 1 を降りる理由も特定できる。`scalar_fast_operand` は `Scalar` と「要素数 1 の
Tensor / Vector」を受けるが、`same_scalar_fast_wrap` が両辺の wrap 一致を要求する。
`[ 1 ]` は `ScalarFastWrap::Tensor([1])`、`RANGE` の要素は `ScalarFastWrap::Scalar`
なので不一致となり、以降の未課金経路へ落ちる。

同じ穴がもう一つある。`add_values`（`SUM` の実体、`arithmetic.rs:661`）は
`metrics: None` で呼ばれ、そもそも `&mut Interpreter` を持たない。シグネチャ上
どのメーターにも到達できない。

5 は特に重い。ベクタに載った無理数（`ExactScalar`）の算術がここを通り、
`check_algebraic_size` を一度も踏まない。`algebraicTerms` の上限も形に依存して
無効化される。

### 3.3 真の不変条件

守るべきなのは経路の不変性ではなく、形の不変性である。

> 同じ算術は、オペランドが scalar でも、長さ 1 の Vector でも、長さ N の Tensor でも、
> 同じ order の work を課金され、同じ名前の resource limit に到達する。

経路不変性（direct / User Word / `EXEC` / `MAP` / `FOLD`）は、既に成立しているので
**新規保証ではなく退行防止**として書く。原案の意図はここへ移す。

### 3.4 差し替え案（P0-1'） — 実装済み

**実装後の実測**

| source | 課金 | 結果 |
|---|---|---|
| `2 3 *` | 1 | ok |
| `[ 2 ] 3 *` | 1 | ok（従来 0） |
| `[ 1 2 3 ] [ 4 5 6 ] *` | 3 | ok（従来 0） |
| `[ 0 999 ] RANGE 2 *` | 1,000 | ok（従来 0） |
| `[ 0 999 ] RANGE SUM` | 1,000 | ok（従来 0） |
| `2 SQRT 3 SQRT + 5 SQRT 7 SQRT + *` | 8,192 | ok（**従来と同一**） |
| `[ 1 20 ] RANGE 1 { * } FOLD` | 20 | ok |
| `[ 1 20 ] RANGE [ 1 ] { * } FOLD` | 20 | ok（**scalar 版と同一**） |
| `[ 1 21000 ] RANGE [ 1 ] { * } FOLD` | 10,000,573 | **error: numericWork**（従来 ok / 271,233 bit） |

scalar はレーン数 1 の退化ケースとして同じ式に載るので、既存の較正
（最重の通常ケース 8,192 単位）は 1 単位も動いていない。1,000 レーンのベクタ演算が
1,000 単位で、宣言値 10,000,000 の 4 桁下にある。

**設計**

- 課金とサイズ検査を、`apply_exact_arithmetic_schema` の**入口に一度だけ**置く。
  最適化経路ごとに散らすと、次に経路が増えたときに同じ穴が空く。現在の 2 箇所も
  そうやって増えた。
- ベクタ／テンソルは「レーン数 × レーン幅」で課金する。scalar はレーン数 1 の
  退化ケースとして同じ式に載るので、既存の課金量は変わらない。
- 結果側のサイズ検査も同じ入口で、全レーンの最大幅に対して行う。
- 単項（`NEG` `ABS` `SQRT` `FLOOR` …）と `SUM` / `add_values` も同じ入口を通す。
  `add_values` は現在のシグネチャではメーターに触れないので、interpreter を渡す形へ
  引き上げる必要がある。ここは機械的な変更では済まない。
- `push_exact_real_broadcast_result` は `check_algebraic_size` を全レーンに対して
  適用する。ここが `algebraicTerms` の唯一の抜け道である。

**受け入れ条件**

1. *形不変性テスト（新規保証）*: 同じ算術を scalar / 長さ 1 Vector / 長さ 1 Tensor /
   長さ N Vector で書き、いずれも `numeric_work_used > 0` であり、同じ低い注入上限で
   同じ `ResourceLimit` 名に到達する。無理数レーンを含むベクタも同じ表に入れる。
2. *症状の解消*: `[ 1 21000 ] RANGE [ 1 ] { * } FOLD` が `numericWork` または
   `bigintBits` で名前つきに拒否される（現在は ok + 271,233 bit）。
3. *経路差分テスト（退行防止）*: direct / User Word / `EXEC` / `MAP` / `FOLD` の
   課金量が同じ order に留まる。**このテストは修正前から通る**ことを、追加した
   コミットの本文に明記する。通ることが期待値であって、通ったから直ったのではない。
4. *回帰*: `[ 1 21000 ] RANGE 1 { * } FOLD` の課金量が修正前後で同じ order に留まる。
   通常ケースの p95 と ordinary-work 回帰（現在の最重ケースは 8,192 単位）を維持する。
5. *parity*: native / WASM で、値だけでなく上限結果の分類・`resource` 名・`limit`・
   `observed` が一致する。
6. `golden/limits.json` の `numericWork` / `bigintBits` を boundary へ移すのは、
   §4 が入ってからにする（下記）。

## 4. 新規 P0-3: 資源上限の診断が MCP 境界で消える

### 4.1 実測

§3.1 の A ケース —— つまり**上限が正しく発火したケース** —— の応答サイズ:

| 項目 | 値 |
|---|---|
| envelope（compact JSON） | 5,773,682 bytes |
| うち `stack` | 6,055,060 bytes（pretty） |
| `mcp.limits.responseBytes` | 1,048,576 bytes |

`backend/native-cli.js:96` と `backend/wasm-worker-entry.js:37` は envelope 全体の
UTF-8 バイト長で切る。したがってこの結果は、エージェントには

```
hostError / code: "responseTooLarge" / limit: "responseBytes" / retryable: false
```

として届く。`numericWork` という診断は届かない。原因は、エラー報告が失敗時スタック
（21,000 要素のベクタと巨大な部分積）をそのまま連載していることにある。

### 4.2 なぜ P0 か

- 資源上限は「境界と理由を監査できること」のためにある。理由が届かないなら、
  上限があること自体が観測できない。
- エージェントへの指示が逆になる。`responseTooLarge` は「出力を小さくしろ」であり、
  `numericWork` は「計算量を減らせ」である。前者を読んだモデルは `LENGTH` を
  取ろうとして、同じ計算をもう一度走らせる。
- 原案 P0-1 の done 条件が達成できない。`numericWork` を `injectedLimit` から
  `boundary` へ移すには、境界を跨ぐ実 source が**サーバ越しに**宣言どおり失敗する
  必要がある。現状ではエンジンが正しく失敗しても、サーバの答えは別の名前になる。

### 4.3 提案

- エラー報告のスタック連載に省略規約を入れる。要素数とバイト数の上限を定め、
  省略したことを `elided`（省略件数つき）として明示する。値を捨てるのではなく
  **捨てたと言う**。原案 §4 の「content と structuredContent の情報同値性を、任意の
  サイズ目標のために壊さない」は、成功結果についての原則である。失敗結果の
  部分状態はそれとは別で、現状は「全部載せた結果、診断ごと落ちる」になっている。
- `diagnosis` / `diagnosis.resourceLimit` / `aiDiagnostic` / `errorFlowTrace` は
  省略対象から外す。落とすなら値であって理由ではない。
- `responseTooLarge` へ落とす前に、診断のみの縮退 envelope を試す経路を持つ。
  縮退したことは `mcp` 側に記録する。

**受け入れ条件**

1. §3.1 の A ケースが MCP 越しに `status: "error"` /
   `diagnosis.resourceLimit.resource: "numericWork"` を返し、`responseBytes` に収まる。
2. 省略が起きたことが応答から判別でき、省略件数が読める。
3. 省略が起きない通常のエラー（未知 Word、ゼロ除算）の応答は 1 バイトも変わらない。
4. native / WASM で省略の閾値と結果が一致する。

## 5. P0-2 は妥当（追認）、ただし一部は現状では実装できない

### 5.1 追認

コード上の根拠を確認した。原案の指摘は正しい。

- `RuntimeMetrics.execution_steps`（`interpreter_core.rs:125`）へ**書き込む箇所が
  リポジトリ全体に存在しない**。読み出しは `wasm_runtime_metrics.rs:37` と
  `agent/report.rs:186` の 2 箇所だけ。
- 上限判定は別フィールド `Interpreter.execution_step_count`
  （`execute_builtin.rs:74, 170`）を使う。
- 実測でも §3.1 の全ケースが `executionSteps: 0`。21,000 回の `FOLD` でも 0。

source of truth を一本化するという提案に同意する。

### 5.2 補強と一点の差し戻し

原案が例示した `resourceUsage` は次の形である。

```json
{
  "resourceUsage": {
    "executionSteps": 20002,
    "numericWork": 123456,
    "peakBigintBits": 4096,
    "peakAlgebraicTerms": 2
  }
}
```

このうち `numericWork` は既に `Interpreter.numeric_work_used` として存在するので、
ほぼゼロコストで載る。**ここから始めるのが妥当。**

一方 `peakBigintBits` と `peakAlgebraicTerms` は、**現状の実装に観測点がない**。
`check_bigint_bits` / `check_algebraic_size` は都度検査するだけで、ピークをどこにも
保持しない。フィールドを増やす前に、観測点を作る作業が要る。作れないなら載せない —— 
原案自身が掲げた「測っていない性能を満点と報告しない」「未測定は未測定と書く」を、
ここにも適用する。ピーク観測を入れるなら、P0-1' で課金地点を入口に集約したあとに
同じ場所へ足すのが安い。

**受け入れ条件**（原案から変更した点に ★）

- 単純な算術、User Word、再帰、`MAP`、`FOLD` で `executionSteps > 0` になる。
- `executionSteps` の観測値が、実際に上限判定に使った値と一致する。
- 成功直前と limit 超過時の off-by-one をテストする。
- ★ `resourceUsage` の初版は `executionSteps` と `numericWork` の 2 キーとする。
  ピーク系は観測点が入ってから足す。
- `resourceUsage` の各キーが `mcp.limits` のキーと機械的に対応する。対応しないキーが
  片側にある状態を、テストで落とす。
- schema、native report、WASM 変換、backend parity、selftest を同時更新する。

## 6. その他の項目の再判定

| 原案 | 判定 | 変更点 |
|---|---|---|
| P1-1 実モデル / Claude Code E2E 評価 | **妥当。ただし着手順を後ろへ** | 現状では資源上限に当たるケースの応答が `responseTooLarge` になる（§4）。この状態で E2E baseline を取ると、修正後に取り直しになる。P0-3 の後に置く。指標の設計と日英 1:1 の要求はそのまま採用 |
| P1-2 quickstart 分割 | **数値は追認、診断は緩和** | `mcp-quickstart.md` 5,503 bytes / `assets/quickstart.md` 24,676 bytes は実測どおり。ただし「読み順が一度逆転する」は言い過ぎ。§2-2 は一般則、§4 は代数的数への特則で、矛盾ではなく前方参照の欠落。§2-2 に「代数的数は §4」の一文を足せば解消する。8 KB 目標での分割は、それとは独立にサイズだけを理由に判断してよい |
| P1-3 npm を release gate として扱う | **妥当** | 変更なし。runbook の内容もそのまま |
| P2-1 応答サイズの次の削減対象 | **妥当、ただし対象を差し替え** | §4 により、最大の削減対象は provenance でも `runtimeMetrics` でもなく**エラー時 envelope**（5.7 MB）であることが判明した。ここを先頭に置く。`runtimeMetrics` の整理は P0-2 の後、実モデル trace でコンテキストコストが確認できてから |
| §4 維持すべき設計 | **全て同意** | 一項目追加: **上限の発火は値の形に依存しない**。今回の穴はこの原則が明文化されていなかったために、経路の話として記録されていた |

## 7. 修正後の PR 分割

| PR | 内容 | 依存 |
|---|---|---|
| PR 1 | 形不変性（P0-1'）。失敗テストを先に置く。ただし §3.4-3 の経路差分テストは「修正前から通る退行防止」として明記する | — |
| PR 2 | エラー報告の省略規約と上限診断の到達性（P0-3） | — |
| PR 3 | 観測カウンターの source of truth と `resourceUsage` 初版（P0-2） | — |
| PR 4 | 実モデル / Claude Code E2E 評価（P1-1） | PR 1–3 |
| PR 5 | quickstart（P1-2） | PR 4（同条件 A/B のため） |
| Release | npm 公開 | owner 判断 |

PR 1〜3 は `golden/limits.json` の coverage 変更を通じて結合している。`numericWork` を
`injectedLimit` から `boundary` へ移せるのは 3 本とも入ったあとである。coverage 変更を
done 条件に含めるなら、3 本を 1 本にまとめてよい。**分けるなら、途中の PR で
`golden/limits.json` を boundary へ書き換えないこと。** 到達しない上限を到達済みと
記載しない、という原案の原則がそのまま当てはまる。

## 8. Claude Code へ渡す実行指示（差し替え版）

```
Ajisai main の最新 HEAD から新しい branch を作り、資源上限が「値の形」に依存して
無効化される問題を塞ぐ PR を作ってください。

先に事実確認をしてください。次の 5 つを実行し、観測を記録します。

  A  [ 1 21000 ] RANGE 1 { * } FOLD
  B  [ 1 21000 ] RANGE [ 1 ] { * } FOLD
  C  { * } 'MULW' DEF [ 1 21000 ] RANGE 1 'MULW' FOLD
  D  [ 1 21000 ] RANGE 1 { { * } EXEC } FOLD
  E  [ 2 ] 2 * 2 * 2 * 2 *

A・C・D は numericWork で拒否され、B・E は課金ゼロで成功するはずです。つまり
MAP / FOLD の block が課金を迂回しているのではなく、オペランドが Vector や Tensor に
なると課金経路そのものが選ばれていません。docs/dev/mcp-readiness.md と
tools/mcp-server/golden/limits.json が「block 内の算術はメーターに届かない」と
書いているのは誤りで、本 PR で訂正済みです。実装前にこの 5 ケースを自分で再現し、
観測が違ったらそこで止めて報告してください。

実装は、課金とサイズ検査を apply_exact_arithmetic_schema の入口に一度だけ集約し、
ベクタ／テンソルをレーン数 × レーン幅で課金する形にしてください。scalar はレーン数 1
の退化ケースとして同じ式に載せ、既存の課金量を変えないこと。単項演算、SUM /
add_values、無理数レーンを含むベクタ（push_exact_real_broadcast_result）も同じ
入口を通してください。add_values は現在 interpreter を持たないので、シグネチャの
変更が要ります。

失敗テストは「形」の軸で書いてください。同じ算術を scalar / 長さ1 Vector /
長さ1 Tensor / 長さN Vector / 無理数レーン入り Vector で表現し、いずれも
numeric_work_used がゼロでなく、同じ低い注入上限で同じ名前の resource limit へ
到達することを確認します。経路差分（direct / User Word / EXEC / MAP / FOLD）の
テストも残しますが、これは修正前から通る退行防止であり、新規保証ではありません。
コミット本文にそう書いてください。

受け入れ条件は B が numericWork または bigintBits で名前つきに拒否されることです。
現状 B は ok を返し 271,233 bit（宣言値 262,144 bit 超）の整数を返します。

同時に、MCP 結果の runtimeMetrics.executionSteps が実際の
Interpreter.execution_step_count と一致するよう source of truth を一本化してください。
RuntimeMetrics.execution_steps は現在どこからも書かれていません。安全関連の観測値を
型付き resourceUsage として分離する場合、初版は executionSteps と numericWork の
2 キーに留めてください。peakBigintBits / peakAlgebraicTerms は観測点が実装に存在
しないので、観測点を作らないまま載せないこと。

意味論、NIL / error / hostError の分類、exactTerms / exactDisplay、source-only 境界、
成功結果における content と structuredContent の情報同値性は変更しないでください。

golden/limits.json の numericWork と bigintBits を boundary へ移すのは、
エラー時 envelope の省略規約（別 PR）が入ってからにしてください。現在、上限が
正しく発火したケースの envelope は 5.7 MB あり、responseBytes 1 MiB を超えて
responseTooLarge に置き換わります。エンジン側だけ直しても、サーバ越しには
numericWork という診断が観測できません。到達していない上限を到達済みと記載しない
でください。

Rust 全テスト、native / WASM parity、MCP selftest、pack smoke、eval、performance、
schema、generated asset drift を通してください。
```

## 9. 原案の誤りはリポジトリ由来である

原案 P0-1 の誤診断は、原案の落ち度というより、リポジトリ側の記述をそのまま受けた
ものである。

- `docs/dev/mcp-readiness.md`:
  「an operation inside a `MAP` or `FOLD` block never reaches the meter」
- `tools/mcp-server/golden/limits.json` の `numericWork` / `bigintBits` note:
  「Arithmetic inside a block never reaches the meter」

どちらも実測に反する（§3.1 A / C / D）。本改訂と同じコミットで両方を訂正した。
外部レビューが同じ結論に至った以上、誤った記述はレビューを一往復まるごと誤らせる。
訂正を本文と同じ PR に含めるのはそのためである。
