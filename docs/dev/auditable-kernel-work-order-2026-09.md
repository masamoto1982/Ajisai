# 監査可能実行カーネルの完成：改修指示書（2026-09）

Status: **非正典（`[設計根拠]`）**。この文書は Ajisai の意味論を定義しない。
正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。
本書と正典が矛盾したら正典が勝つ。

対象実装者: Claude（またはそれに準ずるエージェント）
作業ブランチ: 各 Phase ごとに新しいブランチを切る
前提文書: `docs/dev/outcome-space-bijection-work-order-2026-09.md`（Phase 1〜3 は
実施済み、Phase 4 は**調査のみ実施しゲート未着手**。本書 Phase 2 がその続きである）

---

## 0. この文書の読み方（実装者向け・最初に必ず読む）

### 0.1 到達したい主張

**走らせる前に、そのプログラムが自分に何をし得るかを完全に知ることができ、
走らせた後に、何をしたかを第三者に証明できる。**

これが「監査可能なエージェント実行カーネル」という位置づけの中身である。前半は
**静的結末予測**（Phase 5）、後半は**実行証明書**（Phase 4）であり、どちらも
Ajisai が**全域言語**であること——ループ語を持たず、`find_reference_cycle`
（`rust/src/interpreter/execute_def.rs`）が DEF 時点で相互再帰を含むすべての循環を
拒否するため入力面が有限であること——に依拠する。チューリング完全な言語はこの
主張を述べることができない。

Phase 1〜3 はその二つを**誠実に名乗るための前提**である。結末の 9.3% が
`structureError` のまま、全単射がゲートで守られていないまま「完全な結末集合を返す」
と名乗ることはできない。

### 0.2 現状の測定値（本書の出発点。すべて `main` = 3a6b82e で実測）

`docs/semantics-table.json`（6,593 セル、11 ドメイン代表値）の集計:

| 区分 | セル数 | 比率 |
| --- | ---: | ---: |
| `value` | 1,466 | 22.2% |
| NIL | 763 | 11.6% |
| ERROR | 4,364 | 66.2% |
| └ 宣言済み条件に解決 | 3,957 | ERROR の 90.7% |
| └ **generic `structureError`** | **407** | ERROR の 9.3% |

**健全性: 完全。** 観測 6,593 件すべてが `spec/outcomes.json` の ID に解決する。
未登録の結末はゼロ。

**非空虚性: 21/41。** レジストリ 41 カテゴリのうち **20 が全数表で未観測**。
NIL 理由は 9 中 8 が目撃済み（`undecidable` のみ未観測）。

`structureError` が残る 18 語の内訳:

| Word | セル | その Word が宣言している条件 |
| --- | ---: | --- |
| DEF | 117 | `invalidName` `protectedWord` `definitionConflict` `selfReferentialDefinition` |
| BIND | 110 | `nonText` `nameIsAWord` `shapeMismatch` `invalidName` `protectedWord` |
| QUANTIZE | 50 | `nonNumeric` `shapeMismatch` |
| RANDOM / TAKE / CONCAT / TOKENIZE | 各 21 | 各々宣言済み |
| DIV | 19 | `nonNumeric` `shapeMismatch` |
| DEL | 10 | `invalidName` `wordNotFound` `protectedWord` |
| FILL | 8 | `invalidShape` |
| SQRT | 2 | （宣言なし） |
| LENGTH / REVERSE / CHARS / JOIN / TRIM / EXEC / PROBE | 各 1 | 各々宣言済み |

未観測 20 カテゴリ:

| 種別 | ID |
| --- | --- |
| declared | `blockContractViolation` `definitionConflict` `invalidName` `missingFollowingSourceUnit` `nameIsAWord` `nonComparableElement` `nonTextElement` `nonTruthGuard` `protectedWord` |
| structural | `builtinProtection` `condExhausted` `divisionByZero` `executionLimitExceeded` `malformedSource` `modeUnsupported` `nameConflict` `recursionLimitExceeded` `resourceLimitExceeded` `selfReferentialDefinition` `stackUnderflow` |

### 0.3 全 Phase 共通の禁止事項

- ❌ **`SPECIFICATION.html` を直接編集しない。** 生成物である
  （`npm run specification:generate`）。
- ❌ **`rust/src/kernel/generated/` を手で編集しない。** 生成物である
  （`npm run word-registry:generate`）。
- ❌ **既存テストを削除・スキップ・`#[ignore]` しない。** 落ちたら実装を直す。
- ❌ **`rust/src/` に 500 行を超える新規ファイルを作らない**
  （`npm run check:file-size` が落ちる）。
