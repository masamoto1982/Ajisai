# CodeBlock/Vector 統合（改修Ⅰ）：Phase 0 実測報告（2026-08）

Status: 非正典・`[観察ノート]`。本書は Ajisai の意味論も互換性方針も定義しない。
正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。

対象：`docs/dev/type-unification-work-order-2026-08.md` §2（Phase 0 — 前提の再検証）。
本書は同§2.5 が要求する成果物であり、**実装・正典ファイルを一切変更していない**。
測定のための `Value::Symbol` 追加は、`main` の作業ツリーから隔離した使い捨て
`git worktree` 上で行い、測定後に破棄した（本ブランチの `git status` は clean）。

---

## 0. 要約

| 指示書 §2 の項目 | 指示書の見積もり | 実測 |
| --- | --- | --- |
| §2.1 `Value::Symbol` 追加で壊れる箇所 | 見積もりなし | **48 箇所 / 16 ファイル**（`--lib`）、**49 箇所 / 17 ファイル**（`--all-targets`） |
| §2.3 非交差性に依存する conformance ケース | 「9 件」（キーワード概算・未検証） | **22 件**（内訳は §3） |
| §2.4 書き換えが要る正典節 | 5 節を例示 | **正典 6 節 + `words.json` 8 語 + 派生表面 12 ファイル** |

**結論：指示書 §0.4 の停止条件が二つ発火している。**

1. 「Phase 0 の結果、統合の技術的コストが §2 の見積もりより著しく大きいと分かった場合」——
   conformance の影響件数は指示書の概算 9 件に対し実測 22 件で、約 2.4 倍である。
2. 22 件は、指示書 §0.4 が Phase 1 の中断ラインとして置いた目安「現行 279 件中の
   一桁台」を、Phase 1 に着手する前の**机上の下限**の時点で既に超えている。
   Phase 1 に進めば、この 22 件は必ず非通過になる（各件の根拠は §3）。

したがって本書は Phase 1 に進まず、ここで停止して人間の判断を仰ぐ。
確認事項は §7 にまとめた。

なお Phase 0 の副産物として、指示書も原提案も記録していなかった事実を三つ観測した
（§5）。うち一つ——**`{ }` という綴りは既に二つの異なる値に使われており、
スタック表示は現時点で単射ではない**——は、指示書 §6 が未決事項として挙げた
「綴りを二つ残すことの意味論上の扱い」に直接効くので、§5.1 に分けて記した。

---

## 1. §2.1 — 内部表現が異なるという事実、および `Value::Symbol` の影響範囲

### 1.1 指示書の引用の検証

指示書が「実測済み」として挙げた行番号は、`593d750`（本書執筆時点の `main`）で
いずれも正しい。

```
rust/src/types/mod.rs:77   Vector(Arc<Vec<Value>>)
rust/src/types/mod.rs:83   CodeBlock(Vec<Token>)
```

### 1.2 (A) 案（Vector 側に寄せる）の影響範囲

`ValueData` に `Symbol(String)` を追加してビルドした。`ValueData` は
`#[non_exhaustive]` ではないため、コンパイラが影響箇所を機械的に列挙する。

```
cargo build --lib          → error[E0004] 48 件 / 16 ファイル
cargo build --all-targets  → error[E0004] 49 件 / 17 ファイル
```

`--all-targets` で 1 件増えるのは `src/types/value_persist.rs:201`
（`#[cfg(any(test, feature = "wasm"))]` のため `--lib` では未コンパイル）。
これは wasm 境界の可逆永続化コーデックであり、**新しい領域を足すと
保存セッションの符号化形式が変わる**ことを意味する。README の互換性方針
（保存セッション・書き出した辞書の可読性）に直接触れる箇所であり、
指示書 §0.1 の確認事項の具体的な現れである。

ファイル別内訳（`--lib`、48 件）：

