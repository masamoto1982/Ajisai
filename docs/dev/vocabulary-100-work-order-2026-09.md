# 語彙 100 語への再設計：改修指示書（2026-09）

Status: **非正典（`[設計根拠]`）**。この文書は Ajisai の意味論を定義しない。
正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。
本書と正典が矛盾したら正典が勝つ。

対象実装者: Claude（またはそれに準ずるエージェント）
添付: `docs/dev/_attachments/vocabulary-100-draft-contracts.json`（新語の契約スケルトン。非正典）

所有者判断: 本書の設計判断（§2 の三分岐・§3 の概念組み替え・§4 の 100 語割り当て）は
採用済み。**後方互換性は要求しない**（所有者の明示指示）。

---

## 0. この文書の読み方

### 0.1 到達したい主張

**Ajisai の語彙は、全域・非再帰の言語が持ちうる計算の天井そのものである。**
したがって語を一つ足すか足さないかは、利便の問題ではなく「その計算が永久に書けるか
書けないか」の問題である。本書はこの一点から 100 語を導出する。

現在の 66 語は「概念 10 個が要求する語」としてよく選ばれており、ユーザー定義で
等コストに書ける冗語は `SUM` の一語しか見つからない。**したがって語数を増やすには
概念の側を動かすしかない。** 逆に、概念を三つ動かせば語数は 100 前後に着地する。
100 は目標値ではなく、§2 の三分岐を採った結果の数である。

### 0.2 現状の測定値（本書の出発点。すべて `spec/words.json` からの実測）

| 区分 | 実測 |
| --- | --- |
| 正準語 | 66 |
| Semantic Kernel / Standard | 36 / 30（`docs/word-manifest.json` の `counts`） |
| 記号エイリアス | 11（`=` `!=` `<` `<=` `>` `>=` `+` `-` `*` `/` `%`） |
| `partiality` 内訳 | `total` 25 / `projecting` 21 / `partial` 20 |

族ごとの語数:

| family | 語数 |
| --- | --- |
| `exactArithmetic` | 16 |
| `collection` | 16 |
| `booleanLogic` / `comparison` / `higherOrder` / `text` | 各 6 |
| `absence` / `dictionary` | 各 3 |
| `control` | 2 |
| `stackModifier` / `output` | 各 1 |

この分布が示すのは、**数とベクトルだけが厚く、残りが薄い**という偏りである。
テキスト 6 語・辞書 3 語・出力 1 語で、キーによる対応は 0 語。

### 0.3 全 Phase 共通の禁止事項

- ❌ **`SPECIFICATION.html` を直接編集しない。** 生成物である（`npm run specification:generate`）。
- ❌ **字句文法を増やさない。** 本書の 100 語は `spec/grammar.json` に一つも新しい
  字面を要求しない（§4.3）。括弧も引用符もリテラル形式も現状のまま。
- ❌ **修飾軸を 2 本目にしない。** 深さ（rank）は軸ではなく語として入れる（§2.2）。
- ❌ **ERROR を捕捉する語を入れない。** 「誤用を NIL に変換しない」
  （`LANG.FAILURE.TRICHOTOMY`）は同一性そのものである。
- ❌ **テキストから Symbol を作る語を入れない。** `LANG.DICTIONARY.ACYCLIC` の
  検査の完全性が「どの語もテキストをコードに変えない」ことに依拠している。
  `DEFINED?` を入れるなら引数は Symbol であって String ではない。
- ❌ 既存テストを削除・スキップ・`#[ignore]` しない。落ちたら実装を直す。
- ❌ `rust/src/` に 500 行を超える新規ファイルを作らない（`npm run check:file-size`）。

---

## 1. 語を採る基準

`LANG.AUTHORITY.IDENTITY` は既に「ユーザー定義で書けるものは Core に入れない、ただし
漸近コストが悪化する場合を除く」と述べている。本書はこれを二つの級に書き直し、
**この二級のどちらにも当たらない語は 100 語に入れない**ことを規律とする。

