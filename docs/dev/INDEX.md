# docs/dev/ INDEX

Status: non-canonical (SPECIFICATION.html §2.2). この索引を含め、`docs/dev/` 配下の
全文書は Ajisai の意味論・互換性方針を定義しない。正典は `SPECIFICATION.html` のみ。

> **`archive/` 内は歴史的文書であり、現行方針ではない。**
> 完了済みの handoff・rollout 計画・instruction review・仕様へマージ済みの提案・
> 陳腐化した運用メモを保存している。現行の設計・実装・方針の根拠として
> 参照してはならない。記述が現状と食い違っていても更新されない。

以下は「生きている」文書の一覧。状態タグの意味:

- `[執筆規約]` — 正典・Reference の執筆規律。SPECIFICATION.html から参照される。
- `[設計根拠]` — 現行実装が依拠する設計文書。コード・CI・ベンチから参照される。
- `[実装済み記録]` — 実装済み最適化・機構の設計記録。当該コードの読解に必要。
- `[方針記録]` — 採用済みの設計判断とその理由の記録。
- `[観察ノート]` — 実装の記述的分析。方針を定めない。
- `[提案・未実施]` — 未承認の提案、または未着手・進行中の作業指示。

## 執筆規約・形式化

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `ajisai-authoring-style.md` | 正典 HTML 文書の執筆規約（コード/数式チャネル分離、KaTeX） | `[執筆規約]` |
| `reference-writing-style.md` | Reference 表面の執筆規約 | `[執筆規約]` |
| `three-layer-documentation-model.md` | ワードヘルプの三層モデル（Reference / LOOKUP / hover） | `[執筆規約]` |
| `ajisai-mathematical-formalization.md` | 数学的形式化。law tests（`rust/tests/*_laws.rs`）と coverage の根拠 | `[設計根拠]` |
| `ajisai-formalization-expansion-roadmap.md` | 形式化拡張のフェーズ定義。law-test ファイル群が参照 | `[設計根拠]` |

## エージェント/CLI・計測

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `agent-cli-output-contract.md` | `ajisai` CLI の `--json` 出力契約（SKILL.md 生成の入力） | `[設計根拠]` |
| `natural-language-surface-design.md` | CLI 自然言語サーフェス（explain / plan-check / modifier / clarify）設計 | `[設計根拠]` |
| `capability-transition-measurement-design.md` | モデル能力階級スイープ計測の設計（`bench/agent-suite/`） | `[設計根拠]` |
| `ai-first-competitive-upgrade-instructions.md` | AI-first 改修 work order。Phase 1–3 実装済み。§6.3 が verified lowering の実装ゲートとして現役。`scripts/generate-skill-md.mjs` が参照 | `[設計根拠]` |
| `ajisai-use-language-identity.md` | 「使う言語」に振り切る設計判断の計測接続記録 | `[方針記録]` |

## ランタイム・値モデル設計

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `virtual-tensor-unit-design.md` | VTU（観測カウンタ・エネルギープロキシ）の設計 | `[設計根拠]` |
| `fintech-value-integrity-design.md` | `QUANTIZE` / `CONSERVE` の設計（SPEC §7.13 / §13.3 の由来） | `[設計根拠]` |
| `web-serial-module-design.md` | `SERIAL` モジュール（Web Serial / Tauri ブリッジ）の設計 | `[設計根拠]` |
| `browser-parallelism-phase5-rollout.md` | COOP/COEP cross-origin isolation ブートストラップの設計 | `[設計根拠]` |
| `gui-current-design-memory.md` | GUI 現行設計メモ（Math view 等） | `[設計根拠]` |
| `physical-resilience-design.md` | shadow validation による意味完全性機構の設計 | `[設計根拠]` |
| `source-provenance-attestation-design.md` | source attestation（`npm run provenance:*`）の脅威モデルと設計 | `[設計根拠]` |
| `spec-impl-drift-tactic.md` | 仕様と実装が食い違ったときの裁定戦術（suite-arbitration） | `[設計根拠]` |
| `wasm-style-reference-interpreter-design.md` | 参照インタープリタ（`tools/ajisai-repro/`）の設計 | `[設計根拠]` |
| `ajisai-self-hosting-design.md` | セルフホスティングの位置づけ（新しい権威層を作らない） | `[方針記録]` |
| `vector-nesting-role-redefinition.md` | Vector ネストの役割再定義（Lisp 的動機の廃止、テンソル/構造データ基盤への固定） | `[方針記録]` |
| `implicit-parallelism-roadmap.md` | 暗黙並列の設計原理（Same Result / Never Slower / Zero Syntax）。elastic/hedged 実行エンジンは opt-in cargo feature `elastic-engine` に隔離済み（デフォルトビルドは常に greedy） | `[設計根拠]` |
| `ajisai-structure-mathematical-observations.md` | CF 値モデルの数学的観察（`_attachments/cf_probe.py` を含む） | `[観察ノート]` |

## 実装済み最適化の設計記録

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `internal-goto-tail-call.md` | 内部 GOTO ①: 末尾呼び出しの後方ジャンプ化 | `[実装済み記録]` |
| `internal-goto-cond-dispatch.md` | 内部 GOTO ②: COND 節ディスパッチの事前計算 | `[実装済み記録]` |
| `internal-goto-literal-vectors.md` | 内部 GOTO ③: リテラルベクタのコンパイル | `[実装済み記録]` |
| `internal-goto-compiled-clauses.md` | 内部 GOTO ④: COND ガード/ボディのコンパイル | `[実装済み記録]` |
| `arith-sqrt-i128-fastpath.md` | 二次無理数 CF 状態の i128 fast path | `[実装済み記録]` |
| `arith-mobius-i128-fastpath.md` | Möbius（単項 Gosper）CF 状態の i128 fast path | `[実装済み記録]` |
| `scalar-fastpath-d1.md` | D1 スカラー算術 fast path の実装記録 | `[実装済み記録]` |
| `hof-kernel-memoization.md` | 純粋 HOF カーネルのメモ化（MAP / 述語族） | `[実装済み記録]` |
| `dependents-inverted-index-reads.md` | 逆依存クエリの転置インデックス読み出し化 | `[実装済み記録]` |
| `hidden-class-shape-optimizations.md` | Hidden class 流の形状最適化（Record レイアウト intern 化・ビルトイン呼び出しサイト特殊化・shape IC） | `[実装済み記録]` |

## 提案・未実施の作業

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `vtu-verified-lowering-design.md` | VTU verified lowering 設計案（未承認。実装禁止ゲートつき） | `[提案・未実施]` |
| `human-surface-blackbox-instruction-review.md` | Human/Machine Surface 二層化・`debug_diagnosis` 配置是正の指示書レビュー（未実施） | `[提案・未実施]` |
| `semantic-metadata-refactor-checklist.md` | 意味論的混在排除チェックリスト（一部実施済み、`Capabilities::PURE` 排除等が残） | `[提案・未実施]` |
