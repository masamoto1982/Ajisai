# 受領証（Receipt）を軸とした改修案 — 「正確に計算できる」から「正しかったと第三者が確かめられる」へ

> Status: **Non-canonical / 方針記録（提案）.** 本書は言語意味論を一切定義しない。
> 正典は `spec/` 配下のソースと、そこから生成される `SPECIFICATION.html` のみ。
> 関連正典: `spec/language-semantics.md`（LANG.OBSERVATION.* / LANG.CONTRACT.CHECK /
> LANG.MACHINE.LIMITS / LANG.DICTIONARY.MUTATION）・`spec/host-protocol-v2.schema.json`・
> `tests/conformance/index.html`。
> 関連設計メモ: `docs/dev/language-coherence-review-2026-08.md`（外部改修案の検証手続き。
> 本書はその手続きを踏襲する）・`docs/dev/spec-impl-drift-tactic.md`（第二権威の禁止）・
> `docs/dev/concept-reduction-2026-07.md`（十概念）・`docs/dev/mcp-readiness.md`（公開規律）。

## 0. 本書の位置づけ

外部評価（Deepseek による Ajisai 評価）と、それに対する批評（以下まとめて **提案群**）を、
リポジトリの実状に突き合わせて検証し、採否と実施順序を定めた記録である。

検証はすべて実行によって行った。参照実装 CLI（`rust/target/debug/ajisai`）に与えた観測と、
`npm run` の各ゲートの実測値を根拠とする。コードの読みだけを根拠にした主張は置いていない。

---

## 1. 結論

提案群の中心的主張——**Ajisai の武器は「正確さ」ではなく「再現・検証可能性」であり、
それらを一本に束ねる形式が受領証（Receipt）である**——は採る。診断は正しい。部品
（内容アドレス同一性・機械可読契約・閉じた正確数値域・構造化ホスト効果・conformance corpus）は
すでに全部あり、束ねられていないだけ、という読みも実状と一致する。

ただし提案群のうち **三つは形を変えて採り、二つは採らない**。理由はいずれも
「Ajisai の同一性の幹に触るか、既存の予算を破るか」である。

| 提案 | 判定 | 形 |
| --- | --- | --- |
| 受領証と `verify` | **採る** | 新概念を足さない。概念6・7・9・10 の合成の観測として、`spec/` に schema を一つ足す |
| NIL 理由の因果連鎖 | **採る（形を変えて）** | 値の同一性には入れない。観測面（report / host protocol の optional field / 受領証）に置く。新語 `WHY` は足さない |
| 決定可能断片の確定 | **採る** | 先に現行の二つの欠陥を直す。断片内では *cannot verify* を返さないことを正典化 |
| 単位・次元 | **採る（Core の外で）** | 値領域は変えない。`#:unit` ディレクティブ＋断片チェッカで検査し、受領証に載せる |
| Core / Surface 二層化 | **採らない（現行の枠内で足す）** | 二層はすでに LANG.SOURCE.DESUGAR として存在する。第二権威を作らない |
| `NTHROOT` 追加 | **採らない** | 批評に同意。加えて語彙予算の観点からも通らない |
| `REDUCE` / `SCAN` 追加、対象4分割、VS Code 拡張・ロゴ | **採らない／後回し** | 批評に同意 |

### 1.1 実行で確認した五つの事実（改修の根拠）

提案群は仕様書を読めない状態で書かれている。実際に走らせて初めて分かった事実が五つあり、
このうち三つは提案群の設計をそのまま実装すると壊れる箇所である。

1. **二つの NIL が出会うと、後から来た理由が消える。**
   `1 0 / [ 1 2 3 ] 9 GET +` → `NIL`（`absence.reason = divisionByZero`）。
   `indexOutOfBounds` は観測面のどこにも残らない。因果連鎖の提案はここに効く。