| ファイル | 件数 |
| --- | --- |
| `src/types/value_children.rs` | 18 |
| `src/types/value_semantics.rs` | 6 |
| `src/types/display.rs` | 4 |
| `src/types/mod.rs` | 2 |
| `src/interpreter/value_extraction_helpers.rs` | 2 |
| `src/interpreter/tensor_ops.rs` | 2 |
| `src/interpreter/simd_ops.rs` | 2 |
| `src/interpreter/arithmetic_meter.rs` | 2 |
| `src/types/value_tensor.rs` / `value_protocol.rs` / `arena.rs` | 各 1 |
| `src/kernel/legacy_adapter.rs` | 1 |
| `src/interpreter/tensor_cmds.rs` / `comparison.rs` / `arithmetic.rs` | 各 1 |
| `src/interpreter/cast/cast_value_helpers.rs` / `cast_conversions.rs` | 各 1 |
| `src/agent/observation_digest.rs` | 1 |

### 1.3 この 48 という数字が下限である理由

E0004 が捕まえるのは**網羅的な match だけ**である。`_ =>` を持つ match は
新しい variant を黙って吸収するので、コンパイラは何も言わない——そして
**黙って誤った振る舞いをするのはこちらの側**である。

- `rust/src` 全体の `ValueData::` 言及：**617 箇所 / 43 ファイル**
- うち `_ =>` を含む match（機械的な近似・目視未検証）：**66 箇所**

つまり (A) 案の実作業は「コンパイラが指す 48 箇所を埋める」ではなく、
「617 箇所の言及を読み、`Symbol` が来たときに何が正しいかを 1 箇所ずつ決める」である。
指示書 §2.1 が「原提案はこれを表現の綴りの問題であるかのように軽く書いている」と
指摘した点は、この数字で裏づけられる。

### 1.4 (B) 案について

指示書 §2.1 の指示どおり、**(B) 案（Vector を CodeBlock 化する）は採らない**ことを
Phase 0 の結論として確認する。理由は指示書の記述（全ベクトル操作が未評価トークン列を
相手にすることになる）に加え、実測で次が分かったため：

`ValueData::Vector` を相手にする演算経路は Tensor 表現と密結合しており
（`value_tensor.rs`・`tensor_ops.rs`・`tensor_cmds.rs`・`simd_ops.rs`、
`ValueData::` 言及だけで計 77 箇所）、要素を `Token` にすると
**密テンソル表現そのものが成立しない**（`DenseTensor` は `Fraction` の列である）。
(B) は Vector の表現変更ではなく、テンソル層の廃止を含む。

---

## 2. §2.2 — 契約推論エンジンの `Token` 依存

指示書が挙げた `word_contract.rs:343/353/358` は現行 `main` で正しい。
`Token` 依存の実測は次のとおり（`Token::` の出現数）。

| ファイル | 行数 | `Token::` |
| --- | --- | --- |
| `word_identity.rs` | 426 | 20 |
| `word_contract.rs` | 497 | 10 |
| `word_contract_flow.rs` | 219 | 6 |
| `word_space.rs` | 499 | 5 |
| `word_cost.rs` | 311 | 3 |
| `word_contract_widen.rs` | 126 | 3 |

指示書が挙げていない `word_identity.rs`（20 箇所）が、この一群で最も
`Token` に依存している。契約推論エンジン一式の `Token` 依存は計 **47 箇所 / 6 ファイル**。

リポジトリ全体では `Token::` は **459 箇所 / 29 ファイル**。うち上位は
`tokenizer_regression_tests.rs`（110）・`tokenizer_regression_tests_2.rs`（86）・
`code_data.rs`（53）・`tokenizer.rs`（23）・`execution_loop.rs`（21）・
`value_persist.rs`（20）・`compiled_plan.rs`（18）。

`compiled_plan.rs`（530 行）は指示書がまったく触れていない。CodeBlock の
内部表現を変えるなら、実行前計画を組む層も同時に書き換えが要る。

