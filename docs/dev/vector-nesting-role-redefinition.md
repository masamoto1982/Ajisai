# Vector ネスト構造の役割再定義 — Lisp 的動機の廃止と「テンソル/構造データ基盤」への固定

> Status: **Non-canonical / 設計メモ.** 本書は言語意味論を一切定義しない。
> 正典は `SPECIFICATION.html` のみ(§4.3 "Role of nesting" が本判断の正典側の反映)。
> 本書はネスト構造を存続させた理由と、存続の根拠となった依存関係の棚卸しを記録する
> 手続き文書である。

## 0. 判断

Vector の入れ子構造(Vector の中に Vector を置ける性質)は**存続**する。
ただしその存在理由を再定義する:

1. **廃止する動機**: Lisp への憧れ(コードとデータを同じ入れ子構造で表す
   homoiconicity 志向)。Ajisai では実行可能コードは `{ }` の CodeBlock であり、
   Vector は決して実行可能にならない。したがって Lisp 的動機は当初から成立して
   おらず、設計判断の根拠として今後一切用いない。
2. **採用する動機**: ネストは次の 2 つの実務のためだけに存在する。
   - **テンソル基盤** — §7.2 のテンソル語(`SHAPE` `RANK` `RESHAPE` `TRANSPOSE`
     `FILL`)と要素単位数値演算は、ネストした Vector を多次元配列として扱う。
     §4.3.1 の dense 表現(DenseTensor)はこの用途の最適化である。
   - **ラグド/混在型の構造データ** — `SPLIT` のサブベクタ結果、`JSON` モジュールの
     配列、マルチトラック音楽データのような「行ごとに長さが違う・型が混ざる」
     構造。これらは flat バッファ + shape では表現できない。

## 1. 経緯

ネスト構造は Lisp に憧れて考案されたが、設計者自身が利便性を感じておらず、
学習曲線を高くするだけではないかとの疑義から廃止が検討された(2026-07)。
棚卸しの結果、ネストは Lisp 的役割ではなく別の実務をすでに担っており、
廃止は複数サブシステムの同時廃止を意味することが判明したため、
「廃止」ではなく「動機の差し替え(再定義)」を採った。

## 2. 存続の根拠 — ネストに依存している資産

| 依存先 | 内容 |
|---|---|
| §7.2 テンソル演算 | `SHAPE` `RANK` `RESHAPE` `TRANSPOSE` `FILL` は「ネストした Vector を多次元配列として扱う」と定義 |
| §4.3.1 dense/nested 二重表現 | DenseTensor(SoA + shape + validity mask)は nested 表現と観測等価であることが前提 |
| 行列演算・要素単位演算 | `[ [ 1 2 ] [ 3 4 ] ] [ [ 5 6 ] [ 7 8 ] ] +` 等(`rust/src/tensor_operation_tests.rs`、`examples/tensor-operations-sample-test.ajisai`) |
| `JSON` モジュール(§9.1) | JSON 配列は本質的にネストする(`rust/src/json_io_tests.rs`) |
| `SPLIT`(§7.1) | 戻り値がサブベクタ群 = 深さ 1 のネスト |
| 音楽 DSL のマルチトラック | `[ 440 550 ] [ 220 275 ] .. SIM PLAY`(`examples/music-playback-sample-test.ajisai`) |
| 実装の深さ制限 | `MAX_VECTOR_NESTING_DEPTH`(実装詳細であり言語意味論ではない) |

## 3. 学習コストへの含意

「学習曲線が高い」という体感の原因は、ネストそのものではなく周辺仕様
(dense/nested 等価規則、No-Rebuild 原則、ブロードキャスト規則、テンソル語の
段階的パイプライン規約)にある可能性が高い。簡素化が必要になった場合の削減候補は
ネストの廃止ではなく、これら周辺仕様の露出削減である。

## 4. 正典への反映

- `SPECIFICATION.html` §4.3 に "Role of nesting (design intent, normative)" と
  "Non-goal — code as data" を追加。ネストの解釈が問われる場面では上記 2 用途を
  通して読むことを規範として固定した。
- `README.md` §3 に同旨の対外説明を追加(非正典)。
