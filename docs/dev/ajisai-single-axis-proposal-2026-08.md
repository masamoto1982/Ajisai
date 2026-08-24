# 単一軸への収束案 — 「絞り込み（narrowing）」を Ajisai の唯一の中心に置く

> Status: **Non-canonical / 提案 `[観察ノート]`.** 本書は Ajisai の意味論を一切定義しない。
> 正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。
> 本書は「Ajisai の中心概念を一つに定める」ための提案であり、**未承認**である。
> 正典側の変更は一切含まない。実装への反映は仕様所有者の承認を前提とする。
>
> 関連: `docs/dev/concept-reduction-2026-07.md`（十概念への削減）・
> `docs/dev/trichotomy-unification.md`（三分法統一と案(b)の保留理由）・
> `docs/dev/vector-nesting-role-redefinition.md`（Lisp 的動機の廃止）・
> `docs/dev/language-coherence-review-2026-08.md`（外部改修案の棄却）・
> `docs/dev/ajisai-minimal-core-identity.md`（同一性の幹）。

## 0. 本書が答えようとしている問い

「Ajisai を、LispやJSONのように *発明されたのではなく発見された* と言われる
プロダクトにしたい」という目標に対し、現状で何が足りず、何を変えれば届くか。

外部評の要約はこうである——**Ajisai には強い着想が複数、並立している。
「これが中心だ」と一言で言える点がまだ分散している。**

本書はこの観察に同意した上で、**中心の候補を一つ提示し、そこから既存の十概念が
導出されることを示し、導出に乗らない部分の除去を提案する**。

---

## 1. 診断 — 「十概念」は削減の結果であって、導出ではない

`concept-reduction-2026-07.md` は約60の設計コンセプトを十に削った。これは
大きな成果であり、本書はその判断を一切覆さない。しかし残った十は、

> 1. 厳密有理数と `SQRT` 閉包 / 2. 三分法 / 3. スタックと Vector / 4. コードブロック /
> 5. 消費修飾子 / 6. 二階層辞書 / 7. 機械可読契約 / 8. 実行前検査 /
> 9. ホストプロトコル / 10. conformance corpus

——**互いに導出関係がない十個の並び**である。「なぜこの十個なのか」「なぜ十一個目が
ないと言えるのか」に、リストそのものは答えない。

Lisp と JSON が「発見された」と感じられるのは、数が少ないからではない。
**一つの構造から残り全部が落ちてくる**からである。cons があれば、コードもデータも
評価器も落ちてくる。キーと値の対があれば、あらゆる構成データが落ちてくる。
Ajisai の十概念には、この落下がまだない。

### 1.1 中心がないことの物的証拠

これは印象論ではない。中心がない言語では、**削除した概念の残骸を「これは死んでいる」と
判定する基準が存在しない**。実測すると、正典 `spec/words.json` に次が残っている。

| 残骸 | 出典（削除済みの概念） | 実装からの参照 |
| --- | --- | --- |
| `interpretationRole`（65語全件・値域6） | 「解釈ロール」。`concept-reduction` §3 が明示的に破棄 | **0件** |
| `capability`（65語全件） | capability ゲート付き hosted effect | **0件** |
| `hostedEffect`（65語全件） | 同上（output 以外の hosted effect は全廃） | **0件** |
| `errorWhen: stackTargetMode`（6語） | 削除済みの TOP/STAK 目標軸 | **0件** |
| `errorWhen: nonNumericOrInterval` / `negativeInterval`（SQRT） | 削除済みの区間演算・computable real 階層 | **0件** |
| `category: tensor`（1語） | 削除済みのテンソル代数 | — |

`LANG.CONTRACT.REGISTRY` は「`words.json` が Word の権威であり、散文はその射影である」と
規定している。したがってこれらは *散文の書き残し* ではなく、**正典が現在も宣言している
到達不能な契約**である。`SQRT` の契約は、この言語に存在しない「区間」に対する
エラー条件を、いま正典として持っている。

さらに、65語には**10本の分類軸**（`family` 12値 / `category` 16値 / `vocabularyTier` 2値 /
`interpretationRole` 6値 / `acceptedDomain` 4値 / `partiality` 3値 / `nilPolicy` 7値 /
`consumption` 4値 / `purity` 3値 / `determinism` 3値）が同居している。
`family` と `category` は目的が重複し、`partiality` は `nilPolicy` と `projection.when` から
導出できる。

**中心が一つあれば、これらは全部「軸に乗らない」で一撃で落ちる。**
落ちていないという事実そのものが、中心が無いことの証拠である。

### 1.2 水のメタファーについて（率直な意見）

Vessel / Water / Flow / Ripple / Bubble / Breach は**命名としては優れている**。
`bubble` と `breach` は、`null` と `exception` より確実に良い名前である。