| 級 | 基準 | 例 |
| --- | --- | --- |
| **A（力）** | 全域・非再帰の言語では**書けない**、または核で書けば漸近的に安い | `FLATTEN` `BSEARCH` `MEMBER` `JSON-DECODE` `GCD` `UPPER` |
| **B（閉包）** | 小さな対称族を閉じ、「あるかどうか迷わせない」 | `CEIL`（`FLOOR` があるなら）`DROP`（`TAKE` があるなら） |

### 1.1 なぜ級 A がこの言語では特別に重いか

再帰も無限ループもない（`LANG.DICTIONARY.ACYCLIC`）。反復は有限ベクトル上の
`MAP` / `FILTER` / `FOLD` / `SCAN` / `ANY` / `ALL` だけである。したがって:

- **対数時間のアルゴリズムは書けない。** 二分探索は「区間を半分にし続ける」反復を
  要し、その反復回数は入力に依存する。ユーザーは永久に書けない。
- **深さが不定の再帰的構造は畳めない。** ネストの深さは値ごとに違うので、深い平坦化も
  JSON のパースも書けない。`CONCAT` は 1 段しか平坦化しない。
- **不動点反復は書けない。** これは設計上の放棄であり、本書は取り戻さない。

Turing 完全な言語では語の欠落は不便にすぎない。Ajisai では欠落は天井になる。
**34 枠はこの天井を上げることに使う。短縮形と糖衣には一枠も使わない。**

### 1.2 級 B を認める理由

100 語は一人の頭に入る上限に近い。可読性は語数ではなく**族の閉包と命名の規則性**で
稼ぐ。`FLOOR` があって `CEIL` がない、`TAKE` があって `DROP` がない、という非対称は
利用者に毎回「あるのか」を問わせる。これは語を一つ減らす代わりに参照を一回増やす
取引であり、100 語規模では割に合わない。

命名規律（本書で確定）:

- 述語（Boolean を返し、副作用を持たない語）は `?` で終える: `NIL?` `HAS?` `DEFINED?`。
- 変換は動詞、観測は名詞: `RESHAPE` は動詞、`SHAPE` は名詞。
- 記号エイリアスは**増やさない**。現行 11 個（算術と比較）で固定する。

---

## 2. 三つの分岐

### 2.1 分岐 1 — Tier 2（計算可能実数）を捨てるか、払い切るか

**現状は最悪の中間である。** `LANG.VALUES.EXACT` は代数体（Tier 1）と一般の計算可能実数
（Tier 2）の二層を定義し、Tier 2 のために予算付き比較・比較由来の UNKNOWN・判定不能な
等価性という機構一式を仕様と実装（`rust/src/types/exact/computable.rs`,
`rust/src/types/exact/cf_budget_tests.rs`）に抱えている。**そして Tier 2 の目撃者は
`PI` ただ一つで、`PI` を使う三角関数は一つもない。**

| 案 | 内容 | 得るもの | 失うもの |
| --- | --- | --- | --- |
| 撤退 | `PI` を削り純代数体にする | 比較が常に全域。UNKNOWN の源が「不在が真理位置に立った」場合だけになり、概念 1・2 が一段簡単になる | 超越関数を永久に持てない |
| 払い切り | `POW` `EXP` `LN` `SIN` `COS` `ATAN` を入れる | 厳密計算の主張が実データに届く。既存機構が働き始める | 信頼数値カーネルが太る。語ごとに厳密エンクロージャ生成器と予算試験が要る |

**採用: 払い切り。** 「厳密に計算し、決められないときは UNKNOWN と正直に言う」は
Ajisai の差別化点であり、それが効くのはまさに超越関数の領域である。エンクロージャ機構
（`rust/src/types/exact/pi.rs` の交代級数による厳密包囲）は既にあり、増分で届く。

**この分岐だけは「語を足す」以上の工事である。** 他の分岐が語彙と契約の仕事で済むのに
対し、ここは信頼カーネルの拡張を伴う。Phase 順序（§7）で最後に置くのはこのためである。

### 2.2 分岐 2 — 形（shape）を語彙に出す

