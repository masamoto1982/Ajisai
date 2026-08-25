# 仕様・実装・README・Reference 日本語版 整合性監査（2026-08）

> Status: **Non-canonical / 観察ノート.** 本書は言語意味論を一切定義しない。
> 正典は `spec/` 配下のソースと、そこから生成される `SPECIFICATION.html` のみ。
> 関連: `docs/dev/spec-impl-drift-tactic.md`（乖離裁定戦術）/
> `docs/dev/type-unification-phase2-report-2026-08.md`（改修Ⅰ Phase 2）/
> `docs/dev/reference-ja-restructure-handoff.md`（Reference 日本語版 再編）。

## 0. 本書の位置づけと方法

四つの面—— 正典（`spec/` と `SPECIFICATION.html`）、実装（`rust/`・`src/`）、
`README.md`、Reference 日本語版（`public/docs/ja/index.html`）——の整合性を
突き合わせた記録である。比較の必要上、Reference 英語版（`public/docs/index.html`）と
`SKILL.md` も対象に含めた。

**検証はすべて実行によって行った。** 参照実装 CLI
（`cargo build --bin ajisai` → `ajisai run <file> --json`）にプログラムを与えた
観測結果を根拠とし、コードの読みだけを根拠にした主張は置いていない。
期待値の比較対象は `stackDisplay`（最終スタックの表示文字列）であり、
Reference が「期待値」列で約束している観測と同じものである。

実施した機械的検査:

| 検査 | 結果 |
| --- | --- |
| 生成・整合 gate 15 種（`specification:check` / `word:reference:check` / `word:manifest:check` / `word-registry:check` / `semantic-kernel:check` / `word-schema:check` / `check:version-sync` / `check:reading-surfaces` / `check:minimal-core` / `check:formalization-coverage` / `check:unreachable-contract` / `check:runtime-metadata` / `semantics:table:check` / `core-word-docs:check` / `check:skill`） | 全て緑 |
| `cargo test --lib` | 986 passed / 0 failed / 2 ignored |
| Reference 日本語版のサンプル 88 行を実機実行 | 2 件不一致（C-1・C-2） |
| Reference 英語版のサンプル 87 行を実機実行 | 同じ 2 件が不一致 |
| `README.md` の "A small taste" 5 例 | 全て一致 |
| `SKILL.md` の `→ stack:` 例 18 件＋禁止パターン 7 件 | 全て一致 |

**要点: 既存の gate はすべて緑であり、以下の指摘はいずれも現行 gate の検査範囲外で
起きている乖離である。** どの gate も、生成物と生成器、または実装とスキーマの
一致は見るが、`SPECIFICATION.html` の散文と他面の散文の一致は見ない。

---

## 1. 正典どうしの矛盾

### A-1【最重要】`spec/host-protocol-v2.schema.json` が統合前の 6 定義域を宣言している

`spec/README.md` は冒頭でこう述べている:

> This directory holds every normative source for the language. Nothing outside
> it defines Ajisai semantics.

その表に `host-protocol-v2.schema.json` は「The host protocol boundary」として
載っており、**正典ソースである**。ところがその 6 定義域の列挙は:

```json
"enum": ["scalar", "boolean", "string", "vector", "nil", "codeBlock"]
```

`SPECIFICATION.html` の `LANG.VALUES.DISJOINT` はこう述べる:

> Values form a disjoint tagged sum of exactly six domains: Scalar, Boolean,
> String, Vector, NIL, and **Symbol**.

**`codeBlock` は改修Ⅰ（CodeBlock/Vector 統合）で定義域ではなくなり、`Symbol` が
6 番目の定義域になった。** スキーマにはその変更が及んでいない。すなわち
`spec/` 配下の二つの正典ソースが、言語の値定義域について異なることを言っている。

波及先（いずれも同じ古い 6 定義域を持つ）:

| 位置 | 内容 |
| --- | --- |
| `spec/freeze/host-protocol-v2.golden.json` | `"type": "codeBlock"` を 1 件 pin している |
| `rust/src/kernel/value.rs:22-41` | `KernelValue` が `CodeBlock` variant を持ち `Symbol` を持たない。doc comment は「The six canonical value domains of Ajisai (SPEC `LANG.VALUES`)」と主張 |
| `rust/src/kernel/host_protocol_v2.rs:57` | `KernelValue::CodeBlock(_) => json!({ "type": "codeBlock" })` |
| `src/host-protocol-v2.ts:23,89` | `{ readonly type: 'codeBlock' }` |

