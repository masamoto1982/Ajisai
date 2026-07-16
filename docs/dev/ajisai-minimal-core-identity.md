# Ajisai Minimal Core — 幹を定める、亜種を生む前に

> Status: **Non-canonical / 設計メモ（§2.2）.** 本書は言語意味論を一切定義しない。
> 正典は `SPECIFICATION.html` のみ。本書は「Ajisai の *同一性* をどの語・どの法則が
> 担うのか」を切り分け、`Ajisai Minimal Core` を宣言するための手続き文書である。
> 関連正典: `SPECIFICATION.html` §2.1（正典順位）・§7（Core Words）・§9.3（辞書語彙階層）・
> "Conformance and Identity"。
> 関連データ: `docs/formalization-coverage.json`（30 代数プリミティブ・213 エントリ）・
> `docs/primitive-test-map.json`・`docs/word-manifest.json`。
> 関連設計メモ: `docs/dev/ajisai-mathematical-formalization.md`（denotational/algebraic
> semantics; 本書の層分けは同書 §1〜§8 の構造に一対一で対応する）・
> `docs/dev/ajisai-self-hosting-design.md`（セルフホストの位置づけ。本書はその
> 「実装言語が一つ増えるだけ」という結論を前提とし、capability-gated kernel profile を
> 採らない立場を引き継ぐ）。

## 0. なぜ幹を定めるのか

紫陽花（植物）は品種改良が盛んで、実に多様な園芸品種がある。それらが「紫陽花」で
あり続けるのは、花序・装飾花・土壌 pH に応じた発色といった**変わらない幹**を共有する
からである。プログラミング言語 Ajisai も同じ道を歩むには、多様な拡張を許しつつ
「これが変われば Ajisai ではない」という幹＝**Minimal Core** を明示する必要がある。

本書は二つの実務的懸念を**同時に**解く。

1. **中心概念が多く、価値が分散している。** 表層語彙は 215（core 97 / module 88 /
   alias 20 / surface 10, `docs/word-manifest.json`）。初見の利用者・AI に「Ajisai は
   結局、何を最も簡単にする言語か」が伝わりにくい。
2. **信頼すべき実装（trusted core）が大きい。** Rust はテスト込みで約 73k 行、
   最大は `continued_fraction.rs`（約 200KB）。

この二つは別問題に見えて、根は一つ——**「幹（Core）と派生（Derived）の境界が
宣言されていない」**という一点である。境界を宣言すれば懸念 1 が、境界の内側を縮めて
外側を Ajisai 自身で書き直せば（セルフホスト）懸念 2 が解ける。

## 1. 出発点 — Ajisai の本質的価値

Ajisai の最大の成果は、継続分数やベクトル演算**そのものではない**。

> 「値がない」「まだ分からない」「プログラムが間違っている」を分離し、それらを
> 隠さず流す計算モデルを、仕様・実装・検証・AI 向け契約まで一貫して構築していること。

したがって Minimal Core は、この本質的価値を担う語と法則から定義しなければならない。
実装の都合（30 代数プリミティブの並び）から定義してはならない。**幹は同一性であり、
同一性はこの三分離とその伝播規律にある。**

## 2. 30 代数プリミティブは同一性の観点で等価ではない

`docs/formalization-coverage.json` の 30 `algebra_primitives` を `algebraic_family` で
束ねると、本質的価値に照らして三層に自然分離する。数式的裏付けは
`ajisai-mathematical-formalization.md` の対応節を併記する。

### 2.1 同一性の層（identity）— 「在る／無い／未決／不正」を分けて隠さず流す

| family / primitive | 担う同一性 | 語（代表） | 形式化 |
|---|---|---|---|
| `bubble.domain/passthrough/handler`（#10-12） | **値がない**を理由付きで透過 | `NIL` `VENT` | §5 Bubble モナド |
| `k3.domain/meet/join/involution`（#1-4） | **まだ分からない**を論理で GLB/LUB 透過 | `TRUE` `FALSE` `AND` `OR` `NOT` | §4 Kleene 3 値代数 K3 |
| `exact-real.budgeted-order`（#6） | 比較が予算内で決まらねば **U を返す**（未決の誠実な保留） | `EQ` `NEQ` `LT` `LTE` `GT` `GTE` `COMPARE-WITHIN` | §3.3 予算付き観測 |
| `observation.structured-diagnostic`（#27）+ `capability.check`（#29） | **プログラムが間違っている**を、ホスト例外を漏らさず構造化診断として観測 | 構造化診断・capability 不足 | §8 観測関数と同一性 |

