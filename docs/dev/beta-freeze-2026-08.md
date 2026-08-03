# Ajisai β版改修指示書 — 57 canonical / 35 Semantic Kernel

> Status: **Non-canonical / 方針記録・改修指示書.**
> 本書は Ajisai を `0.2.0-beta.1` として固定するための語彙、互換性、実装順序、完了条件を定める。
> 正典は `spec/` 配下と、そこから生成される `SPECIFICATION.html` である。
> 実装作業は GitHub Issue #1429 で追跡する。

## 1. 決定事項

`REFLECT` を含む commit `ebb66a5f9d14a6c8d6610488724e476e652abc35` をα版の基準点とする。
このcommitを含む従来履歴はα版であり、本書の完了条件をすべて満たした最初のcommitからβ版を開始する。

β版の最初の版番号は `0.2.0-beta.1` とする。既存の版番号、CLI `version`、protocol/schema version
の仕組みは維持する。変更するのは開発段階の境界と、β版で保証する現行形式であって、版管理機構そのものではない。

β版の公開語彙は、単一の平坦な Core dictionary に置かれる **57 canonical Words** とする。
57語は内部設計上、次の二層に分類する。

- **Semantic Kernel: 35 Words** — Ajisai の意味論的能力と同一性を成立させる設計上の幹。
- **Standard Words: 22 Words** — 実用性、可読性、標準アルゴリズム、観測可能な実行契約のために標準搭載する語。

この分類は namespace、module、import、prelude、別辞書を導入しない。利用者からは57語すべてが同じ Core Word であり、
Core は引き続き sealed、User dictionary は `DEF` で作られる。

`REFLECT` 追加時の `70/70` は形式化と executable witness の網羅性を表すものであり、70語すべてが
あらゆる再設計に対して不可約であることの証明ではない。同様に、35語は世界的・数学的最小性を主張しない。
35語は本書の選定規則に基づく Semantic Kernel、57語はβ版で実用可能な canonical surface である。

## 2. 改修の原則

### 2.1 Semantic Kernel の残存条件

Kernel Word は、次のいずれかを満たす。

1. Ajisai の互いに素な値領域を構築または観測する。
2. 制御、効果、辞書変更、NIL回復、code/data境界の唯一の明示操作である。
3. 削除すると、残存する stack/flow 機構からその能力を定義できなくなる。
4. exact arithmetic、reasoned NIL、explicit evaluation、Vector、機械可読契約という Ajisai の同一性に必要である。

### 2.2 Standard Word の残存条件

Standard Word は Kernel から派生可能であっても、次のいずれかを満たす場合に残す。

1. 頻出する概念を直接表し、User Word による反復実装より可読性と監査性が大きく向上する。
2. NIL policy、ERROR時のスタック復元、Vector lifting、境界値などを一つの canonical contract に固定する。
3. 標準アルゴリズムを一つに固定し、各利用者が異なる境界条件や複雑度を実装することを防ぐ。
4. 短絡評価、隔離された高階実行、materialization ceiling、原子的失敗など、単純なUser Wordでは同じ観測契約を保証しにくい。

利用頻度だけでは残さない。また「派生可能」という理由だけでも削除しない。表現力、観測可能な挙動、安全性、
実装複雑度、利用時の摩擦を合わせて判断する。

## 3. 公開Core: 57 Words

### 3.1 Semantic Kernel: 35 Words

#### Truth / comparison — 7

`TRUE` `FALSE` `AND` `NOT` `EQ` `LT` `GT`

#### Exact arithmetic — 6

`ADD` `MUL` `DIV` `FLOOR` `NEG` `SQRT`

#### Vector / iteration — 6

`GET` `LENGTH` `CONCAT` `COLLECT` `RANGE` `FOLD`

#### Text boundaries — 4

`CHARS` `JOIN` `NUM` `STR`

#### Control / absence / modifier / dictionary / effect / reflection — 12

`COND` `EXEC`

`NIL` `NIL?` `NIL-REASON` `VENT`

`KEEP`

`DEF` `DEL` `LOOKUP`

`PRINT`

`REFLECT`

### 3.2 Standard Words: 22 Words

#### Truth / comparison — 4