**この一群は互いに整合しているので CI は緑になる。** `rust/src/kernel/host_protocol_v2.rs`
はスキーマの enum を読んで自分を検証し、`src/host-protocol-v2.contract.test.ts` も
同じスキーマを読む。突き合わせの相手が同じ古い正典なので、乖離が検出されない。

なお実運用の値モデル `rust/src/types::ValueData` は正しく `Symbol` を持ち
`CodeBlock` を持たない。Semantic Spine（`rust/src/kernel/`）は
「Phase 1 skeleton・deliberately unwired」と自称する移行途中の面であり、
統合の波及先から漏れたのはそのためと見られる。

実測（統合後の振る舞い）:

| プログラム | 実測 |
| --- | --- |
| `{ 1 2 + } [ 1 2 + ] EQ` | `TRUE` |
| `{ 2 MUL }` | `[ 2/1 MUL ]` |
| `{ }` | `[ ]` |
| `[ 1 2 ADD ] EXEC` | `3/1` |
| `{ 1 2 + } [ 0 ] GET` | `1/1` |
| `[ FOO ] [ 0 ] GET` | `FOO`（`"type": "symbol"`） |

### A-2 `spec/README.md` の gate 説明が語数上限を 57 と書いている

`spec/README.md` 末尾:

> `npm run semantic-kernel:check` enforces the budgets that keep the language
> small: at most 400 lines of kernel, 12 semantic families, **57 canonical Words**,
> and 16 aliases

実際の `scripts/check-semantic-kernel.mjs:70` は `> 70` で落とす。現在の語数は 65。
400 行・12 families・16 aliases は正しく、語数だけが古い（57 は β 境界時点の値）。

---

## 2. README の陳腐化

### B-1【重要】README:80 が「コードとデータは同じ形を占めない」と述べている

```
A vector is a sequence: values in order, nested as deeply as the data needs.
Executable code is a separate kind of value that lives in `{ }` code blocks,
so code and data never occupy the same shape.
```

正典 `LANG.SOURCE.CODE` はこう述べる:

> Code is a Vector holding source for later evaluation — **not a distinct domain
> from data, but the same Vector domain** (LANG.VALUES.DISJOINT) read as
> executable by a Word whose contract requests it. `{ }` and `[ ]` are two
> spellings of the identical construction.

**README だけが統合前の記述のまま残っている。** 上の実測表のとおり、
`{ 1 2 + } [ 1 2 + ] EQ` は `TRUE` であり、ブロックはスタック上で
どのベクトルとも同じ姿で表示される。

Reference は日英とも既に正しい。日本語版「データとしてのコード」:

> コードブロックとベクトルは**同じ種類の値**です。`{ }` と `[ ]` は同じ構築の
> 2 通りの綴りなので、スタック上のブロックはどんなベクトルとも同じ姿で表示されます。

英語版「Code as data」も同内容。**四面のうち README だけが取り残されている。**

関連（軽微）: README:38 の「言語を一枚の絵で」の表が、Water の observable idea に
「Scalars, booleans, strings, vectors, code blocks」と code blocks を並置している。
定義域の列挙としては 6 定義域（Symbol を含む）と食い違うが、
「水の純粋な形」の例示なので B-1 ほど強い矛盾ではない。

---

## 3. Reference（日英共通）のサンプル誤り

いずれも**両版に同一の形で存在する**。実機で確認した。

### C-1 `DEF` した語を `MAP` に渡す例の期待値が実測と違う

`public/docs/ja/index.html:2199` / `public/docs/index.html:1973`

```
{ [ 2 ] * } 'DBL' DEF
[ 1 2 3 ] 'DBL' MAP
```

| | 値 |
| --- | --- |
| Reference の期待値 | `[ 2/1 4/1 6/1 ]` |
| 実測 | `[ [ 2/1 ] [ 4/1 ] [ 6/1 ] ]` |

`[ 2 ]` は長さ 1 のベクトルなので、要素との積はブロードキャストにより
1 要素ベクトルになる。実測:

| プログラム | 実測 |
| --- | --- |
| `1 [ 2 ] *` | `[ 2/1 ]` |
| `[ 1 2 3 ] { [ 2 ] * } MAP` | `[ [ 2/1 ] [ 4/1 ] [ 6/1 ] ]` |
| `[ 1 2 3 ] { 2 * } MAP` | `[ 2/1 4/1 6/1 ]` |