**判定（§0.4 第三条件に対して）**：契約推論エンジンの書き換えは、この時点では
「作り直し」ではなく「増分」に見える——`Token` 依存 47 箇所は、
`Vec<Token>` を `&[Value]` に読み替える機械的な変換で大半が吸収できる形をしている。
ただしこれは**コードを読んだ上での判断であって、実測ではない**。確定は Phase 1 の仕事である。

---

## 3. §2.3 — 非交差性に依存する conformance ケース（確定一覧）

corpus は 279 件、すべて `data-category="core"`。ベースラインは全件通過
（`cargo test --lib conformance_suite_passes` → `conformance: 279 case(s) passed`）。

全 279 件を読み、「CodeBlock と Vector が別領域であることを前提に、その前提の
成立または破綻を主張しているケース」を数え上げた結果は **22 件**である。
指示書 §2.3 のキーワード概算 9 件は、`REFLECT` 群を数え落としていた。

### 3.1 領域の非交差性を直接主張する（1 件）

| id | source | 期待 | 統合後 |
| --- | --- | --- | --- |
| `core-exec-rejects-vector` | `[ 1 2 ADD ] EXEC` | error: `expected code block` | **意味を失う**。統合後、これは成功して `3/1` を残す |

`core-exec-rejects-scalar`（`5 EXEC`）と `core-probe-rejects-non-codeblock`
（`5 PROBE`）は、指示書 §2.3 が挙げた `nonCodeBlock` 系だが**該当しない**——
どちらも被験値が Scalar であり、統合後も Scalar は実行可能にならないので
エラーはそのまま成立する。この二件は数に含めていない。

### 3.2 `REFLECT` の存在そのものを主題とする（13 件）

`reflection-code-to-data` / `reflection-data-exec` / `reflection-data-does-not-execute` /
`reflection-empty-round-trip` / `reflection-dynamic-def` / `reflection-symbol-string-distinct` /
`reflection-malformed-errors` / `reflection-keep` / `reflection-lexemes-preserved` /
`reflection-undefined-symbol-is-not-resolved` / `reflection-dictionary-independent` /
`reflection-wrong-header-errors` / `reflection-arbitrary-vector-errors`

13 件すべてが `REFLECT` を含み、`REFLECT` を含む非 `reflection-*` ケースは 0 件である。
指示書 §3.1-3 のとおり `REFLECT` が恒等写像になるなら、13 件すべてが主題を失う。
特に次の三件は、統合後に**主張が反転する**：

- `reflection-data-does-not-execute`：`[ 'AJISAI-CODE-1' ... ] REFLECT` が `{ 1 2 ADD }` を
  返すこと、すなわち「データ側の綴りから来た値は評価されない」ことを主張している。
  統合後、この主張自体が空になる。
- `reflection-arbitrary-vector-errors`：`[ 'ordinary' 'vector' ] REFLECT` が
  `missing AJISAI-CODE-1 header` で ERROR になること。統合後、任意の Vector が
  そのまま実行可能な記述なので、「コードデータでない Vector」という区別が消える。
- `reflection-symbol-string-distinct`：`{ ADD } REFLECT { 'ADD' } REFLECT EQ` → `FALSE`。
  これは Symbol と String が別物であることの唯一の現行証拠であり、
  (A) 案で `Value::Symbol` を昇格させるなら、**この 1 件だけは形を変えて残す価値がある**。

### 3.3 `LANG.VALUES.VECTOR`（Vector リテラル中の裸の名前）に依存する（3 件）

| id | source | 現行の期待 |
| --- | --- | --- |
| `core-vector-name-is-data` | `[ FOO 1 ]` | `[ 'FOO' 1/1 ]` |
| `core-vector-literal-ignores-dictionary` | `{ [ 99 ] } 'FOO' DEF` / `[ FOO 1 ]` | `[ 'FOO' 1/1 ]` |
| `core-vector-misspelling-is-an-element` | `[ 1 LENGHT 2 ]` | `[ 1/1 'LENGHT' 2/1 ]` |