しかしこれは**中心ではない**。中心の資格は「そこから他が導出されること」であり、
水の比喩からは何一つ導出されない。「なぜ数は厳密でなければならないのか」に
水は答えられない。README の「The language in one picture」の位置に置くべきものは、
答えられる方である。

**提案:** 水は語彙規約（`docs/dev/` の命名ガイド）へ降格し、README の一枚絵の位置には
§2 の軸を置く。`bubble` / `breach` の語は残す。

---

## 2. 提案する中心 — 単一の軸「絞り込み（narrowing）」

> **Ajisai の操作はただ一つ、記述を値へ絞り込むことである。
> 十の概念はすべて、この軸の上のどこに立つかで決まる。**

- **記述（description）** — まだ絞り込まれていないもの。`{ 1 2 + }`、Word の契約、
  ユーザ定義の `#:contract` 宣言。
- **値（value）** — 絞り込みが終わったもの。`3`、`[ 1 2 3 ]`、`'hello'`。
- **絞り込み（narrowing）** — 記述を値へ近づける唯一の操作。実行はその一形態である。

軸の上のどの高さでも、絞り込みの結果は**必ず三つのいずれか**になる。

| 結果 | 実行時（値に対して） | 検査時（記述に対して） |
| --- | --- | --- |
| 値へ絞れた | 値 | `verified`（推論された契約そのもの） |
| 何も残らなかった（理由付き） | `NIL` + `reason` | `cannot verify` + `gap.*` |
| そもそも絞り込みが定義されない | ERROR（breach） | `violated`（`contractViolation`） |

`trichotomy-unification.md` は、この対応が**類似ではなく同一である**ことを、
三つの独立した構造的証拠（gap の伝播が NIL passthrough と同形であること、
`ajisai contract` が既に「値」の場合を実装していること、gap id と NIL reason が
同種のオブジェクトであること）で既に確立している。

**本書の主張は、この既に発見された事実を、比較表ではなく言語の中心に据えることである。**
実行時三分法と検査時三値は「二つの三分法が偶然似ている」のではない。
**一つの軸を二つの高さで切っただけ**である。

### 2.1 十概念が軸から落ちること

| 概念 | 軸からの導出 |
| --- | --- |
| 1. 厳密数 | **丸めは軸を逆走する。** 丸めた値は、絞り込まれた記述ではなく *広げられた* 記述である。近似を許す言語には軸が定義できない。**「厳密」は好みではなく、軸の存在条件である。** |
| 2. 三分法 | 軸の上の任意の点で絞り込みが取りうる結果、その全部（§2 の表） |
| 3. スタックと Vector | 軸の**絞り切った端**。値の置き場 |
| 4. コードブロックと明示評価 | 軸の**絞り切っていない端**。「明示評価」とは「絞り込みは自動では起きない」の言い換え |
| 5. 消費修飾子 | 絞り込みが入力を消費するかどうか。軸そのものではなく、軸を進むときの副条件 |
| 6. 封印 Core / User | Core = 絞り込みの**公理**。User = 公理から導いた絞り込み。封印は公理系の閉性 |
| 7. 機械可読契約 | 記述に対する絞り込みの記述。**軸を一段上がったところの Word** |
| 8. 実行前検査 | 記述に対して絞り込みを実行すること。**7 を走らせること以外の何物でもない** |
| 9. ホストプロトコル | 軸を**外から観測する唯一の窓** |
| 10. conformance corpus | 絞り込みの**不動点の一覧**。「これが Ajisai か」を決める |

十概念のうち十が落ちる。**そして 1 が「導出」になることが、この軸の最大の収穫である。**
現行の正典は「数は厳密である」を宣言している。軸の下では、それは宣言ではなく定理になる。

---

## 3. 改修案

大胆な順に並べる。番号は優先度ではない。**Ⅲ・Ⅴ・Ⅵ・Ⅶ は独立に実施でき、
Ⅰ・Ⅱ を採らなくても価値がある。**

---

### 改修 Ⅰ — 容器を一つにする（`{ }` と `[ ]` を同一領域の二つの綴りにする）

**現状。** `CodeBlock` と `Vector` は互いに素な二領域であり（`LANG.SOURCE.CODE`）、
両者を渡る唯一の橋として `REFLECT` があり、その中間表現は
`[ 'AJISAI-CODE-1' [ 'symbol' 'PRINT' ] ... ]` というバージョン付きタグ配列である
（`rust/src/types/code_data.rs`）。

**軸から見ると。** これは二領域ではない。**同じ構造の、軸の上の二つの高さ**である。
`{ 1 2 + }` と `[ 1 2 + ]` は、名前が解決済みか否かだけが違う。
`REFLECT` は「橋」ではなく、**軸を持たない言語がその高さの差を言い表せないために
必要になった代用品**である。