**期待値のほうが正しく、コードが誤っている。** `{ 2 * }` に直せば期待値どおりになる。
このサンプルの主旨は「ユーザー定義ワードは引用された名前で高階ワードと組み合わさる」
であって、ブロードキャストの実演ではないので、コードを直すのが素直である。

なお `SKILL.md:92` は同じ構成を**正しく**書いている:

```
`[ 0 4 ] RANGE { [ 2 ] * } MAP` → stack: `[ [ 0/1 ] [ 2/1 ] [ 4/1 ] [ 6/1 ] [ 8/1 ] ]`
```

### C-2 分散のサンプルに余分な `KEEP` がある

`public/docs/ja/index.html:2453` / `public/docs/index.html:2124`

```
[ 1 2 3 4 5 ] 'XS' BIND
XS KEEP SUM XS LENGTH DIV 'M' BIND
XS M SUB 'D' BIND D D MUL SUM XS LENGTH DIV
```

| | 値 |
| --- | --- |
| Reference の期待値 | `2/1` |
| 実測 | `[ 1/1 2/1 3/1 4/1 5/1 ] 2/1` |

`XS` は束縛の参照なのでそれ自体が値を積む。`KEEP` はそこにもう一部コピーを残し、
その残骸が最後までスタック下に居座る。`KEEP` を外した実測:

| プログラム | 実測 |
| --- | --- |
| `[ 1 2 3 4 5 ] 'XS' BIND XS KEEP SUM` | `[ 1/1 2/1 3/1 4/1 5/1 ] 15/1` |
| 上のサンプルから `KEEP` を外したもの | `2/1` |

**期待値のほうが正しく、コードが誤っている。** `KEEP` を削除すれば期待値どおりになる。

### 参考: 誤りではなかったもの

自動比較では外れたが、規約上正しい 3 件:

- `{ 1 } 'ADD' DEF` の期待値列が文字列 `error`（実測もエラー: *Cannot redefine built-in word: ADD*）。
- `42 PRINT` の期待値列が `—`（スタックが空であることを表す。実測どおり）。
- `'TEST' PRINT` の表は見出しが「期待値」ではなく**「期待される出力」**であり、
  出力領域の内容 `TEST` を載せている。実測どおり。

---

## 4. Reference の網羅漏れ

### D-1 `PROBE` が日英どちらの Reference にも一度も現れない

正準 65 語のうち、Reference が一度も名指ししていないのは `PROBE` だけである
（残り 64 語はすべて言及がある）。`PROBE` は Semantic Kernel の語であり、
`docs/word-reference.md` と `SKILL.md` には記載がある。

実測:

```
{ 1 2 ADD } PROBE
→ [ [ 'purity' 'pure' ] [ 'determinism' 'deterministic' ] [ 'nil' 'propagates' ]
    [ 'effects' [ ] ] [ 'confidence' 'complete' ] [ 'gaps' [ ] ] ]
```

README が「十概念」の一つに挙げる「a pre-execution check of user declarations
against those contracts」を**言語の内側から**触れる唯一の手段であり、
Reference の「制御構文」か「ワードと名前」に居場所があるはずのものが空いている。

---

## 5. 日英 Reference の乖離

`docs/dev/reference-ja-restructure-handoff.md` の再編が日本語版で完了し、
英語版の再生成が未了である。§3.4 の裁定により「一時的な逆転」は想定内だが、
その残余として次が観測される。

### E-1 日本語版の冒頭コメントが現状と食い違っている

`public/docs/ja/index.html:11-20` は今もこう宣言している:

> このファイルは `public/docs/index.html`（英語版・**正典**）の日本語訳であり…
> **構成・id・サンプルコードは英語版と 1 対 1 で対応させ**、地の文だけを翻訳する。

実際には日本語版が 11 セクション、英語版が 20 セクションであり、1 対 1 対応は
成立していない。handoff §3.4 は「再生成後には再び真になる」として
**この記述を変更しないことを決定済み**なので、これは新たな判断を要する指摘ではなく、
**英語版再生成が未了であることの指標**として記録する。

### E-2【既知バグ・英語版に残存】nav の並びと DOM の並びが一致していない