`OR` `NEQ` `LTE` `GTE`

条件式の意図を直接表し、LOOKUP、hover、契約検査で概念を直接参照できるため残す。

#### Exact arithmetic — 6

`SUB` `MOD` `ROUND` `ABS` `MIN` `MAX`

通常の計算での頻度に加え、丸め規則、符号、NIL、ERROR、Vector lifting、`KEEP`時のスタック効果を
標準契約として固定するため残す。

#### Vector / higher-order — 9

`TAKE` `REVERSE` `FILL` `SORT` `INDEX-OF` `MAP` `FILTER` `ANY` `ALL`

Ajisai の vector-oriented な実用表面を保つため残す。特に `MAP` `FILTER` `ANY` `ALL` は
要素訪問順、CodeBlockのスタック隔離、ERROR時の復元を標準化する。`ANY` `ALL` は短絡評価を持つ。
`FILL` はallocation前のmaterialization ceiling検査を持つ。`SORT` は決定性、比較規則、原子的失敗、
実用的な計算量を標準実装に固定する。

#### Text algorithms — 3

`TRIM` `TOKENIZE` `SUBSTITUTE`

Stringをcode-point Vectorへ分解して毎回実装させず、Unicode code point、空文字列、separator、
置換境界などの挙動を一つの契約として固定するため残す。

## 4. 削除する13 Words

次の13語を canonical Core から削除する。

`CEIL` `SIGN`

`INSERT` `REPLACE` `REMOVE` `SPLIT` `REORDER` `UNIQUE` `CONTAINS`

`STARTS-WITH?` `ENDS-WITH?` `CHR`

`EAT`

### 4.1 削除理由

- `CEIL` — `NEG FLOOR NEG` で直接構成でき、独立した標準契約を維持する利益が小さい。
- `SIGN` — 比較と `COND` で構成でき、用途が限定される。
- `INSERT` `REPLACE` `REMOVE` — immutable Vector に対する mutation-shaped API を減らす。
- `SPLIT` — 可変個数出力によるスタック効果が複雑で、`TAKE`、`LENGTH`、`GET`、`FOLD` に寄せる。
- `REORDER` — permutation 専用で用途が限定される。
- `UNIQUE` — `FOLD` と `INDEX-OF` から構成できる。
- `CONTAINS` — より情報量の多い `INDEX-OF` から構成できる。
- `STARTS-WITH?` `ENDS-WITH?` — `CHARS`、`TAKE`、`LENGTH`、`EQ` から構成できる。
- `CHR` — 単一code pointのcollectionと `JOIN` で構成できる。
- `EAT` — operand consumption は既定動作であり、非既定動作の `KEEP` だけを操作として残す。

削除語について、互換prelude、deprecated canonical name、自動変換器、旧Wordを復元するUser Word bundleは提供しない。

## 5. 語彙メタデータの変更

### 5.1 正典の分類軸

`spec/words.json` の各残存entryに、公開Core内での設計上の分類を表すフィールドを追加する。
推奨フィールド名は次のとおりとする。

```json
{
  "vocabularyTier": "kernel"
}
```

許可値は `kernel` と `standard` のみとする。

Standard Wordには必要に応じて次の分類を持たせる。

```json
{
  "vocabularyTier": "standard",
  "standardKind": "operational"
}
```

`standardKind` の許可値は次の4種類とする。

- `shorthand` — 直接的な短縮表現。
- `namedPattern` — 頻出する定型処理に安定した名前と契約を与える。
- `algorithm` — 境界条件や複雑度を含む標準アルゴリズム。
- `operational` — 短絡、隔離、復元、資源制限など、native実装の観測契約に意味がある。

既存の `docs/formalization-coverage.json` にある `core_tier`（`identity` / `flow` / `material`）は
別の分類軸である。`vocabularyTier` の代用にせず、名称・意味を変更しない。

### 5.2 Standard Word の完全性

Standard Wordを「便利な付録」や互換層として扱ってはならない。22語すべてにKernel Wordと同じ水準で、
次を要求する。

- `spec/words.json` の完全なmachine-readable contract
- Specification clause
- Reference page、hover、`LOOKUP`
- formalization entry
- semantic role、algebraic family、derivationまたはnative保持理由
- executable law test
- conformance case
- runtime dispatchとcompiled pathの一致
- `REFLECT` code-data representation上のcanonical nameの整合