**提案。** 記述の領域を一つにし、`{ }` と `[ ]` を**同一の値領域に対する二つの綴り**とする。
綴りの差は人間向けの注記であって、意味論上の差ではない。

- 「Vector は決して実行可能でない」は「**記述は明示的に絞り込まれるまで実行されない**」へ
  一般化される。これは弱い規則ではなく、**全データに一様に効く強い規則**である。
- `[ FOO ]` が辞書状態に依らず同じ値を表す性質は保たれる。値は変わらず、
  絞り込んだときに何が起きるかだけが辞書に依存する（現在の CodeBlock と同じ）。
- `REFLECT` は恒等写像になり、消える。`AJISAI-CODE-1` タグとそのバージョンも消える。

**収支。**

| 減るもの | 増えるもの |
| --- | --- |
| 値領域 `CodeBlock`（概念 −1） | 値領域 `Symbol`（概念 +1) |
| Core Word `REFLECT`（65 → 64） | — |
| 正典節 `LANG.SOURCE.REFLECTION`（−1） | — |
| `AJISAI-CODE-1` バージョンタグと `code_data.rs` の検証経路 | — |
| 「コードとデータの境界」という規則 | 「絞り込みは明示」という既存規則の再利用 |

領域数は差し引き 0 だが、**`Symbol` は新概念ではない**。`REFLECT` の中間表現に
`[ 'symbol' 'PRINT' ]` として既に存在している。この改修は概念を増やすのではなく、
**符号化されて隠れていた概念を昇格させ、符号化を消す**。

**得られる能力。** `COND` の節（`{ guard } { body } ... COND`）とデータが同じ形になるため、
プログラムが実行時にデータから制御構造を組み立てられる。現在これは `REFLECT` と
マジックタグを経由しないとできない。

**過去の判断との関係（重要）。**
`vector-nesting-role-redefinition.md` は「Lisp 的動機（homoiconicity 志向）は
設計判断の根拠として今後一切用いない」と記録している。本書はこれを**尊重する**。
本提案の根拠は Lisp への憧れではなく §2 の軸である。

その上で事実確認が要る。同文書 §2「存続の根拠」が挙げたネストの依存先——
テンソル語（`SHAPE` `RANK` `RESHAPE` `TRANSPOSE`）、`JSON` モジュール、`SPLIT`、
音楽 DSL のマルチトラック——は、**`concept-reduction-2026-07.md` により全て削除済み**である
（65語に一つも残っていない。生存は `FILL` と要素単位持ち上げのみ）。
つまり当時「動機の差し替え」を選んだ理由の側が、いま存在しない。
**判断そのものは有効だが、記録された根拠は失効している。**
再検討は、私の趣味ではなく、リポジトリの現状が要求している。

**破壊的変更である。** `[ FOO ]` の要素が Text から Symbol へ変わり、
`{ 1 2 + } [ 1 2 + ] EQ` が `TRUE` になる。ベータ互換方針（「一つの現行形式のみ、
変換器を置かない」）の下では、これはバージョンを上げる変更である。

**リスク。** 本書中で最も高い。`{ }` と `[ ]` が同値になると、`MAP` の第二引数が
記述であることを示す**構文的手掛かりが弱まる**（Lisp が quote と位置で解いた問題と同型）。
綴りを二つ残すのはこの緩和策だが、綴りが意味を持たない以上、規約でしかない。

**推奨。** 即時採用ではなく、**別ブランチでのスパイク**として。判定基準は
「conformance corpus が何件通らなくなるか」と「Reference の例が読みにくくなるか」の二点。

---

### 改修 Ⅱ — 検査を Word にする（軸を一段上げる操作を言語の中へ）

**現状。** 契約推論は `rust/src/agent/contract_*.rs` に実装され、
`ajisai check --contract` / `ajisai contract` という **CLI からしか届かない**。
Ajisai プログラムは自分自身の検査結果を読めない。

**軸から見ると。** 検査は「記述に対する絞り込み」であり、実行と同じ操作である。
それが言語の外にあるのは、**軸が言語の中心でないことの直接の帰結**である。

**提案。** 既存の推論エンジンを Core Word として露出する。

```
記述  操作対象の記述（CodeBlock / 改修Ⅰ後は記述）
知識  オペランドについて分かっていることの Vector（契約レコード）
PROBE 結果の知識 Vector を返す
```

- **新しい値領域を作らない。** データ入力・データ出力である。契約レコードは
  ただの Vector（`words.json` のレコードと同じ形）。
- **第四の結果カテゴリを作らない。** `PROBE` の返り値は普通の値であり、
  実行時の三分法は 3 のままである。
- `gap.*` id を NIL reason レジストリへ統合し、`NIL-REASON` で読めるようにする。

**`trichotomy-unification.md` の保留理由に対する反論。**
同文書は案(b)（gap を NIL reason レジストリへ統合）を「payoff は *検査結果が
Ajisai プログラムから扱えるようになること* だが、それは検査器自身が言語の中から
到達可能なとき——すなわちセルフホストのとき——にしか存在しない」として保留した。

