# Phase 7: Tier 2 limited practical use — design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 7: Tier 2 を限定的に実用化する（引き継ぎ指示書 §14）。

## 目的

`Computable`（Tier 2）と `Starved` 経路は実装・テスト済みだが、通常語彙から Tier 2 値を
構築できなかった。本フェーズは、Tier 2 を実際に観測できる最小語彙を追加する。

## 追加した語彙（最小単位、§14.3）

- `MATH@PI`: π を Tier 2 computable real として push する定数（mass 0 → 1）。
- `MATH@ENCLOSE`: `[ x ] [ budget ] -> [ [lo hi] ]`。明示 water 予算で値の有理区間を観測する。
- 既存の `COMPARE-WITHIN` が Tier 2 値で `Starved` → 論理 `UNKNOWN` に到達可能になった。

## π の生成アルゴリズムと不変条件（§14.4）

`rust/src/types/exact/pi.rs`。**浮動小数は一切使わない。**

- 基底境界: Machin の公式 `π = 16·arctan(1/5) − 4·arctan(1/239)`。各 arctangent を
  交代 Taylor 級数の連続部分和で挟む（交代級数の挟み込み定理により、単調減少項の交代級数の
  極限は任意の連続部分和の間にある）。全て exact `Fraction` 演算。~512 bit 精度で 1 回だけ計算し
  `OnceLock` でメモ化する。
- ジェネレータ: 基底境界 `[LO, HI]` を `2^-step` の dyadic グリッドへ外側丸め
  （lo は floor、hi は ceil）。これにより
  - 決定的、有理端点、
  - `enclosure(k+1) ⊆ enclosure(k)`（細かいグリッドの外側丸めは粗いグリッドに含まれる）、
  - 常に π を含む（floor(LO) ≤ LO ≤ π ≤ HI ≤ ceil(HI)）、
  - 幅は単調非増加で基底幅へ収束、
  を満たす。グリッドが基底精度より細かくなると `[LO, HI]` で頭打ちになるため、
  **どの water 予算でも per-step コストが有界**（無制限な再計算をしない）。

帰結: π を自身と比較すると（別プロセスでも同一区間列のため）決して分離せず、有限予算で
必ず `Starved` → `UNKNOWN` になる（正しい Kleene 挙動）。π と 3 のような分離可能な対は
数ステップで決定する。基底精度（~512 bit）より細かい分離を要する比較は honest に starve する。

## UNKNOWN の扱い（§14.6）

- `UNKNOWN` は `ValueData::Nil` + `NilReason::LogicallyUnknown` として格納されるが、
  `is_operational_nil()` は false — 理由付き欠落（NIL）ではない。value protocol では
  `{ type: truthValue, value: unknown }` として直列化され、NIL とは区別される。
- Tier 0／1 の比較は予算に関わらず決定し、UNKNOWN へ退行しない（回帰テストで固定）。
- `COMPARE-WITHIN` の `diagnosis.agreedPrefix` に消費 refinement ステップが残る（既存）。
- 実行 receipt（Phase 6）は UNKNOWN 結果を value protocol 経由で `resultIdentity` に正しく反映する。

## 演算範囲（§14.5）

初期段階では Tier 2 に対し安全性を証明できる演算のみ。加算・減算・乗算・否定・区間観測・比較は
既存の `ExactReal` 実装が担う。逆数・除算・ABS・SIGN・FLOOR/CEIL 等は Tier 2 で `None`
（未定義を便宜実装しない）。特にゼロから分離できない値の逆数は有限 water で安全に作らない。

## 参照実装・conformance の対応方針（§14.7）

- conformance suite（`tests/conformance/index.html`）に Tier 2 ケースを追加:
  `MATH@PI MATH@PI 64 COMPARE-WITHIN → UNKNOWN`、`MATH@PI 3 64 COMPARE-WITHIN → 1/1`、
  `MATH@PI 8 MATH@ENCLOSE → [201/64, 805/256]`。いずれも `data-category="module"`。
- Python 参照実装との差分照合（`tools/ajisai-repro/compare.py --conformance`）は
  **core ケースのみ**を対象にする（module ケースは除外）。したがって Tier 2 語彙は現時点で
  production（Rust/WASM）実装に対する conformance であり、Python 参照への移植は今後の課題。
  Python を第二の正典として扱わない方針（引き継ぎ §2）に沿う。

## 互換性

- 表層構文: 変更なし（新規 module word のみ）。
- CLI JSON / WASM wire format: 変更なし。Tier 内部表現は露出せず、enclosure は有理数のみ返す。
- Tier 0／1 の既存結果: 不変（回帰テストで固定）。
- GUI: 新語が module catalog に追加される（wasm 再生成で反映）。

## 非対象（初期スコープ外）

- 2 つ目以降の定数（`MATH@E` 等）と補助語（`REFINE` / `APPROX-WITHIN` / `DECIDE-WITHIN`）。
- Tier 2 除算・逆数など未証明演算。
- Python 参照実装への Tier 2 移植。

## 仕様上の未解決点

- π 定数は ~512 bit の rigorous enclosure であり、それより細かい分離を要する比較は starve する。
  これは実装上の精度上限であって新しい正典意味論ではない。将来、精度を step とともに真に
  無限へ伸ばす stateful generator を導入する場合も、観測結果（分離/starve の判定）は
  content identity と water 予算で説明できる範囲に留める。
