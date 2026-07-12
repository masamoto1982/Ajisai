# 実装移植性レビュー (2026-06)

> Status: **Historical review note.** Some findings in this document have since been resolved.
> See `docs/formalization-coverage.json` and `tests/conformance/index.html` for the active portability contract.
> 正典は `SPECIFICATION.md` のみ。
> 本ノートは Ajisai の「実装言語非依存・同一体験再現」という設計目標
> (`PORTABILITY.md`) に対して、現参照実装 (Rust/WASM) がその目標を
> どの程度満たしているかを評価し、改修案を提示する。

## 評価の観点

`PORTABILITY.md` 原則 1 / 10 は、Ajisai の同一性を「特定実装ではなく
conformance suite が定義する入出力対応」と規定する。したがって移植性の
品質は次の二点で測られる。

1. **conformance suite が言語現象をどれだけ固定しているか**
   (= 別実装が同じ suite を通したとき、本当に「同じ Ajisai」になるか)。
2. **参照実装の観測可能表層が `SPECIFICATION.md` と一致しているか**
   (= suite に載っていない現象でも、仕様を読んだ別実装が同じ挙動を再現できるか)。

以下の所見は `cargo test --lib conformance_suite_passes` が通る状態
(19 ケース) を起点に、参照実装へ直接プログラムを流して観測した実測に基づく。

---

## 総評

実装規模は Rust だけで約 53,000 行、ユニットテストは 1,200 件超あり、
**参照実装としての作り込みは厚い**。二面プレーン分離、決定的ホスト
(`DeterministicHostEnv`)、構造化 Host Effect、HTML 形式の言語非依存な
conformance フォーマットなど、移植性を支える「土台」の設計は良質である。

一方で、**移植性を保証する仕組みそのものが目標に追いついていない**。
最大の問題は、同一性の唯一の定義であるはずの conformance suite が
言語仕様に対して極端に小さく、仕様と実装の観測表層に複数の乖離が
あっても suite では一切検出されない点である。現状では「19 ケースを
通す二つの実装」が、修飾子・真偽値・無理数演算などほぼ全領域で
異なる挙動をしていても、どちらも自称 Ajisai になれてしまう。

---

## 所見

### A. conformance suite が言語を固定できていない (最重要)

Status: **Partially resolved** — 本レビュー時点では 19 ケースだったが、現行 suite は 53 ケースで Boolean / exact-real / Gosper / STAK などを追加固定している。全仕様節の代表 case 化は継続課題。

suite は 19 ケース。これに対し仕様は 1,100 行超で、連分数比較予算・
NICF・Gosper 演算・Kleene 3 値論理・密テンソル・NIL 診断メタデータ・
修飾子 (`TOP`/`STAK`/`EAT`/`KEEP`)・モジュール・子ランタイムなどを規定する。
**次の中核現象は suite に 1 ケースも無い**:

- 修飾子全般 (`STAK`/`KEEP`/`EAT` と組合せ) — 言語アイデンティティの根幹
- 「決定する」比較の結果 (`GT`/`LT`/`EQ` が真偽を返す経路)
- 3 値論理の `TRUE`/`FALSE` 確定値 (suite は `UNKNOWN` 経路のみ)
- テンソル (`SHAPE`/`RANK`/`RESHAPE`/`TRANSPOSE`/`FILL`)
- `COND`、`FILTER`/`SCAN`/`UNFOLD`/`ANY`/`ALL`/`COUNT`
- `COMPARE-WITHIN`、`agreedPrefix` 診断
- 負添字・`GET`/`INSERT`/`REPLACE`/`REMOVE`、`CONCAT`/`REVERSE`/`RANGE`
- Record 値、NIL の `reason`/`origin` メタデータ
- 子ランタイム (`SPAWN`/`AWAIT`/...)

**改修案**: suite を「仕様の各節につき最低 1 ケース」を満たすまで拡張する。
本 PR では、参照実装が**仕様どおりに振る舞う**ことを実測確認できた範囲
(ベクトル放送演算、負添字 `GET`、`CONCAT`/`REVERSE`/`RANGE`/`LENGTH`/`TAKE`、
`SHAPE`/`RANK`、`KEEP` 修飾子、`OR-NIL` フォールバック、`COMPARE-WITHIN`
の 3 経路、`SORT`、`FOLD`、`SHAPE`/`RANK`/`TAKE`) を追加し、出発点を
19 → 37 ケースへ広げた。
残りの領域 (特に下記 B〜D で乖離が判明した領域) は、乖離の解消方針が
決まってから追加すべきで、現挙動を suite に焼き付けてはならない。

### B. 真偽値が仕様の `truthValue` 軸として観測できない

Status: **Resolved** — 現行 conformance は `TRUE`/`FALSE` literal、決定済み比較、`AND`/`OR`/`NOT`、および `TRUE 1 EQ => FALSE` を固定する。

仕様 §2.3 / §7.4 / §7.5 は真偽値を `truthValue` 軸 (`true`/`false`/`unknown`)
として規定する。しかし実測では:

```
TRUE        => 1/1
FALSE       => 0/1
5 3 GT      => 1/1
3 3 EQ      => 1/1
TRUE FALSE AND => 0/1
```

`TRUE`/`FALSE` および全比較・論理語の結果が**数値 `1/1`/`0/1` として
表示される**。3 値のうち `UNKNOWN` だけが固有綴り `UNKNOWN` を持ち、
`true`/`false` は数値名前空間に潰れている。仕様を読んだ別実装は
`5 3 GT` を `true` と表示するのが自然で、参照実装の `1/1` とは一致しない。
両者とも現 suite を通る。これは移植性の直接的な破れである。