### 5.3 Kernelとの関係

Standard Wordは形式化資料で次のいずれかを宣言する。

- `derivable` — Kernel Wordによる実行可能なwitnessを持つ。
- `operational` — Kernelが同じ計算能力を持つことのwitnessに加え、native Wordとして残す観測契約を明記する。

`derivable` の対象は原則として次の16語とする。

`OR` `NEQ` `LTE` `GTE` `SUB` `MOD` `ROUND` `ABS` `MIN` `MAX`
`TAKE` `REVERSE` `INDEX-OF` `TRIM` `TOKENIZE` `SUBSTITUTE`

`operational` の対象は次の6語とする。

`MAP` `FILTER` `ANY` `ALL` `FILL` `SORT`

operational Wordについて、User Wordによるwitnessがnative Wordと同じ効果回数、短絡点、ERROR復元、
計算量、資源制限まで再現するとは主張しない。Kernelの表現能力と、Standardの追加保証を分けて記録する。

## 6. `check-minimal-core` の改修

`scripts/check-minimal-core.mjs` は「全canonical Wordsにwitnessがある」だけの検査から、
35語Semantic Kernelと22語Standardの構造を検査するgateへ変更する。

最低限、次を検査する。

1. `spec/words.json` のcanonical inventoryが正確に57語である。
2. `vocabularyTier=kernel` が正確に35語、`standard` が正確に22語である。
3. Kernelの名前集合が本書 §3.1 と一致する。
4. Standardの名前集合が本書 §3.2 と一致する。
5. 削除13語がinventoryに存在しない。
6. 57語すべてにformalization entryと存在するlaw-test fileがある。
7. Kernel 35語にPrimitive、Derived、HostedEffectの有効なwitnessがある。
8. Standard 22語に `derivable` または `operational` の関係があり、必要なwitnessと保持理由がある。
9. formalization側にのみ存在する孤立Core Wordがない。
10. `core_tier` と `vocabularyTier` を混同した値がない。

成功時の出力は、少なくとも次の二つの数を明示する。

```text
[minimal-core] 35/35 Semantic Kernel Words have executable witnesses.
[minimal-core] 22/22 Standard Words have complete contracts and law witnesses.
```

スクリプト名は既存CIとの連続性のため維持してよい。内部的に補助スクリプトへ分ける場合も、
`check-minimal-core` をβ版の集約gateとして残す。

## 7. Manifest、Reference、生成物

### 7.1 Manifest

Word Manifestは、少なくとも次のcountを機械可読に公開する。

```json
{
  "counts": {
    "canonicalWords": 57,
    "semanticKernelWords": 35,
    "standardWords": 22
  }
}
```

manifest schemaを変更する場合はschema versionを一度上げ、生成スクリプト、fixture、consumerを同じ変更で更新する。
各Word entryにも `vocabularyTier` を出力する。

aliasとsurface-formのcountはcanonical Word数に加算して「語彙数」と表示しない。

### 7.2 ReferenceとSpecification

公開面では、次の表記を統一する。

```text
57 canonical Words / 35-Word Semantic Kernel
```

Referenceは57語すべてを通常のCore Wordとして検索・閲覧可能にする。そのうえで、Kernel/Standardを
badge、filter、補助列などで識別可能にする。Standardを二級扱いせず、契約・例・エラー条件を省略しない。

README、Specification、Reference、Word Manifest、generated Rust docs、hover、`LOOKUP`、SKILL.md、
conformance metadataに残る「70 Words」「35 Words」の公開語彙表現を更新する。

### 7.3 生成対象

少なくとも次を再生成し、手編集との差分を残さない。

- `SPECIFICATION.html`
- Word Reference
- Word Manifest
- generated Rust Word registry / `WordId`
- generated documentation and hover data
- SKILL.md
- conformance inventory and fixtures
- formalization coverage projections

## 8. 13語削除の実装指示

各削除Wordについて、次を同一作業系列で処理する。