- ❌ **`unsafe` を書かない。**
- ❌ **`docs/dev/` 以外に散文ドキュメントを新設しない。**
- ❌ **結末カテゴリの判定に人間向け `message` を使わない。** 文言は変わりうる。
  安定 ID のみを表・ゲート・証明書に入れる。
- ❌ **「まだ誰も踏んでいないから」を理由に generic のまま残さない。**
  Phase 1 の目的は残数をゼロにすることであって減らすことではない。

**本書では後方互換性の維持を求めない。** 所有者の明示指示により、プロトコルの
破壊的変更・`SCHEMA_VERSION` の引き上げ・既存フィールドの削除・移行記録の省略が
すべて許可されている。「互換のために残す」を理由に死んだ表現を延命しないこと。
改修規模を小さく見せるための分割・先送りもしないこと。

### 0.4 停止条件（詰まったら黙って続行せず、ここで止めて報告する）

- Phase 1 で、既存のどの宣言済み条件にも当てはまらない失敗を見つけた場合
  （新カテゴリの追加は語彙の変更であり所有者判断。§1.4 落とし穴 E も読むこと）
- Phase 2 で、ある宣言済み ID の目撃者がどう書いても作れないと判断した場合
  （→ 削除の候補。削除は語彙の変更であり所有者判断）
- Phase 3 で、真理値強制の除去によって Reference が現に教えているイディオムが
  書けなくなる場合（§3.4 落とし穴 B）
- Phase 4 で、`resourceUsage` の値が同一プログラム・同一プロファイルで
  再現しないと判明した場合（証明書が成立しない）
- Phase 5 で、ある Word について健全な有限予測が原理的に作れないと判断した場合
- 正典（`spec/`）の記述と実装が食い違っていることを発見した場合
  （→ `docs/dev/spec-impl-alignment-methodology.md` の裁定手順に載せる案件）

### 0.5 よく使う検証コマンド

```sh
# Rust（作業ディレクトリは rust/）
cd rust && cargo fmt --check
cd rust && cargo clippy --all-targets -- -D warnings
cd rust && cargo test --all-targets

# CLI をビルド（Node 側ジェネレータが使う）
cargo build --bin ajisai --manifest-path rust/Cargo.toml

# リポジトリ全体のゲート（ルートで）
npm run check:file-size
npm run outcome-registry:check
npm run semantics:table:check
npm run word-registry:check
npm run check:skill
npm run check:docs-dev-drift
npm run check:mcp-assets

# WASM 両バンドル（Rust の挙動を変えたら必ず）
npm run build:mcp-wasm && npm run build:wasm && npm run test:mcp-backends
```

**CI 全ステップの一覧は `.github/workflows/test.yml` にある。ローカルで
`check:skill` と `check:mcp-assets` を忘れると CI が落ちる**（前回そうなった）。
`SKILL.md` は CLI の実出力を埋め込む生成物なので、エラー文言を変えたら
`npm run generate:skill` と `node tools/mcp-server/sync-assets.js` が要る。

### 0.6 用語

| 用語 | 意味 |
| --- | --- |
| 結末（outcome） | 1 プログラムの実行が到達する分類。`value` / `nil:<reason>` / `error:<category>` |
| 三分法 | 値 / NIL（理由付き）/ ERROR（`LANG.FAILURE.TRICHOTOMY`） |
| 目撃者（witness） | ある結末 ID を実際に生成する実行可能なプログラム |
| 健全性 | 観測された結末がすべて宣言済みであること |
| 非空虚性 | 宣言された結末がすべて目撃されること |
| declared カテゴリ | 専用の `ErrorCategory` 変種を持たず `ErrorCategory::Declared` 経由でのみ存在する ID |
| structural カテゴリ | `ErrorCategory` の固定変種を写す ID（`spec/outcomes.schema.json` の `kind` 定義） |
| 受領証（receipt） | source × engine × limits × outcome × resources を束ねた検証可能な組 |

### 0.7 Phase 一覧と依存

| Phase | 内容 | 依存 |
| --- | --- | --- |
| 1 | generic `structureError` の掃討（407 → 0） | なし |
| 2 | 全単射ゲートの完成（+ 小修繕 2 件） | Phase 1 |
| 3 | COND の真理値強制の除去 | なし（1・2 と並行可） |
| 4 | 実行証明書 | Phase 1（結末 ID が安定していること） |
| 5 | 静的結末予測 `outcomes` | Phase 1・2・3 すべて |

Phase 5 が最終目的地である。Phase 1〜3 は Phase 5 が「完全」と名乗るための前提。

---

## Phase 1 — generic `structureError` の掃討（407 → 0）

### 1.1 目的

