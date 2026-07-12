# Ajisai AIファースト不可約化・事象作用強化プロンプトレビューと改訂案

## 評価サマリ

提示されたプロンプトの方向性は妥当である。Ajisai を「表面語彙は豊かだが、AI が読む意味論は少数の代数的原理へ圧縮される言語」として扱い、`word-manifest`、formalization coverage、law test、structured diagnosis を中心に据える方針は、既存の数式化・簡約レビュー資料と整合する。

ただし、元プロンプトをそのまま Codex に渡すと、単一 PR で仕様・メタデータ・実装共通化・診断・CI まで一気に改修しようとして、差分が大きくなりすぎる。さらに、すでに導入済みの仕組みを「新規導入」と誤認させる表現や、既存スキーマとの差分を曖昧にする表現が含まれるため、段階的な改修指示へ整理する必要がある。

特に問題となる点は次のとおり。

1. プロンプト本文が重複しており、同じ指示が二回出ている。Codex が優先順位やスコープを誤解する原因になる。
2. `docs/formalization-coverage.json` にはすでに `algebra_primitives` と `semantic_role` が存在し、`scripts/check-formalization-coverage.mjs` も一部の整合性を検証している。元プロンプトの「導入してください」は、既存仕組みの拡張として書くべきである。
3. `docs/word-manifest.json` は生成物であり、`scripts/generate-word-manifest.mjs` の出力仕様を確認せず直接編集する指示は再生成時の巻き戻りを招く。
4. 「全 surface word を分類」「全 sugar の observation equivalence」「HostedEffect schema 統一」「structured diagnosis 分離」を同一 PR の完了条件にしており、レビュー不能な巨大 PR になりやすい。
5. `Primitive`、`Derived`、`Sugar`、`HostedEffect`、`Extension` の分類基準は良いが、`moduleword`、compatibility alias、surface form、標準ライブラリ的な pure module word をどの粒度で分類するかが未定義である。
6. `HostedEffect` の capability は runtime capability、UI safe preview、portable Core contract の三つと混同されやすい。元プロンプトは `effect_boundary`、`safety_level`、`capability` の責務分離をもう少し明示すべきである。
7. `Exploratory` を意味論的負債として扱う方針は妥当だが、探索的機能をただちに失敗扱いにすると現行開発を止める。初期段階では Core portability からの隔離とレビューゲートを fail、滞留件数を warning にするのが安全である。
8. Phase 4 の実装共通化は、仕様メタデータ PR とはリスクが異なる。メタデータ検証 PR と interpreter refactor PR を分けるべきである。
9. 「PR作成のたびに改修の進捗度を報告」は良い運用要求だが、報告形式がない。PR 本文にチェックリスト、今回の完了率、残作業を必ず入れる形式へ落とし込むべきである。

## 改訂方針

元プロンプトは「最終ビジョン」としては維持しつつ、Codex が安全に実行できるよう次の三段階へ分割する。

- **Track A: Semantic Metadata Foundation** — manifest / coverage / primitive registry / validation / simplifier report を整備する。
- **Track B: Observation and Diagnosis Laws** — sugar observation equivalence、HostedEffect pure-preview guard、structured diagnosis をテスト可能にする。
- **Track C: Algebraic Implementation Compression** — arithmetic、K3 logic、comparison、structure lift、higher-order を小さな実装 PR に分けて共通 schema 化する。

最初に着手する PR は Track A に限定する。Track A が安定してから Track B、Track C へ進む。

## 改訂プロンプト

以下を Codex 向けの改修プロンプトとして使用する。

~~~markdown
# Codex向け改修プロンプト: Ajisai AIファースト不可約化・事象作用強化

あなたは Ajisai プロジェクトの改修を担当します。

## 0. 目的

Ajisai を単に小さな言語へ削るのではなく、**人間向け表面は実用的に豊かで、AI が読む意味論は不可約な少数の代数的原理へ圧縮される言語**へ近づけてください。

今回の開発ループを **事象作用** と呼びます。