**改修案 (要・著者判断)**: 表示層で `TruthValue` ロールを持つ値を
`TRUE`/`FALSE`/`UNKNOWN` (あるいは protocol の `true`/`false`/`unknown`)
として描画し、3 値を同一名前空間で観測可能にする。確定したら
比較・論理の conformance ケースを追加して固定する。

### C. 無理数演算が `~` 付き有理数近似へ退避する (言語アイデンティティ違反)

Status: **Resolved** — 現行 conformance は `SQRT` と Gosper 加算を nested continued-fraction observation として固定し、`sqrt(...)` / `~n/d` への退避を regression として扱う。

仕様 §1 / §4.2 / §7.3 は「全数値は厳密実数、演算は連分数表現上で
Gosper 法により実行し、近似実数や切り詰め有理数を中間生成しない」と
明言する。しかし実測:

```
'math' IMPORT 2 SQRT          => sqrt(2/1)
'math' IMPORT 2 SQRT 1 ADD    => ~1136689/470832
```

- `√2` の表示が `sqrt(2/1)` で、§4.2.3 が必須とする入れ子連分数形
  `( a0 ( a1 ( a2 ... )))`(または RawNumber の `n/d`)と異なる第三の形式。
- `√2 + 1` が `~`(近似マーカー)付きの**有理数近似** `1136689/470832`
  に化ける。これは §7.3 の「近似実数・切り詰め有理数を中間生成しない」に
  正面から反し、本言語の看板である exact-real の約束を破る。

これが最も深刻な仕様違反で、conformance の `core-unknown-spelling`
ケースは `√2 − √2 = 0` の比較経路しか踏まないため検出されない。

**改修案 (要・著者判断)**: 無理数オペランドに対する算術を Gosper
双一次変換の遅延 CF ストリームとして実装/接続し、表示を §4.2.3 の
入れ子連分数形(遅延は表示予算で `...)` 打切り)へ統一する。`~` 近似
退避は削除する。`exact-algebraic-equality-spec-proposal.md` および
`vector-nested-continued-fraction-instruction-review.md` と整合させること。

### D. `STAK` 修飾子が基本操作で失敗する

Status: **Resolved** — 現行 conformance は `1 2 3 3 STAK ADD => 6/1` と `1 2 3 3 STAK KEEP ADD => 1/1 2/1 3/1 6/1` を固定する。

```
1 2 3 STAK ADD   => ERR: Stack underflow
1 2 3 .. ADD     => ERR: Stack underflow
```

仕様 §6.1 は `STAK` を「スタック全体を被演算対象とする」と定義する。
`1 2 3 STAK ADD` はスタック全体の総和 `6` を期待する基本ケースだが
underflow になる。`STAK` は言語の中核修飾子であり、これが基本算術で
動かないのは重大。conformance に修飾子ケースが無いため未検出。

**改修案**: `STAK` 適用時に対象語へスタック全体を被演算列として渡す
経路を検証・修正し、`TOP`/`STAK` × `EAT`/`KEEP` の 4 組合せを
conformance に追加する。

### E. conformance が「非正典」である Display 文字列を観測している

conformance ランナーは `value.to_string()` (Display) を比較対象にする。
しかし仕様 §2.3 は「display strings は非正典であり機械判断に使うな」と
する。つまり**同一性の契約が、仕様が非正典と宣言した表層の上に
立っている**という自己矛盾がある。実際、所見 B/C の乖離はいずれも
「Display は非正典だから別実装は別表示でよい」という逃げ道を許す。

**改修案**: 二択。(1) conformance は安定した serialization ロール
(protocol field) を観測対象に切り替える。または (2) 仕様側で
「conformance が固定する描画は canonical serialization である」と
明記し、Display の非正典原則の例外として正典化する。どちらでも良いが、
現状の宙吊りは解消すべき。

### F. 文字列性 (encoding ロール) の保持が語によって不揃い

```
'hi' TRIM                    => 'hi'          (文字列として表示)
'hello' CHARS                => [ [ 104/1 ] ... ]   (数値ベクトル)
'hello world' ' ' TOKENIZE   => [ [ 104/1 101/1 ... ] ... ] (数値ベクトル)
[ 'a' 'b' ] JOIN             => 'ab'          (文字列として表示)
```

文字列の encoding 契約 (§1, §7.6) が、語の出力でロール保持されたり
落ちたりする。`TOKENIZE`/`CHARS` の結果が文字列ではなく数値ベクトルに
見えるのは、別実装が「同じ体験」を再現する際の不確定要素になる。

**改修案**: 文字列を生む/通す語の出力 encoding ロール保持規則を仕様で
明確化し、保持される前提のケースを conformance に追加する。

---

## 優先度付き改修ロードマップ

| 優先 | 項目 | 種別 | 本 PR |
| --- | --- | --- | --- |
| P0 | C: 無理数算術の近似退避を除去し CF ストリーム化 | 仕様準拠の実装修正 | Resolved / conformance 固定済み |
| P0 | A: conformance を仕様各節へ拡張 | テスト拡充 | Partially resolved / 現行 53 ケース |
| P1 | B: 真偽値を `truthValue` 軸として観測可能化 | 実装+仕様整合 | Resolved / conformance 固定済み |
| P1 | D: `STAK` 修飾子の基本動作修正 | 実装修正 | Resolved / conformance 固定済み |
| P2 | E: conformance 観測対象 vs §2.3 の矛盾解消 | 仕様/ランナー判断 | 報告のみ |
| P2 | F: 文字列 encoding ロール保持の規定 | 仕様明確化 | 報告のみ |

P0 の二点 (C と A) が最優先。C は看板機能 (exact-real) の正しさ、
A は移植性契約そのものの実効性に直結する。B/D は中核機能の観測・動作の
正しさで、いずれも conformance 未カバーゆえ放置されている点が共通する。
</content>
</invoke>