18 語 407 セルの generic `structureError` を、その語が宣言する条件へ配線しきる。
完了時、`structureError` は「**まだどの Word も宣言していない新種の失敗**」だけを
意味する語になり、全数表に 1 セルも現れない。

### 1.2 触ってよいファイル（ホワイトリスト）

```
rust/src/interpreter/execute_def.rs
rust/src/interpreter/execute_del.rs
rust/src/interpreter/bindings.rs
rust/src/interpreter/value_extraction_helpers.rs
rust/src/interpreter/arithmetic_division.rs
rust/src/interpreter/tensor_cmds.rs
rust/src/interpreter/cast/cast_text_ops.rs
rust/src/interpreter/vector_ops/**          （残余の 1 セル語）
rust/src/interpreter/control.rs             （EXEC / PROBE の残余）
rust/src/interpreter/math_ops.rs            （SQRT の残余）
spec/words.json                             （§1.4 落とし穴 E の語彙追加のみ）
spec/outcomes.json                          （同上）
tests/conformance/index.html                （期待文言の更新）
rust/tests/**                               （同上）
docs/semantics-table.json                   （再生成）
docs/word-reference.md                      （再生成）
rust/src/kernel/generated/word_registry.rs  （再生成）
SKILL.md, tools/mcp-server/assets/**        （再生成）
```

### 1.3 事前に読むファイル

- `rust/src/error.rs` — `AjisaiError` の変種と `ErrorCategory` への写像
- `spec/outcomes.schema.json` の `kind` の定義（**落とし穴 A の根拠**）
- `docs/dev/outcome-space-bijection-work-order-2026-09.md` の「追記」二つ
  （第一波・第二波で確立した共有ヘルパー方式の記録）

### 1.4 ⚠️ 落とし穴

#### 落とし穴 A：`declared()` で作ってはいけない ID がある

`spec/outcomes.json` の各エントリは `kind` を持つ。

- `kind: "declared"` → `AjisaiError::declared` で作る。
- `kind: "structural"` → **専用の `AjisaiError` 変種で作る。**
  `AjisaiError::declared("shapeMismatch", …)` のように書くと、プロトコル文字列は
  同じでも内部表現が別物になり、レジストリ検査と診断分類が食い違う。

第二波でこの取り違えを実際にやり、`AjisaiError::ShapeMismatch { left, right, axis }`
へ差し替えた。**`declared()` を書く前に必ず `kind` を見ること。**
structural は 16 種（§0.2 の表）。

#### 落とし穴 B：共有ヘルパーは呼び出し元全部の語彙を見てから直す

これが本改修群の中心的教訓であり、第一波・第二波の全部の作業がこの規律の適用だった。

1. ヘルパーの呼び出し元を**全部**列挙する。
2. 全員が同じ条件を宣言している → **ヘルパー自体を直す。**
3. 語彙が割れている → **ヘルパーは触らず、呼び出し側にローカルなラッパーを置く。**

`value_extraction_helpers.rs` の `extract_integer_from_value(` がこの型の代表で、
GET は `invalidIndex`、PUT/RANDOM は `nonInteger`、TAKE は `invalidCount` と
呼び出し元ごとに違う。既に 3 つのラッパーが置かれている（`position.rs`,
`shape_ops.rs`, `quantity.rs`）。追加のラッパーを書くときはこの 3 つに倣うこと。

**調査済みの事実（再調査不要）:** `extract_word_name_from_value(` の呼び出し元は
DEF・DEL・`higher_order/common.rs` の 3 箇所だが、higher_order の呼び出しは
`val.is_text()` が真の枝の中にあるため**エラーを返し得ない**。したがって実質的な
呼び出し元は DEF と DEL の 2 つだけであり、両者に `nonText` を持たせれば
（§1.4 落とし穴 E）**ヘルパー自体を直してよい**。

#### 落とし穴 C：1 つの Word が複数の実行経路を持つ

`ADD` は少なくとも 5 経路ある（scalar fastpath / SIMD / exact-real scalar /
rational flat broadcast / lane-wise recursive）。第二波で `FlatTensor::from_value`
だけ直して「直った」と思ったが、`TRUE 1 ADD` は依然 `structureError` を返した。
原因は `rectangular_shape(` が `ValueData::Boolean` と `ValueData::Symbol` と
`ValueData::Text` に `None` を返し、**ラギッド経路に分岐していた**ため。
結局 3 箇所（`tensor_ops.rs`, `tensor_lane_ops.rs`, `arithmetic.rs`）の修正が要った。

**必ず実行して確かめること。** 各 Word について最低でも Text / Boolean / Vector /
NIL / ExactScalar（`2 SQRT`）の 5 形を通す。表を再生成して当該セルが消えたことを
確認するまで「直った」と言わないこと。