`LANG.COLLECTIONS.LIFT` は shape・軸の対応・長さ 1 の放送という理論を持つ。
実装にも `DenseTensor` と shape がある。**しかしプログラムから shape は観測できない。**
`LENGTH` が最外軸の長さを返すだけである。概念と語彙が一致していない。

さらに `MAP` は最外軸しか歩けないので、深いネストを扱う手段が実質ない。深い平坦化は
級 A（書けない）である。

**採用: 5 語** — `SHAPE` `RESHAPE` `FLATTEN` `DEPTH` `RANK`。

`RANK` が肝である。「ブロックを深さ *n* の部分ベクトルに適用する」一語で、APL の
ランク概念が軸を増やさずに入る。**修飾軸を 2 本（KEEP × 深さ）にする案は採らない。**
「修飾軸は一本」は `LANG.MODIFIERS.CONSUMPTION` が掲げる看板であり、`RANK` を語として
持てば同じ力が手に入るのに、軸を増やすと全語の契約に深さ欄が生える。

### 2.3 分岐 3 — キーによる対応（第 7 の値領域）

**最大の欠落。** 現在、名前付きフィールドを持つデータは「並行ベクトル ＋ `INDEX-OF`
（線形探索）」でしか表現できない。AI-first を掲げ MCP 経由で構造化データを扱う言語
としては、ここが最も高く付いている。

- 級 A として正当: キー引きは O(1)〜O(log n)、`INDEX-OF` は O(n)。
- 表（table）は「列の Record」として無料で付いてくる。
- JSON 形のデータが素直に乗る（§2.3 と `JSON-DECODE` は同じ分岐の表裏である）。

**採用: 第 7 の値領域 Record（順序つきキー→値）と 9 語。**

**正直なコスト: これは 9 語の追加ではなく、100 語すべての契約の再レビューである。**
どの語も「Record を渡されたらどうなるか」に答えねばならない。これが第 7 領域に対する
最強の反論であり、本書は採用する代わりに**封じ込め規則**を同時に固定する。

> **封じ込め規則（正典化対象）**
> 1. Record は**算術・比較に対してのみ値方向へ持ち上がる**。キーは不変で、結果は同じ
>    キー列を持つ Record。
> 2. 契約が Record を名指ししていない語は、Record を受けたら ERROR（`nonVector` 系）。
>    暗黙のベクトル化・暗黙の変換は一切しない。
> 3. Record の同一性は「キー列と値列の denotation」であり、構築順序は観測できない
>    （`LANG.VALUES.DENOTATION` の適用）。キー列は挿入順を保つ（順序は観測可能な構造）。

この 3 条があれば、既存 66 語のうち再レビューで実際に変更が要るのは算術・比較の
22 語だけになり、残りは規則 2 の一行で片付く。

---

## 3. 10 概念の組み替え

現行 10 概念のうち、**7（語ごとの契約）と 8（実行前検査）は一つの考え**であり、
**9（ホストプロトコル）と 10（適合コーパス）はプログラムの意味ではなくプロジェクトの
同一性**である。この 2 枠を畳むと、§2.2・§2.3 の 2 概念がちょうど収まる。

| # | 新・概念 | 現行との関係 |
| --- | --- | --- |
| 1 | 丸めのない厳密実数（代数体＋予算付き計算可能実数） | 現 1。§2.1 で Tier 2 が実体を持つ |
| 2 | 三つの結末：値／理由つきの不在／誤り（三値真理を含む） | 現 2 |
| 3 | スタックと、値のベクトル（テキストを含む） | 現 3 |
| 4 | **形と階（shape / rank）** | **新設**。現 3 に埋もれていた持ち上げ法則を概念に昇格 |
| 5 | **キーによる対応（Record と表）** | **新設** |
| 6 | コードはベクトル、評価は語が求めたときだけ | 現 4 |
| 7 | 一本の修飾軸 | 現 5 |
| 8 | 二層辞書と内容同一性 | 現 6 |
| 9 | 語ごとの機械可読契約と、実行前の検査 | 現 7 ＋ 現 8 |
| 10 | 一つのホストプロトコルと、適合を決める実行可能コーパス | 現 9 ＋ 現 10 |

