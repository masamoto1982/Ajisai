# Phase 5: compiled-artifact reuse design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 5: コンパイル成果物をセッション間で再利用する（引き継ぎ指示書 §12）。

## 目的

GUI Worker は実行のたびに `applyInterpreterSnapshot` から `interpreter.reset()` を呼び、
スタックや辞書だけでなく、意味的に不変なコンパイル成果物（`CompiledPlan`）まで破棄していた。
本フェーズは、内容が変わっていないユーザー語の `CompiledPlan` を reset をまたいで再利用し、
毎回の再コンパイルを避ける。初期スコープは同一 Interpreter / 同一 Worker 内の再利用に限る
（Worker 間の `Arc<CompiledPlan>` 直接共有・IR 直列化は対象外）。

## 権威と安全性の根拠

再利用の鍵は、既存の content identity（§8.6, `word_identities`）である。この identity は
語本体トークンと、依存語の identity を SCC 単位で畳み込んだ値であり、名前に依存しない。
`CompiledPlan` は依存語を名前で呼ぶ（`CallUserWord` / `CallQualifiedWord`）ため、
本体と全依存が同一なら plan は再利用しても観測結果を変えない。したがって、

- 本体が変われば identity が変わる → 別キー → 再利用しない。
- 依存語が再定義されれば依存 identity が変わり、参照元 identity も変わる → 別キー。
- builtin / user / free-symbol の解決区分が変わっても identity が変わる。

を content identity が構造的に保証する。加えて、plan の lowering 形状を変える compile flag
（`cond_dispatch` / `vector_literal` / `compiled_clause`）と plan schema version をキーに含める。

## 導入した構造

### `ArtifactStore`（`rust/src/interpreter/artifact_store.rs`）

- キー: `ArtifactKey { content_identity, flags: CompileFlags, schema_version }`。
- 値: `Arc<CompiledPlan>`。
- 上限付き LRU。容量超過で least-recently-used を退去（`eviction_count`）。
- 観測メトリクス: `build` / `hit` / `miss` / `eviction`。キャッシュなので、退去や
  再利用の無効化は「後で再コンパイルされる」だけで結果を変えない。

### reset の分離（`rust/src/interpreter/session_lifecycle.rs`）

引き継ぎ指示書 §12.4 の「セッション状態だけを消す操作」と「artifact も含めて完全に消す操作」を分離した。

- `execute_reset()`: 従来どおりの全消去に加え `artifact_store.clear()`。CLI / テスト用。
- `execute_session_reset()`: セッション状態のみ消去し、`artifact_store` を保持。GUI Worker 用。
- 両者は `reset_session_state()` を共有する。

`SessionState` / `ArtifactStore` の寿命分離は、この 2 メソッドの責務差として実現した。

### plan 取得経路（`build_or_reuse_compiled_plan`）

`get_execution_plan_set` の per-def epoch キャッシュ miss 後にのみ artifact store を参照する。
hit 時は plan を現在 epoch へ re-stamp して per-def キャッシュへ格納し、同一セッション内の
後続呼び出しが store 再参照ではなく epoch fast path を通るようにする。re-stamp のための clone は
再コンパイルより十分安価で、reused plan も初回は Shadow Validation を通る
（`validated_until_epoch` は 0 から始まる）。miss 時は従来どおりコンパイルし、store へ挿入する。

QuantizedBlock は epoch signature に依存するため、artifact store の対象外とし、
従来どおりセッションごとに再構築する。

## WASM / GUI 境界

- WASM: `reset_session()` を追加（既存 `reset()` は互換のため全消去のまま維持）。
  内部 stack 抽象や artifact store は公開しない。`collect_runtime_metrics` に
  `artifactCache{Build,Hit,Miss,Eviction}Count` を追加（cost-model 観測のみ）。
- GUI: `applyInterpreterSnapshot` は `reset_session` があればそれを、無ければ `reset()` を呼ぶ
  （古い wasm bundle への段階移行）。cost summary に「Compiled word reuse」行を追加。

## 無効化・A/B

`AJISAI_NO_ARTIFACT_REUSE`（および `set_artifact_reuse_enabled(false)`）で再利用を無効化できる。
無効化しても観測結果は不変（content-identity keyed のため）で、store は一切 hit しない。

## 必須テスト（`rust/src/interpreter/artifact_store_tests.rs`）

- session reset をまたいで plan build が起きない（reuse）。
- 全 reset は store を空にし、再コンパイルさせる。
- 語名が違っても同一内容なら再利用する。
- 同名でも依存 identity が異なれば再利用しない。
- 再定義後に古い plan を再利用しない。
- compile flag を変えると再利用しない。
- 容量超過で退去し、`artifact_store_len` が上限内に収まる。
- 再利用の有効/無効で観測結果が一致する。

GUI 側は `src/workers/interpreter-snapshot.test.ts` に `reset_session` 優先と reset フォールバックを追加。

## 非対象（初期スコープ外）

- Worker 間で `Arc<CompiledPlan>` を直接共有する。
- compiled executor pointer の直列化。
- IndexedDB への内部 `CompiledPlan` 保存。
- ブラウザ間の内部 IR 互換保証。

## 互換性

- 表層構文: 変更なし。
- CLI JSON: 変更なし。
- WASM: `reset_session` と artifact メトリクスの additive 追加のみ。既存 wire を維持。
- GUI: 表示は cost summary の additive 行のみ。既存表示は不変。
- conformance / reference interpreter: 影響なし。

## 仕様上の未解決点

なし。