#### 落とし穴 D：DIV と QUANTIZE はスカラー経路が別にある

`arithmetic_division.rs` の `create_structure_error("number", "string")` と
`tensor_cmds.rs` の QUANTIZE 系（`single-element number` を期待する箇所）は、
Phase 1 の第二波で直した broadcast 経路とは**別の入口**である。`1 'a' DIV` と
`1 'a' QUANTIZE` を実行して確認すること。

#### 落とし穴 E：DEF と BIND の語彙は非対称であり、これは是正する

同一の「名前が文字列でない」検査に対し、BIND は `nonText` を宣言し DEF/DEL は
宣言していない。`extract_word_name_from_value(` の doc コメントは
「`nonText` is the honest error for everything else」と書いているのに実装は
generic を返している——**文書が実装より先に正しかった**例である。

**本書の決定（実装者が判断しなくてよい）:** DEF と DEL の `errorWhen` に
`nonText` を追加し、ヘルパーを `nonText` へ配線する。BIND の
`binding_names(` の 2 箇所も同様に `nonText` へ。

DEF の残り 2 件（本体が Vector でない / 本体が空）は既存語彙に該当がない。
**`invalidDefinitionBody` を新設**し、`spec/outcomes.json` に
`kind: "declared"` で登録、DEF の `errorWhen` に追加する。BIND の
destructuring 長さ不一致（`bindings.rs` の `vector of N elements` 検査）は
BIND が既に宣言している `shapeMismatch` に該当するが、**structural なので
`declared()` ではなく `AjisaiError::ShapeMismatch` を使う**（落とし穴 A）。

#### 落とし穴 F：`comparison.rs` は 500 行予算の 1 行手前

現在 499 行。1 行でも足すと `npm run check:file-size` が落ちる。触る必要が
生じたら、同ファイル内の重複を先に畳んでから足すこと（第二波で
`scalar_fastpath_pair` と `record_fastpath_hit` を抽出して 9 行空けた）。

#### 落とし穴 G：修正すると期待文言を持つテストが落ちる

`tests/conformance/index.html` と `rust/tests/` には `expectError` の部分文字列
一致がある。第一波で 7 件、第二波で 4 件が落ちた。**落ちたテストは実装ではなく
期待値の側が古い**——ただし必ず実行して新しい文言が正しいことを確かめてから直すこと
（「落ちたから期待値を新しい出力に合わせる」は禁止。それでは何も検証していない）。

### 1.5 手順

1. `docs/semantics-table.json` から `error:structureError` のセルを Word 別・
   入力タプル別に抽出し、作業リストを作る（§0.2 の表が出発点）。
2. Word ごとに、その generic を出している raise site を特定する。
   `grep -rn "create_structure_error" rust/src` が入口。
3. 落とし穴 B の手順で「ヘルパー直し」か「ラッパー」かを決める。
4. 落とし穴 A で `declared()` か専用変種かを決める。
5. 直したら**必ず CLI で実行**して `aiDiagnostic.kind` を確認する。
6. 全部終わったら再生成: `word-registry:generate` → `word:reference` →
   `semantics:table` → `generate:skill` → `sync-assets.js`。
7. WASM 両バンドルを再ビルドし `test:mcp-backends`。

### 1.6 受け入れ条件

- `docs/semantics-table.json` に `error:structureError` のセルが **0 件**。
- `npm run outcome-registry:check` が通る。
- `cargo test --all-targets` / `clippy -D warnings` / `fmt --check` が通る。
- §0.5 のゲートが全部通る（`check:skill` と `check:mcp-assets` を忘れない）。
- `npm run test:mcp-backends` が native/WASM 一致を報告する。

### 1.7 コミット

1 コミット。件名は `Route every remaining generic structure error to its declared condition`。
本文に「407 → 0」と、新設した `invalidDefinitionBody` の根拠、DEF/DEL への
`nonText` 追加の根拠を書く。

---

## Phase 2 — 全単射ゲートの完成

### 2.1 目的

`docs/dev/outcome-space-bijection-work-order-2026-09.md` の Phase 4 は
**調査だけ実施され、ゲートは作られていない**。`scripts/check-outcome-bijection.mjs`
も `spec/outcome-witnesses.json` も存在しない。結果、全単射は「誰かが手で測った」
状態にとどまり、**CI は何も守っていない**。第一波・第二波の修正は明日サイレントに
退行し得る。ここを閉じる。

### 2.2 触ってよいファイル

```
spec/outcome-witnesses.json          （新規）
spec/outcome-witnesses.schema.json   （新規）
scripts/check-outcome-bijection.mjs  （新規）
scripts/generate-semantics-table.mjs （ドメイン追加）
package.json                         （スクリプト登録）
.github/workflows/test.yml           （ゲート配線 + 小修繕Ⅰ）
scripts/check-unreachable-contract.mjs （doc コメントのみ）
rust/src/agent/**                    （小修繕Ⅱ）
docs/dev/INDEX.md
```