2. **`UNIQUE` は NIL を理由で区別する。**
   `1 0 / [ 1 2 3 ] 9 GET 2 COLLECT UNIQUE LENGTH` → `2/1`。
   `Value` の等価性は `data` と NIL reason を見る（`rust/src/types/mod.rs` の `Hash for Value`）。
   したがって **理由連鎖を値に持たせると `UNIQUE` / `TALLY` / `GROUP` の観測が変わり、
   conformance を破る**。連鎖は provenance 側に置くしかない。これは
   `value_persist.rs` がすでに「absence metadata は provenance であって同一性ではない」と
   宣言している線と一致する。

3. **`check --contract` が偽の *violated* を出す経路がある。**
   `{ { 1 + } EXEC } 'DYN' DEF` に `#:contract DYN ( 1 -- 1 )` を宣言すると severity `error`
   （「inferred arity is dynamic」）。しかし実行すると `5 DYN` → `6/1`、arity は `( 1 -- 1 )`。
   ブロックは静的に既知のリテラルであり、解析可能である。
   LANG.CONTRACT.CHECK は「静的に既知でないブロック・動的制御は *cannot verify*」と規定して
   いるのだから、**既知のブロックを dynamic 扱いし、しかも *violated* と断定するのは正典違反**である。

4. **再帰があると推論が全軸を底に落とす。**
   `{ KEEP 0 LTE { } { 1 - COUNTDOWN } COND }` の推論結果は
   `arity: dynamic` / `purity: effectful` / `determinism: non-deterministic` /
   `space: unbounded` / `nil: may-create-nil`。
   純粋な語だけで書かれた再帰が `effectful` になる。purity・determinism・effects・capability は
   呼び出しグラフ上の最小不動点で決まる決定可能な軸であり、再帰を理由に諦める必要はない。
   「*検証不能* が逃げ道として残る」という批評の指摘は、**再帰の扱いが原因の大半**である。

5. **受領証の再現性はホスト上限に相対的である。**
   LANG.MACHINE.LIMITS は「実行段数上限と実体化上限は host safety control であり、
   **数値は実装自由**、生じる帰結の種別だけが規範」と定めている。つまり実体化上限に触れた計算は
   別実装で `NIL(spaceExhausted)` にならないことがある。
   **受領証は「上限に触れたか否か」を分類しない限り、実装間で再現しない。**
   提案群はここを見落としている。§3.4 で扱う。

### 1.2 予算という制約（外部からは見えない拘束）

改修案を書く前に、この設計が実際に通さねばならない予算を測った。

| 予算 | 現在値 | 上限 | 残り |
| --- | --- | --- | --- |
| `spec/language-semantics.md` の行数 | **399** | 400 | **1 行** |
| semantic family | **12** | 12 | **0** |
| canonical Word | **65** | 70 | 5 |
| alias | 12 | 16 | 4 |
| 新規 Rust ファイルの行数 | — | 500 | — |

（`node scripts/check-semantic-kernel.mjs` / `scripts/check-file-size-budget.mjs`）

この表が本書の設計を大きく決めている。

- **正典本文に新しい節を丸ごと足す余地は無い**（1 行しか空いていない）。新しい規範は
  `spec/host-protocol-v2.schema.json` と同じく **独立した schema ソース**として置き、
  本文の参照は 1〜2 行に収める。それでも本文を縮める必要がある（§3.5）。
- **新しい semantic family は作れない。** 受領証も単位も、family を要求する設計にはできない。
- **語を足す余地は 5 語しかない。** `WHY` と `NTHROOT` と単位語をそれぞれ足すような案は、
  この 5 語を「まだ需要が確認されていない機能」で使い切る。足さない設計を先に試すべきである。

---

## 2. 受領証は十一番目の概念ではない

提案群の最大の危険は、受領証を新しい中心概念として導入してしまうことである。それは
「Ajisai は十概念とそれ以外を持たない」という同一性の宣言を壊す。

しかし受領証は新概念を要さない。すでにある概念の**合成の観測**にすぎない。