**この推論には飛躍がある。** 検査器が Ajisai *で書かれている* ことと、
Ajisai *から呼べる* ことは別である。`PROBE` があれば、Rust 実装のままで payoff は成立する。
セルフホストは(b)の必要条件ではない。保留の技術的理由は、`PROBE` の導入によって消える。

**得られるもの。** 「検査と実行は同じ操作である」が、設計メモの主張から
**プログラムで示せる事実**になる。エージェントが自分の書いたコードを実行前に
Ajisai の中で検査し、その結果に応じて分岐できる。AI-first を掲げる言語として、
これは CLI フラグより本質的に強い。

**語名。** `PROBE` を第一候補とする。`FORECAST` / `ASSAY` / `SOUND`（測深）も候補。
水の語彙に寄せるなら `SOUND` が意味的に最も正確だが、音との衝突がある。

**コスト。** Core 65 → 66。`concept-reduction` の「増えるのは意図的な仕様変更でしか
ありえない」に該当し、`npm run semantic-kernel:check` の上限更新を伴う。
**ただし §1 の死語を落とせば正味は減る。**

---

### 改修 Ⅲ — 到達不能な契約を正典から外し、再発を CI で止める（低リスク・即時可能）

**根拠は §1.1 の実測。** 提案は二段構えである。

**(a) 除去（要・仕様所有者承認）**

| 対象 | 件数 | 実装参照 |
| --- | --- | --- |
| `interpretationRole` フィールド | 65語全件 | 0 |
| `capability` フィールド | 65語全件 | 0 |
| `hostedEffect` フィールド | 65語全件 | 0 |
| `errorWhen: stackTargetMode` | 6語 | 0 |
| `errorWhen: nonNumericOrInterval` / `negativeInterval` | SQRT | 0 |
| `category`（`family` へ畳む） | 65語全件・16値 | 要精査 |
| `partiality`（`nilPolicy` + `projection.when` から導出） | 65語全件 | 要精査 |

`spec/words.schema.json` と `scripts/generate-word-registry.mjs` の同時更新を伴う。

**(b) 再発防止 — 「到達不能契約」ゲート（本改修の本体）**

新しい CI チェックを提案する。

> **`spec/words.json` の全フィールドと全 enum 値は、実装または生成器の少なくとも
> 一箇所から読まれていなければならない。読まれていない契約は、正典が宣言している
> 到達不能な約束である。**

これは `npm run semantic-kernel:check` が概念の**数**を上限で止めているのに対し、
概念の**生死**を止める。数の上限だけでは、死んだ概念は数に含まれたまま残り続ける
（現に残っている）。

**なぜこれが「発見された感」に効くのか。** 発見された言語には、後から足された
説明のつかない部分がない。このゲートは、説明のつかない部分が**入れない**ことを
機械的に保証する。中心を宣言するだけでは中心は保たれない。**中心を保つ機構が要る。**

---

### 改修 Ⅳ — 数値領域を宣言ではなく導出にする

**現状。** 正典は「有理数と `SQRT` 閉包が数値領域の全てであり、π・e・対数は
Ajisai の値ではない」と**宣言**している。これは読者に「なぜ平方根までで、
立方根は駄目なのか」「なぜこの線引きなのか」を残す。線引きが恣意的に見える限り、
**発明されたように読める**。

**軸から見ると。** 値とは絞り込みが終わった記述である。π の記述は終わらない。
したがって π は値ではない——**これは好みの表明ではなく、軸の定義から出る帰結**である。

**提案。** `LANG.VALUES.EXACT` を次の形に組み替える。

> Ajisai の数は、Core 算術の下で閉じており、かつ実装が提示する正規形によって
> 等号と順序が有限時間で決定できる実数の全体である。
> \(\mathbb{Q}(\sqrt{d_1},\dots,\sqrt{d_k})\) は**この条件を満たす現在の証拠**であって、
> 条件そのものではない。

**得られるもの。**
- π の除外が「捨てた」ではなく「軸に乗らない」になる。
- 将来の立方根や一般代数的数への拡張が、**原理の変更ではなく証拠の差し替え**になる。
  言語が数について恣意的な好みを持っているように見えなくなる。
- 「比較は全域である」が、独立した性質ではなく**同じ条件の片割れ**になる。

**コスト。** 正典 1 節の書き換え。実装変更なし。`words.json` に判定基準を
記録するかは要検討。**費用対効果は本書中で最も高い。**

---

### 改修 Ⅴ — 同一性の一法則を立て、散在する三つの規定を畳む

**現状。** 「値は、それがどう作られたかではなく、何を表すかである」という同じ主張が、
正典の三箇所に別々に書かれている。