値空間 V が**直和（direct sum）**であり各成分が互いに素（disjoint）であること——
#1 は「numbers と素」、#10 は「FALSE・数値ゼロ・論理 UNKNOWN と素」と明記——が、
この「分離」の実体である。null / NaN / false / 例外を混ぜないという設計は、比喩では
なくデータ構造レベルの非交差性として保証されている（§1.1 値空間 V）。

### 2.2 流れの層（flow）— 値を運ぶ機構

| family / primitive | 役割 | 語（代表） | 形式化 |
|---|---|---|---|
| `state-transformer.combinator/composition/identity`（#21-23） | Σ = Stack × Dict × Eff 上の合成モノイドと恒等 | `COND` `EXEC` `EVAL` `FLOW` `IDLE` | §2 状態変換子モノイド |
| `modifier.consumption/region`（#17-20） | 消費・領域選択の直交軸 | `KEEP` `EAT` `STAK` `TOP` `FORC` | §6 修飾子コンビネータ |
| `dictionary.lookup/finite-partial-map`（#24-25） | 決定的名前解決・有限部分写像 | `DEF` `IMPORT` `LOOKUP` `record` | §9-quinquies/F 辞書 |
| `eff.append`（#26） | 自由モノイド効果ログ Eff への追記機構 | （効果の土台） | §9-sexies/G 効果代数 |

### 2.3 素材の層（material）— 強力だが同一性ではない → 派生ライブラリ／セルフホスト対象

| family / primitive | 内容 | 語（代表） | 形式化 |
|---|---|---|---|
| `exact-arithmetic`（#5,7,8） | bihomographic・continued-fraction・gosper | `ADD` `SUB` `MUL` `DIV` `MOD` `FLOOR` `>CF` | §3.1-3.2 数＝行列積 |
| `exact-scalar`（#9） | codepoint 列としての文字列 | `CHARS` `JOIN` `TRIM` `TOKENIZE` | §9-octies |
| `structure-lift`（#13-16） | 添字函手・zip・reshape 群（ベクトル/テンソル） | `MAP` `FILTER` `FOLD` `UNFOLD` `SCAN` `RESHAPE` | §7 ベクトル/テンソル |
| `observation.digest`（#28） | 正準直列化上の決定的ダイジェスト | `CRYPTO@HASH` | §8 観測代数 |
| `handle.domain`（#30, 既に Exploratory） | 子ランタイム/監督ハンドル | `SPAWN` `AWAIT` `KILL` `MONITOR` | §9-septies |
| hosted effects（#29 経由） | 能力ゲート付き副作用 | `NOW` `RANDOM` `PRINT` `IO@*` `MUSIC@*` `JSON@*` `TIME@*` `MATH@*` | §9-sexies/G |

**ChatGPT の本質的価値と完全に一致する:** 継続分数もベクトルも素材の層に落ちる。
同一性は 2.1 の第一層だけである。

## 3. 境界の決定 — Minimal Core = 同一性 ＋ 流れ

Minimal Core は **2.1（同一性）＋ 2.2（流れ）** とする。素材の層（2.3）は Core の外、
派生ライブラリ（後方互換保証の対象外・セルフホスト対象）とする。

- **同一性のみ（最小）は採らない。** 純粋だが、算術もベクトルも失うため、Core 単体で
  意味のある計算例を書けない。「使える最小核」として弱い。
- **既存 30 をそのまま Core とはしない。** 実装形状に忠実だが、同一性と素材が混在し、
  「Ajisai の中心価値」を語彙として絞り込む効果が出ない。
- 採る境界（同一性＋流れ）は、Bubble / UNKNOWN / 診断が**実際に「流れる」**ための
  最小限の運搬機構（修飾子・COND・辞書解決・EVAL）まで含む、自己完結した使える核。

## 4. Minimal Core の定義（語のリストではなく、値モデル＋規律＋流れ）

Minimal Core は次の三部で定義される。**語の集合ではなく、この構造が Ajisai の同一性**である。

1. **値空間 V の分離（§1.1）.** V は互いに素な直和成分を持つ:
   数・真偽（K3, U を含む）・文字・Bubble（NIL）。この非交差性そのものが
   「値がない／まだ分からない／不正」の分離である。