| 受領証の構成要素 | 由来する既存概念 | 既存の実装 |
| --- | --- | --- |
| プログラムと依存の同定 | 概念6（内容アドレス同一性） | `interpreter/word_identity.rs`（BLAKE3、`#` + 64 hex） |
| 環境の同定 | 概念7（機械可読契約） | `spec/words.json` |
| 入出力の同定 | 概念1・2（正確値と三帰結） | `types/value_persist.rs` / `value_protocol.rs` |
| 効果列の同定 | 概念9（唯一のホストプロトコル） | `spec/host-protocol-v2.schema.json` |
| 実装非依存性の根拠 | 概念10（conformance corpus） | `tests/conformance/index.html`（274 ケース） |

したがって受領証の位置づけはこう定める。

> **受領証は、ある一回の計算に対して、conformance corpus が言語全体に対して言っていることを
> 個別に発行したものである。** corpus は「この実装は Ajisai か」を判定する。受領証は
> 「この結果は Ajisai の結果か」を判定する。判定者はどちらも実装ではなく仕様である。

この定義には副次効果がある。批評が「コンプライアンススイートが QA 資産から規範に昇格する」と
書いた変化が、実際には**すでに起きている**（`PORTABILITY.md` 第 1・10 条、
LANG.CONFORMANCE.CORPUS「corpus は "これは Ajisai か" の決定手続きであり、散文は答えない」）。
足りないのは corpus の地位ではなく、**個別の計算にそれを適用する形式**だけである。

---

## 3. 受領証スキーマ v1

### 3.1 何を含むか

`spec/receipt-v1.schema.json`（新規の正典ソース）。JSON。フィールドは以下。

```jsonc
{
  "receiptVersion": 1,
  "hostProtocolVersion": 2,

  "program": {
    "sourceDigest": "#<64hex>",      // 正規化後のソースの digest
    "canonicalForm": "#<64hex>"      // 脱糖後の正準トークン列の digest（表層形に不変）
  },

  "environment": {
    "coreRegistryDigest": "#<64hex>",   // spec/words.json の正準 digest
    "conformanceDigest": "#<64hex>",    // tests/conformance/index.html の digest
    "hostProfile": {                    // LANG.MACHINE.LIMITS の実際の設定値
      "maxExecutionSteps": 100000,
      "maxMaterializedElements": 100000,
      "maxNumericWork": 10000000,
      "maxCollectionWork": 20000000
    },
    "implementation": { "name": "ajisai-core", "version": "0.2.0-beta.1" }  // 非規範
  },

  "dictionary": {
    "userWords": [ { "name": "TAX", "identity": "#<64hex>" } ]  // 内容アドレス同一性
  },

  "input":  { "stackDigest": "#<64hex>", "values": [ /* 正準形 */ ] },
  "result": {
    "status": "ok",
    "stackDigest": "#<64hex>",
    "values": [ /* 正準形 */ ],
    "effects": [ { "kind": "consoleWrite", "payloadDigest": "#<64hex>" } ]
  },

  "absence": [ /* §4 の因果連鎖 */ ],
  "obligations": [ /* §5 の実行時債務とその充足 */ ],

  "resources": { "executionSteps": 2, "numericWork": 1, "collectionWork": 0 },
  "reproducibility": "limit-independent"   // または "limit-relative"（§3.4）
}
```

`resources` と `implementation` を除く全フィールドが照合対象である。
`ajisai run --receipt out.json` が発行し、`ajisai verify out.json --program src.ajisai` が
再実行して照合する。照合結果は三値：**verified** / **mismatch** / **not-comparable**。

### 3.2 正準値エンコーダが要る（既存の二つは使えない）

受領証の要は値の digest である。ここで実装上の落とし穴が一つある。

**既存の二つの値エンコーダはどちらも受領証に使えない。**

- `types/value_protocol.rs`（観測プロトコル）は **意図的に非可逆**である。ソース自身が
  「`ExactScalar` は *marked* な有理近似として観測され、`CodeBlock` は `nil` として隠される」と
  書いている。近似をハッシュしても意味がない。