| 箇所 | 記述 |
| --- | --- |
| `LANG.VALUES.EXACT` | \(\sqrt8\) と \(\sqrt2+\sqrt2\) は一つの値である |
| `LANG.VALUES.NIL` | 同じ reason の二つの NIL は同じ値である |
| `LANG.DICTIONARY.MUTATION` | 同じ content identity の二つの定義は同じ Word である |

**提案。** 一つの法則を立てる。

> **`LANG.IDENTITY.DENOTATION` — 値とは、それが表すものであり、
> どう作られたかではない。**

三つの規定はその**実例**になる。さらに、現在は選択として書かれている
「AI 可読表示は元のソースリテラルを覚えず、値から導いた連分数形を使う」も、
この法則からの**帰結**になる（歴史を覚える表示は、この法則に反する）。

**軸との関係。** 絞り込みは構成上、歴史を忘れる。この法則は軸の系である。

**コスト。** 正典に 1 節追加、3 節を参照へ書き換え。実装変更なし。conformance
も既存のまま。**最も安く、最も「発見された」感に効く。**

---

### 改修 Ⅵ — フレーム表を四行から二則へ

**現状。** `LANG.SOURCE.FRAME` は「ブロックが何を見て何を残すか」を**四行の表**で
定めている（`DEF`/`EXEC` / `COND` / `MAP`・`FILTER`・`ANY`・`ALL` / `FOLD`）。
これは正典の中で最も「発明された」ように読む箇所である。四つの呼び出し規約が
並んでいるように見えるからである。

**実際には二則しかない。**

1. **全スタック** — `EXEC` と `DEF` の本体。ブロックはスタック全体を見て、
   いくつでも残す。
2. **隔離 n入力・1出力** — それ以外全部。`COND`・`MAP`・`FILTER`・`ANY`・`ALL` は
   n=1、`FOLD` は n=2。**n は Word ごとの数値であって規約ではない**——
   そして n は `words.json` の `stack.inputs` に既にある。

**提案。** 表を二則に書き換え、n を契約レジストリからの射影とする。四行の表は
`LANG.CONTRACT.REGISTRY`（散文は契約の射影である）に対する例外になっていた。

**コスト。** 正典 1 節の書き換え。実装変更なし。

---

### 改修 Ⅶ — `examples/` を conformance に接続するか、削除する（衛生・即時）

**実測。** リポジトリ直下の `examples/*.ajisai` は、**テストからも CI からも
一件も参照されていない**（参照があるのは `rust/examples/*.rs` の cargo example のみ）。
そして中身は現行言語ではない。

- `examples/editor-input-assist-sample-test.ajisai` は `SCALAR` `VECTOR` `MATRIX`
  `TENSOR` を説明している。**いずれも65語に存在しない。**
- `examples/cast-operations-test.ajisai` は `[ 42 ] STR` のように、スカラを
  ベクタで包む旧形式を使っている。

README はこのディレクトリを「More examples」としてリンクしている。

**なぜこれが中心の問題なのか。** `LANG.CONFORMANCE.CORPUS` は
「『これは Ajisai か』を決める手続きは corpus であり、散文はそれに答えない」と
規定している。**その言語が、検証されていない example ディレクトリを配って
いてはならない。** 中心を持つとは、中心の外にあるものを配らないということである。

**提案。** 二択。(a) `ajisai test` の対象に組み込み CI で回す。
(b) 削除し、README の導線を Reference（全例が Playground で開ける・検証済み）に一本化する。
**(b) を推す。** Reference が既に検証済みの例を持っている以上、(a) は第二の例集という
第二権威を作る。

---

## 4. 採らない案

| 案 | 採らない理由 |
| --- | --- |
| 「Ajisai 2」として意味論を再確定し、変換器で移行する | `language-coherence-review-2026-08.md` §1 が既に棄却済み。conformance corpus と契約レジストリを同時に捨てることになり、`spec-impl-drift-tactic.md` の第一不変条件（第二権威の禁止）に正面から反する。**本書は正典を作り直さない。** |
| `UNKNOWN` / 三値 Kleene 論理の復活 | 軸はこれを**必要としない**。絞り込みの途中状態は検査時にのみ存在し、実行時の値には決してならない。改修Ⅱが `PROBE` を「データ入力・データ出力」に限定しているのはこのためである。実行時の結果カテゴリは 3 のまま動かない。**改修Ⅱを「UNKNOWN の復活」と読んではならない。** |
| 水のメタファーの拡張・体系化 | 命名としては良いが、そこから何も導出されない（§1.2）。中心の位置に置くと、中心が二つになる。 |
| 現行 GUI・Reference 様式の変更 | `concept-reduction` が維持対象として明示。本書は触れない。 |
| セルフホスト | 改修Ⅱの前提ではない（§改修Ⅱ）。独立の判断であり、本書は立場を取らない。 |

