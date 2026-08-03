# Ajisai β版改修指示書 — 57 canonical / 35 Semantic Kernel

Status: **Non-canonical / 方針記録・改修指示書**。正典は `spec/` 配下と、そこから生成される `SPECIFICATION.html` である。実装作業は GitHub Issue #1429 で追跡する。

Implementation status: **Phase 2** — canonical inventoryとruntime surfaceから削除予定13語を除去済み。Standard契約完成、互換層削除、version固定は後続Phaseで行う。

## 1. β版の境界

- REFLECT を含む `ebb66a5f9d14a6c8d6610488724e476e652abc35` をα版の基準点とする。
- 全完了条件を初めて満たす commit からβ版を開始し、最初の版番号を `0.2.0-beta.1` とする。
- 公開Coreは単一の平坦なdictionaryにある57 canonical Wordsであり、35語のSemantic Kernelと22語のStandard Wordsに設計分類する。namespace、module、import、prelude、別dictionaryは導入しない。
- 35語は数学的・世界的な最小集合ではなくAjisaiの意味論的な幹である。57語がβ版の実用的なcanonical surfaceである。

## 2. 固定する語集合

### Semantic Kernel（35）

- Truth / comparison: `TRUE FALSE AND NOT EQ LT GT`
- Exact arithmetic: `ADD MUL DIV FLOOR NEG SQRT`
- Vector / iteration: `GET LENGTH CONCAT COLLECT RANGE FOLD`
- Text boundaries: `CHARS JOIN NUM STR`
- Control / absence / modifier / dictionary / effect / reflection: `COND EXEC NIL NIL? NIL-REASON VENT KEEP DEF DEL LOOKUP PRINT REFLECT`

### Standard Words（22）

- Truth / comparison: `OR NEQ LTE GTE`
- Exact arithmetic: `SUB MOD ROUND ABS MIN MAX`
- Vector / higher-order: `TAKE REVERSE FILL SORT INDEX-OF MAP FILTER ANY ALL`
- Text algorithms: `TRIM TOKENIZE SUBSTITUTE`

Standard Wordsは互換層や任意bundleではない。Kernelと同水準のmachine-readable contract、Specification、Reference、hover、LOOKUP、形式化、law test、conformance、runtime/compiled一致、REFLECT表現を持つ。

### 削除するWords（13）

`CEIL SIGN INSERT REPLACE REMOVE SPLIT REORDER UNIQUE CONTAINS STARTS-WITH? ENDS-WITH? CHR EAT`

互換prelude、deprecated canonical name、自動変換器、復元bundleは提供しない。registryから隠すだけではなく、dispatch、executor、compiled path、alias、documentation、tests、migrationを削除する。

## 3. 分類metadata

残存する全entryは `spec/words.json` に `vocabularyTier: "kernel" | "standard"` を持つ。Standard entryは次のいずれかの `standardKind` を持つ。

- `shorthand`: 直接的な短縮表現
- `namedPattern`: 頻出定型処理の安定した名前と契約
- `algorithm`: 境界条件や複雑度を含む標準アルゴリズム
- `operational`: 短絡、隔離、復元、資源制限などnative実装の観測契約

`docs/formalization-coverage.json` の `core_tier`（`identity` / `flow` / `material`）は別軸であり、名称・意味を変更しない。

Kernelとの関係は、次の16語を `derivable`、次の6語を `operational` とする。

- derivable: `OR NEQ LTE GTE SUB MOD ROUND ABS MIN MAX TAKE REVERSE INDEX-OF TRIM TOKENIZE SUBSTITUTE`
- operational: `MAP FILTER ANY ALL FILL SORT`

operational Wordは、Kernelの表現能力とは別に、効果回数、短絡点、ERROR復元、計算量または資源制限に関するnative保持理由を明記する。

## 4. `check-minimal-core` gate

集約gateは次を検査する。

1. canonical inventoryが57語であり、Kernel 35語、Standard 22語の名前集合が本書と一致する。
2. 削除13語がinventoryに存在しない。
3. 57語すべてにformalization entryと存在するlaw-test fileがある。
4. Kernel witnessがPrimitive、Derived、HostedEffectのいずれかとして有効である。
5. Standardが`derivable`または`operational`を宣言し、必要なwitnessと保持理由を持つ。
6. 孤立Core Wordがなく、`core_tier`と`vocabularyTier`を混同していない。