概念は**読者のための区分**であり、`family` は**法則を共有する語の区分**である。
両者は一致しない（例: `RANK` は概念 4 に属するがブロックを評価するので族は
`higherOrder`）。この非一致は意図的であり、Reference は概念で、適合試験は族で並べる。

---

## 4. 100 語の割り当て

### 4.1 概念ごとの語数

| # | 概念 | 語数 |
| --- | --- | --- |
| 1 | 厳密実数 | 24 |
| 2 | 真理と不在 | 17 |
| 3 | ベクトル（18）＋テキスト（8） | 26 |
| 4 | 形と階 | 5 |
| 5 | 対応 | 9 |
| 6 | コードブロックと高階 | 7 |
| 7 | 修飾軸 | 1 |
| 8 | 辞書と同一性 | 5 |
| 9 | 契約 | 2 |
| 10 | ホスト境界 | 4 |
| | **合計** | **100** |

### 4.2 語の一覧（`+` が新語）

```
1  厳密実数(24)  ADD SUB MUL DIV MOD NEG ABS MIN MAX FLOOR +CEIL ROUND
                 QUANTIZE SQRT +POW +GCD +RATIO RANDOM PI
                 +EXP +LN +SIN +COS +ATAN                    削除: SUM
2  真理と不在(17) TRUE FALSE AND OR NOT SELECT EQ NEQ LT LTE GT GTE
                 NIL NIL? NIL-REASON +ABSENT +FAIL
3  ベクトル(18)   GET PUT TAKE +DROP LENGTH CONCAT REVERSE SORT ORDER
                 UNIQUE TALLY +MEMBER +BSEARCH INDEX-OF ZIP RANGE
                 FILL COLLECT                                 移動: GROUP→5
   テキスト(8)    CHARS JOIN TRIM TOKENIZE +SEARCH +REPLACE NUM STR
4  形と階(5)      +SHAPE +RESHAPE +FLATTEN +DEPTH +RANK
5  対応(9)        +RECORD +KEYS +VALUES +AT +WITH +WITHOUT +HAS? +MERGE
                 GROUP（再定義: Record を返す）
6  高階(7)        EXEC MAP FILTER FOLD SCAN ANY ALL
7  修飾(1)        KEEP
8  辞書(5)        BIND DEF DEL +DEFINED? +DIGEST
9  契約(2)        PROBE +CONTRACT
10 ホスト(4)      PRINT +FORMAT +JSON-DECODE +JSON-ENCODE
```

新語 35、削除 1（`SUM`）、66 − 1 + 35 = 100。

### 4.3 字句文法は一つも増えない

Record リテラルの構文は**作らない**。Record は `RECORD`（キー列と値列から構築）で
のみ生まれる。`spec/grammar.json` は無改訂で通る。100 語の再設計で新しい字面が
一つも増えないことは、この案の安全弁である。

### 4.4 採らなかった語（待機リスト）と理由

| 語 | 級 | 落とした理由 |
| --- | --- | --- |
| `SUM` | なし | `FOLD ADD` と等コスト。現行唯一の冗語なので削除側に回した |
| `PRODUCT` | B | `SUM` を落とすなら対の相手も要らない |
| `UPPER` / `LOWER` | A | Unicode のケース表を信頼カーネルに入れる覚悟が要る。ロケール問題も抱える。**2 枠空けば最優先で復帰**（テキスト処理の実用上は痛い欠落である） |
| `COST` | — | Record 導入後は `CONTRACT` の戻り値が Record になるので `'cost' AT` で足りる。**Record が語を一つ節約した実例** |
| `USES` | A | 辞書の依存辺の読み出し。AI の再構成には効くが、`DIGEST` で当面代替できる |
| `FIND`（述語で最初の要素） | B | `FILTER` ＋ `0 GET` と同コスト。短絡が観測できるのは副作用付きブロックのときだけ |
| `TRANSPOSE` | B | `ZIP` が行列転置を、`RANK` が一般の軸適用を担う |
| `ROTATE` | B | `TAKE` `DROP` `CONCAT` で書ける |
| `NOW` | — | **入れてはならない。** 言語内に時計を持つと決定性と監査性が壊れる。時刻はホストからデータとして入る（§9 参照） |
| `TRY`（ERROR 捕捉） | — | `LANG.FAILURE.TRICHOTOMY` に反する。§0.3 の禁止事項 |