(A) 案は「符号化されて隠れていた Symbol を昇格させ、符号化を消す」ことなので、
統合後 `[ FOO ]` は String `'FOO'` ではなく Symbol `FOO` を含むはずである。
3 件とも期待値が変わる。

`core-vector-truth-names-denote-values`（`[ TRUE FALSE NIL ]`）は期待値自体は
変わらない見込みだが、根拠となる正典節が書き換わるので §4 の対象には含めた。

### 3.4 表示の綴りが `{ }` である（5 件）

| id | source | 現行の期待 |
| --- | --- | --- |
| `core-lift-comparison-vector-scalar` | `[ 3 4 ] 4 LT` | `{ TRUE FALSE }` |
| `core-lift-comparison-scalar-vector` | `3 [ 4 5 ] LT` | `{ TRUE TRUE }` |
| `core-lift-comparison-equal-lengths` | `[ 1 2 ] [ 3 1 ] LT` | `{ TRUE FALSE }` |
| `core-lift-no-singleton-projection` | `[ 3 ] 4 LT` | `{ TRUE }` |
| `core-lift-comparison-nil-lane` | `[ 1 NIL ] 2 LT` | `{ TRUE NIL }` |

この 5 件は指示書も原提案も想定していない。詳細は §5.1。統合後に描画が
一つの綴りへ収束するなら、5 件とも期待文字列が変わる。

### 3.5 影響を受けない側の規模（参考）

`{ }` を使うケース 61 件、`[ ]` を使うケース 109 件、両方を使うケース 30 件。
上記 22 件以外の `{ }` 使用ケース（DEF 本体・`MAP`/`FILTER`/`COND` の被演算子など）は、
綴りが同義になっても**書かれたとおりに動く限り**期待値は変わらない。ただし
§5.3 の高階語の被演算子判別が変わるため、Phase 1 で実測が要る。

---

## 4. §2.4 — 書き換えが要る正典節と派生表面

### 4.1 正典（`spec/language-semantics.md`）

指示書が挙げた 5 節はすべて該当する。加えて 1 節を追加した。

| 節 | 現行の規定 | 統合との衝突 |
| --- | --- | --- |
| `LANG.VALUES.DISJOINT` | 「Scalar, Boolean, String, Vector, NIL, CodeBlock の**ちょうど六つ**の互いに素な領域」 | 数が変わる。(A) 案で `Symbol` を昇格させると 6→6（CodeBlock が消え Symbol が入る）であり、**「五つになる」という指示書 §2.4 の記述は誤り**。§7 の確認事項に含めた |
| `LANG.VALUES.VECTOR` | 「`[ FOO ]` は `FOO` が定義済み Word かどうかに関わらず String `FOO`」「誤記された名前は String 要素でありエラーではない」 | 裸の名前の意味が String → Symbol に変わる |
| `LANG.SOURCE.CODE` | 「コードとデータは別の値領域である。実行可能なソースは CodeBlock に住み、Vector はデータであって決して実行可能ではない」 | 第2段落がまるごと反転する |
| `LANG.SOURCE.REFLECTION` | 「CodeBlock と Vector は互いに素な領域のままである」「`REFLECT` は唯一の可逆な境界」「`EXEC` は引き続き CodeBlock のみを受け取る」 | 節ごと削除。Core Word 数 66 → 65 |
| `LANG.SOURCE.FRAME` | ブロックの評価規則（whole-stack / isolated frame）。「あるブロックの中に書かれたブロックは、書かれた場所ではデータである」 | 「Vector 形の値をどの文脈で命令列と解釈するか」に一般化される |
| `LANG.VALUES.DENOTATION`（**指示書に無し**） | 「内部表現は観測不能であり、表示は値から導かれる」 | §5.1 の観測（表示が単射でない）と併せて読む必要がある |

`LANG.MODIFIERS.CONSUMPTION`（317 行付近）と `LANG.COLLECTIONS.HIGHER`（301 行付近）も
本文中で CodeBlock に言及しているが、これは高階語の `KEEP` 規則の説明であり、
領域の非交差性そのものには依存していない。文言の更新のみで足りる。