Phase 1中のみ、削除予定13語を明示した70語inventoryを移行状態として許す。成功時は少なくとも以下を出力する。

```text
[minimal-core] 35/35 Semantic Kernel Words have executable witnesses.
[minimal-core] 22/22 Standard Words have complete contracts and law witnesses.
```

## 5. Manifestと公開面

Word Manifestは`canonicalWords: 57`、`semanticKernelWords: 35`、`standardWords: 22`を機械可読に公開し、各Wordにも`vocabularyTier`を出力する。aliasやsurface formをcanonical countに加算しない。

README、Specification、Reference、Manifest、Rust docs、hover、LOOKUP、SKILL.md、conformance metadataは **57 canonical Words / 35-Word Semantic Kernel** に統一する。Referenceでは57語すべてを通常のCore Wordとして扱いながら、Kernel/Standardを識別可能にする。

## 6. 互換層の終了

- HostProtocolV2を唯一のcurrent protocolとし、V1 schema、golden、contract test、`kernel::legacy_v1`、GUI adapterを削除する。
- module/import廃止後のno-op API、空field、UI、execution-mode/hedged-trace互換とworker hedging branchを削除する。
- stack snapshot、user-word export、persistenceは現行のlossless/versioned形式だけを受理し、α形式reader、downgrade、fallback、migration、dictionary recoveryを残さない。
- 削除Word専用aliasとlegacy canonicalization pathを削除する。記号糖の新設・再編は別scopeとする。

## 7. 実装順序

1. **契約と分類**: metadata schema、57語の分類、35/22構造gateを追加する。13語が残る間は移行中であることを明示する。
2. **13語削除**: 正典、生成registry、runtime、compiled path、tests、docs、fixturesを収束させ、inventory gateを57語へ切り替える。
3. **Standard契約**: 16 derivable witnessと6 operational contractを完成させる。
4. **互換層削除**: V1 protocol、旧snapshot/import/export、module/import残骸、hedged互換を削除する。
5. **公開面とversion**: 全生成物を再生成し、公開表記と全versionを`0.2.0-beta.1`へ同期し、β境界commitを記録する。

巨大な一括PRを避け、各Phaseをレビュー可能なPRに分けてよい。ただし中間状態をreleaseしない。

## 8. 必須law test

- derivable 16語にはKernelのみを使う実行可能なAjisai programまたは同値性testを置く。
- `MAP` / `FILTER`はindex順、isolated stack、effect順、truth/NIL policy、ERROR復元を検査する。
- `ANY` / `ALL`は短絡点と未訪問effectの非実行を検査する。
- `FILL`はshape overflowとmaterialization ceilingをallocation前に検査し、`spaceExhausted` NILを返す。
- `SORT`はdeterministic order、比較ERROR時の原子性、元operand復元を検査する。
- source、REFLECT decode、compiled planを含む全入口で削除13語がUnknown Wordまたは不正tokenになることを検査する。

## 9. 完了条件

- inventory、分類、名前集合が57/35/22で一致する。
- 削除13語の正典、生成物、runtime、docs、tests、migrationが残らない。
- Standard 22語が完全な契約、形式化、law test、conformanceを持つ。
- Manifest/Reference/runtime/compiled/hover/LOOKUP/REFLECTが同じ57語を認識する。
- V1 protocol、旧persistence/import/export、module/import残骸、hedged互換がproduction codeにない。
- Rust fmt、clippy、library/integration tests、WASM、TypeScript、lint、Vitest、semantic firewall、生成同期、形式化、minimal-core、reading surface、version同期の全gateが通る。
- package、crate、Tauri、配布metadataが`0.2.0-beta.1`で一致し、α基準点とβ境界commitがREADME/Specificationに記録される。

## 10. Non-goals

35語だけの公開、Kernel/Standardのnamespace分離、Standardの任意bundle化、α形式の自動変換、旧protocol readerの維持、記号糖再設計、35語の数学的最小性の主張は行わない。