| 版 | nav の並び | DOM の並び | 一致 |
| --- | --- | --- | --- |
| 日本語版 | intro tokens values stack operators vector-ops strings-ops control words output patterns | 同左 | ○ |
| 英語版 | … numbers **identity-model** stack … | … numbers stack … output **identity-model** | × |

英語版では `identity-model` が**目次で 5 番目・DOM で 20 番目（最後）**にある。
モバイルのスワイプ送り `goToPageDelta()` は `document.querySelectorAll('.ref-page')`
すなわち DOM 順を見るため、「Exact Numbers」からスワイプすると目次が予告する
「Identity Model」ではなく「Stack and Modifiers」へ進む。
handoff §3.5 が両版の既存バグとして記録したもので、**日本語版は再編で解消済み、
英語版に残存**している。

### E-3 日本語版にあって英語版に無い内容

| 内容 | 日本語版 | 英語版 |
| --- | --- | --- |
| ホストの安全制御の表（`executionLimitExceeded` / `resourceLimitExceeded` / `recursionLimitExceeded`） | あり | なし |
| サンプル `[ 0 999999999999 ] RANGE NIL-REASON` → `NIL 'spaceExhausted'`（実測一致） | あり | なし |
| サンプル総数 | 80 | 79 |

サンプル数と Playground ボタン数は両版とも一致している（日 80/80・英 79/79）。

### E-4 良好だった点

- 日本語版は英語版の **id を一つも落としていない**。旧ページ id
  （`#numbers` `#logic` `#cond` `#vectors` `#strings` `#define` `#bindings`
  `#iterative` `#dictionaries` `#higher-order` `#arithmetic` `#comparison`
  `#nil` `#identity-model`）はすべて `<h3 id>` として温存されている。
  handoff §5.4 の要件を満たしている。
- 内部 `href="#…"` の未解決リンクは**両版とも 0 件**。
- 相対リンク（`../../SPECIFICATION.html` `../../index.html`
  `../../vendor/katex/…` `../index.html`）はデプロイ構成
  （`public/` → `dist/` ＋ `cp SPECIFICATION.html dist/`）と整合している。

---

## 6. 正典 vs 実装: limit の数え方

### F-1 正典は「2 つ」、実装は 3 つの limit カテゴリを持つ

`LANG.MACHINE.LIMITS`:

> **Two limits exist** and they mean different things. The execution-step limit
> bounds total work… The materialization ceiling bounds how large a single
> generated collection may become…

実装 `rust/src/error.rs:165-170` の `ErrorCategory`:

| カテゴリ | 意味 |
| --- | --- |
| `executionLimitExceeded` | ステップ予算 |
| `resourceLimitExceeded` | `max_bigint_bits` / `max_algebraic_terms` / `max_numeric_literal_digits` / `max_source_bytes` / `max_numeric_work` |
| `recursionLimitExceeded` | 非末尾再帰の深さ（`MAX_USER_WORD_DEPTH = 256`） |

これに加えて materialization ceiling が `NIL(spaceExhausted)` を投影する。

Reference 日本語版は 3 つとも表に載せており（実装と一致）、README は正典どおり
2 つと書いている（正典と一致）。**正典が実装より狭い**、というのが乖離の向きである。

`resourceLimitExceeded` の doc comment 自身が
「Separate from `ExecutionLimitExceeded` so『the program never terminated』と
『one value grew past the declared size ceiling』が答えを共有しなくなる」
と述べており、区別は意図的なものである。したがって「実装が余計なものを作った」
ではなく、**正典の節が区別を書き落としている**可能性が高い。

なお `recursionLimitExceeded` について Reference が述べる
「`COND` 末尾の自己呼び出しはネストの深さに数えられない」は実装と一致する
（`RecursionLimitExceeded` の doc comment、および `tail_call_tests.rs` が
深さ 2000 の末尾再帰の成功を確認している）。

---

## 7. 実装内のコメント陳腐化

reading surface ではないので緊急ではないが、実装者が正典を引くときの案内が
壊れている。

### G-1 `SPEC §N.N` 形式の参照が現行正典の構成と対応していない

`rust/src` と `src` 配下の **85 ファイル・延べ 200 件超**が `SPEC §2.3`・`SPEC §7.4.1`・
`SPEC §11.2`・`SPEC §4.5.0` のような節番号で正典を参照している。参照先には
`§20`・`§14.4`・`§13.2`・`§13.1`・`§12.3` も含まれる。