- `types/value_persist.rs`（永続化コーデック）は可逆だが **正準ではない**。
  `decode(encode(v)) == v` は保証するが、`v == w` である二つの値が同じバイト列になる保証はない。
  実際 `Value` の等価性は約分前の分数と約分後、`Vector` と同じ内容の `Tensor`、
  基底の異なる代数値（`√12` と `2·√3`）を同一視する。

したがって **第三のエンコーダ**「正準 digest 形式」が要る。要件は一行で書ける。

> `a == b` ⇔ `digest(a) == digest(b)`

幸い、この正規化規則はすでに書かれている。`Hash for ValueData` が `PartialEq` と一致するよう
（dense 化、代数値の rebase、分数の約分）作られており、`value_hash_tests.rs` がそれを固定している。
正準エンコーダは **その規則を、64bit ハッシュではなく安定バイト列として書き直したもの**である。
新規実装ではなく既存規則の再エンコードなので、テストは既存のペアリング性質試験を再利用できる。

受け入れ条件：`value_hash_tests.rs` と同じ値の組すべてについて
`a == b ⇒ digest(a) == digest(b)` と `a != b ⇒ digest(a) != digest(b)` を property test で固定する。

### 3.3 段階（トレース Merkle 木は最後）

批評は「トレースを Merkle 木にすれば部分検証できる」と書くが、これは最初にやることではない。
現行の `errorFlowTrace` は**エラーと NIL の事象だけ**を記録しており、全 Word 呼び出しの
トレースは存在しない。全段トレースの新設は、実行経路への侵襲・`executionSteps` 予算・
ファイル行数予算のすべてに触る。

順序はこうする。

- **R1 — 境界受領証（トレースなし）**：プログラム＋環境＋入力 → 出力＋効果列。
  現行の `--json` レポートにすでにある材料だけで組める。侵襲ゼロ。**これで主張の 9 割が立つ。**
- **R2 — 因果 provenance**：§4 の absence 連鎖を受領証に載せる。
- **R3 — 段トレースと Merkle（既定オフ、要求時のみ）**：需要が確認されてから。
  粒度は行ではなく Word 呼び出し。`--receipt-trace` を明示した時だけ収集する。

R1 の時点で「別マシン・別実装で再実行してハッシュ一致を確認する」は完全に成立する。
Merkle 木が要るのは「巨大な計算の一部だけを検証したい」場合であり、それは需要が観測されてからでよい。

### 3.4 再現性クラス — 受領証が正直であるための一項目

§1.1(5) の帰結。受領証は必ず `reproducibility` を持つ。

- **`limit-independent`** — 実行中、どのホスト上限にも触れていない。
  `resources` の各値がプロファイル値に達していない、かつ `spaceExhausted` / 段数超過が
  一度も起きていない。この受領証は**上限設定の異なる適合実装でも同じ結果になる**。
- **`limit-relative`** — 上限に触れた（`NIL(spaceExhausted)` が生じた、段数上限で停止した）。
  この結果はホストプロファイルに依存する。照合側は同一プロファイルでのみ **verified** を返し、
  異なるプロファイルでは **not-comparable** を返す。

この一項目があるかないかで、受領証の主張の強さが変わる。無ければ「同じ受領証が別実装で
再現しない」事例が必ず出て、形式そのものの信用が落ちる。あれば、その事例は
**形式が事前に宣言していた通りの挙動**になる。

### 3.5 正典への載せ方（1〜2 行で足りる）

`spec/language-semantics.md` の残り行数は 1 である。新節 `LANG.OBSERVATION.RECEIPT` は
6 行前後を要するので、**そのままでは通らない**。二つの方針を比較した。

| 方針 | 行コスト | 評価 |
| --- | --- | --- |
| A: 新節 `LANG.OBSERVATION.RECEIPT` を立てる | +6 行。既存節から 6 行を捻出 | 概念が増えたように読める。避けたい |
| **B: schema を正典ソースにし、本文は既存節に 1〜2 行足す** | **+2 行**。既存節から 2 行を捻出 | `host-protocol-v2.schema.json` と同じ前例。採用 |