### 4.2 契約レジストリ（`spec/words.json`）

Core Word 66 語のうち、CodeBlock に言及するのは **8 語**：
`MAP`(4) `FILTER`(1) `FOLD`(1) `ANY`(1) `ALL`(1) `EXEC`(5) `PROBE`(3) `REFLECT`(6)。

`errorWhen` に `nonCodeBlock` を持つのは **7 語**（`MAP` `FILTER` `FOLD` `ANY` `ALL`
`EXEC` `PROBE`）。統合後、`nonCodeBlock` は `nonVector` と区別できなくなる
（`nonVector` は 19 語で使用）。この 2 条件をどう畳むかは Phase 2 の設計判断だが、
**契約語彙の変更は `spec/words.schema.json` と各 `--check` ジェネレータを連鎖的に動かす**。

### 4.3 正典から派生する表面（12 ファイル）

`CodeBlock` / `code block` の出現数：

| ファイル | 件数 | 種別 |
| --- | --- | --- |
| `SPECIFICATION.html` | 11 | 生成物（`spec/` から） |
| `spec/words.json` | 20 | 正典 |
| `spec/language-semantics.md` | 11 | 正典 |
| `tools/mcp-server/assets/words.json` | 20 | 同期アセット |
| `docs/word-reference.md` | 12 | 生成物 |
| `tests/conformance/index.html` | 10 | 正典（corpus） |
| `public/docs/index.html` | 7 | Reference（手書き） |
| `SKILL.md` | 7 | 生成物 |
| `tools/mcp-server/assets/quickstart.md` | 7 | 同期アセット |
| `README.md` | 3 | 手書き |
| `src/wasm-interpreter-types.ts` / `src/workers/interpreter-snapshot.ts` | 各 2 | GUI 型定義 |
| `src/gui/*.ts`（3 ファイル）/ `src/workers/interpreter-execution-worker.ts` | 各 1 | GUI |

CI ゲートは `.github/workflows/test.yml` に 49 ステップ（指示書 §0.5 の「30 ステップ」は
現行より古い）。うち `--check` 付きジェネレータが 7 本あり（`specification:check` `word:manifest:check` `word-registry:check` `word:reference:check` `core-word-docs:check` `check:skill` `semantics:table:check`）、正典を触れば全部が動く。

**指示書 §0.5 のコマンド名に誤りがある**：`npm run check:semantic-kernel` という
スクリプトは存在しない。正しくは `npm run semantic-kernel:check`。
`npm run check:reading-surfaces` と `tools/mcp-server/sync-assets.js` は正しい。

---

## 5. 指示書・原提案が記録していない発見

Phase 0 は「前提の再検証」なので、前提に無かった事実も記す。以下はすべて
現行 `main` の `ajisai run` で実際に走らせて確認した観測である。

### 5.1 `{ }` という綴りは既に二つの異なる値に使われており、表示は単射でない

```
[ TRUE FALSE ]                    => stack: { TRUE FALSE }
{ TRUE FALSE }                    => stack: { TRUE FALSE }
[ TRUE FALSE ] { TRUE FALSE } EQ  => stack: FALSE
```

同じ文字列 `{ TRUE FALSE }` が、**等しくない二つの値**の表示である。左は真理値役割
（`Interpretation::TruthValue`）を持つ Vector、右は CodeBlock で、振る舞いは完全に異なる：

```
[ TRUE FALSE ] LENGTH   => 2/1            { TRUE FALSE } LENGTH   => error (expected vector)
[ TRUE FALSE ] EXEC     => error          { TRUE FALSE } EXEC     => TRUE FALSE
[ TRUE FALSE ] REFLECT  => error          { TRUE FALSE } REFLECT  => [ 'AJISAI-CODE-1' ... ]
```

根拠は `rust/src/types/display.rs` の `format_as_boolean`：真理値役割の Vector は
`{ ... }` で描画される。§3.4 の 5 件はこの経路の出力を期待値にしている。