現行 `SPECIFICATION.html` は **12 節**構成であり、`§13` 以降は存在しない。
また現行正典の参照単位は節番号ではなく `LANG.*` clause id である。
番号が残る限り、`SPEC §11.2`（実際には Bubble Rule＝§6 Partiality and Failure）を
§11 Conformance と読み違える余地が残る。

出現頻度上位:

| 参照 | 件数 |
| --- | --- |
| `SPEC §2.3` | 29 |
| `SPEC §7.4.1` | 17 |
| `SPEC §11.2` | 13 |
| `SPEC §4.5.0` | 12 |
| `SPEC §4.2.3` | 11 |

一方 `LANG.*` clause id での参照は 72 ファイル・26 種あり、うち 24 種は
現行正典に実在する。移行が途中で止まっている状態と見られる。

### G-2 存在しない clause `LANG.VALUES.CODEBLOCK` を参照している

`rust/src/kernel/value.rs:43`:

```rust
/// A quoted, unevaluated token sequence (SPEC `LANG.VALUES.CODEBLOCK`).
pub struct CodeBlock { … }
```

この clause は改修Ⅰで削除された。A-1 と同根である。

（`rust/src/types/mod.rs:84` の `LANG.SOURCE.REFLECTION` 参照は
「now removed」と明記してあるので問題ない。）

### G-3 「空ベクトルは表現不能」というコメントが残っている

`rust/src/interpreter/nil_conformance_tests.rs:183-184`:

> `emptyVector`, a condition no program can reach because **an empty vector is
> inexpressible**

コミット `ed4ea6fe`（2026-08-01, "feat(values): admit the empty Vector"）以降、
空ベクトルは表現可能である（`[ ]` → `[ ]`、`[ ] LENGTH` → `0/1`）。
結論（`SORT`/`UNIQUE` の projection は `never` でよい）は別の理由で今も正しいので
テストとしては健全だが、**述べている理由が偽**である。
handoff §3.5b が指摘済み・未修正。

### G-4 Symbol の `shape` が `codeBlock` のまま

`rust/src/types/value_semantics.rs:208,228` は `ValueData::Symbol` を
`SemanticKind::Code` / `ValueShape::CodeBlock` に写像する。CLI の `--json` 出力は
`"type": "symbol"` を正しく出す一方、同じノードの `semantics` に
`"semanticKind": "code"`, `"shape": "codeBlock"` を併記する。

実装のコメントが「the closest existing bucket on this coarse axis is `Code`,
though a lone Symbol … is arguably its own thing; **unresolved**」と自認しており、
既知の未決事項である。A-1 を片付けるときに一緒に判断するのが自然。

---

## 8. その他のドキュメント面

### H-1 `SKILL.md` が String を「codepoint vector with text role」と書いている

`SKILL.md:30`（生成元は `scripts/generate-skill-md.mjs:375` のハードコード文字列）:

> Strings: `'single quotes'` (**a codepoint vector with text role**).

正典 `LANG.VALUES.DISJOINT` は String を 6 定義域の一つとする。
`rust/src/types/mod.rs` のコメント自身が、その符号化は過去のものであり
`'A' [ 65 ] EQ` が `TRUE` を答えていたのは欠陥だった、と記録している。

実測:

| プログラム | 実測 |
| --- | --- |
| `'A' [ 65 ] EQ` | `FALSE` |
| `'AB' LENGTH` | エラー（*expected vector, got other format*） |
| `'AB' [ 0 ] GET` | エラー（同上） |

`check:skill` は SKILL.md が生成器の出力と一致するかしか見ないので、
生成器に埋め込まれた古い散文は検出できない。

なお `SKILL.md` の実行例 18 件と「禁止パターン」7 件は**すべて実測と一致**した
（`DUP` / `[ 1 ] IF` / `( 1 2 )` / `"hello" PRINT` / `// comment` はいずれも
主張どおり失敗する）。誤っているのはこの 1 文だけである。

### H-2 README が Reference 日本語版に言及していない

README の「Documentation」表と「Repository map」は
`public/docs/`（英語版）だけを挙げ、`public/docs/ja/` に触れていない。
日本語版は英語版ページからはリンクされているが、README からは辿れない。

---

## 9. 整合していた点（記録）

指摘の裏返しとして、確認して問題がなかったものを残す。