### 2.3 事前に読むファイル

**`docs/dev/outcome-space-bijection-work-order-2026-09.md` の §4.3 落とし穴 A〜D を
必ず読むこと。** あれは本 Phase のために書かれ、実施されないまま残っている。
特に落とし穴 B（目撃者は「書いてある」ではなく「実行した」でなければならない）と
落とし穴 C（目撃者のいない宣言は削除の候補であって例外リストの候補ではない）は
本 Phase の設計そのものである。

### 2.4 ⚠️ 落とし穴

#### 落とし穴 A：未観測 20 件の理由は 3 種類あり、対処が違う

| 理由 | 該当 | 対処 |
| --- | --- | --- |
| 多段プログラムでしか到達しない | `definitionConflict` `selfReferentialDefinition` `nameConflict` `builtinProtection` `protectedWord` `nameIsAWord` `invalidName` `wordNotFound` 系 | `spec/outcome-witnesses.json` に手書き（DEF してから DEL する等） |
| 資源上限に依存する | `executionLimitExceeded` `recursionLimitExceeded` `resourceLimitExceeded` `modeUnsupported` `condExhausted` | 目撃者にプロファイル指定を持たせる。§2.4 落とし穴 C |
| ドメイン代表が足りない | `undecidable` `nonComparableElement` `nonTextElement` `nonTruthGuard` `blockContractViolation` `malformedSource` `divisionByZero`(ERROR) `stackUnderflow` `missingFollowingSourceUnit` | ドメイン追加か手書き目撃者。§2.4 落とし穴 B |

Phase 1 完了後に**必ず測り直すこと**。Phase 1 が `nonText` 等を配線するので
未観測リストは縮む。本書の 20 件は Phase 1 前の値である。

#### 落とし穴 B：`undecidable` にはドメイン代表 `PI` が要る

`NilReason::Undecidable` は Tier 2（`PI`）の比較が細分化予算を使い切ったときにだけ
出る。現在のドメイン 11 個に Tier 2 の代表がないため、全数表は永久にこれを目撃
できない。`PI` をドメインに足すこと。ただし——

**セル数は掛け算で増える。** 現在 11 ドメイン 6,593 セル。12 個目を足すと
2 引数語で約 2 割増える。既存の作業指示書 §3.6 のセル予算を必ず確認すること。
`PI` は評価コストも高い（連分数の細分化）ので、生成時間も測ること。

#### 落とし穴 C：structural カテゴリの目撃はプロファイル依存

`resourceLimitExceeded` や `executionLimitExceeded` は上限プロファイルを変えれば
必ず出せるが、「どのプロファイルで出したか」を記録しない目撃は再現しない。
目撃者エントリに `profile` を持たせ、ゲートはそのプロファイルで実行すること。
`docs/dev/mcp-host-profiles.md` にプロファイル対照表がある。

#### 落とし穴 D：ゲートは両方向を見る。片方だけのゲートを書かない

- **健全性**: 全数表と目撃者の実行結果に現れるすべての結末 ID が
  `spec/outcomes.json` に存在すること。
- **非空虚性**: `spec/outcomes.json` のすべての ID に、全数表**または**
  目撃者ファイルのどちらかに目撃があること。

「いずれかに目撃があること」であって「全数表にあること」ではない。片方だけを
見るゲートを書くと、手書き目撃者が無意味になるか、全数表で目撃できる ID を
二重に書かされるかのどちらかになる。

#### 落とし穴 E：例外リストを作らない

ゲートが落ちたときの選択肢は「目撃者を書く」か「ID を消す」の 2 つだけである。
「目撃困難につき除外」のリストを作った瞬間、このゲートは何も主張しなくなる。
到達不能と判断したら削除する（削除は所有者判断 → §0.4 停止条件）。

### 2.5 手順

1. Phase 1 完了後の未観測リストを測り直す。
2. `spec/outcome-witnesses.schema.json` を書く。1 エントリ =
   `{ id, source, profile?, expect: { status, kind }, rationale }`。
   `rationale` は「なぜ全数表で目撃できないか」を書く必須フィールドにすること
   （空欄を許すと、全数表で目撃できる ID がここに流れ込む）。
3. `spec/outcome-witnesses.json` を書く。
4. `scripts/check-outcome-bijection.mjs` を書く。**目撃者は実際に CLI で実行し**、
   `aiDiagnostic.kind` が期待と一致することを確認する。文字列照合だけの実装に
   しないこと。
