# Semantic Spine — 意味論・実装・Host Protocol の再統合計画

> Status: **Non-canonical / 設計メモ（`[方針記録]`）.** 本書は Ajisai の意味論・
> 互換性方針を定義しない。正典は `spec/` 配下の各ソースと、そこから生成される
> `SPECIFICATION.html` のみ。本書は「整理後の正典」と「整理前の実装座標系」の乖離を
> 記述し、その収束点（Semantic Spine）を作る移行手続きを定める。既存コードの一括削除を
> 先に行わないこと、まず構造上の収束点を作ることを原則とする。
>
> 前提文書: `concept-reduction-2026-07.md`（十概念への削減）、
> `reduction-consistency-audit-2026-07.md`（削減後整合監査）。本書はその実装側続編である。

---

## 0. 結論の要約

正典側は6値領域・二値論理・二階層辞書・最小 NIL・機械可読 Word 契約へ収束済みだが、
Rust 実装は**削減前の座標系**をまだ主軸に使っている。個別削除ではなく、意味論が存在
できる場所を1箇所（Semantic Spine）へ限定し、旧概念を「意味」から「内部実装 / 表示 /
互換 adapter」へ降格させることで複数の問題を同時に解く。

移行は9フェーズ。**Phase 1〜7 で新構造を並存させ、Phase 8 で consumer を切替え、
Phase 9 で到達不能になった旧型・旧ファイルを一括削除する。** 旧コードの削除を最初に
行わない。

---

## 1. 現在の semantic value model の実体

中心は `rust/src/types/mod.rs`。

### 1.1 `ValueData`（`mod.rs:371-390`）— 7 variant

```rust
pub enum ValueData {
    Boolean(bool),
    Scalar(Fraction),
    ExactScalar(ExactReal),                 // ← 正典6領域に無い
    Vector(Arc<Vec<Value>>),
    Tensor { data: Arc<DenseTensor>, shape: Arc<Vec<usize>> }, // ← 正典6領域に無い
    Nil,
    CodeBlock(Vec<Token>),
}
```

`String` variant は**存在しない**。

### 1.2 `Interpretation`（`mod.rs:346-369`）— 第二の型系, 8 role

```rust
pub enum Interpretation {
    Unassigned, RawNumber, Interval, Text, TruthValue,
    Timestamp, Nil, ContinuedFraction,
}
```

### 1.3 `Value`（`mod.rs:502-507`）

```rust
pub struct Value {
    pub data: ValueData,
    pub hint: Interpretation,               // ← 第二型系を値に添付
    pub absence: Option<AbsenceMetadata>,   // ← NIL の経緯を値に添付
}
```

### 1.4 派生軸（`semantic/value_axes.rs`）

