# ユーザー表面への情報隠蔽 — 対費用効果調査と実施範囲

Status: 提案・未実施（調査記録を含む。§2.1 の漏れは実測で確認済み）
Authority: non-canonical. 本書は Ajisai の意味論・互換性方針を定義しない。
正典は `SPECIFICATION.html` のみ。

先行文書: `hidden-class-shape-optimizations.md`（直近の先例）、
`implicit-parallelism-roadmap.md`（Zero Syntax / Same Result / Never Slower）、
`three-layer-documentation-model.md`（ワードヘルプ三層）、
`cost-model-observability-design.md` / `cost-model-user-guidance-design.md`
（コストは診断チャネル）、`human-surface-blackbox-instruction-review.md`
（コード構造側の二層化 — 本書とは軸が異なる）。

---

## 0. 原則 — 二層公開モデル

hidden-class 改修で採った方針を、言語全体の規律として明文化する。

- **使う人（Ajisai を書くだけの人）**: 内部機構の知識ゼロで恩恵を受ける。
  機構は「速度」としてだけ現れる。エラー・NIL・スタック表示・LOOKUP・
  Reference・Playground に、内部機構の語彙も経路依存の差異も現れない。
- **作る人（処理系を作る・移植する・計測する人）**: 全開示。
  `SPECIFICATION.html`、`docs/dev/`、`RuntimeMetrics` / `energyProxyScore`、
  CLI `--json`、kill switch（`AJISAI_NO_*`）、差分テスト・shadow validation が
  このチャネルであり、隠蔽の対象では**ない**。むしろ本書の施策は
  「作る人チャネルには自由に書ける」ことを保証するために、チャネルの境界を
  機械検査にする。

### 既に隠蔽が完了している範囲（先例）

| 機構 | ユーザーからの見え方 | 開示チャネル |
| --- | --- | --- |
| D1 スカラー fast path / 内部 GOTO ×4 / HOF メモ化 | 速度のみ | 各 `[実装済み記録]` 文書 + kill switch |
| Record レイアウト intern・呼び出しサイト特殊化・shape IC | 速度のみ | `hidden-class-shape-optimizations.md` |
| VTU 密表現・暗黙並列 | 速度のみ（「新しい概念ゼロ」） | roadmap + 観測カウンタ |
| コストモデル | Reference の語彙（速いレーン/昇格/比較予算） | SPEC §4.8 + カウンタ |
| 文書役割 | Reference = 使う人 | Specification = 作る人（README の表） |

つまり情報隠蔽は既に Ajisai の支配的規律である。残る仕事は新しい隠蔽機構の
発明ではなく、**(1) 現に漏れている箇所の修理、(2) 漏れを構造的に再発不能に
する検査、(3) 計画済みのユーザー語彙表面の完成**の三つに絞られる。

---

## 1. ユーザー可視面の棚卸し

使う人に届く経路は有限で、以下で全てである（= 隠蔽を「行き渡らせる」対象領域）。

| # | 面 | 実体 | 現状 |
| --- | --- | --- | --- |
| 1 | エラーメッセージ | `AjisaiError` の `Display` → GUI/CLI | **漏れあり（§2）** |
| 2 | NIL プロトコル | `NIL-REASON` 等の protocol string | 意味論の一部。ただし経路依存で揺れる（§2.1） |
| 3 | スタック表示・ヒント | `Interpretation` / レンダリング | 差分テストで保護済み |
| 4 | LOOKUP / hover | `BuiltinSpec` 四セクション | 三層モデル Phase 2/3 未完 |
| 5 | Reference サイト | `public/docs/` | コストモデル頁あり。良好 |
| 6 | Playground GUI | 実行結果・辞書パネル | Cost パネルは未実装（計画済み follow-up） |
| 7 | ブラウザコンソール | `eprintln!` 類 | trace feature / `AJISAI_TRACE` でゲート済み。良好 |
| 8 | CLI `--json` / メトリクス | 診断チャネル | 作る人・AI 向け。隠蔽対象外 |
| 9 | 環境変数スイッチ | `AJISAI_NO_*` | docs/dev のみに記載。良好 |

---

## 2. 現に漏れている箇所（調査結果）

### 2.1 経路依存のエラー恒等性違反 — 実測で確認済み・最重要

同じ意味の計算が、**どの内部経路が実行したかによって観測可能な結果そのものが
変わる**。デフォルトビルド（`elastic-engine` なし）で再現:

```
[ 1 ] [ 0 ] /                 → NIL                （Bubble Rule 通り）
[ 1 2 3 ] { 0 / } MAP         → [ NIL NIL NIL ]    （一般経路: 要素ごとに泡）
[ 1 2 3 ] { [ 0 ] / } MAP     → エラー "MAP fast kernel: division by zero"
[ 1 2 3 ] { [ 0 ] % } MAP     → エラー "MAP fast kernel: modulo by zero"
```

三重の違反が重なっている:

1. **Bubble Rule 違反**: well-formed なゼロ除算は NIL 泡（reason
   `divisionByZero`）を返すべきところ、bulk fast kernel
   （`rust/src/interpreter/higher_order/fast_kernels.rs`）はプログラム全体を
   ハードエラーで落とす。
2. **プロトコルの分岐**: 一般経路の `ErrorCategory` / `NilReason` は
   `divisionByZero` だが、kernel 経路は `AjisaiError::Custom` 経由で
   `custom` になる。`NIL-REASON` / エラー分類を読むプログラムから
   **内部ルーティングが観測できる**。
3. **機構語彙の漏出**: "fast kernel" はユーザーが知らなくてよい語彙である。

