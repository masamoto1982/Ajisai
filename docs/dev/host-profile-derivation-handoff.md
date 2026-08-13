# ホストプロファイル導出統一 引き継ぎ書

作成日: 2026-08-13
対象セッション: 走査系の非二次化が入った**後**の、独立した 1 セッション
関連: [`mcp-host-profiles.md`](./mcp-host-profiles.md) /
[`collection-word-billing-2026-08-13.md`](./collection-word-billing-2026-08-13.md)

## 0. この文書の位置づけ

非正典。資源上限は `SPECIFICATION.html` §2.5 により**ホストの安全制御であって
言語の意味論ではない**。本書はその制御値の決め方についての作業指示であり、
意味論には触れない。

**この文書は単独で読めるように書いてある。** 前セッションの文脈を引き継がずに
着手してよい。逆に、本書に書いていない判断を前セッションの記憶から補わないこと。

## 1. 着手条件（重要）

**走査系の非二次化が main に入り、コレクションメーターが再較正されるまで着手しない。**

理由は §5 に書く。要約すると、本作業が固定しようとしている導出の錨が、
非二次化によって動く定数だからである。先に着手すると、間もなく動く数値の上に
規則を建てることになる。

非二次化そのものは本書の対象外であり、そちらの引き継ぎ書はまだ無い。

## 2. 決まったこと

**値を統一するのではなく、導出を統一する。**

MCP プロファイルと playground プロファイルは今後も異なる値を持つ。変えるのは、
その差が何の関数なのかを一つに決め、書き出し、検査することである。

### 退けた案（再検討しないこと）

議論済みなので、次のセッションで蒸し返さない。

- **上に揃える（playground の値を MCP にも）** —— numericWork が 100 倍になると
  5 秒の壁時計が先に発火する状態に戻る。「何が高くついたか」を上限名で言えなくなり、
  2026-08-13 の作業がそのまま失われる。
- **下に揃える（MCP の値を playground にも）** —— playground が守っている相手は
  「自分の CPU を使う自分」だけで、絞る利得がない。`[ 0 100001 ] RANGE` が自分の
  タブで拒否され、教える場が言語より狭くなる。
- **一つのプロファイルにする** —— §2.5 が各ホストの選択を認めている。二つの
  ホストは脅威モデルが違う（他人のマシン・並行・呼び手が待っている壁時計 対
  自分のタブ・単一・いつでも閉じられる）。

## 3. 現状（2026-08-13 実測）

### 3.1 二つのプロファイル

`LOCAL_AGENT_RUNTIME_LIMITS`（`rust/src/agent/api.rs`）と
`RuntimeLimits::default()`（`rust/src/interpreter/runtime_limits.rs` の
`DEFAULT_MAX_*`）。MCP サーバの `LIMITS`（`tools/mcp-server/index.js`）は前者と
一致しており、そこにアダプタ専用の 4 つ（`wallTimeMs`, `responseBytes`,
`concurrentExecutions`, `executionSteps`）が加わる。

| 上限 | MCP / `ajisai agent` | playground / `ajisai run` | 倍率 | 導出の有無 |
|---|---:|---:|---:|---|
| `executionSteps` | 100,000 | 100,000 | 1x | なし |
| `numericLiteralDigits` | 4,096 | 4,096 | 1x | なし |
| `bigintBits` | 262,144 | 1,000,000 | 3.8x | なし |
| `materializedElements` | 100,000 | 1,000,000 | 10x | なし |
| `numericWork` | 10,000,000 | 1,000,000,000 | 100x | **あり** |
| `collectionWork` | 20,000,000 | 2,000,000,000 | 100x | **あり** |
| `algebraicTerms` | 512 | 100,000 | 195x | MCP 側のみ |
| `sourceBytes` | 65,536 | 67,108,864 | 1024x | なし |

倍率が 1x から 1024x まで散らばっていて、規則が無い。**これが直す対象である。**
差があること自体は問題ではない。

### 3.2 導出があるのは work の対だけ

各メーターの最も遅い測定経路は `numericWork` が 14,465 units/ms、
`collectionWork` が 30,800 units/ms（`collection-word-billing-2026-08-13.md` §6）。
そこから両プロファイルの含意時間を割り出すと、

- MCP: 10,000,000 ÷ 14,465 ≒ 691 ms、20,000,000 ÷ 30,800 ≒ 649 ms。
  どちらも約 0.7 秒。合計 1.34 秒に対し `wallTimeMs` 5,000 が 3.7 倍の余裕。
- playground: 1e9 ÷ 14,465 ≒ 69 秒、2e9 ÷ 30,800 ≒ 65 秒。どちらも約 67 秒。

**playground 側も偶然ではなく同じ 1:2 で揃っている**（`DEFAULT_MAX_COLLECTION_WORK`
が `2 * DEFAULT_MAX_NUMERIC_WORK` として定義されているため）。つまり導出の骨格は
既に半分できていて、書かれていないだけである。

### 3.3 上限の生存性は両プロファイルで成立している（実測済み）

着手前の仮説「playground 側の上限は生存確認されておらず、`numericWork` が先に
発火して死んでいるのではないか」を実測で検証した。**仮説は外れた。**
両プロファイルとも、三つの内部コスト上限が名前どおりに発火する。

| プロファイル | `algebraicTerms` | `bigintBits` | `collectionWork` |
|---|---|---|---|
| MCP | cascade(10) で発火。`numericWork` は 1,000 万中 2,113,536 | chain(20) で発火。8,606,478 | `[ 0 99999 ] RANGE UNIQUE` |
| playground | cascade(17) で発火。10 億中 520,124,416 | chain(80) で発火。122,321,640 | `[ 0 999999 ] RANGE UNIQUE` |