1. Ajisai の語彙・実装・観測仕様を数式的な semantic graph へ書き下す。
2. semantic graph を簡約し、Derived / Sugar / HostedEffect / Exploratory を機械的に判定できるようにする。
3. 簡約できる箇所は、テストで観測同値を守りながら実装へ反映する。
4. その結果を manifest / coverage / diagnosis / law test / docs に戻す。

削るべきものは表面語彙ではありません。削るべきものは **不可約でない意味論** です。

## 1. 全体原則

- 実用語彙、標準モジュール、記号糖衣は安易に削除しない。
- Primitive は少数かつ admission reason 付きで管理する。
- Derived は仕様上だけでなく、可能な範囲で実装上も共通 schema に寄せる。
- Sugar は canonical word へ展開でき、展開前後で structured observation が一致する。
- HostedEffect は Core semantics から隔離し、capability / request / effect payload の差分として表現する。
- Exploratory は「未来の核」ではなく「まだ Core へ還元できていない意味論的負債」として扱う。
- AI が読める manifest / coverage / diagnosis / law test / simplifier report を仕様の中心に置く。

## 2. 既存状態の扱い

作業前に次を確認してください。

- `docs/word-manifest.json` は生成物である可能性が高い。直接編集する前に `scripts/generate-word-manifest.mjs` と生成元を確認する。
- `docs/formalization-coverage.json` には既に `algebra_primitives`、`semantic_role`、`derived_from` が存在する可能性がある。新規導入ではなく、既存スキーマの拡張・欠落補完として扱う。
- `scripts/check-formalization-coverage.mjs` は既に一部の coverage 検証を担っている。新規 validator を増やす前に、既存 checker へ追加できるかを確認する。
- 既存の law tests と docs/dev の設計レビュー資料を先に読み、同じ概念を別名で重複実装しない。

重点確認ファイル:

- `docs/word-manifest.json`
- `docs/formalization-coverage.json`
- `scripts/generate-word-manifest.mjs`
- `scripts/check-formalization-coverage.mjs`
- `docs/dev/ajisai-mathematical-formalization.md`
- `docs/dev/archive/ajisai-algebraic-simplification-review.md`
- `docs/dev/archive/ajisai-algebraic-simplification-rollout-plan.md`
- `docs/dev/semantic-metadata-refactor-checklist.md`
- `SPECIFICATION.md`
- `PORTABILITY.md`
- `rust/src/builtins/builtin_word_definitions.rs`
- `rust/src/builtins/builtin_word_details.rs`
- `rust/src/coreword_registry.rs`
- `rust/src/core_word_aliases.rs`
- `rust/src/surface_forms.rs`

## 3. PR分割方針

このプロンプト全体を一つの PR で完了しようとしないでください。原則として、次の順で小さな PR に分割します。

### PR 1: Semantic Metadata Foundation

目的:

- AI-readable semantic graph の土台を整える。
- Primitive registry と role validation を強化する。
- 未分類・未接続・過剰 primitive をレポートできるようにする。

主な作業:

1. `docs/formalization-coverage.json` の `algebra_primitives` を確認し、各 primitive に最低限次を持たせる。
   - `id`
   - `algebraic_family`
   - `kind`
   - `description`
   - `admission_reason`
   - `introduced_by`
   - `can_derive`
   - `status`
2. semantic role の許可値を明確にする。
   - `Primitive`
   - `Derived`
   - `Sugar`
   - `HostedEffect`
   - `Exploratory`
   - `Extension`
   - `Deprecated`
3. `Derived` は `derived_from` を持つ。`derived_from` は登録済み primitive / schema を参照する。
4. `Sugar` は `desugars_to` または同等の canonical expansion を持つ。
5. `HostedEffect` は `capability` と `effect_schema` を持つ。
6. `Exploratory` は `reason`、`exit_options`、`review_gate` を持つ。
7. `scripts/check-formalization-coverage.mjs` または同等の checker で上記を検証する。
8. `scripts/ajisai-simplify-report.*` または `rust/src/bin/ajisai_simplify_report.rs` を追加し、Markdown の AI Simplifier Report を生成する。
9. `SPECIFICATION.md` / `PORTABILITY.md` / docs/dev に、semantic graph と primitive admission test を説明する。