2. **三つの伝播規律.**
   - Bubble パススルー: 理由付き NIL は派生語を**変えずに透過**する（#11）。
   - Kleene 強 meet/join/involution: U は論理を GLB/LUB で透過する（#2-4）。
     `FALSE AND U → FALSE`, `TRUE OR U → TRUE`。
   - 構造化診断: error はホスト例外の漏洩ではなく、観測可能な構造化値（#27）。
3. **流れ（§2）.** 状態変換子モノイドの合成。値がその上を流れる基盤（COND・修飾子・
   辞書解決・EVAL）。

**幹と枝の関係（本書の要）:** Core は算術やベクトルを**所有しない**。しかし
**素材の層のあらゆる語は Core の伝播規律に拘束される**。`ADD` は派生語だが、
NIL を受ければ Core の Bubble パススルーに従って NIL を透過し、比較 `LT` は素材の
連分数順序を用いつつ Core の budgeted-order 契約（未決なら U）に従う。これが
「単なる小さな標準ライブラリ」ではなく**カーネル**である理由——Core は演算アルゴリズムを
持たず、**欠落・未決・不正が演算をどう貫流するかの法則**を持つ。

## 5. 一つの微妙な決定 — 数の「領域」は Core、数の「演算」はライブラリ

直和 V は成分として数を**含まねばならない**（数が在るからこそ、数を真偽・NIL から
分離できる）。しかし数の上の**演算**（`ADD` `MUL`、継続分数の gosper 正規化）は
素材の層に置き、派生ライブラリとする。

- Core が持つ: 「数は V の一成分である」という領域宣言と、比較 `LT/EQ/…` の
  **契約**（結果は真偽であり、未決なら U）。
- Core が持たない: `ADD/MUL` の**実装**、連分数エンジン（§3.1-3.2）。

この分離は境界の選択（同一性＋流れ）の直接の帰結だが、比較語は「結果は同一性・
オペランドは素材」という**境界語**である点に注意が必要。比較語の *契約* は Core、
*実装* は素材。この点は仕様提案時に §7 の Core Word contract で明示的に裁定する
（未確定事項として記録）。

## 6. 縮小効果と、正直な限界

- **概念の分散（懸念 1）:** 「Ajisai とは何か」の答えが *215 語の看板* から
  *値の四分離＋三伝播規律＋流れ* に収束する。学ぶべき中心が固定される。
- **trusted core（懸念 2）:** 素材の層の語を Rust ビルトインから Ajisai prelude へ
  セルフホスト移送すれば、その分だけ「壊れると言語同一性が崩れる領域」が縮む。
- **正直な限界:** セルフホストで縮むのは**語彙の実装**であって**数値エンジンではない**。
  `continued_fraction.rs`（200KB）は素材だが Ajisai で書き直せず trusted core に残る。
  数値エンジンの規模と性能モデルは、本書とは別課題（利用者向けコストモデルの明文化）
  として扱う。Minimal Core の縮小効果を過大に主張しないこと。

## 7. 非目標との整合（self-hosting memo の引き継ぎ）

`ajisai-self-hosting-design.md` が却下した **capability-gated kernel profile /
「公式ビルドだけが実行できる語」** は本書でも採らない。Minimal Core は
**文書上の同一性宣言**であって、実行特権の層ではない。移植性目標（§2.4）と
`PORTABILITY.md` 原則 2（実装は参照実装の一つにすぎない）を保つ。Core の後方互換
保証は「仕様が定義する同一性を破らない」という規律であり、特定ビルドの特権ではない。

## 8. 次の手順（段階的・後戻り可能）

1. **境界の命名（コード変更ゼロ）— ✅ 実装済み.** `formalization-coverage.json` の
   30 プリミティブと 213 エントリに `core_tier: identity | flow | material | sugar` を
   付与した（同ファイル `core_tier_summary` に決定規則を記録）。規則は優先順位付きの
   決定的導出——(1) Sugar→sugar、(2) Exploratory→material、(3) moduleword→material
   （Hosted Modules §9.3）、(4) 残りは home `algebraic_family`（observation は digest/handle
   由来なら material、他は identity）。`derived_from` は「語が従う法則の全列挙」であって
   「語が何であるか」ではないため、第一信号には使わない。`core_tier` は
   `check-formalization-coverage.mjs` で必須化し、`word-manifest.json` へも伝播させた。
   結果: **Minimal Core（identity+flow）= 47 語**、material = 138、sugar = 28。