5. `package.json` に `outcome-bijection:check` を登録。
6. `.github/workflows/test.yml` の「Exhaustive semantics table is in sync」の
   直後に配線。
7. `scripts/check-unreachable-contract.mjs` の doc コメントに、この範囲が
   新ゲートへ移ったことを追記。

### 2.6 同時に行う小修繕

**小修繕Ⅰ — `specification:check` の復旧。**
`.github/workflows/test.yml` のコメントは「`SPECIFICATION.html` は意図的に空に
してある / `specification:check` は alpha の間は意図的に外してある」と書いているが、
**実際には 639 行の正典が存在し、`npm run specification:check` はローカルで通る**。
コメントが嘘になっており、ゲートが理由なく落ちている。コメントを削除し
`specification:check` を「Verify generated specification surfaces are in sync」
ステップに追加する。

**小修繕Ⅱ — 契約推論の自己矛盾。**
`ajisai contract --json` が同一オブジェクト内で `"nil": "nil-propagating"` と
`"suggested": "… nil-free …"` を併記する。`docs/dev/mcp-hard-use-findings-work-order-2026-09.md`
の F-8 として文書課題に分類されているが、**同じ応答の中で二つの語彙が矛盾している
のは実装側の欠陥**である。`suggested` 行の生成が `nil` フィールドと同じ推論結果を
読むように直す。

### 2.7 受け入れ条件

- `npm run outcome-bijection:check` が通り、CI に配線されている。
- `spec/outcomes.json` のすべての ID に目撃がある（例外リストなし）。
- 目撃者はすべて実行され、期待どおりの `aiDiagnostic.kind` を出す。
- `npm run specification:check` が CI で走っている。
- `ajisai contract --json` の `nil` と `suggested` が一致する。

### 2.8 コミット

3 コミットに分ける。(1) 目撃者ファイル + ゲート + CI 配線、
(2) `specification:check` 復旧、(3) 契約推論の自己矛盾修正。

---

## Phase 3 — COND の真理値強制の除去

### 3.1 目的

`LANG.VALUES.DISJOINT`（スカラーは Boolean ではない）と `LANG.VALUES.TRUTH` を
**例外なく**成立させる。現在、言語で唯一の条件分岐だけがこの規則の外にある。

実測（`main` = 3a6b82e）:

```
5 [ [ 1 ] [ 'fired' PRINT ] ] COND   → ok（スカラー 1 が真として発火する）
1 1 AND                              → error (nonTruthValue)
[ 1 2 3 ] [ 1 ] FILTER               → error (nonTruthValue)
```

`rust/src/interpreter/control_cond.rs` に `FINDING (not fixed here)` として
既知の逸脱が明記されている。AND/OR/NOT と `extract_predicate_boolean(` からは
同じ強制が既に除去済みで、**ここが最後の 1 箇所**である。

### 3.2 触ってよいファイル

```
rust/src/interpreter/control_cond.rs
rust/src/interpreter/compiled_plan.rs 系（COND の compiled 経路がある場合）
tests/conformance/index.html
rust/tests/**
docs/reference/**（ja/en 両方）
examples/**
SKILL.md, tools/mcp-server/assets/**（再生成）
docs/semantics-table.json（再生成）
```

### 3.3 手順

1. `control_cond.rs` のスカラー 0/1 フォールバックと単要素 Vector アンラップを
   削除し、`nonTruthGuard` へ一本化する。
2. `is_unknown_guard_result(` の枝は**残す**。U ガードが次の節へ落ちる規則
   （LANG.VALUES.TRUTH）は真理値強制ではなく三値論理そのものである。
3. 落ちたテスト・例・Reference を移送する（§3.4 落とし穴 B）。
4. 再生成一式 + WASM 再ビルド。

### 3.4 ⚠️ 落とし穴

#### 落とし穴 A：COND には compiled 経路がある

`compiled_plan.rs` の `lower_cond_dispatch` が節を compile 時に分割し、
`compiled_clause_enabled` のときは各節の guard が sub-plan として実行される
（`runtimeMetrics` の `condClauseCompiledCount` / `condDispatchFastCount` が
その計数）。

**調査済み（再調査不要）:** `op_cond`（解釈）と `op_cond_dispatch`（compile 済み）は
どちらも `run_cond_core` に合流し、guard 判定は `evaluate_guard_isolated` に
集約されている。したがって修正箇所は 1 つで足りる**見込み**である。ただし
`evaluate_guard_greedy` という別経路も存在するため、**両方を読んでから直し、
compiled / 非 compiled の両モードで目撃テストを置くこと**。片方だけ直して
「COND は直った」と報告しないこと。

#### 落とし穴 B：本当の作業は削除ではなくイディオムの移送