- **語数**: 65 canonical / 36 Semantic Kernel / 29 Standard が
  README・Reference 日英・`spec/words.json`・`docs/word-manifest.json`・
  `SPECIFICATION.html` で一致。README:13 の「57 canonical Words」は
  β 境界コミットについての歴史的記述であり、現在値の主張ではない。
- **バージョン**: 4 manifest すべて `0.2.0-beta.2`（`check:version-sync` 緑）。
- **リンク**: README が張る `SPECIFICATION.html#…` アンカー 25 本すべてが実在。
  Reference 両版の内部アンカー未解決 0 件。
- **記号エイリアス**: Reference の糖衣構文表（`+ - * / % = != < <= > >=`）が
  `docs/word-manifest.json` の `symbol_alias` 11 件と完全一致。
  エイリアス面 22 種すべてが両版で言及されている。
- **語の実在**: `check:reading-surfaces` が 4 つの reading surface について
  88 登録面・43 例名・4 ホストコマンド以外を名指ししていないことを確認（緑）。
- **順序付け・計数・形状の表** 7 行（`ORDER` `UNIQUE` `TALLY` `ZIP` `SUM`
  `PUT` `GROUP`）すべて実測一致。
- **トークン分割の表** 3 行（`3 4 +` / `3 4+` / `[ 1 2 ]LENGTH`）実測一致。
- **再帰深さ**: Reference の「256 層あたり」は `MAX_USER_WORD_DEPTH = 256` と一致。
- **空ベクトル**: `[ ]` `[ ] LENGTH` `[ ] NIL?` `[ ] SUM` `[ ] JOIN` の
  Reference 記述はすべて実測一致（handoff §3.5b の結論を再確認）。
- **NIL passthrough**: `NIL 1 EQ` → `NIL`、`[ 4 -1 ] SQRT` → `[ 2/1 NIL ]` 一致。
- **ブロックの評価文脈**: 「ブロックが見るもの」表の 5 行が実測と一致
  （`{ + } 'ADDW' DEF 3 4 ADDW` → `7/1`、`100 [ 1 2 ] { + } MAP` → Stack underflow、
  `[ 1 2 3 ] { 1 GT TRUE } FILTER` → 全要素残留）。
- **`DEF` の遅延解決**: `{ TOTALL 1 } 'FOO' DEF` は成功し、`FOO` 呼び出しで
  *Unknown word: TOTALL* になる、という記述が実測一致。

---

## 10. 対応の優先順位（提案）

本書は方針を定めない。以下は観察から導かれる順序の提案であり、判断は利用者に属する。

| # | 項目 | 性質 | 備考 |
| --- | --- | --- | --- |
| 1 | A-1 `host-protocol-v2.schema.json` の 6 定義域 | **正典の修正** | `codeBlock` → `symbol`。freeze golden・`KernelValue`・TS 型・Rust serializer が連動する。互換性方針（README の「一つの現行形式」）に触れるため、利用者の判断が要る |
| 2 | B-1 README:80 | 散文の修正 | 正典と Reference が既に正しいので、README を寄せるだけ。判断の余地は小さい |
| 3 | C-1 / C-2 Reference のサンプル | 散文＋コードの修正 | 日英両版。どちらも「期待値が正しくコードが誤り」なので、コードを直す。実測で確認済み |
| 4 | H-1 `generate-skill-md.mjs:375` | 散文の修正 | 生成器を直して `npm run generate:skill` |
| 5 | F-1 `LANG.MACHINE.LIMITS` の「Two limits」 | **正典の判断** | 実装の 3 カテゴリを正典に書くか、正典の 2 分類のままとするか。設計判断 |
| 6 | D-1 `PROBE` の Reference 項目 | 加筆 | 日本語版を先に書き、英語版再生成時に反映（handoff §6.3 の順序） |
| 7 | E-2 英語版の nav/DOM 不一致 | バグ修正 | 英語版再生成に含めるのが自然 |
| 8 | A-2 `spec/README.md` の 57 → 70 | 散文の修正 | 1 行 |
| 9 | G-1〜G-4 実装コメント | 整理 | reading surface ではない。G-2 は A-1 と同時に。G-1 は件数が多く、別途の作業単位が要る |
| 10 | H-2 README への日本語版リンク | 加筆 | 1 行 |

**E-1（日本語版冒頭コメントの 1 対 1 宣言）は handoff §3.4 で「変更しない」と
決定済みのため、対応表に含めていない。** 英語版再生成の完了をもって自然に真に戻る。
