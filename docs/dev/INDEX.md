# docs/dev/ INDEX

Status: non-canonical. この索引を含め、`docs/dev/` 配下の全文書は Ajisai の
意味論・互換性方針を定義しない。正典は `spec/` 配下の各ソースと、そこから生成される
`SPECIFICATION.html` のみ。

状態タグの意味:

- `[執筆規約]` — 正典・Reference の執筆規律。
- `[設計根拠]` — 現行実装が依拠する設計文書。コード・CI から参照される。
- `[方針記録]` — 採用済みの設計判断とその理由の記録。
- `[観察ノート]` — 実装の記述的分析。方針を定めない。

## 執筆規約・形式化

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `ajisai-authoring-style.md` | 正典 HTML 文書の執筆規約（コード/数式チャネル分離、KaTeX） | `[執筆規約]` |
| `reference-writing-style.md` | Reference 表面の執筆規約 | `[執筆規約]` |
| `three-layer-documentation-model.md` | ワードヘルプの三層モデル（Reference / LOOKUP / hover） | `[執筆規約]` |
| `ajisai-mathematical-formalization.md` | 数学的形式化。law tests の根拠 | `[設計根拠]` |
| `devlog-format.md` | Blogger 開発ログ記事の形式（目的/手段/結果/課題の四項目・約500字）と誠実性の規律 | `[執筆規約]` |

## 言語・実装

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `concept-reduction-2026-07.md` | 十概念への削減。何を捨て、何を残し、実装削減がどの順で進むか | `[設計根拠]` |
| `semantic-spine-migration-plan.md` | 整理後正典と整理前実装の乖離を収束させる Semantic Spine 移行計画（9 Phase） | `[方針記録]` |
| `ajisai-minimal-core-identity.md` | 何が変われば Ajisai でなくなるか——同一性の幹の切り分け | `[方針記録]` |
| `ajisai-self-hosting-design.md` | セルフホスティングの位置づけ（新しい権威層を作らない） | `[方針記録]` |
| `vector-nesting-role-redefinition.md` | Vector ネストの役割（Lisp 的動機の廃止） | `[方針記録]` |
| `spec-impl-drift-tactic.md` | 仕様と実装が食い違ったときの裁定戦術 | `[設計根拠]` |
| `language-coherence-review-2026-08.md` | 外部「整合化改修案」の検証と、正典収束計画への差し替え | `[方針記録]` |
| `ajisai-structure-mathematical-observations.md` | CF 値モデルの数学的観察 | `[観察ノート]` |
| `ajisai-single-axis-proposal-2026-08.md` | 中心概念を「絞り込み（narrowing）」一本に定める提案。到達不能契約の実測と、七つの改修案。うちⅡ・Ⅲ・Ⅴ・Ⅵ・Ⅶは実施済み（PR #1563/#1564/#1567）、Ⅰは指示書のみ | `[観察ノート]` |
| `type-unification-work-order-2026-08.md` | 改修Ⅰ（CodeBlock/Vector 統合）の指示書。読み取り専用の前提再検証（Phase 0）→ 使い捨てブランチでの測定スパイク（Phase 1）→ 実装（Phase 2、未着手）の三段階。破壊的変更であり互換性方針の扱いを人間の判断待ちとしている | `[設計根拠]` |
| `type-unification-phase0-report-2026-08.md` | 改修Ⅰの Phase 0 実測報告。`Value::Symbol` 追加で壊れる 48/49 箇所、非交差性に依存する conformance 22 件、書き換えが要る正典 6 節の確定一覧。指示書の停止条件が発火しており Phase 1 は未着手 | `[観察ノート]` |
| `type-unification-phase1-report-2026-08.md` | 改修Ⅰの Phase 1 スパイク報告。使い捨てブランチ（main 非マージ）で (A) 案を実装し conformance 279 件中 23 件の非通過を実測（Phase 0 の見積もり 22 件とほぼ一致）。`COND` の節ブロック発見が統合と構造的に相容れないという新しい壁を発見 | `[観察ノート]` |

## エージェント/CLI・GUI

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `agent-cli-output-contract.md` | `ajisai` CLI の `--json` 出力契約 | `[設計根拠]` |
| `cli-repl-phase8a-design.md` | `ajisai repl` の設計メモ | `[設計根拠]` |
| `cli-test-phase8a-design.md` | `ajisai test` の設計メモ（`#@` directive コメント） | `[設計根拠]` |
| `gui-current-design-memory.md` | GUI 現行設計メモ | `[設計根拠]` |
| `mcp-host-profiles.md` | ホストごとの資源上限プロファイル対照表と、意図された差分 | `[設計根拠]` |
| `mcp-readiness.md` | MCP 製品化の実装トラッカー（達成した exit criteria のみを記録する） | `[方針記録]` |
| `mcp-claude-code-handoff.md` | MCP 開発の次担当への引き継ぎ（現行方針・禁止事項） | `[方針記録]` |
| `host-profile-derivation-handoff.md` | ホスト間で上限の値ではなく導出を統一する作業の引き継ぎ。走査系の非二次化の後に着手 | `[方針記録]` |
| `competitive-advantage-work-order-2026-08.md` | 競争優位の研磨（観測ダイジェスト・全数意味論表・gap ID・三分法統一・コスト契約）の改修指示書。Phase 単位で実装する。設計判断は本書で確定済み | `[設計根拠]` |
| `trichotomy-unification.md` | 実行時三分法と静的検査三値の対応を統一した理由と、reason レジストリ統合（案(b)）を今やらない技術的理由・再検討条件 | `[方針記録]` |
| `cost-contract-design.md` | `#:contract` のコスト軸（steps/numeric/collection）の設計根拠。クラス格子・join規則・多項式を今やらない理由・機械非依存性の正確な意味 | `[設計根拠]` |
| `cost-discoverability-work-order-2026-08.md` | 推論されたコストを `ajisai contract` に出す改修指示書。付録 A に SHA-256→BLAKE3 置換を採用しない根拠と再検討条件 | `[設計根拠]` |
| `competitive-advantage-round2-2026-08.md` | 競争優位の研磨・第二巡の提案（層0: アリティ推論の不健全性の確認、層1: ブロック契約値化・再帰の不動点・Assume-Guarantee、層2: 制約付きデコード面・観測スペクトル・意味論差分、層3: 検証済み修復・反例最小化、層4: 計算証明書）。実装指示書ではない | `[観察ノート]` |
| `reference-ja-restructure-handoff.md` | Reference 日本語版の再編（水のメタファー導入・制御構造の集約）の引き継ぎ。着手前に §8 の確認事項を利用者へ | `[方針記録]` |