比較語は要素持ち上げ後 `[ 7 ] [ 5 ] GT` に `[ TRUE ]` を返す。単要素 Vector の
アンラップを消すと、**`[ n ]` 包みの比較をガードに使っている例が全部落ちる**。
これが前任者が延期した理由そのものである。

移送先は「ガードはスカラーを直接比較する」形（`7 5 GT`）。Reference（ja/en）・
`tests/conformance/index.html`・`examples/`・`SKILL.md` の元ネタを機械的に
grep して洗い出すこと。**Reference が現に教えているイディオムが書けなくなる場合は
§0.4 の停止条件**——その場合、単要素 Vector の扱いは意味論の問題であり所有者判断。

#### 落とし穴 C：NIL 主体が COND に届く規則を壊さない

「NIL 主体 → 全ガードが U → どの節も発火しない」は、COND が `inspectNil` を
宣言している理由そのものである。`is_unknown_guard_result(` を消したり、U を
`nonTruthGuard` に流したりしないこと。テストで固定すること。

### 3.5 受け入れ条件

- `5 [ [ 1 ] [ 'x' PRINT ] ] COND` が `nonTruthGuard` を返す。
- `5 [ [ 0 ] [ 'x' PRINT ] ] COND` も同様。
- NIL 主体の COND は従来どおり（U ガードで次節へ）。
- compiled 経路・インタプリタ経路の両方に目撃テストがある。
- 全ゲート通過。

---

## Phase 4 — 実行証明書（Execution Receipt）

### 4.1 目的

`observationDigest` は**観測だけ**を束ねている。第三者が「この source を、この
engine で、この上限のもとで走らせると、確かにこの結末になる」を検証する材料に
なっていない。束ねる対象を広げ、受領証にする。

現在の digest 実装（`rust/src/agent/observation_digest.rs`）は質が高い——
意味論的正規化、BLAKE3、4 つの表現罠（代数的正規形の非一意性、Vector/Tensor 同値、
`hint` は意味でない、`stackDisplay` は値でない）を解決済み。**土台はできている。
足りないのは束ねる範囲だけである。**

### 4.2 束ねるもの

| 要素 | 出所 | 理由 |
| --- | --- | --- |
| source digest | 入力そのもの | 何を走らせたか |
| engine version | `ajisai version` | どの実装か |
| registry digest | `spec/words.json` + `spec/outcomes.json` の内容ハッシュ | どの語彙・どの結末空間か |
| limit profile | 適用された上限プロファイル | どの制約下か |
| outcome status | `ok` / `error` の三分法 | 何が起きたか |
| observation digest | 既存 | 結果は何か |
| resourceUsage | `executionSteps` / `numericWork` / `collectionWork` | いくら使ったか |

### 4.3 ⚠️ 落とし穴

#### 落とし穴 A：`runtimeMetrics` を絶対に束ねない

`compiledPlanCacheHitCount` などは**最適化の状態**であり、同じプログラムでも
セッションの履歴で変わる。束ねた瞬間に受領証は再現しなくなる。
`LANG.AUTHORITY.FREEDOM`（どの経路が走るかは観測不能）にも反する。

#### 落とし穴 B：`resourceUsage` の決定性を先に確認する

上表に入れる前に、同一プログラム・同一プロファイルで 3 軸が完全に再現することを
**実測で確かめること**。もし経路選択（fastpath / SIMD）で値が動くなら、
それは受領証に入れられない（→ §0.4 停止条件）。動かないことを確認したら、
その事実をテストで固定すること。

#### 落とし穴 C：Tier 2 は digest を `None` に落とす

`ExactReal::Computable` に出会うと既存 digest は値を捏造せず `None` を返す設計。
受領証もこの誠実さを継承すること。「証明書が出せない」を「証明書は出たが中身が
近似」にしてはいけない。`PI` を含む結果は受領証なし、と明示的に報告する。

#### 落とし穴 D：スキーマタグを上げる

`DIGEST_SCHEMA_TAG`（`b"AJISAI-OBS-1"`）は「このバイト文法のバージョン」であり、
文法が変われば上げる規律がモジュールに明記されている。受領証は digest の
上位概念なので**別のタグ**を持たせること（既存タグの意味を変えない）。

#### 落とし穴 E：`PRINT` の出力列は束ねる

`PRINT` はホスト相対だが**決定的な出力列**を生む（digest モジュールの冒頭が
根拠を書いている）。効果を束ねないと「何も出力しないプログラム」と
「出力するプログラム」が同じ受領証を持つ。

### 4.4 受け入れ条件

