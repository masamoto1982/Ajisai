# Ajisai Beta Freeze — 35語の基底と互換層の終了

> Status: **Non-canonical / 方針記録.** 本書はβ版へ到達するための完了条件を固定する。
> 正典は `spec/` 配下と生成済み `SPECIFICATION.html`。実装作業は GitHub Issue #1429 で追跡する。

## 1. 版境界

`REFLECT` を含む commit `ebb66a5f9d14a6c8d6610488724e476e652abc35` をα版の基準点とする。
このcommitを含む従来履歴はα版であり、β版は本書の完了条件をすべて満たした最初のcommitから始まる。

β版の最初の版番号は `0.2.0-beta.1` とする。既存の版番号・CLI `version`・JSON schema version
の仕組みは維持する。変えるのは開発段階の境界であって、版管理機構ではない。

## 2. 選定規則

Core Word は、次のいずれかを満たす場合だけ残す。

1. Ajisai の互いに素な値領域を構築または観測する。
2. 制御、効果、辞書変更、NIL 回復、code/data 境界の唯一の明示操作である。
3. 削除すると、残存する stack/flow 機構からその能力を定義できなくなる。
4. exact arithmetic、reasoned NIL、explicit evaluation、Vector、機械可読契約という
   Ajisai の同一性を保つために必要である。

利用頻度、実装済みであること、既存プログラムとの互換性は残存理由にしない。

## 3. β基底: 35 Words

### Truth / comparison

`TRUE` `FALSE` `AND` `NOT` `EQ` `LT` `GT`

`OR` は De Morgan、`NEQ` は `EQ NOT`、`LTE` は `GT NOT`、`GTE` は `LT NOT` で表せる。

### Exact arithmetic

`ADD` `MUL` `DIV` `FLOOR` `NEG` `SQRT`

`SUB` は `NEG ADD`。`MOD` は `KEEP DIV FLOOR MUL NEG ADD` で表せる。
`CEIL` `ROUND` `ABS` `SIGN` `MIN` `MAX` は算術・比較・制御のライブラリ操作とする。

### Vector / iteration

`GET` `LENGTH` `CONCAT` `COLLECT` `RANGE` `FOLD`

位置編集、分割、並べ替え、集合風操作、高階ショートカットは、残存する sequence 基底上の
アルゴリズムであり、独立した意味論境界ではない。

### Text boundaries

`CHARS` `JOIN` `NUM` `STR`

String と code-point Vector、String と Scalar の境界だけを Core に残す。文字列加工は
Vector アルゴリズムとする。単一 code point の `CHR` は `COLLECT JOIN` で表せる。

### Control / absence / modifier / dictionary / effect / reflection

`COND` `EXEC`

`NIL` `NIL?` `NIL-REASON` `VENT`

`KEEP`

`DEF` `DEL` `LOOKUP`

`PRINT`

`REFLECT`

`EAT` は既定動作そのものなので削除し、非既定の `KEEP` だけを操作として残す。
`REFLECT` は CodeBlock token sequence を値として露出・再構築する唯一の境界なので残す。

## 4. 削除する35 Words

`OR` `NEQ` `LTE` `GTE` `SUB` `MOD` `CEIL` `ROUND` `ABS` `SIGN` `MIN` `MAX`
`INSERT` `REPLACE` `REMOVE` `TAKE` `SPLIT` `REVERSE` `REORDER` `FILL` `SORT` `UNIQUE`
`CONTAINS` `INDEX-OF` `MAP` `FILTER` `ANY` `ALL` `TRIM` `TOKENIZE` `SUBSTITUTE`
`STARTS-WITH?` `ENDS-WITH?` `CHR` `EAT`

互換 prelude、deprecated alias、自動変換器は提供しない。

## 5. 互換層の終了

β版は現在の意味論を一つだけ持つ。したがって以下を削除する。

- runtime Word の記号 alias と、その専用 token/desugar 経路
- HostProtocolV1 の schema、golden、contract test、adapter
- module/import 削除後に残った空配列・no-op の WASM API と GUI
- hedged execution 削除後に残った execution mode / trace API と worker 分岐
- 古い stack restore、module state、user-word export を読む persistence fallback
- example-word version migration と、削除済み辞書名の救済処理

protocol/version field、CLI `version`、JSON schema version は将来の破壊的変更を識別するため残す。
「版を持つこと」と「α互換を実装し続けること」は別である。

## 6. β到達条件

- `spec/words.json`、生成 `WordId`、runtime dispatch、Reference、Specification、hover/LOOKUP、
  conformance inventory がすべて35語で一致する。
- 削除語は tokenizer、canonicalizer、dictionary、compiled plan、REFLECT decoder のどこからも
  Core Word として到達できない。
- production code に V1/module/hedged/旧snapshot fallback が残らない。
- `OR` `NEQ` `LTE` `GTE` `SUB` `MOD` `CHR` の導出 witness を law test として持つ。
- Rust/TypeScript/WASM の全品質ゲートと生成物同期チェックが通る。
- package、Rust crate、Tauri の版が `0.2.0-beta.1` に同期する。

35語は、あらゆる言語再設計に対する数学的最小性を主張しない。上の規則のもとで選んだ、
Ajisai として自己完結し、実用可能な最小基底である。