---

## 5. 検証方法

提案の各項は、次で検証できる（**いずれも未実施**）。

| 改修 | 検証 |
| --- | --- |
| Ⅰ | スパイクブランチで conformance corpus を回し、非通過件数を数える。Reference の全例の可読性を目視 |
| Ⅱ | `PROBE` が既存 `ajisai check --contract` と全 corpus ケースで同一結論を出すことの差分テスト |
| Ⅲ(a) | 除去後に `cargo test --lib` / `cargo test --tests` / `npm run check` / `npm run test` が全緑 |
| Ⅲ(b) | 現行 `words.json` に対しゲートを走らせ、§1.1 の6件を検出すること |
| Ⅳ | 実装変更なし。conformance の観測不変を確認 |
| Ⅴ | 同上 |
| Ⅵ | 同上。加えて `stack.inputs` が全ブロック評価語で正しいことの確認 |
| Ⅶ | (b) を採るなら README のリンク切れがないこと |

---

## 6. 課題

- **改修Ⅰの構文的手掛かりの喪失は、規約でしか緩和できない。** `{ }` と `[ ]` を
  同値にした上で綴りを二つ残すのは、意味を持たない綴りを残すということである。
  これが実際に読みにくさを生むかは、スパイクで例を書いてみるまで分からない。
  **本書はこの点について結論を持たない。**
- **改修Ⅱのコスト見積もりをしていない。** 既存推論エンジンの露出とはいえ、
  契約レコードの Vector 表現・`gap.*` の reason レジストリ統合・`words.json` への
  `PROBE` 契約追加の工数は測っていない。
- **改修Ⅲ(a) の `category` と `partiality` は精査していない。** `interpretationRole` /
  `capability` / `hostedEffect` は grep 0件を確認したが、この二つは
  一般語のため grep では判定できず、実際の参照箇所を追っていない。**除去可能と
  断定していない。**
- **§2 の軸が「唯一の」中心であることは証明されていない。** 十概念が落ちることは
  示したが、他により良い中心がないことは示していない。本書は候補を一つ出したに
  すぎない。
- **`examples/` が実際に実行に失敗することは確認していない。** 参照されていないこと
  （grep）と、65語に存在しない語を説明していること（レジストリ照合）は確認したが、
  ビルド済み CLI がないため実行はしていない。
- **本書はどの改修も実施していない。** 正典・実装ともに無変更である。

## 7. 追記（実施済み・改修Ⅵ・Ⅲ）

改修ⅥとⅢ(a)(b)を実施した。実施の過程で、本書 §1.1 と §3 改修Ⅲの記述に**訂正が要る事実誤認**が見つかったので、原文は書き換えずここに記録する。

### 7.1 §1.1「0件」は不正確だった — `capability`/`hostedEffect` は MCP 経由で外部に転送されていた

§1.1 の表は `capability`・`hostedEffect`・`interpretationRole` の実装参照を「0件」としていたが、この grep は `rust/src` と `src` のみを対象とし、**`scripts/` と `tools/mcp-server/` を調べていなかった**。実際には:

- `scripts/generate-word-reference.mjs` が `entry.capability` / `entry.hostedEffect` を読み、`docs/word-reference.md` に `Capability / hosted effect: `none` / `none`` の行として出力していた。
- `tools/mcp-server/index.js` の `word_contract` ツールと `ajisai://words/{name}` リソースは `spec/words.json` のエントリを**そのまま**返す。したがってこの3フィールドは、MCP 経由で接続する外部の AI エージェントから観測可能だった。

`interpretationRole` については訂正なし——上記のいずれからも読まれておらず、`docs/dev/reduction-consistency-audit-2026-07.md` の **D14**（2026年7月・本書執筆前に既に記録されていた判定）が同じ結論に達している。

`capability`/`hostedEffect` は「参照されていない」のではなく、**`effects` フィールド（正典 `LANG.CONTRACT.REGISTRY` が挙げる契約構成要素の一つ）と Rust 側の独立した `WordProfile` 列挙型の両方に完全に重複していた**。`PRINT` の `effects: ["consoleWrite"]` と `hostedEffect: "consoleWrite"` は同じ事実の二重表現であり、`WordProfile::Hosted` の判定は `words.json` を経由せず `name == "PRINT"` から独立に導かれている。除去は「未到達だから」ではなく「正典が既に持つ `effects` と重複するから」を理由に行った。`generate-word-reference.mjs` は `effects` から `**Effects:**` 行を出すよう書き換えた。

### 7.2 §1.1 に未記載だった `errorWhen` の一般化検証は失敗した