対処（小規模）: kernel がゼロ除算・ゼロ剰余に遭遇したら、(a) 一般経路と同じ
NIL 泡をレーンに書く、または (b) kernel が辞退して一般経路へフォールバックする。
いずれでも観測結果は一般経路と一致する。同時に、`shape_ic_tests.rs` の
「ON = OFF」差分手法を**エラー/NIL 面まで**拡張したテストを添える
（現行の差分テストはスタック値・レンダリング・ヒントのみを比較しており、
「成功時は等しい」ことしか証明していない。この穴が本件を通した）。

### 2.2 機構語彙の漏出（文言のみ・値は正しい）

- `"PRECOMPUTE rejected: nested PRECOMPUTE is not supported in Phase 1"`
  （`comptime/policy.rs`）— 内部ロードマップ用語「Phase 1」。
- `"FOLD: expected return value from quantized block, got empty stack"`
  （`higher_order/runners.rs`）— 「quantized block」。
- `"Internal error: invalid consume access"`（`json.rs`）— invariant 違反の
  報告自体は正当だが、語彙を「internal invariant violation（発生したら
  処理系のバグ）」系に統一し、機構名は含めない。

---

## 3. 対費用効果ランキング

| 優先 | 施策 | コスト | 効果 |
| --- | --- | --- | --- |
| **S1** | 経路不変のエラー/NIL 恒等性（§2.1 の修理 + 差分テストのエラー面拡張） | 小 | 実証済みの意味論バグの修理。隠蔽の根拠（Same Result）を回復 |
| **S2** | ユーザー可視文字列の内部語彙ファイアウォール | 小 | **恒久予防。「全体に行き渡らせる」の実装そのもの** |
| **A3** | 経路等価性ハーネスへの一般化 | 中 | 将来のあらゆる最適化が自動的に「隠蔽されていること」を証明される |
| **B4** | Playground Cost パネルをコストモデル語彙で実装 | 中 | 計画済み follow-up。機構名を見せずに性能直感を与える |
| **B5** | 三層ドキュメントモデル Phase 2/3 完遂 | 中 | 使う人が SPECIFICATION（作る人向け）へ送られなくなる |
| C6 | 純粋定義体の自動 comptime 折り畳み | 大 | `PRECOMPUTE` の意味論（定義時実行）と重なるため慎重に。保留 |
| C7 | 暗黙並列ロードマップ続行 | 大 | 既存計画がそのまま本原則の旗艦。本書からの新規指示なし |

### S2 — 内部語彙ファイアウォール（最も費用対効果が高い）

`scripts/check-semantic-firewall.sh` に既にある「外部面への漏出を rg で落とす」
手法を、ユーザー可視文字列へ拡張する:

- 対象: `AjisaiError` の `Display` 実装・`AjisaiError::from(文字列)` /
  `Custom` に渡されるリテラル・`nil_diagnostics` の説明文・
  `BuiltinSpec` の LOOKUP/hover フィールド・GUI のラベル文字列。
- 禁止語彙（ユーザー可視文字列内）: `fast kernel` `fast path` `fastpath`
  `quantized` `compiled plan` `epoch` `memo` `intern` `inline cache`
  `shape IC` `hedged` `Phase <N>` など、docs/dev の機構語彙一式。
- 許可領域: テスト・`docs/dev/`・trace 出力（`AJISAI_TRACE` /
  cfg(feature) ゲート内）・`RuntimeMetrics` フィールド名・CLI `--json` キー。
  ここは作る人チャネルであり、firewall は触れない。

これが最優先である理由: 個別の漏れ修理（§2.2）は firewall を書けばその検査に
引っかかる形で列挙・消化でき、以後の新しい最適化が文言を漏らすことも CI で
止まる。隠蔽が「各改修の心がけ」から「機械検査される不変量」に変わる。

### A3 — 経路等価性ハーネス

`shape_ic_tests.rs` の `assert_ic_on_equals_off` を一般化し、
「同一プログラムを 経路 A / 経路 B で実行し、スタック値・レンダリング・
ヒント**・エラー文字列・ErrorCategory / NilReason** が一致する」ことを
主張する共通ハーネスにする。適用対象: bulk fast kernel（kill switch を
一つ新設するか、kernel が辞退する入力形で対照を作る）、quantized block、
（feature 有効時）hedged 実行。以後の最適化はこのハーネスに載せることを
`[実装済み記録]` 文書のテンプレート要件とする。

### B4 — Cost パネルの語彙規律

`collect_runtime_metrics()` は既に WASM に出ている。パネル実装時の規律を
一つだけ足す: **表示語彙は Reference のコストモデル頁の語彙**
（速いレーン / 昇格 / データ移動 / 比較予算）とし、内部カウンタ名
（`scalarFastpathCount` 等）は表示しない。カウンタ名は CLI `--json` と
WASM API（作る人・AI 向け契約）に留める。

---

## 4. 隠してはならないもの（非対象の明示）

- **意味論を持つ表面**: `NIL-REASON` 等の protocol string、`PRECOMPUTE` の
  定義時実行という時点意味論、`FORC`、`SPAWN`/`AWAIT` 系の明示並行、
  `CONSERVE`。これらは機構ではなく言語であり、隠蔽の対象にすると
  Same Result が壊れる。
- **作る人チャネル**: SPECIFICATION の実装規律章（§13–§16）、`docs/dev/`、
  `RuntimeMetrics` / `energyProxyScore`、kill switch、shadow validation。
  全開示を維持する。§3 の firewall は境界の検査であって、このチャネルの
  記述を制限するものではない。

---

## 5. 実施順序

S1 → S2 → A3 → B4 → B5。C6/C7 は本書の範囲外（既存 roadmap / 別途判断）。
S1 と S2 は独立に着手可能で、どちらも単一 PR 規模。