**方針 B を採る。** 追記先は LANG.CONFORMANCE.CORPUS（受領証は corpus の個別適用だから）。
文面案：

> 実装は、一回の実行の観測を `spec/receipt-v1.schema.json` に適合する受領証として発行してよい。
> 受領証の照合は、corpus と同じ対応関係を単一の実行に適用したものである。

捻出元の候補（意味を落とさずに縮められる箇所を実測した）：

- `LANG.SOURCE.TEXT`（22 行、本文中で最大）— 数値文法の 3 段落を 2 段落へ。**−4 行**
- `LANG.AUTHORITY.FREEDOM`（10 行）— 2 段落を 1 段落へ。**−4 行**

どちらか一方で足りる。**この圧縮は受領証の作業に先立って単独の PR にする**（正典の圧縮と
機能追加を同じ差分に混ぜない）。

---

## 4. NIL の因果連鎖 — 値ではなく provenance に置く

### 4.1 現状と、提案をそのまま実装した場合に壊れるもの

現状は §1.1(1)(2) の通り。理由は一つだけ保持され、合流時に片方が消える。そして
理由は `Value` の等価性に**入っている**。

提案群は「理由を累積・連鎖させ、`WHY` ワードで展開する」と書く。これをそのまま値に対して
行うと、`UNIQUE` / `TALLY` / `GROUP` が「同じ理由だが履歴の違う NIL」を別物として扱い始める。
274 件の conformance ケースの期待値が動き、beta の互換性凍結（README「beta 用に書かれた
プログラム・保存セッション・辞書は beta が読む」）を破る。

### 4.2 採る形

- **連鎖は値に持たせない。** `Value` の等価性・`Hash` は現状のまま（直接理由のみ）。
  これは `value_persist.rs` の「absence metadata は provenance であって同一性ではない」と
  同じ線であり、新しい原則ではない。
- **連鎖は観測面に置く。** 具体的には三箇所：
  1. `errorFlowTrace` を伸ばす。すでに absence イベントは記録されている（origin / reason /
     stackLenBefore / stackLenAfter）。足すのは **合流時に捨てた側の理由** と、
     **リフティング時の要素インデックス**。
  2. ホストプロトコルの absence payload に **optional field** として `causes` を足す。
     `spec/README.md` が「現行バージョン内では optional field の追加のみ許される」と
     規定しているので、**プロトコル版を上げずに済む**。
  3. 受領証の `absence` 配列に載せる。

- **`WHY` は足さない。** 理由は二つ。第一に語彙予算（残り 5 語）を、まだ需要が観測されていない
  語で使う判断は早い。第二に `NIL-REASON` の契約はすでに
  「Read the **direct** reason of an operational NIL」と書かれており、直接理由と連鎖の区別は
  語彙の側で予告されている。連鎖の読み出しは観測面（`--json` / MCP / GUI）で先に提供し、
  **言語内から連鎖を読みたいという需要が実際に観測されてから**語を足すか決める。

### 4.3 デモ価値

「行47が欠測 → 合計が未定義 → 平均が未定義」という鎖は、上の 1.–3. だけで出せる。
`examples/` に一つ、欠測を含む集計の例を置き、受領証の `absence` に鎖が載っている様子を示す。
これは Pandas / SQL NULL / NaN のいずれも構造的に出せない出力である。

---

## 5. 決定可能断片 — 先にバグを直し、それから正典化する

批評の「*検証不能* が逃げ道として残る限り保証の価値が生まれない」は正しい。ただし実測すると、
**逃げ道の大半は断片の定義が緩いことではなく、二つの実装上の欠陥**から来ている。

### 5.1 先に直す二つの欠陥