改修Ⅲ(b) の当初案は「`errorWhen` の値ごとに Rust ソース中の文字列一致を取る」という汎用ゲートだった。実装して現行 `words.json`（削除前）に対して実行したところ、`stackTargetMode` 等の3件だけでなく、**27件中27件が「未到達」と誤検出された**。原因は、`errorWhen` の値（`nonCodeBlock` 等）の大半が Rust 側では camelCase 識別子ではなく散文のエラーメッセージ（例: `"expected number, got other format"`）としてしか存在しないため。`stackTargetMode`/`nonNumericOrInterval`/`negativeInterval` が死んでいると判定できたのは文字列一致ではなく、**TOP/STAK 修飾子軸と区間演算という概念自体が言語から削除されている**というドメイン知識によるものだった。

このチェックは一般化できないと判断し、**破棄した**。改修Ⅲ(b) で実装した自動ゲート（`scripts/check-unreachable-contract.mjs`、`npm run check:unreachable-contract`）は、**トップレベルの分類フィールド名の到達可能性のみ**を検査する。`errorWhen` 値ごとの到達可能性チェックは自動化されていない——3件の既知の死んだ値は本改修で手作業により削除したが、将来同種の値が紛れ込んでも、このゲートは検出しない。

### 7.3 `category` と `partiality` は精査した結果、除去対象から外した

- **`category`**（16値）は `family`（12値）の**下位区分**であり、5本の生成スクリプト（`generate-core-word-docs.mjs` 等、ドキュメント・skill.md・conformance manifest・word manifest の生成）から実際に読まれ、`family` にはない粒度の情報を運んでいた。重複ではなく、生きた別軸。**除去しない。**
- **`partiality`** は `scripts/generate-word-registry.mjs` により **`rust/src/kernel/generated/word_registry.rs` へコンパイルされ**、`rust/src/builtins/builtin_word_details.rs:149` の `match spec.partiality { ... }` で実際の分岐ロジックに使われていた。`nilPolicy` と `projection.when` から論理的に導出可能だとしても、それは「未到達」とは別の問題であり、除去は契約検証ロジックの書き換えを伴う別規模の作業になる。**除去しない。**

§3 改修Ⅲ(a) の課題節が「精査していない」としていたこの2点は、精査した結果、除去しないと判断した。

### 7.4 実施内容と検証

- **改修Ⅵ**: `LANG.SOURCE.FRAME` の四行表を二則の散文へ書き換えた。当初案の前提「n は `words.json` の `stack.inputs` から導出できる」は誤りだった——`stack.inputs` は呼び出し全体のオペランド数（`MAP` は2＝コレクション＋ブロック）であり、フレームが見る値の数（`MAP` は1＝要素のみ）とは異なる。契約レジストリにフレーム引数専用のフィールドは存在しないため、各語のフレーム引数は散文で直接述べた。
- **改修Ⅲ(a)**: `interpretationRole`・`capability`・`hostedEffect` を65エントリ全件、`errorWhen` の `stackTargetMode`（6語）・`nonNumericOrInterval`・`negativeInterval`（SQRT）を削除。`spec/words.schema.json`・`tools/mcp-server/result.schema.json` の対応する宣言も削除し、`tools/mcp-server/assets/` を再同期した。
- **改修Ⅲ(b)**: `scripts/check-unreachable-contract.mjs` を新設し、Quality Gate に組み込んだ（`npm run check:unreachable-contract`）。範囲は §7.2 の通りフィールド名到達可能性のみ。

検証: `cargo test --all-targets` 1108件全通過、生成 Rust ファイル（`word_registry.rs`・`generated_core_word_docs.rs`）に差分なし（＝削除は Rust 側の型・分岐に一切影響しない）。npm ゲート31ステップ（MCP ブロック9ステップを含む）全通過。`cargo fmt` / `clippy` 緑。

## 8. 追記（実施済み・改修Ⅱ）

改修Ⅱ（契約推論を Core Word として露出）を実施した。`PROBE` として実装し、65→66語、Semantic Kernel 36→37語。

### 8.1 実装の要点

`rust/src/agent/word_contract.rs`（改称なし、既存の推論エンジン）の中核関数 `infer_word_contract_inner` は、当初の想定より再利用しやすかった。これは辞書に登録された「名前付き Word」を前提に見えたが、実際の再帰的な走査は `WordDefinition`（`lines: Arc<[ExecutionLine]>` 他）に対して行われており、名前は再帰検出とキャッシュキーの構成にしか使われていない。そこで CodeBlock の生トークン列を、DEF が使うのと同じ `parse_definition_body` で行分割し、**辞書に一切挿入しない**使い捨ての `WordDefinition` を合成して同じ関数に渡す、という最小差分の設計で足りた。

**キャッシュキー衝突という実装上の罠。** 合成 `WordDefinition` の `registration_order` を固定値にすると、内容の異なる複数の CodeBlock が同じ依存語集合を呼ぶ場合に、契約推論の内部キャッシュが誤って他方の推論結果を返す危険があった。`next_registration_order()` を呼ぶたびに新規発行することで回避した。詳細は `word_contract_probe.rs` のコメントに残した。