- `compute` の応答に受領証フィールドが載る。
- 同一 source × 同一プロファイルで受領証が完全一致することをテストで固定。
- source を 1 文字変えると受領証が変わることをテストで固定。
- 上限プロファイルだけ変えると受領証が変わることをテストで固定。
- Tier 2 を含む結果では受領証が `null` になり、その旨が報告されることを固定。
- native / WASM で受領証が一致する（`test:mcp-backends`）。

---

## Phase 5 — 静的結末予測（`outcomes`）

### 5.1 目的

**プログラムを受け取り、走らせずに、それが産み得る結末の完全な有限集合を返す。**

Ajisai だけが原理的に答えられる問いであり、本改修群の最終目的地である。
材料はすべて既にある:

- `docs/semantics-table.json` — 語 × 入力ドメイン → 結末
- `ajisai contract` の推論 — arity / purity / NIL / determinism / cost / space の合成
- `agent check` — 実行なしの解決
- 全域性 — 答えが有限であることの保証

これを **MCP の 5 つ目のツール**として露出する。

### 5.2 ⚠️ 落とし穴

#### 落とし穴 A：過大近似は可、過小近似は嘘

予測集合が実際に起きうる結末を**取りこぼしたら、この機能は嘘をつく**。
安全側（余分な結末を含む）に倒すこと。そして**過大近似したことを応答に明示する**
（`exact: true/false` のような形で）。「完全」と名乗る箇所と近似の箇所を
混ぜないこと。

#### 落とし穴 B：全数表はドメイン代表であって型ではない

表の行は「`scalarOne` を入れたらこうなった」であり、「スカラー一般ならこうなる」
ではない。合成のためには**ドメインの抽象**（値ではなく集合）が要る。
既存 11 ドメインの `motivatedBy`（どの宣言条件に到達するために存在するか）が
その設計の出発点になる。ここを飛ばして代表値のまま合成すると、予測は
すぐに過小近似になる（＝落とし穴 A 違反）。

#### 落とし穴 C：予測は上限プロファイル相対である

`spaceExhausted` / `resourceLimitExceeded` はプロファイルが決める。
プロファイルを受け取らない予測は不完全。既定プロファイルを明示して答えること。

#### 落とし穴 D：予測器は「実行しない」が、検証は「実行する」

CI ゲートの形はこうである: サンプルプログラム N 本について、
**予測集合を計算し、実際に実行し、実測の結末が予測集合に含まれることを検査**する。
これが無いと予測器は静かに嘘をつき始める。Phase 2 の目撃者ファイルは
そのままこのゲートの入力に使える（目撃者は「source と期待結末」の組だから）。

#### 落とし穴 E：`RANDOM` は値が不定でも結末は確定する

「予測できない」と「結末が予測できない」を混同しないこと。`RANDOM` の
*値*はシード依存だが、*結末の分類*（`value` か `nonInteger` か `negativeCount` か）
は入力ドメインで決まる。予測するのは結末であって値ではない。

### 5.3 受け入れ条件

- `outcomes` ツールが MCP に露出し、published JSON Schema に適合する。
- Phase 2 の全目撃者について、予測集合が実測結末を含む。
- 過大近似は `exact: false` で明示される。
- 予測はプロファイルを明示して返す。
- CI に予測 vs 実測のゲートが配線されている。

---

## 付録 A：本改修群が壊してはならない強み

以下は既に達成されており、**どの Phase でも劣化させてはならない**。

1. **健全性 100%** — 観測されたすべての結末がレジストリに解決する。
2. **native / WASM 完全一致** — `test:mcp-backends` が全ゴールデンケースと
   全上限境界で一致を報告する。
3. **正確算術** — `2 SQRT 2 SQRT MUL` = `2/1`、`8 SQRT` = `2 SQRT 2 SQRT ADD`。
   digest はこの意味論的同値を保存する（表現ではなく値をハッシュする）。
4. **診断の具体性** — 第一波・第二波で `structureError` に潰れていた条件を
   語ごとの宣言済み条件へ配線した。Phase 4 の受領証も Phase 5 の予測も、
   **具体的なカテゴリを一般化して潰さないこと**（`nestedExecutionError` を
   採用しなかった判断と同じ理由）。
5. **生成物は生成する** — word registry / word reference / semantics table /
   SKILL.md / MCP assets はすべて生成物。手で編集した瞬間に一貫性が壊れる。

## 付録 B：この改修群が答える問い

| 問い | 答える Phase |
| --- | --- |
| このプログラムは私に何をし得るか（走らせる前） | 5 |
| このプログラムは何をしたか（第三者に証明） | 4 |
| この言語が産み得る結末は全部で何種類か | 2 |
| その結末は具体的に何と呼ばれるか | 1 |
| 規則に例外はないか | 3 |
