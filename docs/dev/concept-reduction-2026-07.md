# 十概念への削減 — 何を捨て、何を残したか

> Status: **Non-canonical / 設計メモ.** 本書は Ajisai の意味論を定義しない。正典は
> `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。
> 本書は削減の**根拠**と、実装削減がどの順で進むかを記録する手続き文書である。

## 0. 問題

Ajisai は各種言語の良い部分を取り込み続けた結果、**名前の付いた設計コンセプトが約60個**に
達していた。語彙が多いことが問題なのではない——語彙は194語だったが、真の質量は語彙の
外にあった。

| 層 | コンセプト数（削減前） |
| --- | --- |
| 言語意味論 | 4分類結果モデル・三値論理・Tier 階層・観測 water・4種の予算・修飾子直交2軸・解釈ロール・NIL 構造化診断・Vector/Record/Tensor/dense・辞書3階層・子ランタイム・4 Profile・フロー質量保存・Cost Model | 15 |
| 性能機構 | VTU・QuantizedBlock/SIMD・shape IC・内部 GOTO 4種・i128 fast path 2種・HOF メモ化・ArtifactStore・comptime staging・elastic/hedged・暗黙並列 | 13 |
| 検証・保証 | shadow validation・mass conservation・Space Contract・Structural Constraint Ledger・execution receipt・provenance attestation・3実装 conformance・MC/DC・トレーサビリティ行列・energyProxyScore | 12 |
| ツール表面 | CLI 18 サブ機能・自然言語サーフェス・三層ドキュメント・能力遷移計測・MCP サーバ・Python 移植 | 10 |
| 権威・メタ | 4層権威モデル・Minimal Core 47語・28 semantic family・セルフホスト方針・drift 裁定 | 5 |

比較対象: Lua は概ね10〜15、R7RS-small は12前後。**Ajisai は4〜5倍**だった。

## 1. 判定基準

コンセプトを3つの問いにかけた。

1. **それが変わると Ajisai でなくなるか。** ならば幹。
2. **観測可能な挙動を変えるか。** 変えないなら、それは概念ではなく計測器・最適化・診断である。
3. **同じ性質を別の角度から既に保証していないか。** していれば重複。

この3問で、L2（性能機構）はほぼ全滅した——設計ノート自身が「VTU は観測してラベルを
付けるだけで実行経路を変えない」「elastic/hedged はデフォルトビルドに含まれず常に
greedy と同一」と明記していたからである。**実行を変えないものは概念ではない。**

L3（検証機構）は重複で落ちた。「最適化経路が意味を壊さない」ことを shadow validation・
mass conservation・route equivalence・differential tests の4通りで、「実行前に安全性が
分かる」ことを `check --contract`・Space Contract・Structural Constraint Ledger の3通りで
保証していた。各行を1つに畳んだ。

## 2. 残した十概念

1. 厳密有理数、`SQRT` による代数的閉包（multiquadratic）まで。丸めなし・任意精度。
2. 値 / 理由付き NIL / ERROR の三分類。第四の結果はない。
3. スタックと Vector。
4. コードブロックと明示評価。
5. 消費修飾子 EAT/KEEP の一軸のみ。
6. Core（封印）/ User の二階層辞書と content addressing。
7. 全 Word の機械可読契約。
8. 実行前の `check --contract`。
9. HostProtocolV1。
10. conformance corpus。

加えて**現行 GUI とその使い勝手**、**Reference の様式**を維持する。

## 3. 捨てたもの（言語意味論）

| 捨てた概念 | 理由 |
| --- | --- |
| UNKNOWN・三値 Kleene 論理 | 第四の結果カテゴリ。真理値表・伝播規則・テスト規律を丸ごと連れてくる |
| Tier 階層と computable real（π・ENCLOSE） | UNKNOWN・観測 water・`COMPARE-WITHIN`・`agreedPrefix`・Starved 経路の発生源。**単一削除で最多の概念が落ちる** |
| 比較 water・区間演算 | 「厳密」を掲げる言語に近似・包含の第2数値ドメインが併存していた |
| TOP/STAK 目標軸 | 全 Word の contract とテストに掛け算で効く横断軸。`STAK` は全語のアリティを可変にしていた |
| 解釈ロール | 値タグが disjoint なら第2の型系にすぎない。表示は値タグから導出する |
| Record | 構造モデルが Vector / dense tensor / Record / JSON object / dataTable の5つ併存していた |
| テンソル代数（rank/reshape/transpose・broadcast 形状規則） | Vector は列であって tensor ではない、と決めた。要素ごとの持ち上げ（LANG.COLLECTIONS.LIFT）は残す |
| モジュール系（9 モジュール・4 import 語・3階層） | 語彙の49.5%がモジュール語だった。フラットな Core に畳んだ |
| 子ランタイム・supervision | Erlang 由来の並行＋監視モデル一式。値ドメイン2つも連れていた |
| output 以外の hosted effect（音楽・シリアル・時刻・乱数・JSON・CSV・ハッシュ） | 言語ではなくホストライブラリ |
| フロー質量保存(§13.1)・Cost Model(§4.8) | 実装自身が「診断専用・実行をゲートしない」と明記。観測可能な挙動を変えないものが normative だった |
| Presentation Profile の遷移系形式化 | GUI の実挙動は維持し、LTS と6不変条件の形式化のみ落とした |
| NIL の origin / recoverability / 構造化 diagnosis | 診断サブシステムが値ドメインに埋め込まれていた。`reason` のみ残す |

## 4. 数値

| 指標 | 削減前 | 削減後 |
| --- | --- | --- |
| 名前の付いた設計コンセプト | 約60 | **14** |
| 正典 Word | 194（9 モジュール） | **69**（フラット Core） |
| エイリアス | 20 | **16** |
| semantic family | 28 | **11** |
| 値ドメイン | 10＋ロール7 | **6** |
| 結果カテゴリ | 4 | **3** |
| 予算概念 | 4 | **2** |
| 正典本文 | 26,637字 | **20,449字** |
| `docs/dev/` 生存文書 | 67 | **14** |

`npm run semantic-kernel:check` がこの数を**上限**として固定する。縮むのは常に自由、
増えるのは意図的な仕様変更でしかありえない。

## 5. 実装削減の順序

仕様は先行して確定した。実装は仕様に追従する。実測に基づく順序は次のとおり。

各段階は「その段階だけで `cargo check` が通る」ようには**ならない**——値ドメインと
修飾子軸は横断的だからである。段階1〜3は一括で通すしかない。

1. **独立サブシステムの削除**（完了済み）。audio/music・serial・data_ops・modules・
   comptime・elastic・child_runtime/parallel・energy_proxy・receipt_recorder・
   artifact_store・shadow_validation・mass_conservation・json・datetime/time・
   hash/random・tier2_ops・quantized_block・shape_ic・CLI 拡張機能。実測 **103 ファイル・
   約30,000行**。`rust/src` は 76,277 → 48,528 行。
2. **TOP/STAK 軸の除去**（完了済み）。`operation_target_mode` は **27 ファイル・214 参照**。
   `match ... operation_target_mode` は StackTop 腕へ畳み、`!= StackTop` ガードは死ぬ。
   完全に機械的で、スクリプト化できる。
3. **値ドメインの削減**（進行中）。`ValueData::{Unknown, Record, ProcessHandle,
   SupervisorHandle}` は **220 参照**。網羅 match の腕は機械的に落とせるが、
   コンストラクタ・`matches!`・タプル match は手作業になる。質量は
   `types/value_operations.rs`（108 参照）に集中する。
4. **語彙レジストリの整合**（完了済み）。`BUILTIN_SPECS` を 98 → 69 エントリに絞り、
   MATH/ALGO から昇格した10語（`SQRT` `ABS` `NEG` `SIGN` `MIN` `MAX` `SORT` `UNIQUE`
   `CONTAINS` `INDEX-OF`）を Core エントリとして追加する。
5. **実行器の再配線**。`execute_builtin` のディスパッチ表を `spec/words.json` の
   `executorKey` に一致させる。`op_sqrt` は削除した `interval_ops` にあったので
   `math_ops` へ移す（有理入力の厳密経路のみ残し、区間フォールバックは落とす）。
6. **二値論理への置換**。`logic_kleene` を落とし、`logic` を Boolean 演算に戻す。
7. **プロトコル境界**。`host-protocol-v1.schema.json` から Unknown・Record・handle・
   module・capability のペイロードを外し、WASM 変換を追従させる。
8. **テスト木**。`rust/src` と `rust/tests` の **45 ファイル**が削除済み語彙
   （`MUSIC@` `TIME@` `JSON@` `MATH@` `ALGO@` `UNKNOWN` `STAK` …）を参照している。
   conformance corpus は削減後の語彙で書き直す。
9. **TypeScript / GUI**。GUI の**挙動**は維持する。Dictionary パネルから Module シートが
   消えるのはモジュール削除の帰結であり、それ以外の面・操作・ショートカットは不変。
10. **Reference 再生成**。様式（`public/docs/` の HTML 構成・検証済み例・Playground
    リンク）を維持したまま、69語へ内容を絞る。

## 6. 非目標

- 実装規模の削減を看板指標として追わない。規模は残した機能の正直な帰結である。
- 「速いから」という理由だけで概念を戻さない。観測可能な挙動を変えない最適化は、
  名前を持たない実装詳細として `compiled_plan` の内側に置く。
- 捨てた概念を「将来の予約」として仕様に残さない。Tier 2 の教訓がそれである——
  到達不能な意味論のために4分類・真理値表・予算・診断フィールド・テスト規律を
  維持していた。必要になったら、そのとき versioned extension として入れる。