（cascade / chain は `rust/src/agent/profile_liveness_tests.rs` の
`algebraic_cascade` / `widening_chain` と同じ構成。probe は使い捨てで、
コミットしていない。再実行したければ同テストのヘルパを
`RuntimeLimits::default()` に対して回せば足りる。）

**ただし `profile_liveness_tests` が検査しているのは MCP プロファイルだけである。**
playground 側は「成り立っているが検査されていない」。これは欠陥ではなく網羅の穴で、
本作業で塞ぐ。

## 4. 作業項目

優先順。1 が本体で、2 と 3 は 1 が定めた規則の適用と固定。

1. **playground プロファイルの時間予算を実測し、唯一のパラメータにする。**
   MCP 側は「呼び手が待っている 5 秒」から導いた。playground には壁時計が無いので、
   代わりに「利用者が『固まった』と判断するまでの時間」を決める必要がある。
   §3.2 の含意値は約 67 秒だが、これは**導出された値ではなく現在値の後付け解釈**
   である。実測（あるいは明示的な判断）で一つ決め、その値から全上限を導く。
   決めた根拠を `mcp-host-profiles.md` に書く。数値だけ変えて根拠を書かないなら、
   本作業は何もしていないのと同じである。

2. **`profile_liveness_tests` を両プロファイルで回す。**
   現在 `refused_by` が `LOCAL_AGENT_RUNTIME_LIMITS` を直接参照しているので、
   プロファイルを引数に取る形にして両方に対して同じ 6 件を主張させる。
   §3.3 で成り立つことは確認済みなので、失敗するなら 1 の導出が壊している。

3. **導出の外にある上限を規則に合わせる。**
   - `executionSteps` —— **唯一 1x**。playground は算術の予算が 100 倍あるのに、
     実行できる Word 数は MCP と同じ。安い Word の長いループが自分のタブでも
     MCP と同じ壁に当たる。`DEFAULT_MAX_EXECUTION_STEPS`
     （`rust/src/interpreter/interpreter_core.rs:9`）が両者で共用されているのが原因。
   - `sourceBytes` —— 64 MB。実測でも導出でもなく、単に大きい丸い数。
     テキストエリアに貼る量として意味を持たない。
   - `materializedElements` (10x) と `bigintBits` (3.8x) と `algebraicTerms` (195x)
     —— これらは時間ではなく**単一値の大きさ**を縛るので、時間予算に比例させるのが
     正しいとは限らない。「画面に出して意味がある大きさ」など別の基準が要るなら、
     その基準を書いた上で時間由来の上限と区別すること。全部を一つの倍率に
     押し込むのは、揃って見えるだけで導出ではない。

## 5. 非二次化が先に入ることで動くもの

着手条件（§1）の根拠。**着手前に必ず現状を再確認すること。** 以下は
2026-08-13 時点の状態であり、非二次化はこのすべてに触れる。

- **`collectionWork` の 30,800 units/ms が動く。** §3.2 の導出はこの定数に
  乗っている。走査が O(n·d) でなくなれば単位あたりの時間が変わり、
  「10M と 20M が同じ時間を買う」という関係が崩れる。
  `rust/examples/collection_word_calibration.rs` を再実行して測り直す。
- **`DEFAULT_MAX_COLLECTION_WORK = 2 * DEFAULT_MAX_NUMERIC_WORK` の 2 が動く可能性。**
  この 2 は「同じ時間を買う」ための係数であって、意味のある比ではない。
- **`profile_liveness_tests` の
  `the_collection_ceiling_is_reachable_inside_the_materialization_one` が壊れる。**
  証人プログラムが `[ 0 99999 ] RANGE UNIQUE` であり、UNIQUE が二次であることに
  依存している。非二次化後は `collectionWork` に到達しなくなる可能性が高く、
  新しい証人が要る。作業項目 2 に着手する前に、このテストが緑であることを確認すること。
- **golden の境界ケースが動く。** `tools/mcp-server/golden/limits.json` の
  `collectionWork` 対（`[ 0 6290 ]` 通過 / `[ 0 6291 ]` 拒否）は現在の価格に
  貼り付いている。

## 6. 測っていないこと

正直に残す。次の担当が「調べ済み」と誤解しないため。

- **playground 利用者が待てる時間を測っていない。** §4-1 の入力であり、
  現在値 67 秒は後付けの解釈にすぎない。誰かの体感で決めるなら、
  決めたのが体感であると書くこと。
- **`materializedElements` / `bigintBits` / `algebraicTerms` の playground 値に
  根拠が無い。** 生存はしている（§3.3）が、なぜその値かは誰も書いていない。
- **`ajisai run`（CLI）を独立したホストとして検討していない。** 現在は
  playground と同じ default プロファイルを使っているが、CLI は自分の端末で
  走るという点では playground に近く、パイプラインに置かれるという点では
  agent に近い。どちらの規則に属するかは未決。

## 7. 触らないもの

- **`resourceUsage` の 3 キー構成**（`executionSteps`, `numericWork`,
  `collectionWork`）。宣言する 11 上限のうち 3 つしか返さないのは意図で、
  累積型だけが「残量」を持つため。サイズ上限を足そうとしないこと。
- **上限は値の意味論ではない**（§2.5）。プロファイルを変えても、同じプログラムが
  同じ値を返すことは変わらない。変わるのは「拒否されるかどうか」と
  「どの名前で拒否されるか」だけである。
- **`mcp-host-profiles.md` の「意図された差分」節。** `[ 0 100001 ] RANGE` の
  ホスト間差などは記録された決定であり、導出を統一しても消さない。
  値が変われば記述を更新する対象であって、削除する対象ではない。