1. `spec/words.json` からentryを削除する。
2. 生成registryを再生成する。
3. executor key、dispatch arm、compiled call、専用moduleを到達不能性確認後に削除する。
4. tokenizer/canonicalizer/dictionary/`LOOKUP`/hover/REFLECT decoderからCore Wordとして到達できないことを確認する。
5. law test、unit test、integration test、conformance caseを削除または残存語による例へ書き換える。
6. examples、README、Specification、Referenceから利用例を削除または書き換える。
7. 削除Word名を前提にしたmigration、recovery、fixtureを削除する。
8. 未使用module、enum variant、error message、documentation linkを削除する。

単にregistryから隠し、executorを残す実装は禁止する。削除語をUser Wordやimportで自動注入する実装も禁止する。

## 9. 互換層の終了

β版は現行形式を一つだけ持つ。次の互換層は予定どおり削除する。

### 9.1 Host/runtime protocol

- HostProtocolV2を唯一の現行protocolとする。
- 実装・文書上で実用的な箇所はunversionedなcurrent protocol名へ整理する。
- protocol/version field自体は将来の破壊的変更識別のため維持する。
- `spec/host-protocol-v1.schema.json`、V1 golden、contract test、`kernel::legacy_v1`を削除する。
- GUIのlive read pathを現行protocolへ直接接続し、V1 adapterを削除する。
- module/import廃止後のno-op API、空配列field、残存UIを削除する。
- execution-mode/hedged-trace互換APIとworker hedging branchを削除する。

### 9.2 Persistence/import

- stack snapshotは現行のlossless format一つだけを受理する。
- 古いbundle向けの `collect_stack` / `restore_stack` downgrade・fallbackを削除する。
- persisted stateからlegacy module/import fieldを削除する。
- user-word exportは現行のversioned documentだけを受理し、旧array形式を削除する。
- example-word version migrationと削除済みdictionary recoveryを削除する。
- persistence/export format versionを一度上げ、α形式readerを残さない。

### 9.3 Aliasと記号糖の境界

記号糖の新しい体系、記号の再割当て、新規記号の採用は**別スコープ**とする。
本改修では `$`、`|`、`!`、`~` などの新しい意味を決定・実装しない。

ただし、削除Wordにしか到達しないalias、到達不能なlegacy canonicalization path、compound modifier互換などは削除する。
特に `EAT` 削除後に `EAT` へcanonicalizeする経路をno-opとして残してはならない。

保持Wordに対する最終的な記号糖の集合は別Issue/PRで決定する。その決定はcanonical Word数57に影響させず、
aliasをcanonical inventoryへ数えないこと。β版の最終release gateでは、別スコープで採用されなかったαaliasを残さない。

## 10. Versionと版境界

package、Rust crate、Tauri、配布metadataを `0.2.0-beta.1` に同期する。
可能なら一つのsource of truthから生成し、難しい場合はCIで一致を強制する。

維持するもの:

- `ajisai version`
- package/crate/application version
- protocol version field
- schema version field
- persistence/export format version

削除するもの:

- α形式を読み続けるfallback
- 旧語彙を復元するmigration
- 旧protocolを透過変換するadapter

最初に本書の全gateを満たしたcommitをβ境界としてREADMEとSpecificationに記録する。

## 11. 推奨実装順序

### Phase 1 — 契約と分類を先に固定

- 本書をレビュー・マージする。
- `vocabularyTier` と必要な `standardKind` のschemaを追加する。
- 57語に分類metadataを付与する。
- `check-minimal-core` を35/22構造へ変更する。
- この段階では13語をまだ削除せず、旧inventoryに対して移行中であることを明示してよい。

### Phase 2 — 13語を正典から削除

- `spec/words.json` から13語を削除する。
- registry、dispatch、executor、tests、docs、fixturesを収束させる。
- inventory gateを57語へ切り替える。

### Phase 3 — Standard Wordの契約を完成

- 22語すべてのformalization、law test、conformanceを監査する。
- 16 derivable WordsのKernel witnessを追加する。
- 6 operational Wordsの実行契約とnative保持理由をlaw testで固定する。

### Phase 4 — 互換層を削除

- HostProtocolV1、旧snapshot、旧import/export、module/import残骸、hedged互換を削除する。
- 削除Word専用aliasとlegacy pathを削除する。
- 記号糖の新設・再編は別PRへ分離する。

