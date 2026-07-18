# Phase 6: execution receipt design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 6: 実行 receipt を導入する（引き継ぎ指示書 §13）。

## 目的

実行結果だけでなく、その結果がどのソース・語・Capability・参照照合に基づくかを機械可読にする。
「証明」とは名乗らず、`execution receipt`（実行 receipt）とする。CLI に
`ajisai run <file> --json --receipt` を追加し、既存 JSON 契約を壊さず `receipt` フィールドを付す。

## 権威と非公開境界

公開してよい情報のみを載せる（引き継ぎ指示書 §13.4）。

- 公開: 参照経路と一致したか、Fallback したか、どの Capability と効果を観測したか、
  どの content identity を持つ語を実行したか、入出力の identity。
- 非公開: SIMD lane 幅、Shape IC 内部状態、QuantizedBlock 内部、pointer identity、
  Tier 内部表現、Rust enum の `Debug` 名、非安定なキャッシュキー。

receipt は provenance 記録であり、「数学的証明」「改ざん不能」とは表示しない。

## 導入した構造

### `ReceiptRecorder`（`rust/src/interpreter/receipt_recorder.rs`）

opt-in の provenance recorder。既定では無効で、receipt 要求時のみ有効化する（§13.5）。
有効時のみ次を記録する。

- 実行された語を、解決後の fully-qualified name で集約（`ExecutedWord { first_seen_order, call_count }`）。
  ループや再帰は 1 エントリに畳み込む。
- Hosted 語が要求した Capability（付与の有無に関わらず）。

記録は完全に観測専用で、値・効果・制御・identity を一切変えない。無効時は各記録点で
bool 1 個の分岐のみ（ホットパスへの影響を最小化）。session reset でデータのみクリアし、
記録フラグは保持する。

記録フックは 2 箇所のみ:

- `execute_word_core_inner`: 解決後の語名を記録。
- `require_host_capability`: 要求 Capability を記録。

### receipt 組み立て（`rust/src/cli/receipt.rs`）

`build_receipt(interp, source, trace) -> Json`。次のフィールドを構築する。

- `schemaVersion`（receipt 形状の版。envelope の `schemaVersion` とは独立）
- `sourceIdentity`（ソースの content digest）
- `implementation { name, version }`
- `specification { declaredVersion }`（仕様に機械可読版がまだ無いため `null`。捏造しない）
- `executedWords`（content identity を持つ user 語のみ。core/module 語は implementation 側）
- `requiredCapabilities` / `grantedCapabilities`（protocol string）
- `observedEffects`（emission 順に `{ order, kind, payload }`）
- `water { stepLimit, stepsUsed, comparisonRefinements }`
- `integrity { shadowValidationPerformed, referenceAgreement, plainFallbacks, integrityMismatches }`
- `absenceEvents`（NIL の reason / origin / recoverability を保持。汎用失敗へ潰さない）
- `resultIdentity`

### resultIdentity（§13.6）

表示文字列は hash しない。CLI/WASM 共有の value protocol（`stack_json`）を
**キー昇順の canonical JSON** へ直列化し、そのバイト列から content digest を計算する。
これにより Value kind、exact 分子/分母、interpretation、NIL reason/origin/recoverability、
論理 UNKNOWN diagnosis、Vector/Tensor/Record 構造を失わない。content digest は §8.6 語 identity と
同じハッシュ族を再利用する（`interpreter::content_digest`）。

## CLI 統合

- `--receipt` フラグを追加。`run --json --receipt` の成功時のみ `receipt` を付す。
- envelope へ `receipt: object | null` を additive に追加（`explanation` / `planCheck` と同じ扱い）。
  `schemaVersion` は据え置き。error 実行と `--receipt` 無しでは `null`。
- 記録有効化は観測専用のため、`--receipt` の有無で stack/output/metrics は不変。

## 互換性

- 表層構文: 変更なし。
- CLI JSON: additive（`receipt` フィールド追加のみ）。既存フィールド不変、`schemaVersion` 据え置き。
  契約は `docs/dev/agent-cli-output-contract.md` §15 に記載。
- WASM / GUI: 変更なし（receipt は CLI 専用）。
- conformance / reference interpreter: 影響なし。

## 必須テスト（`rust/src/cli/receipt_tests.rs`）

- schemaVersion と source/result identity の存在。
- 記録有効/無効で結果不変（観測透過性）。
- resultIdentity が値を区別し、同一入力で安定。
- executedWords が content identity と callCount を集約（core/module 語は除外）。
- observedEffects の kind と順序保持。
- required/granted Capability の記録。
- absence event が reason/origin/recoverability を保持。
- integrity / water フィールドの存在。
- 内部最適化語彙（simd, quantiz, tier, epoch, pointer 等）を receipt へ漏らさない。

## 非対象（初期スコープ外）

- `ajisai verify`（receipt schema と再実行比較の意味を設計してから）。
- 暗号学的証明機構・署名。
- error 実行への receipt 付与（現状は成功実行のみ）。
- WASM への receipt 露出。

## 仕様上の未解決点

- 仕様に機械可読な版番号が無いため `specification.declaredVersion` は `null`。
  将来版が定義されれば additive に埋められる。この点で新しい正典意味論は確定していない。
EOF