**(a) 静的に既知のブロックを dynamic 扱いしている（§1.1(3)）。**
`{ 1 + } EXEC` の `{ 1 + }` はリテラルであり、その場で解析できる。現状は `EXEC` を見た時点で
`ContractFlow::Dynamic` に落ち、しかも confidence が保守的でないため
`Severity::Error`（`rust/src/agent/contract_decl.rs:369`）を出す。
**これは *violated* の誤報**であり、*cannot verify* の過剰報告より重い。実行すれば `( 1 -- 1 )`。

修正：`EXEC` / `MAP` / `FILTER` / `FOLD` / `ANY` / `ALL` / `COND` の引数ブロックが
**その場のリテラル**であるとき、ブロック本体を通常の本体として解析する。
併せて `ContractFlow::Dynamic` から `Severity::Error` を出す経路を見直す
（「動的である」は「宣言と矛盾する」ではない）。

**(b) 再帰が全軸を底に落としている（§1.1(4)）。**
純粋な語だけの再帰が `effectful` / `non-deterministic` になる。
purity・determinism・effects・capability は、呼び出しグラフ上で
「呼ぶ語のいずれかが effectful なら effectful、さもなくば pure」という**単調な最小不動点**で
決まる。再帰＝SCC は既に `word_identity.rs` が扱っており（SCC を単位としてハッシュしている）、
同じ SCC 走査を契約推論に適用すればよい。arity と space は再帰では一般に決まらないので
*cannot verify* のままでよい。**軸ごとに諦める**のが正しく、まとめて諦めるのは過剰である。

この二つを直すだけで、`check --contract` の *cannot verify* は大きく減る。
定量化は作業の一部とする（`examples/` 全件に対する verified / violated / cannot-verify の
比率を修正前後で計測し、`docs/dev/` に記録する）。

### 5.2 断片 F の定義（提案）

正典に置く定義。以下をすべて満たす本体を **断片 F** とする。

1. 本体の全 Word が Core 語、または断片 F に属する User 語である。
2. 高階語に渡すブロックが、その場のリテラルである。
3. `REFLECT` による動的な語構築を含まない。

**F の内側では、次の軸について *cannot verify* を返してはならない**（返す実装は非適合）：

- スタック効果（consumes / produces）
- NIL 生成の有無
- purity / determinism / effects / capability
- 実体化上限に対する space class

さらに **再帰があっても**、purity / determinism / effects / capability の四軸は
*cannot verify* を返してはならない（§5.1(b)）。

これらはすべて有理数上の線形算術と有限束の最小不動点で閉じるので、SMT ソルバを要さない。

### 5.3 断片外は「実行時債務」にする

F の外側の宣言は、*cannot verify* のまま放置せず **`obligations`（実行時債務）** に分類する。
受領証はこの債務が実行時に充足されたことを記録する。

```jsonc
"obligations": [
  { "word": "NORMALIZE", "axis": "arity", "declared": "( 1 -- 1 )",
    "static": "cannot-verify", "runtime": "satisfied" }
]
```

これで「事前検証と事後証明の役割分担」が形式として成立し、グレーゾーンが消える、という
批評の主張が**実際に成り立つ**。事前に *cannot verify* だった項目が、受領証の上では
必ず satisfied か violated のどちらかとして決着する。

### 5.4 ゲート

property test を足す。断片 F に属するプログラムをランダム生成し、
`check --contract` が上記の軸で `note`（cannot verify）を返したら失敗させる。
これが「F の内側では逃げ道がない」ことを機械的に保つ唯一の方法である。

---

## 6. 単位・次元 — 値領域には入れない

提案群は「各値に次元指数ベクトルを持たせる」と書く。値領域に入れる形では採らない。

**入れない理由**（費用の実測）：

- 値領域の変更は概念1と2の変更であり、同一性の幹（`ajisai-minimal-core-identity.md`）に触る。
- 65 語**すべて**の契約に次元規則を書く必要がある（`spec/words.json` の全エントリ）。
- 274 件の conformance ケースの期待値が動く。
- ホストプロトコルの Value payload が変わる。これは optional field の追加では済まず、
  **プロトコル版を上げる破壊的変更**になる（`spec/README.md`）。