**ファイルサイズ予算超過。** `infer_contract_for_block` を `word_contract.rs` に足すと540行になり、`check:file-size` の500行予算（§14.1）を超えた。既存の `word_contract_flow.rs`/`word_contract_lattice.rs`/`word_contract_widen.rs` という分割規約に倣い、`word_contract_probe.rs` へ切り出した。`infer_word_contract_inner` を `pub(crate)` に昇格させたのはこの分割のためだけである。

### 8.2 設計判断

**出力形状。** `[ 'key' value ]` ペアの Vector、6フィールド（`purity`/`determinism`/`nil`/`effects`/`confidence`/`gaps`）。当初案が想定した「`words.json` のレコードと同じ形」ではなく、`#:contract` 宣言が検証できる部分集合（purity, nil）に「cannot verify をデータとして読めるようにする」ための2軸（`confidence`, `gaps`）を足した最小集合にした。arity とコストは意図的に外した——arity は `stack.inputs`/`outputs` のような単純な数値では表現できず（改修Ⅵで判明した通り、フレーム引数と呼び出しオペランド数は別物）、コストは `ajisai contract` が既に持つコストクラス表現（`cost.steps`/`numeric`/`collection`）を Ajisai 値としてどう表すか別途の設計判断が要るため、次の版に持ち越した。

**family 分類。** 提案時は「reflection family」を想定していたが、`semantic-families.json` の `reflection` family は `dictionaryAccess: "none"` を宣言しており、これは REFLECT が「辞書状態と無関係」であることを正確に表す一方、PROBE は依存語を辞書解決する（結果が辞書状態に依存する）ため、この family へ入れると偽の主張になる。この2フィールドはどの検査スクリプトからも参照されておらず実害はないが、今回の作業全体が「宣言されているが正しくない事実」を潰す取り組みだったため、`control`（COND/EXEC と同じ family。`dictionaryAccess` 等の追加フィールドを持たない）に分類した。

**Kernel/Standard。** Kernel とした。README の Kernel 判定基準「値域を構築・観測する語、または制御・作用・辞書変更・局所命名・NIL回復・コード/データ境界のための唯一の明示操作」に "pre-execution contract inference" を追加した——PROBE は「実行前契約検査のための唯一の明示操作」であり、VENT が「NIL回復のための唯一の明示操作」であるのと同じ構造。

**`docs/formalization-coverage.json` への統合。** 新語を追加すると `check:minimal-core` が「Core の全語は `Formalized` ステータスかつ `Primitive`/`Derived` の `semantic_role` を持つ」ことを要求する——`Exploratory` は `kind: coreword`（実際の辞書語）には使えず、`kind: semantic-area`（SPAWN/AWAIT のような「まだ辞書語になっていない検討中の領域」）専用だった。これは意図的なゲートで、「代数的裏付けなしに正典語彙を増やせない」という制約そのものである。PROBE は `state-transformer.composition`（EXEC と共通の基盤——構造を評価せずに観測する点が異なる）と `observation.structured-diagnostic`（gap を構造化診断として観測する）から `Derived` とした。

### 8.3 検証

`{ 1 2 ADD } PROBE` → `pure`/`deterministic`/`complete`/`gaps: []`。`{ 42 PRINT } PROBE` → `effectful`/`effects: ['consoleWrite']` **かつ出力ストリームは空**（ブロックは一度も実行されていない——設計上の核心である「never evaluates its operand」を実行結果で確認)。`{ UNDEFINED-WORD } PROBE` → `confidence: conservative`/`gaps: ['gap.unresolvedWord']`。非 CodeBlock オペランドと NIL オペランドは ERROR、KEEP は正しくオペランドを保持。

Rust 単体テスト7件を `probe.rs` に追加。conformance corpus に4件追加（275→279）。`cargo test --all-targets` 1115/1115。npm ゲート30ステップ（MCP ブロック9ステップ含む）全通過。`cargo fmt`/`clippy` 緑。

**副産物として見つかった問題を2件、その場で修正した。** (1) `rust/tests/beta_removed_words.rs` が無関係なテストのプレースホルダー名として `'PROBE' DEF` を使っており、新語追加で衝突・失敗した——`'CALL-REMOVED' DEF` へ改名。(2) `scripts/check-minimal-core.mjs` の成功時ログが `kernelWords.size` 等の実測値ではなく `36/36`・`29/29`・`13` という**ハードコードされた文字列**を無条件に出力しており、検証ロジック自体は正しく37を数えていたのに、成功メッセージだけが古い数を表示し続ける状態だった。実測値から動的に組み立てるよう修正した——本書がこのセッション全体を通じて繰り返し見つけてきた「宣言されているが実態と結びついていない値」の、CIスクリプト内での再発である。