これが重要なのは、指示書 §6 が「`{ }` と `[ ]` という綴りを二つ残すことの
意味論上の扱いは未決定」と書いた論点に対し、**現状はすでに「二つの綴り」ではなく
「一つの綴りが二つの領域を指す」状態にある**ことを意味するからである。統合は
新しい曖昧さを持ち込む変更ではなく、既にある曖昧さを解消する側に働きうる。
これは原提案の動機づけを補強する、Phase 0 で初めて出てきた材料である。

同時に `LANG.VALUES.DENOTATION`（「表示は値から導かれる」）との関係も点検が要る。
現行の描画は値から導かれてはいるが単射ではなく、`EQ` が `FALSE` を返す二値が
同一表示になる。これは統合とは独立に存在する drift であり、
統合を採らない場合でも別途の裁定に値する。

### 5.2 Vector リテラルの中では、ブロックの括弧が黙って消える

```
[ { 1 2 } ]      => stack: [ 1/1 2/1 ]
[ 1 { 2 } 3 ]    => stack: [ 1/1 2/1 3/1 ]
```

`rust/src/interpreter/vector_literal.rs` の `collect_vector_with_depth` は
`Token::BlockStart` / `Token::BlockEnd` を `_ => { i += 1 }` の catch-all で読み飛ばす。
結果、`[ { 1 2 } ]` は `[ 1 2 ]` と同一の値になる——ネストした CodeBlock は
Vector の要素にならず、括弧だけが消えて中身が平坦化される。

これは「Vector はデータであって決して実行可能ではない」（`LANG.SOURCE.CODE`）とも
「ネストした Vector は他と同じ一つの要素である」（`LANG.VALUES.VECTOR`）とも
整合しない振る舞いで、正典のどの節にも書かれていない。統合すればこの穴は
自然に閉じる（`{ }` と `[ ]` が同義になるので `[ { 1 2 } ]` は `[ [ 1 2 ] ]` になる）が、
**統合しない場合はこれ単独でバグとして裁定が要る**。

### 5.3 高階語は現在、二つの被演算子を「位置」ではなく「領域」で見分けている

```
[ 1 2 ] { 2 MUL } MAP   => [ 2/1 4/1 ]
[ 1 2 ] [ 2 MUL ] MAP   => error: expected code block ..., got other value
{ 2 MUL } [ 1 2 ] MAP   => error: expected code block ..., got other value
```

領域が一つになると、`MAP` は被演算子を位置だけで見分けることになる。
`[ 1 2 ] [ 2 MUL ] MAP` は統合後に成功しなければならず、逆順の
`{ 2 MUL } [ 1 2 ] MAP` は「コレクション `{ 2 MUL }` を各要素に対して
ブロック `[ 1 2 ]` で写す」という**別の正当な読み**を持ってしまう。
`MAP` `FILTER` `FOLD` `ANY` `ALL` の 5 語すべてが同じ問題を持つ。
これは表記の更新では吸収できず、意味論の再設計に属する。

（余談として、上記のエラー文言 `expected code block (: ... ;) or word name` は
`: ... ;` という現行の言語に存在しない綴りを含んでいる。統合とは無関係の
古い文言の残骸であり、単独で修正できる。）

---

## 6. Phase 1 スパイクの推定規模

指示書 §2.5 は「半日で見通しが立つ規模」か「複数日かかる規模」かの明記を求めている。

**複数日かかる規模である。** 根拠：

- (A) 案の骨格（`Value::Symbol` 追加 → `cargo build --lib` を通す）だけなら、
  48 箇所を機械的に埋める作業で半日に収まる。ここまでは指示書 §3.1-1〜4 に対応する。
- しかし §3.1-5（conformance 非通過件数の記録）に意味のある数字を出すには、
  トークナイザ・`vector_literal.rs`・`execution_loop.rs`・`compiled_plan.rs` を
  実際に繋いで**プログラムが走る状態**にする必要がある。`_ =>` 側の 66 箇所の
  判断がここで効いてくる。