- semantic family 予算が 12/12 で埋まっている。次元は新 family を要求しやすい。

**採る形**：`#:contract` と同じ地位の **tooling 専用ディレクティブ** `#:unit` を足す。

```text
#:unit  DISTANCE  m
#:unit  DURATION  s
#:contract SPEED ( 2 -- 1 ) pure nil-free
#:unit  SPEED     m/s
```

- 次元指数は有理数ベクトルとして持つ（`SQRT` は指数を 1/2 倍する。批評の指摘通り、
  平方根閉包を持つ言語だからこそ次元の平方根が破綻しない — ただしこれは**チェッカ側**の話で、
  値側の話ではない）。
- 検査は §5.2 の断片チェッカに合流する。次元整合は有理数上の線形算術で閉じるので、
  断片 F の内側では常に決定できる。
- 結果は受領証の `obligations` / 検証済み項目に載る。

これで「正確・次元整合・監査可能」の三点は揃う。値は一切変わらないので、
conformance も host protocol も語彙予算も無傷である。**言語を大きくせずに、
F# の units of measure が持たないもの（正確性と受領証）を足せる。**

---

## 7. 可読性（いわゆる Forth 問題）

批評は「Ajisai Core（連結型・監査可能 IR）と Ajisai Surface（可読層）に二層化せよ」と書く。
**この形では採らない。**

理由は一つで、`docs/dev/spec-impl-drift-tactic.md` が第一の不変条件として掲げる
**第二権威の禁止**に反するからである。脱糖規則を別の層の文書として立てれば、
「Surface が何を意味するか」を定める第二の正典が生まれる。

そして**二層はすでに存在する**。

- `LANG.SOURCE.NORMALIZE` / `LANG.SOURCE.DESUGAR` が、表層形を評価前に正準概念へ降ろすことを
  正典として規定している。
- `rust/src/surface_forms.rs` と `core_word_aliases.rs` が、その表層形の登録簿である。
- `word_identity.rs` の `body_content_key` は**正準化後**のトークン列をハッシュしている
  （数値リテラルは既約分数へ、別名は正準名へ）。

つまり批評が求めた性質——「表層をいくら可読にしても監査可能性が損なわれない」——は、
**新しい層を作らなくても、すでに構造として成立している**。受領証の
`program.canonicalForm` を脱糖後の digest と定義すれば、表層形の追加は受領証に対して不変である。

したがって可読性の作業は「二層化」ではなく「**登録された表層形を増やす**」になる。
ただし **優先度は最後**とする。理由：

- 表層形の追加は 274 件の conformance ケースの記法に影響しうる。
- 受領証・断片・MCP が効き始める前に構文を触ると、何が採用障壁だったのかの計測ができなくなる。
- 現時点で「読みにくさが採用を止めた」という観測が無い。**先に受領証を出して、
  その反応を観測してから決める。**

具体的な表層形の候補（パイプライン記法・名前付き束縛）は本書では決めない。別途、
実際の利用者の記述例を集めてから検討する。

---

## 8. 配布 — MCP に `verify_receipt` を一つ足す

批評は「MCP サーバとして公開せよ。ツールは `eval` / `check` / `verify_receipt` の三つで足りる」と
書くが、**MCP サーバはすでに存在する**（`tools/mcp-server/`）。現行ツールは
`compute` / `check` / `infer_contracts` / `word_contract` の四つ。

したがって作業は「MCP サーバを作る」ではなく：

1. `compute` に `receipt: true` オプションを足す。
2. `verify_receipt` ツールを一つ足す。
3. `tools/mcp-server/result.schema.json` に受領証を追加（optional field）。

**公開規律は `docs/dev/mcp-readiness.md` の release rule を守る**
（P0 が 100% に達するまで公開の exact-computation MCP として売り出さない、
P2 に再現可能な結果が出るまでリモートサービスを運用しない）。受領証は P1 の中に置く。

