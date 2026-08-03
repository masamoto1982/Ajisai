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

## 言語・実装

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `concept-reduction-2026-07.md` | 十概念への削減。何を捨て、何を残し、実装削減がどの順で進むか | `[設計根拠]` |
| `beta-freeze-2026-08.md` | β版改修指示書。57 canonical / 35 Semantic Kernel、互換層終了、版境界と品質gate | `[方針記録]` |
| `semantic-spine-migration-plan.md` | 整理後正典と整理前実装の乖離を収束させる Semantic Spine 移行計画（9 Phase） | `[方針記録]` |
| `ajisai-minimal-core-identity.md` | 何が変われば Ajisai でなくなるか——同一性の幹の切り分け | `[方針記録]` |
| `ajisai-self-hosting-design.md` | セルフホスティングの位置づけ（新しい権威層を作らない） | `[方針記録]` |
| `vector-nesting-role-redefinition.md` | Vector ネストの役割（Lisp 的動機の廃止） | `[方針記録]` |
| `spec-impl-drift-tactic.md` | 仕様と実装が食い違ったときの裁定戦術 | `[設計根拠]` |
| `language-coherence-review-2026-08.md` | 外部「整合化改修案」の検証と、正典収束計画への差し替え | `[方針記録]` |
| `ajisai-structure-mathematical-observations.md` | CF 値モデルの数学的観察 | `[観察ノート]` |

## エージェント/CLI・GUI

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `agent-cli-output-contract.md` | `ajisai` CLI の `--json` 出力契約 | `[設計根拠]` |
| `cli-repl-phase8a-design.md` | `ajisai repl` の設計メモ | `[設計根拠]` |
| `cli-test-phase8a-design.md` | `ajisai test` の設計メモ（`#@` directive コメント） | `[設計根拠]` |
| `gui-current-design-memory.md` | GUI 現行設計メモ | `[設計根拠]` |