---

## 5. 新語の契約スケルトン

`docs/dev/_attachments/vocabulary-100-draft-contracts.json` に、新語 35 件と
再定義 3 件（`GROUP` `TALLY` `PROBE`）の契約を `spec/words.json` の `entries` と同じ形で
置いた。**このファイルは非正典である。** 正典レジストリに直接足さない理由:

`spec/words.json` は 10 本以上のゲート（`word:manifest:check`,
`word-registry:check`, `core-word-docs:check`, `check:conformance-coverage`,
`check:unreachable-contract`, `semantics:table:check`, `outcome-bijection:check`,
`check:traceability` ほか）の入力である。実行器のない語を登録した瞬間に全部落ちる。
したがって**語は Phase ごとに実装と同時に正典へ移す**（§7）。添付はその移送元である。

添付の各エントリで、実装者が判断を引き継ぐ欄:

| 欄 | 本書で確定済みか |
| --- | --- |
| `name` `aliases` `family` `category` `stack` `consumption` | 確定 |
| `nilPolicy` `projection` `errorWhen` `partiality` | 確定（§6.2 の新結末 ID に依存する箇所のみ要登録） |
| `purity` `determinism` `effects` | 確定 |
| `cost` | 確定（`class` のみ。`exact` は実装時に確認） |
| `clauses` | **暫定**。新設クローズ（`LANG.RECORDS.*` ほか）の ID は §6.1 の正典改訂で確定する |
| `documentation` | **暫定**。`summary` は設計意図を述べたもので、Reference 品質の文面は実装時に書き直す |
| `executorKey` | **暫定**。Rust 側の命名規約に合わせる |

### 5.1 添付の検証状況（実測）

添付の 38 件から `draftStatus` / `draftPhase` / `draftNote` の 3 欄を落とした形を
`spec/words.schema.json` の `word` 定義に突き合わせた結果:

| 検査 | 結果 |
| --- | --- |
| 必須欄の欠落 | 0 件 |
| 未知の欄 | 0 件（draft 3 欄を落とした後） |
| enum 違反 | **14 件**——すべて `family` が `record` / `shape` であること |

つまり添付は、**§6.2 の `family` enum 拡張ただ一つを除いて、そのまま正典に移せる形に
なっている。** 移送時は上記 3 欄を落とすこと（`words.schema.json` は
`additionalProperties: false`）。

なお `npm` の依存が本環境に入っていないため、Node 製のゲート群
（`word:manifest:check` ほか）は未実行である。添付は非正典であり、
どのゲートの入力にもなっていないので現時点で落ちるゲートはない。

---

## 6. 正典ソースへの波及

### 6.1 散文（`spec/language-semantics.md`）

| クローズ | 改訂内容 |
| --- | --- |
| `LANG.AUTHORITY.IDENTITY` | 語数表（66 → 100）。kernel/standard の再配分 |
| `LANG.VALUES.DISJOINT` | 6 領域 → **7 領域**（Record を追加）。「Record は Vector ではない」を明記 |
| `LANG.VALUES.EXACT` | Tier 2 の目撃者が `PI` だけでなくなる旨 |
| `LANG.VALUES.NIL` | 理由空間が閉じた `core` と**利用者宣言**の二層になる旨（`ABSENT`） |
| `LANG.FAILURE.TRICHOTOMY` | ERROR を利用者が宣言できる旨（`FAIL`）。捕捉できないことは不変 |
| `LANG.COLLECTIONS.LIFT` | Record への持ち上げ（§2.3 封じ込め規則 1）。shape の観測可能性 |
| **`LANG.RECORDS.*`（新設）** | 第 7 領域の構造・同一性・キー順序・封じ込め規則 |
| **`LANG.COLLECTIONS.RANK`（新設）** | 深さ指定の適用。修飾軸を増やさない旨 |
| `LANG.DICTIONARY.RESOLUTION` | Core 語数 66 → 100 |
| `LANG.CONTRACT.CHECK` | `CONTRACT` / `PROBE` が Record を返す旨 |
| `LANG.OBSERVATION.PROTOCOL` | 新 wire type `record` |