2. **仕様提案 — ✅ 実装済み.** `SPECIFICATION.html` に §2.6 "Ajisai Minimal Core" を
   規範的に追記した。Minimal Core = `core_tier` の `identity`+`flow`（機械可読な
   `formalization-coverage.json` を tier 帰属の典拠とする）。**後方互換保証(規範)**——
   Minimal Core 語の可観測契約(スタック効果・NIL パススルー・UNKNOWN・構造化診断下の
   挙動)は版を超えて安定であり、それを変えることは Ajisai の同一性への破壊的変更。
   material/sugar 層は保証対象外(園芸品種が育つ層)だが Minimal Core の伝播規律には拘束
   される。比較境界語(`EQ`〜`GTE`)は「契約は Minimal Core・実装(連分数順序)は material」
   と明記——契約は既存 §7.4 で決着済みのため裁定は追認にとどめた。既存の三つの "Core"
   語義(§9.3 Core Words 階層・Core Profile・§7.14 Coreword)との区別、および「新しい
   権威層・実行特権を作らない」ことも明記(self-hosting memo の非目標を引き継ぐ)。
3. **導出可能性の実証 — ✅ 実装済み（当初案は §8.2 により再構成）.**
   当初の「Ajisai prelude でビルトインを再定義して差し替える」案は **§8.2「ビルトイン語は
   再定義できない」により不可**。真のセルフホストは独立した別インタプリタ（Python 移植と
   同格）を意味し、1 語差し替えでは実現しない。代わりに **導出可能性の witness** を採った：
   素材語 `MATH@SIGN` を Minimal Core の語だけ（`NIL?` `LT` `GT` `COND` `IDLE` とリテラル）で
   User 語 `SIGN2` として再実装し、有理数域と NIL で両者が一致することを法則テスト
   （`rust/tests/minimal_core_derivation.rs`, proptest 128 例＋スポット＋NIL）で示した。
   これは trusted Rust core を縮めるのではなく、**Minimal Core の導出力を裏付ける証拠**。
   加えて witness は**オラクルとしての発見**を surface し、それを**是正済み**である：当初
   `SIGN2` は遅延無理数 `2 SQRT` を正しく `1` と符号付けする一方、ビルトイン `MATH@SIGN` は
   `apply_unary`（有理数限定）のため同入力を `SIGN: expected a number` で拒否していた。これは
   Python 移植が SPEC_GAPS を炙り出したのと同じ機序（`MATH@MIN`/`MAX` は全数域を扱うのに
   `MATH@SIGN` だけが有理数限定、という §7.4/§7.4.3 との不整合）。**バグとして修正**し、
   `op_sign` を `MATH@MIN`/`MAX` と同じ予算付き比較（対 `0`）に置き換えて全数域対応にし、
   未決時は U を返す挙動を **§7.4.3 に規範追記**（SIGN を comparison-dependent words に追加、
   §7.14 の Projecting/Passthrough 分類にも追加）。これにより witness の等価性は有理数だけで
   なく**遅延無理数まで含む全 admitted domain で成立**するようになった（同テストの
   `minimal_core_sign_matches_builtin_on_lazy_irrationals` が防護）。導出可能性の witness が
   ビルトインの欠陥を炙り出し、その修正によって Minimal Core と material 層の一致がむしろ
   強まった——「素材語は Core の規律に拘束される」という §2.6 の枠組みが実地で機能した例。
   さらに同じ `apply_unary`（有理数限定）欠陥を共有していた**隣接語 `MATH@NEG`/`MATH@ABS`
   も是正**：`NEG` は `ExactReal::neg` で連分数表現に直接作用する純粋算術として全数域 total 化
   （比較を含まず U を生じない）、`ABS` は対 `0` の予算付き比較で符号を決め負なら否定する
   比較依存語として全数域対応（未決時 U）。仕様は `ABS` を §7.4.3 の comparison-dependent
   words と §7.14 の Projecting/Passthrough に追加、`NEG` は Total/Passthrough と明記した。
4. **移行の計測.** `primitive-test-map` / `word-manifest` から「Core だけで書ける素材語」を
   静的判定し、trusted Rust core 行数をファイルサイズ予算と同じ発想で予算化する。