PR 1 の完了条件:

- Primitive registry に admission reason がある。
- 未登録 primitive 参照が fail する。
- `Derived` / `Sugar` / `HostedEffect` / `Exploratory` の必須メタデータ欠落を検出できる。
- 未分類 word は、初期段階では明示的 allowlist または warning として報告される。
- AI Simplifier Report をコマンドで生成できる。
- 既存テストまたはメタデータ検証が通る。

### PR 2: Observation and Diagnosis Laws

目的:

- Sugar、NIL、UNKNOWN、HostedEffect violation、portability violation を structured observation として比較できるようにする。

主な作業:

1. sugar spelling と canonical spelling の observation equivalence test を追加する。
2. 比較対象は、可能な範囲で次の structured fields とする。
   - stack result
   - NIL reason
   - UNKNOWN diagnosis
   - error category
   - effect trace
   - safety classification
   - AI diagnostic payload
3. 人間向け message と AI 向け structured diagnosis の責務を分ける。
4. HostedEffect が pure evaluation / safe preview で実行されないことを law test で確認する。

PR 2 の完了条件:

- Sugar は canonical expansion を持つ。
- Sugar 展開前後で structured observation が一致する。
- テストは可能な限り文字列断片ではなく structured field を検証する。
- HostedEffect は safe preview / pure evaluation で実行されない。

### PR 3以降: Algebraic Implementation Compression

目的:

- Derived と分類された word を、実装上も共通 schema へ寄せる。

小さな領域単位で PR を分けること。

候補:

1. arithmetic
   - `ADD = exact_arithmetic_schema(coeff_add)`
   - `SUB = exact_arithmetic_schema(coeff_sub)`
   - `MUL = exact_arithmetic_schema(coeff_mul)`
   - `DIV = exact_arithmetic_schema(coeff_div)`
   - `MOD = exact_arithmetic_schema(coeff_mod_or_projection)`
2. K3 logic
   - `AND = meet_K3`
   - `OR = join_K3`
   - `NOT = involution_K3`
3. comparison
   - normalize operands
   - budgeted order attempt
   - exact result / UNKNOWN / structured diagnosis
4. structure lift
   - scalar operation
   - structure_lift
   - shape_policy
   - observation
5. higher-order
   - traversal scheme
   - block application
   - accumulator / predicate / projection policy

実装例外が必要な場合は、manifest、coverage、またはコードコメントに `implementation_exception_reason` を明示する。

## 4. Primitive Admission Test

新しい Primitive を追加する前に、必ず次を確認してください。

1. 既存 primitive から Derived として表現できないか？
2. HostedEffect として Core から隔離できないか？
3. Sugar として canonical word へ展開できないか？
4. 既存 algebraic_family の係数違い・射影違い・lift 違いではないか？
5. それでも不可避な場合のみ、新 Primitive として登録する。

新規 Primitive には、最低限 `admission_reason`、`introduced_by`、`can_derive`、`status` を付ける。

## 5. Exploratory Debt Policy

Exploratory は Core portability contract に混入させない。

各 Exploratory word / concept には次を付ける。

```json
{
  "semantic_role": "Exploratory",
  "reason": "Useful behavior not yet reduced to Core algebra.",
  "exit_options": ["Derived", "HostedEffect", "Extension", "Remove"],
  "review_gate": "before-core-stabilization"
}
```

優先確認対象:

- `SPAWN`
- `AWAIT`
- `STATUS`
- `KILL`
- `MONITOR`
- `SUPERVISE`
- `PRECOMPUTE`
- Elastic execution
- Hedged execution
- FastGuarded execution
- `compiled_plan`
- `quantized_block`
- `shadow_validation`

初期段階では Exploratory の存在自体は warning でよい。ただし、Core classification への混入、reason 欠落、exit_options 欠落は fail にする。

## 6. HostedEffect Policy

IO、TIME、SERIAL、CRYPTO、MUSIC などの実用機能は削除しない。
ただし、Core semantics には混ぜない。

HostedEffect は次の schema のインスタンスとして扱う。

```text
HostedEffect =
  capability.check
  -> request construction
  -> Eff append
  -> structured observation
```

