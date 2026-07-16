# Trusted-core size — なぜ縮まないのか、そして懸念4への本当の答え

> Status: **Non-canonical / 設計メモ（§2.2）.** 本書は言語意味論を一切定義しない。
> 正典は `SPECIFICATION.html` のみ。本書は「実装規模を実際に削減できるか」を実測で
> 検討した記録であり、その結論と、外部評価の「懸念4（実装が大きい）」への回答を残す。
> 関連: `SPECIFICATION.html` §2.6（Ajisai Minimal Core）・§7.14（Coreword contract）・
> `docs/formalization-coverage.json`（`core_tier`）・
> `docs/dev/ajisai-minimal-core-identity.md`・`rust/tests/minimal_core_derivation.rs`。

## 0. 問い

外部評価（ChatGPT）は Ajisai の懸念として「**実装規模がすでに大きい**」を挙げた。
Rust はテスト込みで約 71k 行、最大は `continued_fraction.rs`（約 200KB）。個人〜少人数で
保守するには大きい。そこで「trusted Rust core を**実際に**縮められるか」を実測した。

## 1. 実測 — Rust 質量の所在

非テスト Rust ソースは **46,686 行**（テストは別途 24,865 行）。質量は次に集中する。

| 領域 | 概算行数 | 代表ファイル |
|---|---|---|
| **正確実数エンジン** | ~7,300 | `continued_fraction.rs`(5,150)・`comparison.rs`(800)・`arithmetic.rs`(720)・`fraction.rs`(677) |
| 契約メタデータ・レジストリ | ~5,400 | `builtin_word_definitions.rs`(1,908)・`module_builtins.rs`(1,452)・`coreword_registry.rs`(1,336)・`module_word_docs.rs`(695) |
| 型・値・意味プロトコル | ~2,900 | `value_operations.rs`(1,443)・`types/mod.rs`(786) ほか |
| exploratory 機能 | ~2,600 | audio/music(1,505)・`parallel.rs`＝子ランタイム(1,069) |
| WASM 境界・CLI・その他 | 残り | `wasm_value_conversion.rs`(741)・`cli/mod.rs`(894) ほか |

## 2. 仮説した削減経路は、いずれも net でゼロだった

### 2.1 セルフホスト（材料語を Ajisai ソースへ移す）
`core_tier = material` の 138 語を分類すると:

- 41 語 … cf エンジン算術に依存（**削減不可**）
- 31 語 … テンソル/構造プリミティブ（Rust アルゴリズム）
- 27 語 … exploratory
- 24 語 … 一見「合成可能」だが、大半は `string`/`CHARS`/`JOIN`/`JSON@PARSE`/`CRYPTO@HASH`/
  `MATH@IS-EXACT` のように **Rust アルゴリズムを要するプリミティブ**（合成ではない）
- 15 語 … hosted effect（**削減不可**）

Ajisai の語だけで真に合成可能なのは `MATH@SIGN`/`MIN`/`MAX`/`ABS` 程度で、対応する
Rust は **約170行**。これを移すには「Ajisai ソースでビルトインを定義する bootstrap-prelude
機構」（ローダ＋登録＋§8/§9.3 の仕様変更、推定100〜250行＋恒久的な複雑性）が要る。
**機構コスト ≥ 削減量** で、trusted core は縮まない。加えて §8.2「ビルトイン語は再定義
できない」の下、これは新しい定義経路という別の複雑性を持ち込む。

> 参考: `rust/tests/minimal_core_derivation.rs` は `MATH@SIGN` が Minimal Core だけで
> 合成できることを実証済み。だが「1語ずつ移す」ことは実装縮小には**ならない**——縮むのは
> 語彙の実装であって、それを支える数値エンジン・パーサ・評価器・辞書は trusted core に残る。

### 2.2 デッドコード削減
`cargo clippy --lib` が報告する「never used」は **feature-gate の誤検知**だった:
`collect_core_builtin_definitions`・`user_dictionary_names/words`・`hover_syntax`/`description`
はいずれも **wasm feature ビルド**（`wasm_interpreter_bindings`）で使われており、デフォルト
feature のリントに現れないだけ。除去すれば wasm ビルドが壊れる。**安全に消せる贅肉はない。**

## 3. 結論 — 実装規模は言語同一性そのもの

質量の中心（~7,300 行）は正確実数エンジンであり、これは §1 Language Identity が掲げる
「**すべての数値は正確実数（連分数）**」を実現する当のものである。これを縮めることは
Ajisai を Ajisai たらしめるものを捨てることに等しい。契約メタデータは宣言的で必要、
exploratory 機能は Web アプリで実際に使われる（audio）か意図的に保持される（子ランタイム）
ものであって死荷重ではない。**「実装が大きい」という観察は正しいが、安全に削れる部分は
ほとんど存在しない。** 規模は機能の正直な反映である。

## 4. 懸念4への本当の答え — 削除ではなく「境界」

保守負担への答えは*コードを消す*ことではなく、**「壊れたら言語同一性が崩れる領域」を
小さく*見なせる*境界を引く**ことである。そしてそれは**既に引いた**:

- **Ajisai Minimal Core = 47 語**（`identity`+`flow`, §2.6・`core_tier`）を「trusted な幹」と
  し、残り 138 の材料語を「導出可能・置換可能なライブラリ」と見なす。
- 幹の可観測契約は §2.6 の**後方互換保証**で固定され、材料層は Minimal Core の伝播規律に
  拘束されつつ自由に進化・再実装できる（園芸品種が育つ層）。
- `minimal_core_derivation.rs` はこの「材料は Core から導出できる」を実行可能な形で実証し、
  その過程で `MATH@SIGN`/`NEG`/`ABS` の欠陥まで炙り出して是正した。

つまり **46,686 行を保守可能にしているのは「行数を減らすこと」ではなく「47 語＋エンジンを
trusted core と見なし、他を派生と扱える境界」** である。懸念4は、この境界の存在によって
既に実務的に解消されている——巨大さは残るが、**理解し保守すべき同一性の面積は 47 語＋
数値エンジンに縮んでいる**。

## 5. 非目標（明示）

- 実装規模の削減を看板指標として追わない。規模は正確実数機能の正直な帰結である。
- Minimal Core の導出力を「trusted core 縮小」と誤って喧伝しない（§2.6 witness の但し書き）。
- 見かけの行数のために機能を犠牲にしない。

## 6. 残された（限定的・要判断の）レバー

将来*本当に*デフォルトビルドを縮めたい場合の候補。いずれも保守面積そのものは減らさない。

1. **exploratory 層の feature-gate**（audio/music＋子ランタイム ~2,600 行）。オプトインの
   cargo feature 化でデフォルト/組込みビルドは縮むが、コードは repo に残り保守対象のまま。
   conformance が audio を叩くかの確認が前提。
2. **契約メタデータの重複統合**（registry 系 ~5,400 行）。真の重複があれば単一ソース化で
   削減しうるが、CI 検証対象でリスク中。要調査。
3. **数値エンジンの整理**（`continued_fraction.rs` 5,150 行の多段リファクタ）。高リスク・高工数。

本メモの立場: これらは「実装が大きい」という*見かけ*には効くが、保守負担という*実質*には
限定的にしか効かない。懸念4の本質的解決は §4 の境界であり、追加のコード削除は要さない。