### 6.2 データ（要編集の正典ソース）

| ソース | 改訂内容 |
| --- | --- |
| `spec/words.json` | 語の追加（Phase ごと） |
| `spec/words.schema.json` | `family` enum に `record` `shape` を追加 |
| `spec/outcomes.json` | NIL 理由に `userDeclared`（**パラメータ付き**）。ERROR に `declaredFailure` `unsortedInput` `duplicateKey` `nonRecord` `notASymbol`。キー不在は既存 `missingField`、JSON の破損は既存 `invalidEncoding` を再利用する（添付の `newOutcomes` が全件） |
| `spec/host-protocol.schema.json` | wire type `record`（キー列と値列を持つ節点） |
| `spec/identity.json` | Record の同一性判定の段位 |
| `spec/semantic-families.json` | 新族 `record` `shape` の法則 |
| `spec/termination.json` | `RANK` の再帰点と減少量（深さ）。**有限性の議論が増える唯一の語** |
| `spec/grammar.json` | **無改訂**（§4.3） |

### 6.3 `userDeclared` が結末の全単射に与える影響（要注意）

`ABSENT` は利用者の書いたテキストを理由にする。`LANG.VALUES.NIL` が
「理由が NIL の観測可能な内容のすべて」と言う以上、そのテキストは理由の一部である。
つまり**理由空間が初めて開いた族を持つ**。`outcome-bijection:check` は宣言済み結末と
目撃者の全単射を検査しているので、レジストリ側に

- `userDeclared` を 1 エントリとして登録し、
- `parameterized: true`（テキスト引数を取る）を立て、
- 目撃者は「`'why' ABSENT` が `userDeclared` を返す」1 件で足りる

という扱いを入れる。これをやらずに `ABSENT` を入れるとゲートが落ちる。
`FAIL` の `declaredFailure` も同じ扱いとする。

---

## 7. Phase 計画

各 Phase は独立して green になる単位である。**Phase ごとに新しいブランチを切る。**
順序は「正典への波及が小さく、他の Phase の前提になるもの」から。

| Phase | 内容 | 語数 | 前提 |
| --- | --- | --- | --- |
| 1 | 族の閉包と削除: `CEIL` `DROP` 追加、`SUM` 削除。命名規律（§1.2）の確認 | +2 −1 | なし。最小の往復で全ゲート経路を通す |
| 2 | 形と階: `SHAPE` `RESHAPE` `FLATTEN` `DEPTH` `RANK` | +5 | `termination.json` に `RANK` の減少量 |
| 3 | 走査の非線形化: `MEMBER` `BSEARCH` `SEARCH` `REPLACE` | +4 | なし |
| 4 | 結末の利用者宣言: `ABSENT` `FAIL` | +2 | §6.3 のレジストリ改訂が先 |
| 5 | 第 7 領域: Record 9 語（`GROUP` 移動・`TALLY` 再定義を含む） | +8 | §2.3 封じ込め規則の正典化。**最大の Phase** |
| 6 | 反射と境界: `DEFINED?` `DIGEST` `CONTRACT` `FORMAT` `JSON-DECODE` `JSON-ENCODE` | +6 | Phase 5（Record を返すため） |
| 7 | 数の払い切り: `POW` `GCD` `RATIO` `EXP` `LN` `SIN` `COS` `ATAN` | +8 | 信頼数値カーネルの拡張。**最後** |

累計: 66 − 1 + 2 + 5 + 4 + 2 + 8 + 6 + 8 = 100。

### 7.1 実施状況