対象例:

- `TIME@NOW`
- `IO@INPUT`
- `IO@OUTPUT`
- `SERIAL@LIST-PORTS`
- `SERIAL@OPEN`
- `SERIAL@READ`
- `SERIAL@WRITE`
- `SERIAL@CLOSE`
- `CRYPTO@CSPRNG`
- `CRYPTO@HASH`
- `MUSIC@PLAY`

HostedEffect の差異は、protocol / payload / capability の差異として表現し、Core primitive として増殖させない。

## 7. AI Simplifier Report

`docs/word-manifest.json` と `docs/formalization-coverage.json` を読み、Markdown レポートを生成するコマンドを追加する。

最低限レポートするもの:

- 未分類 word
- `Derived` なのに `derived_from` がない word
- 未登録 primitive 参照
- 未使用 primitive
- `Sugar` なのに expansion がない word
- `HostedEffect` なのに capability / effect schema がない word
- `HostedEffect` が Core に混入している疑い
- `Exploratory` の reason / exit_options 欠落
- Derived なのに独立実装されている疑いがある word
- primitive の重複・過剰分割候補

このレポートは仕様変更ではなく、設計レビュー資料として扱う。CI に組み込む場合、初期段階では warning でよい。

## 8. 禁止事項

- 実用語彙を安易に削除すること。
- `MUSIC` / `JSON` / `TIME` / `SERIAL` などを単純に消すこと。
- Primitive を理由なしに増やすこと。
- Derived と分類した word を、理由なく独立意味論として実装し続けること。
- Sugar に独自意味論を持たせること。
- HostedEffect を Core semantics に混ぜること。
- Exploratory を理由なしに標準 Core へ昇格させること。
- 人間向けエラーメッセージだけを仕様扱いすること。
- AI が読めない自然言語ドキュメントだけで済ませること。
- 生成物を、生成元を確認せず手編集して恒久仕様扱いすること。

## 9. PRごとの進捗報告ルール

PR を作成するたびに、PR 本文に必ず次を含める。

```markdown
## Progress

- Overall event-action hardening progress: X% / 100%
- This PR track: Track A | Track B | Track C
- Completed in this PR:
  - [x] ...
- Remaining:
  - [ ] ...
- New semantic debt found:
  - ...
- Risk / review focus:
  - ...
```

進捗率は厳密な工数見積もりではなく、次の目安で保守的に更新する。

- 0-20%: metadata schema と validator の土台
- 20-40%: primitive registry / role classification / simplifier report
- 40-60%: sugar / diagnosis / HostedEffect observation law
- 60-80%: arithmetic / K3 / comparison など主要 Derived 実装の共通 schema 化
- 80-100%: higher-order / structure lift / docs / CI の安定化

## 10. 最終構造

Ajisai を次の三層構造へ近づける。

```text
Human Surface
  実用的で読みやすい語彙、記号糖衣、標準モジュール、応用デモ

AI Semantic Graph
  canonical name, semantic role, derived_from, nil policy, effect boundary,
  algebraic family, diagnosis, law tests

Algebraic Core
  少数の primitive, exact real, K3 logic, structure lift,
  stack/value transformer, HostedEffect schema
```

人間は `+`, `MAP`, `JSON`, `MUSIC@PLAY` を便利に使える。
AI はそれらを次のように読める。

```text
Sugar -> canonical word
canonical word -> Primitive | Derived | Sugar | HostedEffect | Exploratory | Extension | Deprecated
Derived -> algebraic family / primitive schema
HostedEffect -> capability + Eff
Exploratory -> reason + exit options + review gate
```
~~~

## この改訂で期待する効果

- 重複した長大プロンプトを、実行可能な段階的指示へ圧縮できる。
- 既存の `formalization-coverage` / checker を活かし、二重実装を避けられる。
- 生成物と生成元の関係を壊さず、再生成可能な metadata 改修にできる。
- PR ごとの進捗報告形式が固定され、長期改修の可視性が上がる。
- 「AIファースト不可約化」という思想を保ちつつ、レビュー可能な小さな変更に分解できる。
