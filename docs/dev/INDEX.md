# docs/dev/ INDEX

Status: non-canonical. この索引を含め、`docs/dev/` 配下の全文書は Ajisai の
意味論・互換性方針を定義しない。正典は `spec/` 配下の各ソースと、そこから生成される
`SPECIFICATION.html` のみ。

状態タグの意味:

- `[執筆規約]` — 正典・Reference の執筆規律。
- `[設計根拠]` — 現行実装が依拠する設計文書。コード・CI から参照される。
- `[方針記録]` — 採用済みの設計判断とその理由の記録。
- `[観察ノート]` — 実装の記述的分析。方針を定めない。

完了した改修指示書と日付付きの記録は、現在を統べる部分を生きている文書へ移したうえで
削除する（2026-09-30 に 23 件。本文はバージョン履歴にある）。移した先はこの索引の
各行に記す。

## 執筆規約・形式化

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `ajisai-authoring-style.md` | 正典 HTML 文書の執筆規約（コード/数式チャネル分離、KaTeX）。**§11 に字句へ文字を取るときの規準**（その文字が文書側で区切りとして稼働しているかを見る。空いている枠を消費するのと、使われている文字を取り上げる交換は別の判断。実例は `{ }`（空いていた→取った）と `:`（稼働中→見送った）。旧 `character-allocation-and-prose-2026-09.md` から移した） | `[執筆規約]` |
| `structured-prose-style.md` | 情報の形の選び方（文 vs label:value vs 表 vs 図）。多言語対応を理由とする | `[執筆規約]` |
| `reference-writing-style.md` | Reference 表面の執筆規約 | `[執筆規約]` |
| `specification-implementation-rules.md` | 実装の工学規律（命名、500行予算、制御フロー、コメント、知識で測る DRY）。1件目は `check-file-size-budget.mjs` が強制 | `[執筆規約]` |
| `three-layer-documentation-model.md` | ワードヘルプの三層モデル（Reference / LOOKUP / hover） | `[執筆規約]` |

## 言語・実装

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `spec-impl-alignment-methodology.md` | 仕様・実装整合化の4フェーズ手順とスイート裁定規則。`spec-impl-drift-tactic.md` の後継 | `[設計根拠]` |
| `ajisai-minimal-core-identity.md` | 何が変われば Ajisai でなくなるか——同一性の幹の切り分け。**付録 B に alpha 期の語彙の採用規則**（追加は「言語内で書けない語」か「導出可能な語一つとの入れ替え」のどちらか。語数そのものは alpha の制約にしない。最初の適用として τ を入れないと決めた記録）**と退役候補の列**（入れ替えに出す導出可能 10 語を失うものが少ない順に固定。`scripts/check-minimal-core.mjs` が列の不変条件を検査する。語数を 90 に抑えて枠を空ける案を採らなかった記録も含む）。旧 `vocabulary-100-work-order-2026-09.md` §7.3・§7.4 から移した | `[方針記録]` |
| `vector-nesting-role-redefinition.md` | Vector ネストの役割（Lisp 的動機の廃止） | `[方針記録]` |

## エージェント/CLI・GUI

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `agent-cli-output-contract.md` | `ajisai` CLI の `--json` 出力契約 | `[設計根拠]` |
| `gui-current-design-memory.md` | GUI 現行設計メモ（モバイル提示で撤回した改修とその理由を含む） | `[設計根拠]` |
| `mcp-evaluation.md` | MCP サーバーの評価ハーネス（二言語コーパス、trace/repair スコアラー、捕捉済みモデルベースライン、性能・応答バイト予算）。`tools/mcp-server/README.md` は接続して使う側の文書に分け、評価の記述はこちらへ移した | `[設計根拠]` |
| `mcp-host-profiles.md` | ホストごとの資源上限プロファイル対照表と、意図された差分。導出の統一で退けた案、`collectionWork` を別上限にした理由、較正ホストの訂正も本書に持つ | `[設計根拠]` |
| `lexicon-emergence-pilot-results-2026-09-23.md` | 語彙創発実験 Phase 1（パイロット、同一モデル 12 体・168 解答）の結果。全問正答で、正答率と圧縮率には情報がなかった。solo 条件だけで 34 類中 13 類が独立到達されたため、停止条件に該当し、H1 は判定不能。H5 に反例（`KEEP` をユーザー語に掛けた場合と、その本体を `EXEC` した場合で結果が違い、この形の語が継承されて 21 解答に伝播）。計測器の実測 4 件（DIGEST は束縛名を区別する → D0α、CONTRACT は `BIND` 本体で `inputs: variable`、探針不足による D1 の過結合、辞書の重複）と、Phase 2 の前提条件。付録 A に `report.md` の数値（H1 の類、H4 の Core 語別使用数）、付録 B に作業指示書のうち参照される設計（禁止事項・停止条件・経路・同一性の三段・仮説 H1〜H5・条件・統制・Phase） | `[観察ノート]` |
| `total-division-three-points.md` | 除算を全域にし、0 の上の組を `1/0`・`-1/0`・`0/0` の三点へ既約化した判断。`divisionByZero` 理由・`zeroDenominator` 規則・密テンソル不在マップを捨てた理由と、三値論理に残る唯一の橋（`0/0` の順序）。 | `[方針記録]` |
| `trichotomy-unification.md` | 実行時三分法と静的検査三値の対応を統一した理由と、reason レジストリ統合（案(b)）を今やらない技術的理由・再検討条件 | `[方針記録]` |
| `cost-contract-design.md` | `#:contract` のコスト軸（steps/numeric/collection）の設計根拠。クラス格子・join規則・多項式を今やらない理由・機械非依存性の正確な意味。**付録 A に SHA-256→BLAKE3 置換を採用しない根拠と再検討条件**（旧 `cost-discoverability-work-order-2026-08.md` 付録 A から移した） | `[設計根拠]` |