| Phase | 状態 | 記録 |
| --- | --- | --- |
| 1 | **実施済み** | `CEIL` `DROP` を追加、`SUM` を削除。語数 67（Kernel 36 / Standard 31）。`CEIL` は alpha 期の退役語リスト（`scripts/check-minimal-core.mjs` の `REMOVED`）に載っていたので、そこから外して復帰させた——`UNIQUE` と同じ経路。`TAKE` と `DROP` は一つの実行器（`split_by_count`）を共有し、末尾越えの投影・不正カウントの ERROR を同じ場所で読む |
| 2 | **実施済み** | `SHAPE` `RESHAPE` `FLATTEN` `DEPTH` `RANK` を追加。語数 72（Kernel 41 / Standard 31）。添付からの逸脱が三つ: (a) 新クローズ `LANG.COLLECTIONS.RANK` は作らず、`LANG.COLLECTIONS.LIFT` と `LANG.COLLECTIONS.HIGHER` の既存段落に書き足した——`language-semantics.md` の行数予算（404 行）に新しい節を入れる余地がなく、規律も「節を足すより短くせよ」であるため。(b) 族 `shape` も作らず、4 語は `collection` 族に置いた——共有する法則は `collection` 族のそれと一致し、族 enum の拡張は `record` の要否と一緒に Phase 5 で判断する。(c) `RANK` のオペランド順は `[ vec ] n [ body ] RANK`（`FOLD` と同じく、ブロックを直前に置く）。契約推論の「直前の `[ ... ]` はコード」という判定にそのまま乗るためで、添付の `[ vec ] [ body ] n` は捨てた。ついでに、その判定表から漏れていた `SCAN` も足した |
| 3 | **実施済み** | `MEMBER` `BSEARCH` `SEARCH` `REPLACE` を追加。語数 76（Kernel 41 / Standard 35）。四語とも Standard の `operational`（`INDEX-OF` で書ける答えを、費用のために核に残す）。`BSEARCH` は昇順検査を O(n) で先に行い、乱れていれば新しい ERROR 分類 `unsortedInput`（`spec/outcomes.json` に `declared` として登録、`spec/outcome-witnesses.json` に目撃者）。比較が予算を使い切れば `SORT` と同じく `undecidable` を投影する。`REPLACE` は alpha 期の退役語リストに載っていたので `CEIL` と同じ経路で復帰 |
| 4 | **実施済み** | `ABSENT` `FAIL` を追加。語数 78（Kernel 43 / Standard 35）。§6.3 のとおり `spec/outcomes.json` に `userDeclared`（`parameterized: true`）と `declaredFailure`（`declared`）を登録し、目撃者は各 1 件。宣言テキストは `AbsenceMetadata.detail`（`Arc<String>`——値の封筒をポインタ 1 本分しか広げないための thin pointer）に載り、`Value` の同一性とハッシュに入る。`NIL-REASON` は `userDeclared` の NIL に対して識別子ではなくテキストを答える。プロトコルには `semantics.absence.detail`、永続化には `ud` / `absent_detail` を足した。`FAIL` は `AjisaiError::declared("declaredFailure", text)` で、オペランドを復元してから停止する |
| 5 | **実施済み** | `RECORD` `KEYS` `VALUES` `AT` `WITH` `WITHOUT` `HAS?` `MERGE` を追加し、`TALLY` `GROUP` を Record を返す語に再定義（族 `record`、Standard `operational` のまま）。語数 86（Kernel 51 / Standard 35）。第 7 領域は `ValueData::Record`（`RecordData`: キー列・値列・ハッシュ索引）で、同一性は二列の denotation、キー順は観測可能構造。表示は `{ 'x': 1/1 }`——`{` `}` は退役字句なのでリテラルと取り違えようがない。プロトコルは wire type `record`（`keys` / `values` の二配列）、永続化も同形。封じ込め規則 1 は `record_lift`（算術・比較 20 語の入口で値方向へ持ち上げ、二つの Record はキー列が等しいときだけ対で組み、違えば `shapeMismatch`）、規則 2 は各語の既存 ERROR がそのまま効く（Record は Vector でないので `nonVector` 系）。新クローズ `LANG.RECORDS.STRUCTURE` を `LANG.VALUES.VECTOR` の直後に置き、`language-semantics.md` の行数予算を 404 → 408 に上げた（第 7 領域は既存節に書き足せる量ではない）。添付からの逸脱: (a) 族 `shape` は作らず（Phase 2 の判断を維持）、`record` だけを足して族は 12 で上限。(b) `WITH` は `rejectNil` でなく `consumeNil`——NIL は「キーの下に蓄えた不在」として値になれるべきで、NIL の Record・NIL のキーだけを語自身が拒む。(c) `nonRecord` は Record 語が Record 以外を受けたときの分類にとどめ、規則 2 には使わない。(d) キーは任意の値（Text に限らない）——`TALLY` の鍵が要素そのものである以上、制限する理由がない。新結末 `duplicateKey` `nonRecord`、`spec/identity.json` に段位 `recordDenotation`、`README` の 10 概念を §3 の表へ書き換え |
| 6〜7 | 未着手 | — |