### Phase 5 — 公開面とversionを固定

- 全生成物を再生成する。
- 全公開面を「57 canonical / 35 kernel」へ更新する。
- versionを `0.2.0-beta.1` に同期する。
- β境界commitを記録する。

巨大な一括PRを避け、各Phaseをレビュー可能なPRへ分けてよい。ただし中間状態をreleaseしない。
最終βgateは全Phaseを横断して判定する。

## 12. 必須law test

### 12.1 Kernel witness

少なくとも次のStandard Wordについて、Kernelのみを使う導出witnessを持つ。

- `OR`
- `NEQ`
- `LTE`
- `GTE`
- `SUB`
- `MOD`
- `ROUND`
- `ABS`
- `MIN`
- `MAX`
- `TAKE`
- `REVERSE`
- `INDEX-OF`
- `TRIM`
- `TOKENIZE`
- `SUBSTITUTE`

witnessは、単なる文書中の疑似コードではなく、実行可能なAjisai programまたは同値性を検査するtestとする。

### 12.2 Operational Standard

次の語はnative契約を直接検査する。

- `MAP` — index順、isolated stack、effectsの訪問順、ERROR復元。
- `FILTER` — predicate truth/NIL/error policy、index順、ERROR復元。
- `ANY` —最初のTRUEで短絡し、未訪問要素のeffectを実行しない。
- `ALL` — 最初のFALSEで短絡し、未訪問要素のeffectを実行しない。
- `FILL` — shape overflowとmaterialization ceilingをallocation前に検査し、`spaceExhausted` NILを返す。
- `SORT` — deterministic order、比較ERROR時の原子性、元operandの復元。

### 12.3 Inventory law

- canonical namesは57個で重複なし。
- Kernelは35個、Standardは22個。
- 削除13語はsource、REFLECT decode、compiled planを含む全entry pointでUnknown Wordまたは不正tokenになる。
- aliasはcanonical countへ含めない。

## 13. 品質gate

次のgateをすべて通す。

- Rust format
- clippy
- Rust library tests
- Rust integration tests
- WASM check/build
- TypeScript type check
- lint
- Vitest
- semantic firewall
- generated-surface synchronization
- formalization coverage
- `check-minimal-core`
- reading-surface check
- package/crate/Tauri version synchronization

環境不足で実行できないgateがあるPRは、その事実と未実行項目を明記し、β境界には使用しない。

## 14. 完了条件

- [ ] `spec/words.json` が正確に57 canonical Wordsを持つ。
- [ ] 35語が `vocabularyTier=kernel`、22語が `standard` である。
- [ ] Kernel/Standardの名前集合が本書と一致する。
- [ ] 削除13語の正典、生成物、dispatch、executor、docs、tests、migrationが残っていない。
- [ ] Standard 22語すべてが完全な契約、形式化、law test、conformanceを持つ。
- [ ] `check-minimal-core` が35 Kernelと22 Standardを別々に検証する。
- [ ] ManifestとReferenceが「57 canonical / 35 kernel」を表示する。
- [ ] runtime、compiled plan、hover、`LOOKUP`、REFLECT decoderが同じ57語を認識する。
- [ ] HostProtocolV1、旧snapshot、旧import/export、module/import残骸、hedged互換がproduction codeにない。
- [ ] 記号糖再編が本改修へ混入していない。
- [ ] 全品質gateが通る。
- [ ] 全versionが `0.2.0-beta.1` に同期する。
- [ ] α基準点とβ境界commitがREADME/Specificationに記録される。

## 15. Non-goals

- 35語だけを公開語彙にすること。
- KernelとStandardを別namespace、別dictionary、import可能moduleに分けること。
- Standard Wordを互換preludeや任意bundleとして配布すること。
- αプログラム、snapshot、exportを自動変換すること。
- 旧protocol readerを残すこと。
- 本改修内で記号糖を再設計すること。
- 35語があらゆる言語設計に対する数学的最小集合だと主張すること。

β版の設計上の要点は、**35語でAjisaiの意味論的な幹を説明・検査し、57語で実用可能な一つのCoreを提供すること**である。