- §5.3 の高階語の被演算子判別は、Phase 1 の途中で必ず突き当たる意味論上の分岐で、
  ここは「小さそうな方から着手する」で回避できない。

Phase 1 を指示するなら、スコープを次のどちらかに絞ることを推奨する。

- **(a) ビルド通過までで止める**：`Value::Symbol` 追加 → `cargo build --lib` 通過 →
  影響 617 箇所のうち実際に判断を要した件数を記録。conformance は走らせない。
  半日〜1 日。得られるのは「表現変更のコスト」の確定値のみ。
- **(b) conformance まで走らせる**：上記に加えて実行経路を繋ぐ。複数日。
  §5.3 の設計判断を Phase 1 の中で仮決めする必要がある。

---

## 7. 人間への確認事項

指示書 §0.4 に従い、以下の回答を得るまで Phase 1 に進まない。

1. **停止条件の扱い（§0）**。conformance 影響件数が概算 9 件に対し実測 22 件だった。
   これは指示書 §0.4 が置いた「一桁台」の目安を超えている。この規模でも
   Phase 1 に進めるか、それとも改修Ⅰ自体を見送るか。

2. **互換性方針（指示書 §0.1）**。`{ 1 2 + } [ 1 2 + ] EQ` が `FALSE` から `TRUE` に、
   `[ FOO ]` が `[ 'FOO' ]` から `[ FOO ]` に変わる。加えて §1.2 のとおり
   `value_persist.rs` の永続化形式も変わるため、**保存セッションの互換性**も対象になる。
   0.2.0-beta.1 の中で吸収するのか、新しいベータ／リリース段階の開始として扱うのか。

3. **領域の数（§4.1）**。指示書 §2.4 は「六つの互いに素な領域が統合後は五つになる」と
   書いているが、(A) 案は `CodeBlock` を落として `Symbol` を足すので **6 → 6** である。
   「概念を減らす」という原提案の軸に照らして、これは意図どおりか。
   （`Symbol` を昇格させず `[ FOO ]` を String のままにする案なら 6 → 5 になるが、
   その場合 `{ ADD }` を実行したときに `ADD` をどう解決するのかが別途未解決になる。）

4. **§5.1 の裁定**。表示が単射でない件は、改修Ⅰを採る／採らないに関わらず存在する。
   改修Ⅰと切り離して単独で扱うか、改修Ⅰの中で解消するか。

5. **§5.2 の裁定**。`[ { 1 2 } ]` が `[ 1 2 ]` になる件も同様に独立したバグである。
   改修Ⅰが見送りになる場合、これは別途の修正対象としてよいか。

6. **Phase 1 のスコープ（§6）**。進める場合、(a) ビルド通過まで／(b) conformance まで、
   のどちらか。

---

## 付録：本報告の測定手順（再現用）

```
# §2.1 の 48/49 件（使い捨て worktree 上で実施し、測定後に破棄）
git worktree add <scratch> HEAD --detach
# <scratch>/rust/src/types/mod.rs の ValueData に `Symbol(String),` を追加
cd <scratch>/rust
cargo build --lib         --message-format=short 2>&1 | grep -c E0004   # 48
cargo build --all-targets --message-format=short 2>&1 | grep -c E0004   # 97（lib と lib test の重複、実体 49）
git worktree remove --force <scratch>

# ベースライン
cd rust && cargo test --lib conformance_suite_passes -- --nocapture     # 279 case(s) passed

# §5 の観測
cargo build --bin ajisai
echo '[ TRUE FALSE ] { TRUE FALSE } EQ' > p.ajisai && ./target/debug/ajisai run p.ajisai
echo '[ { 1 2 } ]'                      > p.ajisai && ./target/debug/ajisai run p.ajisai
echo '[ 1 2 ] [ 2 MUL ] MAP'            > p.ajisai && ./target/debug/ajisai run p.ajisai
```