Phase 7 を最後に置くのは、ここだけが語彙と契約の仕事ではなく**数値カーネルの工事**
だからである。Phase 1〜6 が終わった時点で語数は 92 であり、そこで止めても言語は
一貫している（その場合 §2.1 の撤退案に切り替え、`PI` を削って 91 語とする選択肢が
残る——**この分岐点は Phase 7 開始時にもう一度所有者に確認すること**）。

---

## 8. 危険と未解決

- **適合コーパスが 1.5 倍になる。** 語ごとに族法則（arity・KEEP・NIL 方針・投影・
  ERROR 境界・持ち上げ・純粋性・効果）の試験が要る。語数の増加より試験の増加の方が
  先に効いてくる。
- **Record は既存語の再レビューを強制する。** §2.3 の封じ込め規則で有界化したが、
  「規則 2 で片付く」と書いた語が本当に片付くかは Phase 5 で 1 語ずつ確認するしかない。
- **`RANK` は有限性の議論を増やす唯一の語。** 深さを減少量とする停止性の議論を
  `spec/termination.json` に書く必要がある。書けなければ `RANK` を落とす。
- **100 語は一人の頭の上限に近い。** 緩和策は §1.2 の命名規律と、Reference を
  §3 の概念順で並べることだけである。効果は未測定。
- **`UPPER` / `LOWER` の欠落はテキスト処理で痛い。** 待機リスト最上位。
- **AI にとっての可読性は未測定。** 「66 語より 100 語の方が AI がよく書ける」という
  主張は本書のどこにも根拠がない。MCP の eval（`npm run eval:mcp`）で Phase 前後を
  比較できるはずだが、本書はその計測を計画していない。

---

## 9. 付随して見つかった実装・正典のずれ

本書の調査中に見つかったもの。いずれも本書の Phase とは独立に直せる。

1. **kernel/standard の語数が正典と記録で食い違う。**
   `spec/language-semantics.md` の `LANG.AUTHORITY.IDENTITY` は 37 / 29 と述べるが、
   `spec/words.json` の `vocabularyTier` 実測と `docs/word-manifest.json` の
   `counts`（記録の正）は **36 / 30**。正典側の数字が古い。
2. **`datetime` を作る語が存在しない。** `Interpretation::Timestamp` と wire type
   `datetime` が実装と `spec/host-protocol.schema.json` にあるのに、それを生む語が
   語彙にない。`rust/src/interpreter/execution_loop.rs` の `apply_word_hint_override(`
   が `"NOW" | "TIMESTAMP"` を参照しているが、両方とも存在しない語である。
   **`NOW` を足すのではなく role ごと削除するのが筋**（§4.4）。
3. **同テーブルに 215 語時代の亡霊が残っている。** `SQRT_EPS` `MATH@SQRT` `LOWER`
   `UPPER` `WIDTH` `STARTS-WITH?` `ENDS-WITH?` `UNFOLD` `REORDER` `SPLIT` `CONSERVE`
   `REFLECT` `BOOL` `INTERVAL` は現行語彙に存在しない。死んだ分岐である。
4. **`INDEX-OF` の hover に旧語彙の痕跡。** 「Bubble/NIL」という語は現行の散文に
   存在しない（`docs/word-manifest.json` 経由で Reference に出る）。