`SemanticKind`（Number/Collection/Record/Code/Process/Supervisor/Absence/**Unknown**）、
`ValueShape`（Scalar/Vector/**Tensor**/Record/CodeBlock/Handle/Absence/**Unknown**）、
`ValueOrigin`（…/**ModuleWord**/…/**Unknown**）。いずれも正典6領域より広い分類を持つ。

---

## 2. 正典6領域との不一致一覧

正典（`spec/language-semantics.md`, `SPECIFICATION.html`）: `Scalar / Boolean / String /
Vector / NIL / CodeBlock`。

| 正典領域 | Rust 実装の実体 | 不一致の性質 | 降格先 |
| --- | --- | --- | --- |
| Scalar | `Scalar(Fraction)` + `ExactScalar(ExactReal)` | 内部表現の別 variant が意味型に昇格 | `ScalarRepr::{Float/Exact}`（B） |
| Boolean | `Boolean(bool)` | 一致 | — |
| **String** | variant 無し。`Vector`+`Interpretation::Text`、または scalar codepoint（`value_protocol.rs:75, 194-198`） | 第一級値が欠落し、第二型系で代替 | `KernelValue::String`（A へ昇格） |
| Vector | `Vector(Arc<Vec<Value>>)` + `Tensor{..}` | 最適化表現が意味型に昇格。等価判定で相互変換（`mod.rs:409-411`） | `VectorRepr::{General/DenseNumeric}`（B） |
| NIL | `Nil` + `Value.absence: AbsenceMetadata` | reason 以外の経緯（origin/recoverability/diagnosis）を値に添付 | trace/diagnostic（C） |
| CodeBlock | `CodeBlock(Vec<Token>)` | 一致 | — |
| （無し） | `Interpretation`（Interval/Text/Timestamp/…） | dispatch を変える第二型系 | presentation metadata（C） |
| （無し） | `SemanticKind/ValueShape/ValueOrigin` の Unknown/Tensor/Record/Process/Supervisor/ModuleWord | 観測分類が6領域を超過 | host/legacy metadata（C/D） |

**要点**: 「6領域 vs 7 variant」だけでなく、`Interpretation` という第二型系と、
`Value.absence` に添付された NIL 経緯が、値モデルの実質的な座標系になっている。

---

## 3. Tensor / Interpretation / UNKNOWN / NIL metadata / module の依存グラフ

検索結果を削除リストにはしない。**意味論の境界を越える入口**を特定する。

### 3.1 Tensor
- 定義: `types/mod.rs` `DenseTensor`(:31-207) / `SparseTensor`(:209-321) / `TensorLaneId`+`NilReasonRegistry`(:323-329) / `ValueData::Tensor`(:384-387)。
- 越境入口（★ = ここを塞げば意味論から消える）:
  - ★ `ValueData::Tensor` variant 自体（`mod.rs:384`）。
  - ★ 等価判定の Vector↔Tensor 相互変換（`mod.rs:409-411, 420-500`）。
  - ★ `ValueShape::Tensor` / protocol 文字列 `"tensor"`（`value_axes.rs:17`, `protocol.rs:25`）。
  - `value_protocol.rs:212-223`（Tensor→protocol children 展開）。
- 純内部（残してよい）: `interpreter/tensor_ops.rs`, `tensor_cmds.rs`, `simd_ops.rs`, `compiled_plan.rs`, dense/sparse storage。

### 3.2 Interpretation
- 定義: `types/mod.rs:346-369`。
- 越境入口:
  - ★ `Value.hint`（`mod.rs:505`）— 値に添付され `PartialEq` にも参加（`mod.rs:511`）。
  - ★ dispatch を変える箇所: `value_protocol.rs` `scalar_to_protocol`(:71-78) が `TruthValue/Timestamp/Text` で type を分岐、`value_to_protocol`(:152-232) が `Text/TruthValue/ContinuedFraction` で構造分岐。
  - `SemanticRegistry.flow_hints: HashMap<u64, Interpretation>`（`mod.rs:519-521`）。
- 表示専用（C へ）: 実際に必要なのは String 化と timestamp/interval の**表示**のみ。

### 3.3 UNKNOWN（削減済みのはずの残骸）
- `SemanticKind::Unknown`, `ValueShape::Unknown`, `ValueOrigin::Unknown`（`value_axes.rs`）。
- `UnknownBehavior::{NeverCreates, MayCreate}`（`word_contract.rs:82-86`）と `widen_unknown`。
- `value_to_protocol` の `is_unknown()` 早期 return コメント（`value_protocol.rs:169-177`）。
- 越境入口: ★ `UnknownBehavior` を `WordContract` が公開フィールドに持つこと、★ `SemanticKind/ValueShape::Unknown` が protocol 文字列に出ること。

### 3.4 NIL metadata
- `AbsenceMetadata { reason, origin, recoverability, diagnosis }`（`semantic/absence.rs:52-58`）。
- `AbsenceOrigin`（16 variant, `absence.rs:4-42`）、`Recoverability`（`absence.rs:44-50`）。
- `NilReasonRegistry = HashMap<TensorLaneId, NilReason>`（`mod.rs:329`）。
- 越境入口: ★ `Value.absence`（`mod.rs:506`）が値に添付され、★ golden protocol の `semantics.absence.{origin,recoverability,diagnosis}` として観測可能（`spec/freeze/host-protocol-v1.golden.json`）。
- 正典 NIL は reason 中心の最小モデル。origin/recoverability/diagnosis は診断・trace（C）へ。

### 3.5 module
- Interpreter フィールド: `module_state`, `import_table: ImportTable`, `module_vocabulary: HashMap<String, ModuleDictionary>`, `module_epoch`（`interpreter_core.rs:62-84,90`）。型: `ModuleDictionary`(:67-70), `ImportedModule`(:72-76), `ImportTable`(:78-81)。
- `Capability::{ModuleOwned, CoreOwned}`（`protocol.rs:47-48`）、`ValueOrigin::ModuleWord`（`value_axes.rs:31`）。
- golden protocol の `importedModules`（`host-protocol-v1.golden.json`）。
- **重要な観察**: `spec/words.json` と `spec/language-semantics.md` に IMPORT / module の**語彙は存在しない**（§1 の grep で 0 件）。つまり module は正典から既に消えており、実装と V1 protocol にのみ残る **legacy 概念（D）**。`module_state` は実際には汎用スクラッチ（Word contract cache の格納先, `word_contract.rs:27,318-339`）としても使われており、名前が誤解を招く。
- 越境入口: ★ Interpreter が module 系フィールドを**辞書解決**に使う箇所（`resolve_word.rs`, `execute_del.rs`, `session_lifecycle.rs`）。Kernel の辞書は Core/User のみにする。

---

## 4. `words.json` と Rust Word 定義の重複箇所

### 4.1 現状
- `spec/words.json`: 69 entry。各 entry に `name, aliases, family, stack{inputs,outputs},
  consumption, nilPolicy, projection, errorWhen, purity, determinism, capability,
  hostedEffect, interpretationRole, clauses, documentation, executorKey, effects`。
- **`words.json` を読む Rust は無い**（`build.rs` 無し、`rust/` 内に `words.json` 参照 0）。
  consumer は JS スクリプトのみ: `generate-word-reference.mjs`, `generate-word-manifest.mjs`,
  `generate-core-word-docs.mjs`, `generate-skill-md.mjs`, `check-semantic-kernel.mjs`,
  `check-word-schema-migration.mjs`, `check-reading-surfaces.mjs`。
- Rust 側は `builtins/builtin_word_definitions.rs` の `BUILTIN_SPECS`（`BuiltinSpec`,
  約1340行）に**手書き**で並列定義。`BuiltinSpec` フィールド（`:7-52`）は
  `name, category, hover_*, executor_key, mass(=stack), summary, role, stack_effect,
  stability, purity, effects, deterministic, safe_preview, partiality, nil_policy,
  safety_level, execution_form`。

### 4.2 二重管理されている意味情報（1事実・複数記述）

| 意味情報 | `spec/words.json` | Rust |
| --- | --- | --- |
| 正規名 | `name` | `BuiltinSpec.name` |
| alias | `aliases` | `core_word_aliases.rs`（別の手書き表） |
| stack 契約 | `stack{inputs,outputs}` | `BuiltinSpec.mass`（`MassContract`） |
| consumption | `consumption` | （`ConsumptionMode` 実装側） |
| NIL 方針 | `nilPolicy` | `BuiltinSpec.nil_policy`（`NilPolicy`） |
| purity | `purity` | `BuiltinSpec.purity`（`WordPurity`） |
| determinism | `determinism` | `BuiltinSpec.deterministic` |
| executor 対応 | `executorKey` | `BuiltinExecutorKey`（`builtin_word_types.rs:1-70`, 手書き enum）+ `execute_builtin.rs` の巨大 `match` |
| docs | `documentation` | `generated_core_word_docs.rs`（既に生成済）+ `BuiltinSpec.summary/role/...` |

**既に片側生成されているのは docs のみ**（`generated_core_word_docs.rs`）。契約本体
（stack/nil/purity/alias/executorKey）は spec と Rust で独立に手書きされ、乖離を
`check-*` スクリプトで**事後照合**している。これを「JSON→Rust 生成」に反転させる。

---

## 5. HostProtocolV1 と runtime model の結合点

- serializer: `wasm_interpreter_bindings/wasm_value_conversion.rs`（Value→JsValue）,
  `wasm_interpreter_state.rs`（stack/dict/status/absence）。両者は `types/value_protocol.rs`
  の `ProtocolNode`（pure (Value,hint)→wire）を共有し、`semantic/protocol.rs` の
  `as_protocol_str` で enum→文字列化。
- V1 が Kernel 内部型に**直結**している箇所（golden `host-protocol-v1.golden.json`）:
  - `semantics.semanticKind`（`SemanticKind`）, `semantics.shape`（`ValueShape`, `"tensor"` 含む）。
  - `semantics.capabilities`（`Capability`, `moduleOwned/coreOwned` 含む）。
  - `semantics.origin` / `semantics.absence.{reason,origin,recoverability,diagnosis}`（`AbsenceMetadata` 直写し）。
  - `importedModules`（module state 直写し）。
  - `collect_hedged_trace`（`wasm_interpreter_state.rs:451-455`, 既に no-op だが V1 が pin）。
- V1 の互換ポリシー: field 削除・改名・意味変更は breaking。よって**V1 は掃除せず凍結**し、
  V1 固有概念（module/tensor/unknown/absence-origin/recoverability/hedged）は adapter 内に閉じる。

---

## 6. Semantic Spine を導入する最小の seam

既存構造にある「くびれ」を再利用する。新しい経路を無から作らない。

1. **`types/value_protocol.rs::value_to_protocol`** — コメント自身が
   「the single source of truth for the machine-facing value wire format」（`:1-9,149-151`）。
   これが既に Value→観測の narrow waist。**Observation / HostProtocolV2 はここを起点**にする。
2. **`BuiltinExecutorKey` + `execute_builtin.rs` の `match`**（`:161-...`）— 既に
   「executor key → primitive」表であり、§11 が求める primitive executor テーブルそのもの。
   `BuiltinExecutorKey` を `words.json.executorKey` から**生成 enum**にすれば、Word 追加時の
   登録漏れがコンパイルエラーになる。
3. **`coreword_registry` / `BuiltinSpec`** — 契約メタデータの集約点。ここを
   `words.json` からの生成物に置換する。
4. **`interpreter/word_contract.rs::WordContract`** — 契約推論は既に存在するが用途は
   検証のみ。runtime はまだ消費していない。§12 の execute wrapper 化の接続先。

最小 seam の結論: **新規 `rust/src/kernel/` を作り、既存 `types`/`interpreter` を
`From`/`Into` adapter で橋渡しする。`value_to_protocol` の入力を `KernelValue` 由来の
`Observation` に差し替えるのが最初の1本。**

---

## 7. 新規ファイル構成案

```text
rust/src/kernel/                     # Semantic Spine（public 型はここだけ）
  mod.rs            # 再エクスポート。仕様6領域外の型を pub にしない invariant の番人
  value.rs          # KernelValue（6 variant）, ScalarRepr, VectorRepr（private repr）
  scalar.rs         # ScalarRepr::{Float(Fraction)/Exact(ExactReal)}（旧 ExactScalar 内部化）
  vector.rs         # VectorRepr::{General/DenseNumeric(DenseTensor)}（旧 Tensor 内部化）
  string.rs         # KernelValue::String(Arc<str>) の正規化・codepoint 変換
  nil.rs            # Nil(NilReason) 最小モデル
  state.rs          # KernelState（Core/User 二層辞書のみ, module 無し）
  observation.rs    # Observation / ObservedValue / PresentationHint（旧 Interpretation の表示分）
  word_contract.rs  # runtime が消費する WordContract（generated から供給）
  execute.rs        # execute_word wrapper（arity/nil/keep/lift/primitive/projection）
  diagnostic.rs     # ExecutionTrace / DiagnosticContext（旧 absence origin/recoverability）

rust/src/kernel/generated/           # words.json からの生成物（build.rs 出力 or チェックイン）
  word_id.rs        # WordId enum
  executor_key.rs   # ExecutorKey enum（現 BuiltinExecutorKey を生成化）
  word_contracts.rs # 各 Word の stack/nil/consumption/purity 契約
  aliases.rs        # alias 表（現 core_word_aliases.rs を生成化）

rust/src/host/
  protocol_v2.rs    # HostProtocolV2（KernelValue/Observation に一致）
  protocol_v1_adapter.rs # Legacy V1 anti-corruption layer（module/tensor/unknown を adapter 内で構築）

rust/build.rs                        # spec/words.json → kernel/generated/*.rs
```

対応する生成器（既存 `.mjs` 群と役割分担）: Rust 契約生成は `build.rs`（または
`scripts/generate-word-registry.mjs` + チェックイン）で行い、JS 側の docs/manifest 生成は現状維持。

---

## 8. 移行 Phase ごとの変更対象ファイル

原則: **Phase 1〜7 は追加のみ（旧構造と並存）。削除は Phase 9。**

| Phase | 目的 | 追加/変更 | 触らない（並存） |
| --- | --- | --- | --- |
| 1. Spine 骨格 | `kernel/` 新設 | `kernel/{mod,value,nil,state,observation,word_contract,execute}.rs`（型と署名のみ） | 既存 runtime 全て |
| 2. 6領域化 | `KernelValue` 6 variant + repr | `kernel/{value,scalar,vector,string}.rs`, `From<ValueData>`/`Into<ValueData>` adapter | `types/mod.rs`（変更なし） |
| 3. registry 生成 | words.json→Rust | `build.rs`, `kernel/generated/*`, `scripts/generate-word-registry.mjs` | 手書き `BuiltinSpec`（当面 oracle） |
| 4. execute wrapper | 契約を runtime が消費 | `kernel/execute.rs`（ADD/SUB/MUL/DIV から横展開）, `word_contract.rs` を runtime 接続 | 既存 `execute_builtin.rs`（fallback） |
| 5. Observation | 内部/観測の分離 | `kernel/observation.rs`, `value_protocol.rs` の入力を Observation へ | golden 出力は不変 |
| 6. HostProtocolV2 | Spine 直写し | `host/protocol_v2.rs`, `spec/host-protocol-v2.schema.json`, 新 golden | V1 serializer |
| 7. V1 adapter | V1 を Spine から生成 | `host/protocol_v1_adapter.rs`（module/tensor/unknown を adapter 内で構築） | golden `host-protocol-v1.golden.json`（不変） |
| 8. GUI/WASM 切替 | consumer を V2/Observation へ | `src/wasm-interpreter-types.ts`, `src/gui/*`, worker | 旧 V1 consumer は adapter 経由で維持 |
| 9. 旧モデル一括削除 | 到達不能型の除去 | §10 参照 | — |

---

## 9. 各 Phase で維持すべき既存テスト

これらは移行中も緑を保ち、Observation 等価の回帰網とする。

- `rust/src/conformance_tests.rs`（conformance corpus）— 全 Phase で不変。
- `spec/freeze/host-protocol-v1.golden.json` + `src/host-protocol-v1.contract.test.ts` — Phase 7 まで完全一致を維持（V1 凍結の証明）。
- `types/value_protocol_tests.rs`（wire format の MC/DC）— Phase 5 で Observation 経由に付け替え後も同一出力。
- `interpreter/word_contract_tests.rs`, `word_space_tests.rs` — Phase 4 で runtime 接続後も契約不変。
- `interpreter/nil_conformance_tests.rs`, `nil_reason_tests.rs`, `nil_diagnostics_tests.rs` — 最小 NIL の reason 保存を Phase 5 で担保。
- `interpreter/scalar_fastpath_tests.rs`, `simd_ops`, `compiled_plan_tests.rs`, `types/exact/*` — 内部最適化が Observation を壊さないことの担保（Phase 2/5）。
- `scripts/check-semantic-kernel.mjs`（69 語/16 alias/12 family 上限）, `check-semantic-firewall.sh`, `check-conformance-coverage.mjs` — CI ゲートとして全 Phase 維持。
- `types/value_persist_tests.rs`（snapshot 往復）— Phase 2 の repr 変更で往復不変を担保。

---

## 10. Semantic Spine 完成後に削除可能になる型・ファイル一覧（Phase 9 候補）

**到達不能になった時点で削除**する。個別書換えではなく「新構造から届かなくなった型/ファイルをまとめて消す」。

意味論から降格・除去できる型:
- `ValueData::Tensor`（variant）, `TensorLaneId`, `NilReasonRegistry`, Vector↔Tensor 等価分岐（`mod.rs:409-500`）。
- `ValueData::ExactScalar`（variant; `ExactReal` 実装は `ScalarRepr::Exact` として内部保持）。
- `Interpretation` enum と `Value.hint`（表示分は `PresentationHint` へ移送）。
- `Value.absence` フィールド, `AbsenceMetadata`, `AbsenceOrigin`, `Recoverability`（reason 以外; trace へ）。
- `SemanticKind::Unknown`, `ValueShape::{Tensor,Unknown}`, `ValueOrigin::{ModuleWord,Unknown}`。
- `UnknownBehavior`, `WaterSensitivity` を `WordContract` の public から除去（内部/legacy へ）。
- `Capability::{ModuleOwned, CoreOwned}`（V1 adapter 内へ）。
- Interpreter の `module_vocabulary`, `import_table`, `ModuleDictionary`, `ImportedModule`, `ImportTable`, `module_epoch`（辞書は Core/User のみ）。`module_state` は用途明確化のためリネーム（例 `runtime_scratch`）。

重複記述として削除できるもの:
- 手書き `BuiltinSpec` の契約フィールド（stack/nil/purity/determinism/executor_key）→ `kernel/generated/*` に置換。
- 手書き `BuiltinExecutorKey`（`builtin_word_types.rs`）→ 生成 `ExecutorKey`。
- 手書き alias 表（`core_word_aliases.rs`）→ 生成 `aliases.rs`。

保持（非目標・§28）: `DenseTensor/SparseTensor` storage, `simd_ops`, `compiled_plan`,
`types/exact/*`（exact arithmetic）, tensor 最適化本体。これらは `VectorRepr`/`ScalarRepr`
の private 実装として残す。

### 10.1 Phase 9 実施記録（第1次削除）

到達可能性を実測し、**到達不能なものだけ**を削除した。削除済み:

| 削除対象 | 到達可能性の実測 |
| --- | --- |
| `TensorLaneId`, `NilReasonRegistry` | 定義のみ。参照 0 |
| `SemanticRegistry`（`flow_hints`/`flow_extensions`）, `ValueExt` | `Interpreter` が構築するのみで read/write 0（フィールドの doc 自身が "no readers yet" と明記） |
| `UnknownBehavior`, `widen_unknown` | `WordContract` に計算・格納されるが観測者 0。UNKNOWN は削減済み概念 |
| `WaterSensitivity`, `widen_water` | 同上 |
| `SemanticKind::{Record,Process,Supervisor,Unknown}` | 構築 0（`as_protocol_str` の arm のみ） |
| `ValueShape::{Record,Handle,Unknown}` | 構築 0 |
| `ValueOrigin::{Computed,CoreWord,BuiltinWord,ModuleWord,UserWord,Optimizer}` | 構築 0。`Value::origin` は Literal/NilPropagation/HostEnvironment/Unknown のみ返す |
| `Capability::{ModuleOwned,CoreOwned}` | 構築 0。V1 golden にも現れない |

削除できなかったもの（**まだ live**）とその理由:

- `ValueData::Tensor` / `ExactScalar` / `Interpretation` / `Value.hint` / `Value.absence`,
  `AbsenceMetadata` / `AbsenceOrigin` / `Recoverability`, module 系
  （`module_vocabulary` / `ImportTable` / …）, `ValueShape::Tensor`。
- 理由: Spine は runtime と**並存**しているだけで、まだ**置換されていない**。実行器・
  `value_to_protocol`・V1 serializer は今も旧 `ValueData` 上で動作しており、これらの型は
  live path から到達可能である。

したがって次の削除には、Phase 4〜8 で作成・検証済みの Spine 経路を**実際に consumer へ
配線する**作業（execute wrapper を dispatch へ、`Observation` を serializer へ、V2 producer を
wasm へ）が先行する。§23 の原則どおり、**到達不能になってから削除する**。

### 10.2 Phase 9 実施記録（module runtime の削除）

正典から削除済みで、登録経路も既に存在しなかった module runtime を削除した:

- `ModuleDictionary` / `ImportedModule` / `ImportTable` と `Interpreter` の
  `module_vocabulary` / `import_table` を削除。
- bare name、修飾名、resolve cache、dependency graph、`DEL` に残っていた module 分岐を削除。
- module の変更元が無いにもかかわらず cache と compiled plan の invalidation に残っていた
  `module_epoch` を削除。
- Word contract cache が借用していた `module_state` は、実際の責務を表す
  `runtime_scratch` に改名。
- 凍結済み V1 / wasm 互換 API は anti-corruption boundary として署名を保持し、module catalog、
  import state、imported module の各結果を常に空として返す。restore は no-op のままとする。

これにより module は Kernel の解決・実行状態から到達不能になり、空の legacy wire shape のみが
host boundary に残る。

追加監査では、module runtime 削除後も error model に残っていた到達不能な
`AjisaiError::UnknownModule` / `ErrorCategory::UnknownModule` / `ErrorLocusKind::ModuleWord`
と `ErrorLocus.module` を削除した。`DICTIONARY@WORD` は module ではなく User dictionary の
Word として分類し、unknown-word 診断も import ではなく User dictionary の定義確認を案内する。
V1 が固定する module 関連 wasm method の**署名**は保持するが、restore method の空ループは
引数を無視する明示的 no-op に縮約した。

### 10.3 Phase 9 実施記録（module メタデータと listing 模型の削除）

10.2 で module runtime を削除した後も、**静的メタデータ側**に module 座標系が残っていた。
到達可能性を実測し、構築者が 0 になったものを削除した。

| 削除対象 | 到達可能性の実測 |
| --- | --- |
| `CanonicalHome`（`Core` / `Module(String)`） | 構築点は `core_word_metadata_from_spec` の 1 箇所のみで、常に `Core`。`Module(_)` は構築 0 |
| `CorewordMetadata.listed_in_core` | 常に `true` の定数フィールド |
| `CorewordMetadata.listed_in_modules` | 供給元は `CORE_BOUNDARY_LISTINGS` の `IO` / `MATH` のみ。両 module とも既に存在しない |
| `CORE_BOUNDARY_LISTINGS` / `MODULE_CORE_LISTINGS` / `apply_*_listings` | `MODULE_CORE_LISTINGS` は空の `module_entries` に対してのみ適用され、効果 0 |
| `build_builtin_word_registry` の `module_entries` | `Vec::new()` のまま `extend` される空ベクタと、その上の空ループ |
| listing 問い合わせ関数群（`get_module_listed_words` / `get_category_listed_words` / `get_core_listed_words` / `get_boundary_words` / `get_canonical_core_words` / `get_canonical_module_words` / `is_listing_only_for_module`） | 参照は自テストのみ。`get_module_listed_words` / `get_category_listed_words` / `get_boundary_words` は参照 0 |
| `get_coreword_metadata` の `MODULE@WORD` 分岐 | `canonical_module()` が常に `None` のため常に `None` を返す。builtin 名に `@` は現れないので、平坦な名前一致に縮約しても挙動は不変 |
| `WordShape`（`builtin_word_types.rs`） | 唯一の消費者と説明されていた `ModuleWord` が存在せず、`#[allow(dead_code)]` で黙らされていた |
| `execute_def.rs` の `collision_modules` | `Vec::new()` 固定のため、module 衝突警告のブロック全体が到達不能 |
| `builtin_word_details.rs` の `dictionary-{,un}import{,-only}` effect arm | どの `BuiltinSpec` もこれらの effect を宣言しない |
| `module_word_call()`（`tests/test_support/generators.rs`） | 参照 0。`MATH` / `JSON` / `ALGO` という存在しない module を前提にしていた |
| `core_word_is_not_shadowed_by_import`（`naming_resolution_laws.rs`） | import 削除後、同一文字列同士を比較する恒真テストになっていた |

`listed_in_categories`（`CAST` / `TEXT` / `TENSOR` / `RUNTIME`）も併せて削除した。参照は
自テストのみで、語の分類は正典側 `spec/words.json` の `family`（11 分類）が担っている。
§4.2 の「1事実・複数記述」を残さない方針に従い、Rust 側の並行分類は保持しない。

あわせて、10.2 の `ErrorLocus.module` 削除時に追随漏れとなり4つの law test binary を
コンパイル不能にしていた `tests/test_support/observe.rs` の `DiagnosisObservation.module`
を削除した（write のみで read 0）。

凍結済み V1 wasm method（`collect_available_modules` / `collect_module_catalog_words_info` /
`collect_import_state` / `restore_import_state`）は §10.2 の方針どおり署名を保持する。
実挙動に合わせて doc comment のみ「常に空」を記述するよう更新した。

これで module は runtime に続き**静的メタデータからも到達不能**になり、
built-in Word は単一の平坦な名前空間になった。

### 10.4 Phase 8 実施記録（生成レジストリへの配線と契約フィールドの削除）

§4.2 の「1事実・複数記述」表のうち、**契約本体**を Rust 側から削除し、
`kernel/generated/word_registry.rs`（`spec/words.json` からの生成物）を
runtime の供給元にした。これで §4.2 が「JSON→Rust 生成に反転させる」と書いた状態になる。

| 意味情報 | 削除した手書き | 新しい供給元 |
| --- | --- | --- |
| executor 対応 | `BuiltinExecutorKey`（enum ファイルごと削除）+ `BuiltinSpec.executor_key` | 生成 `WordId`。executorKey は元から一意な PascalCase 識別子なので、**WordId がそのまま executor key** である（第二の enum を作らない） |
| stack 契約 | `BuiltinSpec.mass` | `stack{inputs,outputs}` → `Arity`。`MassContract` は analyzer 語彙として残し、生成 arity から射影する |
| NIL 方針 | `BuiltinSpec.nil_policy` + 手書き `NilPolicy`(5値) | 生成 `NilPolicy`(7値) |
| purity | `BuiltinSpec.purity` + 手書き `WordPurity`(3値) | 生成 `Purity`(4値) |
| determinism | `BuiltinSpec.deterministic`（bool） | 生成 `Determinism`(3値) |

**狭い語彙が実際に何を誤らせていたか**（すべてテストで顕在化した）:

- `WordPurity` に `conditional` が無く、高階語 7 語（MAP/FILTER/FOLD/ANY/ALL/EXEC/VENT）が
  すべて `pure` として記録されていた。
- `deterministic: bool` は `stateRelative` と `hostRelative` を区別できず、
  「pure なら deterministic」という誤った不変条件を成立させていた。正典では
  `EAT`/`KEEP`/`VENT` は pure かつ stateRelative であり、両軸は独立である。
  `LOOKUP` の「文書化された例外」も、この bool のための例外だったので消えた。
- `NilPolicy` に `passthroughThenProject` が無く、DIV と丸め族 4 語が `createsNil` として
  記録され、**NIL を通す**という半分が言えていなかった。
- `NIL?`/`NIL-REASON` の arity は「retain するので Fixed では表せない」として `Dynamic` に
  していたが、正典の `1 -> 2`（retain 対象＋答え）が正確であり、静的解析を無効化する必要は
  無かった。同時に `[ x ] -> [ ]` の prose を (1,1) と読んでいた prose パーサのバグ
  （空グループの空白綴り `[ ]` 未対応）が露出した。PRINT が `Dynamic` だったため
  検査自体が skip されていた。

**残った重複**: `BuiltinSpec.effects`。`spec/words.json` は同じ effect を別綴りで持つ
（`consoleWrite` / `console-write`）ため、統一は観測される wire 文字列の変更を伴う。
別変更として残す。`category` / `partiality` / `safety_level` / `safe_preview` は
正典が宣言しない runtime 固有の分類なので、重複ではない。

**副次的に到達不能になったもの**: `BuiltinExecutorKey::Force`。対応する Word（`FORC` / `!`）は
既に語彙から削除済みで、`executor_key: Some(Force)` を持つ `BuiltinSpec` は 1 件も無かった。
生成 enum に Force が無いため arm ごと削除した。`force_flag` 自体は
`execute_def` / `execute_del` から読まれているが、**書き手が居ないため常に false** である。
フラグと関連分岐の削除は別変更（到達可能性は測定済み）。

---

## 11. 最重要 invariant（CI 最優先ルール）

> **Ajisai の言語仕様に存在しない概念は、Semantic Spine（`kernel/` の public API）に
> 存在してはならない。** Spine より下の private 実装には最適化のための複雑さを許容する。

実装手段の段階:
1. 型閉包（最終防衛線）: `kernel::KernelValue` が正典6 variant のみ。`Tensor`/`ExactScalar`/
   `Interpretation`/`Unknown`/module を public 型として**表現不能**にする。
2. テキスト checker（補助・既存維持）: `check-semantic-firewall.sh` に、`kernel/mod.rs` の
   public re-export に禁止語（Tensor/ExactScalar/Interpretation/Unknown/module/…）が現れないことを追加。
3. Observation 比較: conformance は内部表現でなく `Observation` を比較（§18）。

---

## 12. 設計判断の優先順位（迷ったとき）

1. 正典仕様　2. Semantic Spine の単純さ　3. 機械検証可能な single source of truth　
4. external behavior 互換（V1 golden）　5. 内部実装互換　6. 局所的変更量。

短期的に差分が増えても、意味論の重複を残さない方を優先する
（**One semantic fact, one authoritative representation.**）。

---

## 13. 最初の1コミットの提案（Phase 1 の入口）

1. `rust/src/kernel/` を新設し、`KernelValue`（6 variant, repr は未実装 stub）と
   `Nil(NilReason)`、`Observation` の**型と署名のみ**を置く。既存 runtime からは未参照。
2. `check-semantic-firewall.sh` に kernel public 閉包チェックを1件追加（禁止語 grep）。
3. 本書を `docs/dev/INDEX.md` に登録。

この時点でビルド・全既存テストは不変。以降 Phase 2 で `From<ValueData>`/`Into<ValueData>`
adapter を足し、旧構造と並存させながら consumer を1本ずつ Spine へ寄せる。