埋め込みについては、Rust コアはすでに `ajisai-core` crate として分離され、
`agent::api`（`compute` / `check`）が「filesystem も terminal も触らない型付き境界」として
用意されている。Python / TypeScript から呼ぶ部品として出すのに、新しい設計は要らない。

デモは金額計算に絞る（消費税の端数処理、按分、リース計算）。`examples/` に受領証つきで置く。

---

## 9. 実施順序

各段は独立した PR とし、前段の受け入れ条件が満たされてから次へ進む。

| # | 段 | 内容 | 受け入れ条件 |
| --- | --- | --- | --- |
| **P0** | 正典圧縮 | §3.5 の −4 行。機能追加を含まない | `npm run semantic-kernel:check` が 4 行以上の headroom を報告 |
| **C1** | 契約推論のバグ修正 | §5.1(a)(b)。リテラルブロック解析、SCC 単位の軸別不動点 | `{ 1 + } EXEC` が verified。純粋再帰が `pure` を保つ。`examples/` の cannot-verify 比率を修正前後で記録 |
| **R0** | 正準値 digest | §3.2。`Hash for ValueData` と同じ規則の安定バイト列化 | `a == b ⇔ digest(a) == digest(b)` の property test |
| **R1** | 境界受領証 | `ajisai run --receipt` / `ajisai verify`、`spec/receipt-v1.schema.json`、`reproducibility` 分類 | 別プロセス・別マシンで verified。上限に触れた計算が `limit-relative` になる conformance ケース |
| **C2** | 断片 F の正典化 | §5.2 の定義と「F 内で cannot verify を返さない」規範、§5.3 の債務 | 断片内ランダム生成プログラムの property test。conformance ケース追加 |
| **N1** | absence provenance | §4.2 の 1.–3.。合流時に捨てた理由、要素インデックス | 合流した二理由が両方観測できる。`UNIQUE` の観測が**変わらない**ことを回帰で固定 |
| **M1** | MCP | §8 の三点 | `npm run test:mcp` / `eval:mcp` 緑。readiness トラッカー更新 |
| **U1** | 単位 | §6 の `#:unit` と断片チェッカへの合流 | 次元不整合が断片内で必ず検出される。値・プロトコル・corpus に変更がないこと |
| **R3** | 段トレースと Merkle | §3.3。既定オフ | 需要が観測されてから着手 |
| **S1** | 表層形 | §7 | 受領証の反応を観測してから決める |

全段に共通のゲート：`npm run specification:check` / `check:semantic-firewall` /
`check:file-size` / `check:agent-cli-contract` / `word-registry:check` /
`cargo test --lib && cargo test --tests`（conformance 274 件を含む）。

---

## 10. 採らない提案とその理由

- **`NTHROOT` の追加** — 批評の理由（一般の n 乗根は最小多項式・終結式を伴う一般代数的数演算に
  なり、正準化コストと実根分離が要る。全順序判定という最重要保証の費用が桁で上がる）に同意する。
  加えてリポジトリ側の理由がある。語彙予算の残りは 5 語であり、
  「comparison is total」は README の一枚看板である。表現力の微増と引き換えにする資産ではない。
- **`REDUCE` / `SCAN` の追加** — 同意。機能の広さは Ajisai が勝てる競争軸ではない。
  加えて、これらは 29 の Standard Word の分類（`derivable` / `operational`）に照らして、
  `FOLD` から derivable であることを示す witness が要る。費用に見合わない。
- **対象を4つ並べるフェーズ戦略** — 同意。焦点が失われる。
- **VS Code 拡張・ロゴ** — 同意。順序が逆。受領証が効き始めてからの話。

---

## 11. 本書の主張を一文で

> Ajisai に足りないのは機能ではなく、**すでに持っている五つの部品が一つの成果物として
> 出てこないこと**である。受領証はその成果物であり、新しい概念を一つも足さずに作れる。
> 作る過程で、契約検査の二つの欠陥（偽の violated と、再帰での過剰な諦め）を直すことになり、
> それが「*検証不能* という逃げ道」を実際に塞ぐ。
