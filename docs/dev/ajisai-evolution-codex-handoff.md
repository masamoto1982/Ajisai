# Ajisai発展改修プロジェクト Codex向け引き継ぎ指示書

Status: work order (non-canonical). Authority: `SPECIFICATION.html` only.
この文書は実装作業の指示書であり、Ajisai の意味論を定義しない。
本指示書と `SPECIFICATION.html` が矛盾する場合は `SPECIFICATION.html` に従う。
`CLAUDE.md` は明示的に非正典である。

**対象:** Ajisai リポジトリの現行スナップショット  
**正典:** `SPECIFICATION.html` のみ  
**目的:** Ajisai の既存の意味論と安全性を維持しながら、以下の 8 項目を順番に実装する。

1. ユーザー定義語の正式な契約
2. `check --contract` の抽象実行化
3. Word metadata の単一ソース化
4. 意味役割の二重管理解消
5. GUI 実行におけるコンパイル成果物の再利用
6. 実行証明書・実行 receipt
7. Tier 2 の限定的な実用化
8. プロジェクト、パッケージ、CLI、DATA モジュールの実用化

---

## 1. この改修の基本方針

このプロジェクトは、Ajisai を別の言語へ作り替える作業ではない。

既存実装には、すでに次の重要な性質がある。

- 正確実数を統一的な観測過程として扱う Tier 0〜2 数値系
- NIL、論理的 UNKNOWN、プログラム誤用の分離
- `CompiledPlan` と通常実行経路の共存
- Shadow Validation による高速経路と参照経路の照合
- `IntegrityMode::Fallback` による参照経路優先
- content-addressed なユーザー語 identity
- SCC を考慮した再帰語の identity 計算
- Capability、purity、effects、partiality、nil policy などの契約情報
- CLI、WASM、GUI、Python 参照実装、conformance suite による多層検証

今回の改修では、これらを破棄したり一般的な型付き言語へ置き換えたりしない。

目標は次である。

> Ajisai の各所に分散している意味情報、契約情報、コンパイル成果物、検証結果を接続し、組み込み語だけでなくユーザー語、CLI、GUI、パッケージにも同じ整合性原則を適用する。

---

## 2. 権威順位

作業中は、以下の順位を厳守すること。

1. `SPECIFICATION.html`
2. 数学的形式化および正典から生成された検証資産
3. conformance suite、law tests、参照実装
4. Rust 本番実装
5. WASM、TypeScript、GUI 実装
6. `docs/dev/` の設計文書
7. README、過去の引き継ぎ文書、アーカイブ文書

実装と仕様が食い違って見える場合、実装を根拠に仕様を黙って変更してはならない。以下のいずれかに分類すること。

- 仕様が明確で実装が異なる: 実装バグ候補
- 実装同士が異なり仕様が一意に決めている: 実装バグ候補
- 仕様が複数の解釈を許す: 仕様の穴候補
- 仕様変更が必要: コード変更とは分離し、影響と提案を報告する

仕様の穴を発見しても、Codex 自身の判断だけで新しい正典意味論を確定してはならない。

---

## 3. 変更してはならない中核原則

以下は、各フェーズで明示的な仕様変更が承認されない限り維持する。

### 3.1 固定表層

- 後置記法を維持する。
- 新しい中置構文を導入しない。
- 利用者定義の構文、演算子、Sugar を導入しない。
- 括弧構文など、現在拒否される構文を便宜上受理しない。
- Coreword の名前解決規則を変更しない。

### 3.2 数値の意味

- Float64 を Ajisai の標準数値へ変更しない。
- Tier や内部表現を言語上観測可能にしない。
- Tier 0、1 で決定可能な比較を予算不足として UNKNOWN にしない。
- 論理的 UNKNOWN を一般的な通信失敗や例外に流用しない。
- 近似を正確値として表示または直列化しない。

### 3.3 NIL、UNKNOWN、エラー

- NIL は理由付き欠落である。
- UNKNOWN は観測予算内で真偽または順序を決定できない状態である。
- 不正な語使用やスタック形状不正はエラーである。
- 三者を一つの `Result` 型や汎用失敗値へ統合しない。
- NIL の `reason`、`origin`、`recoverability` を高速経路で失わない。

### 3.4 最適化

- SIMD、VTU、QuantizedBlock、Elastic、Hedged、形状 IC などを言語意味にしない。
- Ajisai プログラムから特定の実装経路を選択させない。
- 高速経路の結果が参照経路と異なる場合、既定の Fallback 原則を維持する。
- 性能改善のために Shadow Validation の観測対象を狭めない。

### 3.5 移植性と正典性

- Rust 実装を唯一の正統実装として扱わない。
- Python 参照実装を第二の正典として扱わない。
- 公式ビルドだけが実行できる秘密の言語機能を追加しない。
- Hosted Capability と実装者の特権を混同しない。

---

## 4. 作業単位

8 項目を一つの巨大変更として実装してはならない。

各フェーズを独立した変更単位とし、次の条件を満たしてから次へ進む。

1. 現状調査を完了する。
2. 変更対象と非対象を文書化する。
3. 既存挙動を固定する回帰テストを先に追加する。
4. 最小構造を実装する。
5. 既存テストと新規テストを通す。
6. 生成物、マニフェスト、provenance を同期する。
7. 残課題と次フェーズへの前提を報告する。
8. 次フェーズへ勝手に着手しない。

一つのフェーズが大きい場合、そのフェーズ内で複数の PR 相当単位へ分割してよい。ただし、フェーズ番号は維持する。

---

## 5. 共通の作業規則

### 5.1 着手前

必ず以下を行う。

- 作業ツリーの変更状態を確認する。
- 利用者がすでに加えた変更を上書きしない。
- 対象実装を `rg` で横断的に確認する。
- 対象機能の既存テストを列挙する。
- `SPECIFICATION.html` の関連節を確認する。
- `docs/dev/` は設計参考として読み、正典とは区別する。
- 変更予定ファイルと依存関係を簡潔にまとめる。

### 5.2 実装中

- unrelated cleanup を混ぜない。
- 大規模な名前変更を同時に行わない。
- 互換アダプタを置く場合は、削除条件をコメントまたは設計文書に明記する。
- 同じ意味の情報を新しい場所へ複製しない。
- 自然文を機械契約としてパースしない。
- 外部 protocol に Rust の `Debug` 表現を出さない。
- 新規または大幅改修する Rust ファイルは原則 500 行以内にする。
- 500 行を超える場合は責務別に分割する。
- 生成ファイルを手編集しない。

### 5.3 生成物

以下は生成元を確認して更新する。

- `SKILL.md`
- `docs/word-manifest.json`
- `docs/primitive-test-map.json`
- conformance manifest
- source provenance attestation
- `src/wasm/generated/`

Rust/WASM 境界を変更した場合は、WASM 成果物も再生成する。

---

## 6. 共通の検証コマンド

変更範囲に応じて、以下を実行する。

### 6.1 Rust

```sh
cd rust

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --verbose
cargo test --tests --verbose
cargo test --lib --features elastic-engine --verbose
cargo test --tests --features elastic-engine --verbose
cargo bench --no-run
```

WASM 対応コードへ影響する場合:

```sh
rustup target add wasm32-unknown-unknown
cargo check --features wasm --target wasm32-unknown-unknown
```

### 6.2 TypeScript と GUI

リポジトリルートで実行する。

```sh
npm install
npm run check
npm run lint
npm test
npm run check:semantic-firewall
npm run check:file-size
```

### 6.3 生成物と追跡可能性

```sh
npm run conformance:manifest
npm run word:manifest
npm run generate:skill
npm run primitive:test-map
npm run check:formalization-coverage
npm run provenance:attest
```

最終確認:

```sh
npm run word:manifest:check
npm run check:skill
npm run provenance:check
```

### 6.4 WASM 境界

```sh
npm run build:wasm

cd rust/wasm-tests
wasm-pack test --node
```

### 6.5 Python 検証資産

仕様または Core 意味論へ影響する場合:

```sh
cd python
python tests/test_spec_examples.py
```

本番 CLI との意味差を確認する場合:

```sh
cd rust
cargo build --bin ajisai --release

cd ..
python3 tools/ajisai-repro/compare.py
python3 tools/ajisai-repro/compare.py --conformance
```

差分が出た場合、参照実装に合わせて本番実装を自動修正してはならない。仕様と suite によって裁定する。

---

## 7. フェーズ間の依存関係

基本順序は変更しない。

```text
Phase 1: ユーザー語契約
    ↓
Phase 2: 抽象実行検査
    ↓
Phase 3: metadata単一ソース化
    ↓
Phase 4: 意味役割の所有権整理
    ↓
Phase 5: コンパイル成果物再利用
    ↓
Phase 6: 実行receipt
    ↓
Phase 7: Tier 2語彙
    ↓
Phase 8: 実用ツールとパッケージ
```

Phase 3 では、Phase 1 で導入した契約 API を内部的に整理してよい。ただし、Phase 1 と 2 で成立した挙動を壊してはならない。

Phase 4 は高リスクである。設計判断が確定する前に全スタック操作を一括置換してはならない。

Phase 5 では、まず同一 WASM インスタンスまたは同一 Worker 内での再利用を成立させる。Worker 間で Rust の `CompiledPlan` を直接共有することは初期スコープ外とする。

---

## 8. Phase 1: ユーザー定義語に正式な契約を持たせる

### 8.1 目的

組み込み語だけでなく、ユーザー定義語についても以下を機械的に問い合わせられるようにする。

- スタック質量
- 純粋性
- 副作用
- Capability
- 決定性
- 評価順序依存性
- NIL の生成、伝播、拒否
- 論理的 UNKNOWN を生成し得るか
- Water 依存性
- 推論が完全か保守的か

Phase 1 では新しい表層構文を追加しない。契約は定義本体と依存語から推論する。

### 8.2 主な関連ファイル

- `rust/src/types/mod.rs`
- `rust/src/interpreter/execute_def.rs`
- `rust/src/interpreter/resolve_word.rs`
- `rust/src/interpreter/word_identity.rs`
- `rust/src/interpreter/mass_conservation.rs`
- `rust/src/interpreter/quantized_block.rs`
- `rust/src/elastic/purity_table.rs`
- `rust/src/interpreter/comptime/policy.rs`
- `rust/src/cli/coverage.rs`
- `rust/src/interpreter/dictionary_*_tests.rs`
- `rust/src/interpreter/dependents_index_tests.rs`

### 8.3 実装要件

新しい共有型を、意味論または解析責務に合う独立モジュールへ置く。

例:

```text
WordContract
  flow
  purity
  effects
  capabilities
  determinism
  orderSensitivity
  nilBehavior
  unknownBehavior
  waterSensitivity
  confidence
```

内部で「推論不能」を表す名前に、Ajisai の論理的 `UNKNOWN` と混同する名称を使わない。`Indeterminate`、`NotInferred`、`Conservative` などを使う。

契約推論は単調な lattice として設計する。

- 純粋から不純へは広げられる。
- 固定質量から動的質量へは広げられる。
- NIL 非生成から NIL 生成可能へは広げられる。
- 完全推論から保守推論へは広げられる。
- 後の反復で危険性を狭める設計にしない。

再帰語は SCC 単位で不動点を求める。収束しない、または実行時構造に依存する場合は保守的契約とする。

契約のキャッシュキーには、少なくとも以下を反映する。

- 語本体の content identity
- 依存語の content identity
- Core または Module 契約バージョン
- 推論器の schema version

`DEF`、`DEL`、再定義、import 変更時に、依存関係に従って無効化する。

### 8.4 移行方針

`WordDefinition.capabilities` などの既存フィールドは、Phase 1 では互換アダプタとして残してよい。

ただし、新しい推論結果と既存フィールドのどちらが権威かを明確にする。新規コードは原則として `WordContract` を参照する。

Phase 3 で削除または統合できるように、重複箇所を一覧化する。

### 8.5 必須テスト

最低限、以下を追加する。

- 算術だけを呼ぶ純粋なユーザー語
- `PRINT` を呼ぶ不純なユーザー語
- `NOW` や乱数を呼ぶ非決定的なユーザー語
- `DIV` や `GET` を呼び NIL を生成し得る語
- `COMPARE-WITHIN` を呼び UNKNOWN を生成し得る語
- 純粋語を複数段経由する依存チェーン
- 不純語を複数段経由する依存チェーン
- 直接再帰
- 相互再帰
- 語の再定義による契約無効化
- `DEL` による依存契約の変化
- import、unimport による解決先変更
- 同一内容の語が同一契約を再利用すること
- content identity が異なる語を誤って共有しないこと

### 8.6 受け入れ条件

- ユーザー語を一律 `Capabilities::PURE` として扱う新規経路がない。
- PRECOMPUTE、Elastic、QuantizedBlock などが別々に同じ純粋性推論を再実装しない。
- ユーザー語を含む解析が、直ちに「不明」として停止せず契約を参照できる。
- 再帰語で無限ループしない。
- 語の再定義後に古い契約が使われない。
- 契約推論は通常実行を行わない。
- Ajisai 表層構文は変更されていない。

---

## 9. Phase 2: `check --contract` を抽象実行器へ発展させる

### 9.1 目的

現在の `plan_check.rs` は、ソース全体に NIL 生成語や `^` が存在するかを調べる軽量検査である。

Phase 2 では、`CompiledPlan` 上を実際の実行順序に沿って抽象実行し、NIL、UNKNOWN、形状、スタック質量、効果の流れを位置ごとに追跡する。

### 9.2 主な関連ファイル

- `rust/src/cli/plan_check.rs`
- `rust/src/cli/plan_check_tests.rs`
- `rust/src/interpreter/mass_conservation.rs`
- `rust/src/interpreter/compiled_plan.rs`
- `rust/src/interpreter/control_cond.rs`
- `rust/src/interpreter/higher_order/`
- Phase 1 で導入した契約モジュール

### 9.3 抽象状態

最低限、各抽象スタックスロットに以下を持たせる。

```text
shape:
  scalar
  vector
  tensor
  record
  text
  codeBlock
  handle
  unknownShape

presence:
  definitelyPresent
  maybeNil
  definitelyNil

truth:
  notTruth
  true
  false
  maybeUnknown
  definitelyUnknown

interpretation:
  known role
  possible roles
  unassigned

effects:
  accumulated capabilities and observable effects
```

必要以上に一般的な静的型システムを導入しない。目的は、Ajisai の既存契約を抽象実行することである。

### 9.4 制御構造

- `COND` は節ごとに解析し、到達可能な body の結果を join する。
- コードブロックが静的に既知なら、そのサブプランを解析する。
- MAP、FILTER、FOLD などは callback 契約を参照する。
- 実行時にしか決まらない callback は保守的に扱う。
- STAK モードは、数や形状を証明できる場合だけ継続する。
- `^` は直前までに生成された対象値へ flow-sensitive に作用させる。
- ソース中の無関係な `^` を全体的な NIL 対策として扱わない。

### 9.5 診断

診断は、可能な限り次を含める。

- 発生語
- 到達先の語
- line index
- op index または token index
- NIL または UNKNOWN が生じ得る理由
- どの契約から導出されたか
- 具体的な修正候補

例:

```text
GETはこの位置でNILを生成する可能性があります。
その値はfallbackを通らずADDの右オペランドへ到達します。
ADDはNILを伝播するため、最終結果もNILになる可能性があります。
```

Token に span が存在しない場合、Phase 2 だけのために大規模な span 付き AST へ移行しない。まず line/op 位置でよい。

### 9.6 互換性

既存の `check --contract` を維持する。

JSON へ追加フィールドを加える場合は原則 additive にする。既存フィールドを削除または意味変更する場合は schema version を検討する。

### 9.7 必須テスト

- NIL 生成語の直後に fallback がある
- NIL 生成語と無関係な位置に fallback がある
- NIL が複数語を経由して RejectsNil 語へ届く
- COND の一方だけが NIL を生成する
- COND の全分岐が同じ形状を返す
- COND の分岐で異なる形状を返す
- ユーザー語内部で NIL が生成される
- ユーザー語の契約を介して NIL 流を検出する
- MAP callback が NIL を生成する
- FOLD callback のスタック質量不正
- KEEP と EAT の質量差
- TOP と STAK の切り替え
- 論理的 UNKNOWN と NIL を混同しない
- 解析不能な箇所で誤った安全判定を出さない

### 9.8 受け入れ条件

- `has_fallback` の単純な全体フラグだけで安全判定しない。
- ユーザー語に到達しても Phase 1 契約で解析を継続できる。
- COND と既知コードブロックを解析できる。
- 抽象実行は Ajisai プログラムを実行しない。
- 解析不能は「安全」と解釈されず、保守的な Note または Advisory になる。
- 既存の通常実行意味論に変更がない。

---

## 10. Phase 3: Word metadata を単一ソース化する

### 10.1 目的

同じ意味を表す情報が以下へ分散している状態を解消する。

- `BuiltinSpec`
- `CorewordMetadata`
- `mass_contract()`
- `elastic/purity_table.rs`
- `ModuleWord`
- module docs
- executor dispatch
- word manifest 生成
- GUI 向け word info
- Capability 判定
- conformance coverage

### 10.2 主な関連ファイル

- `rust/src/builtins/builtin_word_definitions.rs`
- `rust/src/builtins/builtin_word_details.rs`
- `rust/src/coreword_registry.rs`
- `rust/src/elastic/purity_table.rs`
- `rust/src/interpreter/execute_builtin.rs`
- `rust/src/interpreter/modules/module_builtins.rs`
- `rust/src/interpreter/modules/module_word_types.rs`
- `rust/src/interpreter/modules/module_word_docs.rs`
- `scripts/generate-word-manifest.mjs`
- `docs/dev/semantic-metadata-refactor-checklist.md`

### 10.3 目標構造

Coreword と Module word の双方が、共有の typed contract schema を持つ。

```text
StaticWordSpec
  canonicalName
  canonicalHome
  executor
  functionalGroup
  documentationGroups
  wordShape
  flowContract
  purity
  effects
  deterministic
  orderSensitive
  partiality
  nilPolicy
  unknownPolicy
  requiredCapability
  safetyLevel
  stability
  documentation
```

executor の関数型が Core と Module で異なる場合、executor まで無理に同じ型へ統合しない。契約 schema を共有し、実行アダプタを分ける。

### 10.4 必須方針

- `mass_contract(name)` の独立した手入力 match を廃止する。
- `builtin_purity()` の独立した手入力 match を廃止する。
- module word の自然文から分類を抽出しない。
- `description`、`summary`、`category` に機械タグを埋め込まない。
- canonical implementation、listing、documentation grouping を別フィールドにする。
- 有限集合は enum または検証済み newtype で表す。
- GUI へ必要のない内部情報を WASM 境界へ追加しない。
- 巨大な一ファイルへ全語を集約しない。カテゴリまたはモジュール単位の宣言ファイルへ分割してよい。
- マクロを使う場合、IDE エラーとテスト失敗が理解可能な形にする。

### 10.5 移行手順

1. 既存の重複と上書き関係をテストで固定する。
2. 共有 schema を導入する。
3. Coreword を共有 schema へ移す。
4. Module word を共有 schema へ移す。
5. mass、purity、Capability、docs の参照先を切り替える。
6. 旧テーブルを削除する。
7. manifest および GUI アダプタを共有 schema から生成する。
8. 残存検索を行う。

### 10.6 受け入れ条件

- 一つの語の mass、purity、effects、nil policy を複数箇所へ手入力しない。
- 旧テーブルと新テーブルを恒久的に並存させない。
- word manifest が新 schema から生成される。
- Core と Module の正典実装位置が明確である。
- listing metadata と言語意味契約が混ざっていない。
- 既存の IMPORT、LOOKUP、GUI 表示、CLI coverage が維持される。
- Phase 1 のユーザー語契約が共有 schema と接続される。

---

## 11. Phase 4: 意味役割の二重管理を解消する

### 11.1 目的

現在の以下の二重管理を解消する。

- `Value.hint`
- `SemanticRegistry.stack_hints`

Module word 実行後の `semantic_sync.rs` による fingerprint 再同期を不要にする。

### 11.2 主な関連ファイル

- `rust/src/types/mod.rs`
- `rust/src/types/value_operations.rs`
- `rust/src/types/value_protocol.rs`
- `rust/src/interpreter/interpreter_core.rs`
- `rust/src/interpreter/execution_loop.rs`
- `rust/src/interpreter/modules/semantic_sync.rs`
- `rust/src/interpreter/control_cond.rs`
- `rust/src/interpreter/value_extraction_helpers.rs`
- `rust/src/interpreter/cast/`
- `rust/src/cli/report.rs`
- `rust/src/wasm_interpreter_bindings/`

### 11.3 重要な注意

このフェーズでは、最初から `Stack = Vec<StackSlot>` への全面置換を開始してはならない。

まず設計メモを作り、少なくとも次の案を比較する。

#### 案A: StackSlot 方式

```text
StackSlot
  value
  interpretation
```

トップレベルの位置依存役割は StackSlot が所有し、Value は構築時既定役割またはネスト要素の役割だけを持つ。

#### 案B: SemanticStack 方式

値列と役割列を private な一つの型へ封じ、長さ不一致や別々の操作を型として禁止する。

#### 案C: Value 所有方式

トップレベル役割を Value へ統合し、`>CF` などは値の役割だけを更新する。ただし位置依存意味と clone 時の挙動を仕様・既存テストに照らして慎重に検証する。

採用条件は、コード量ではなく次で判断する。

- SPEC §12 の意味を忠実に表せる
- 位置依存の再解釈を保持できる
- 値構築時の既定役割を保持できる
- Vector、Tensor、Record 内部の役割を失わない
- NIL passthrough で理由と役割を失わない
- Shadow Validation で同じ観測比較ができる
- CLI と WASM の wire format が変わらない
- 並列経路と子ランタイムで安全に扱える

### 11.4 段階移行

1. 現状挙動を固定する回帰テストを追加する。
2. 新しいスタック抽象を導入する。
3. push、pop、truncate、extend、snapshot を新抽象へ集約する。
4. Coreword 経路を移行する。
5. Module word 経路を移行する。
6. COND、HOF、子ランタイム、Shadow Validation を移行する。
7. CLI、WASM 境界を移行する。
8. `semantic_sync.rs` と fingerprint 方式を削除する。
9. 旧 API を削除する。

### 11.5 必須回帰テスト

- `>CF` 後の表示
- `>CF` 対象外スロットの役割
- KEEP による値保持
- NIL passthrough
- UNKNOWN の TruthValue 役割
- JSON parse 後に入力 Text 役割が漏れない
- Text、Boolean、Timestamp、Interval
- Vector と Tensor の leaf role
- Record の構築と直列化
- COND guard 実行前後
- MAP、FILTER、FOLD 前後
- Shadow Validation の stack 比較
- WASM と CLI の protocol 一致
- stack と role の長さ不一致を構造上作れないこと

### 11.6 受け入れ条件

- トップレベルスタックスロットの役割に唯一の権威がある。
- `Value.hint` と `stack_hints` のどちらを優先するかを実行時に推測しない。
- `semantic_sync.rs` の fingerprint 比較が削除される。
- 役割同期のために Arc ポインタ同一性を使わない。
- wire format と表示結果が既存テスト上同一である。
- Phase 4 のために Ajisai の表層意味論を変更しない。

---

## 12. Phase 5: コンパイル成果物をセッション間で再利用する

### 12.1 目的

GUI Worker 内で毎回 `interpreter.reset()` した際に、意味的に不変な成果物まで失われる状態を改善する。

対象候補:

- tokenized body
- body content store
- `CompiledPlan`
- 推論済み `WordContract`
- Shadow Validation 済み情報
- HOF memo に使用できる静的 kernel 情報
- 解決済み依存 identity

### 12.2 主な関連ファイル

- `rust/src/interpreter/interpreter_core.rs`
- `rust/src/interpreter/execute_def.rs`
- `rust/src/interpreter/execute_builtin.rs`
- `rust/src/interpreter/execution_plan_set.rs`
- `rust/src/interpreter/compiled_plan.rs`
- `rust/src/interpreter/word_identity.rs`
- `rust/src/interpreter/epoch.rs`
- `rust/src/interpreter/shadow_validation.rs`
- `src/workers/interpreter-snapshot.ts`
- `src/workers/interpreter-execution-worker.ts`
- `src/workers/execution-worker-manager.ts`
- WASM state 復元処理

### 12.3 初期スコープ

まず同一 Interpreter または同一 Worker 内で再利用する。

以下は初期スコープ外とする。

- Rust の `Arc<CompiledPlan>` を Web Worker 間で直接共有する
- compiled executor pointer を直列化する
- IndexedDB へ内部 `CompiledPlan` を保存する
- ブラウザ間で内部 IR 互換性を保証する

Worker 間共有を将来検討する場合は、再構築可能な中間 artifact descriptor だけを対象にする。

### 12.4 目標構造

```text
SessionState
  stack
  imports
  dictionaries
  output
  effects
  child runtimes
  execution modes
  water

ArtifactStore
  body identity → parsed body
  artifact key → compiled plan
  artifact key → inferred contract
  artifact key → validation summary
```

`reset()` を少なくとも次へ分離する。

- セッション状態だけを消す操作
- artifact も含めて完全に消す操作

既存の公開 `reset()` の意味を変更する場合は、WASM 利用箇所とテストを確認する。安全なら新しいメソッド名を追加し、GUI だけを段階移行する。

### 12.5 ArtifactKey

辞書 epoch だけに依存するキーでは不十分である。

最低限、以下を検討する。

- body content identity
- 依存語 content identities
- Core／Module 契約 schema version
- compile feature flags
- relevant configuration flags
- `CompiledPlan` schema version

名前や登録順だけをキーにしない。

### 12.6 必須テスト

- 同一 snapshot の連続実行で plan build 回数が増えない
- 語名が違っても同一内容なら安全に再利用できる
- 同名でも依存 identity が異なれば再利用しない
- 再定義時に古い artifact を使わない
- import 変更時に必要な artifact だけ無効化される
- `DEL` 後に孤児 artifact が回収される
- Shadow Validation の結果を誤った語へ流用しない
- configuration flag 変更時に不適切な plan を使わない
- GUI snapshot 復元結果が従来と同一
- ArtifactStore に上限または回収方針がある
- 長時間 Worker で無制限に増えない

### 12.7 受け入れ条件

- GUI の各実行で不変 artifact を全破棄しない。
- セッション状態と artifact cache の寿命が分離される。
- 再利用の安全性が content identity と依存 identity で説明できる。
- epoch だけに依存した誤共有がない。
- artifact 再利用を無効化しても結果が変わらない。
- metrics で build、hit、miss、eviction を確認できる。

---

## 13. Phase 6: 実行 receipt を導入する

### 13.1 目的

実行結果だけでなく、その結果がどのソース、語、Capability、参照照合に基づくかを機械可読にする。

「証明」という名称で過剰な安全保証を主張しない。初期名称は `execution receipt` を推奨する。

### 13.2 主な関連ファイル

- `rust/src/cli/mod.rs`
- `rust/src/cli/report.rs`
- `rust/src/interpreter/shadow_validation.rs`
- `rust/src/interpreter/interpreter_core.rs`
- `rust/src/interpreter/error_flow_trace.rs`
- `rust/src/interpreter/word_identity.rs`
- `rust/src/semantic/absence.rs`
- `rust/src/semantic/protocol.rs`
- `docs/dev/agent-cli-output-contract.md`
- `docs/dev/source-provenance-attestation-design.md`
- `scripts/generate-source-attestation.mjs`

### 13.3 CLI 案

```sh
ajisai run program.ajisai --json --receipt
```

JSON の既存トップレベル契約を壊さず、`receipt` フィールドを追加する。

将来の `ajisai verify` は、receipt schema と再実行比較の意味を設計してから追加する。Phase 6 で無理に暗号学的証明機構まで作らない。

### 13.4 Receipt の候補フィールド

```json
{
  "schemaVersion": "1",
  "sourceIdentity": "...",
  "implementation": {
    "name": "ajisai-core",
    "version": "..."
  },
  "specification": {
    "declaredVersion": "..."
  },
  "executedWords": [
    {
      "resolvedName": "EXAMPLE@FOO",
      "contentIdentity": "..."
    }
  ],
  "requiredCapabilities": [],
  "grantedCapabilities": [],
  "observedEffects": [],
  "water": {
    "stepLimit": 100000,
    "comparisonRefinements": 0
  },
  "integrity": {
    "shadowValidationPerformed": true,
    "referenceAgreement": true,
    "plainFallbacks": 0,
    "integrityMismatches": 0
  },
  "absenceEvents": [],
  "resultIdentity": "..."
}
```

内部最適化方式を安定 protocol として公開しない。

公開してよい情報:

- 参照経路と一致したか
- 不一致時に参照結果へ Fallback したか
- どの Capability と効果が観測されたか
- どの内容 identity を持つ語を実行したか
- 入出力の identity

公開しない情報:

- SIMD lane 幅
- Shape IC の内部状態
- QuantizedBlock 内部表現
- pointer identity
- Tier 内部表現
- Rust enum の `Debug` 名
- 非安定なキャッシュキー

### 13.5 実行語の記録

receipt 有効時だけ軽量な provenance recorder を有効にしてよい。

記録は名前だけでなく、解決後の fully qualified name と content identity を使う。

同一語をループで大量実行した場合は、順序付き全イベント列ではなく、意味上必要な情報へ集約してよい。

```text
identity
firstSeenOrder
callCount
```

observable host effect は順序を保持する。

### 13.6 Result identity

表示文字列だけを hash しない。

CLI と WASM で共有する protocol representation を canonical JSON へ変換し、そのバイト列から identity を計算する。

以下を失わない。

- Value kind
- exact numerator／denominator
- interpretation
- NIL reason
- NIL origin
- recoverability
- logical UNKNOWN diagnosis
- Vector、Tensor、Record 構造

### 13.7 受け入れ条件

- receipt なしの通常実行へ意味変更がない。
- receipt 生成を有効にしても実行結果が変わらない。
- content identity と表示文字列を混同しない。
- host effects の順序が記録される。
- Shadow Validation の Fallback が receipt へ反映される。
- 内部最適化経路を公開契約にしない。
- schema version を持つ。
- 同じ入力、同じ定義、同じ Capability 条件で安定した identity が得られる。
- receipt を「数学的証明」や「改ざん不能」と誤表示しない。

---

## 14. Phase 7: Tier 2 を限定的に実用化する

### 14.1 目的

現在、`Computable` と `Starved` 経路は実装・テストされているが、通常語彙から Tier 2 値を構築できない。

Phase 7 では、Tier 2 を実際に観測できる最小語彙を追加する。

### 14.2 主な関連ファイル

- `rust/src/types/exact/computable.rs`
- `rust/src/types/exact/observation.rs`
- `rust/src/types/exact/value.rs`
- `rust/src/interpreter/comparison.rs`
- `rust/src/interpreter/math_ops.rs`
- `rust/src/interpreter/interval_ops.rs`
- `rust/src/interpreter/modules/module_builtins.rs`
- `rust/src/interpreter/tier2_isolation_tests.rs`
- Python 参照実装および conformance suite

### 14.3 初期語彙候補

MATH モジュールで、次を候補とする。

- `MATH@PI`
- `MATH@E`
- `MATH@ENCLOSE`
- `MATH@REFINE`
- `MATH@APPROX-WITHIN`
- `MATH@DECIDE-WITHIN`

実際の名前は既存命名規則と仕様上の整合を確認して決定する。

最初からすべてを追加する必要はない。推奨される最小単位は次である。

1. Tier 2 定数を一つ導入する。
2. Water を明示した enclosure 観測を導入する。
3. `COMPARE-WITHIN` で UNKNOWN へ到達する conformance case を追加する。
4. その後、二つ目の定数と補助語を追加する。

### 14.4 数学的要件

Tier 2 generator は以下を満たす必要がある。

- 決定的
- 各 step の区間が有理数端点
- `I(k+1) ⊆ I(k)`
- 真の値を常に包含
- 区間幅が収束する
- 不正入力で単調性が破れない
- 同じ総 Water で同じ観測列を与える

π や e の enclosure 生成アルゴリズムは、証明可能な上下界を使う。通常の浮動小数計算を有理数へ包み直す方法は禁止する。

アルゴリズムの根拠と不変条件を設計文書へ記載する。

### 14.5 演算範囲

初期段階では、Tier 2 に対して安全性を証明できる演算だけを許可する。

優先:

- 加算
- 減算
- 乗算
- 否定
- 区間観測
- 比較

慎重に扱う:

- 逆数
- 除算
- ABS
- SIGN
- MIN／MAX
- FLOOR／CEIL
- Tier 2 同士の一般合成

ゼロから分離できない値の逆数を、有限 Water で安全に作れると仮定してはならない。

Tier 2 除算の契約が未定義なら、Phase 7 では対象外とする。

### 14.6 UNKNOWN

Tier 2 導入後、UNKNOWN が実際に語彙から到達可能になる。

以下を確認する。

- UNKNOWN は NIL ではない。
- Kleene 論理が維持される。
- diagnosis に観測進捗が残る。
- Water を増やすと決定できる例と、有限 Water では決定できない例を区別する。
- Tier 0／1 の比較が UNKNOWN へ退行しない。
- receipt が UNKNOWN を正しく表現する。

### 14.7 受け入れ条件

- 少なくとも一つの通常語彙から Tier 2 値を構築できる。
- その値は正しい有理区間列を返す。
- `COMPARE-WITHIN` から論理的 UNKNOWN へ到達できる。
- Tier 0／1 の既存結果に変更がない。
- Tier 内部表現は wire format へ露出しない。
- 浮動小数による疑似的な上下界を使用しない。
- 未定義の Tier 2 除算を便宜上実装しない。
- 参照実装と conformance suite の対応方針が明記される。

---

## 15. Phase 8: 実用ツール、プロジェクト、パッケージ、DATA モジュール

Phase 8 は一つの巨大 PR にしない。少なくとも 8A〜8C へ分割する。

### 15.1 Phase 8A: CLI とプロジェクト基盤

候補コマンド:

```text
ajisai repl
ajisai fmt
ajisai test
ajisai new
```

#### REPL

- Rust CLI で実装する。
- Python REPL をそのまま正典扱いしない。
- ユーザー辞書、imports、stack をセッション内で保持する。
- 構造化診断を表示できる。
- 非対話環境でもテスト可能な入出力分離を行う。

#### Formatter

- 固定表層と Sugar を維持する。
- 意味が変わる書き換えをしない。
- GUI の `src/gui/code-formatter.ts` と同じ期待結果を共有テストで固定する。
- Rust と TypeScript で独立実装する場合、共通 corpus を正本にする。
- formatter を構文正典にしない。

#### Test runner

Ajisai Core へ `ASSERT` を安易に追加しない。

まずホスト側 runner として、次のいずれかを設計する。

- test manifest に期待 stack と output を書く
- `.expected.json` を併置する
- テスト専用コメント directive を使う

コメント directive を採用する場合も、通常言語意味論とは分離する。

### 15.2 Phase 8B: Manifest と lockfile

推奨ファイル:

```text
ajisai.toml
ajisai.lock
```

初期 manifest 候補:

```toml
[project]
name = "example"
version = "0.1.0"
entry = "src/main.ajisai"
specification = "..."

[capabilities]
allow = ["io.output"]

[dependencies]
```

初期 lockfile には以下を記録する。

- 依存パッケージの source identity
- 公開語の content identities
- 必要 Capability
- 適合対象の仕様バージョン
- manifest schema version

初期パッケージ機構では、ローカル path dependency から始めてよい。

以下は初期スコープ外とする。

- 公開中央 registry
- 自動アップロード
- 任意 URL からの無検証実行
- 署名基盤
- semantic version だけによる identity 保証

`ajisai add` は、依存元と検証モデルが決まるまで導入しないか、ローカル path 限定とする。

### 15.3 Phase 8C: DATA モジュール

Core へ新しい表構文を追加せず、まず Module word として実装する。

既存資産を活用する。

- Vector
- Record
- RecordShape
- Tensor
- JSON module
- HOF
- QuantizedBlock
- Capability system

初期候補:

```text
DATA@CSV-PARSE
DATA@CSV-STRINGIFY
DATA@SELECT
DATA@WHERE
DATA@GROUP
DATA@JOIN
DATA@SORT-BY
DATA@CHUNK
```

ただし、一度にすべて追加しない。

推奨順:

1. CSV 文字列と Record vector の相互変換
2. 列選択と行選択
3. group
4. join
5. chunked processing

CSV parse 自体は純粋な変換とする。ファイル読込は既存 IO または Hosted Capability に任せる。

DATA module は、欠損値の理由を保持する。

例:

- 列が存在しない
- 数値変換できない
- 行の列数が不一致
- join key が存在しない

これらをすべて同じ汎用 NIL reason へ潰さない。新しい理由を追加する場合は仕様上の扱いを確認する。

### 15.4 Phase 8 の受け入れ条件

- 単一ファイル以外のプロジェクトを再現可能に実行できる。
- manifest と lockfile の役割が分離されている。
- Capability がプロジェクト単位で確認できる。
- test runner が Core 意味論を変更しない。
- formatter が意味を変えない。
- パッケージ identity が名前と version だけに依存しない。
- DATA module が Core 構文を増やさない。
- Hosted I/O と純粋なデータ変換が分離される。
- 各サブフェーズが独立してテスト可能である。

---

## 16. 各フェーズの完了報告形式

Codex は各フェーズ終了時に、次の形式で報告する。

```text
## 実施フェーズ
Phase N: ...

## 実装概要
- ...

## 主な設計判断
- 判断:
- 理由:
- 却下した代替案:

## 変更ファイル
- path: 変更理由

## 追加・更新したテスト
- test name: 検証内容

## 実行した検証
- command: PASS / FAIL / NOT RUN
- 未実行理由:

## 互換性
- 表層構文:
- CLI JSON:
- WASM:
- GUI:
- conformance:
- reference interpreter:

## 残課題
- ...

## 次フェーズへの前提
- ...

## 仕様上の未解決点
- なし
または
- 該当節:
- 複数解釈:
- 実装で暫定決定していないこと:
```

テストを実行できなかった場合、実行したと主張してはならない。

---

## 17. フェーズ停止条件

次のいずれかに該当した場合、現在フェーズの安全な範囲まで実装し、報告して停止する。

- 正典仕様に複数の互換しない解釈がある
- 既存 conformance case 同士が矛盾する
- 既存利用者の変更と衝突する
- 破壊的 protocol 変更が不可避
- Phase 4 で役割所有権を一意に決められない
- Tier 2 アルゴリズムの上下界を証明できない
- package security model なしにネットワーク取得が必要になる

停止時も、調査結果、追加した回帰テスト、安全に完了した変更を残す。

---

## 18. 最終目標

8 フェーズ完了時、Ajisai は次の状態を目指す。

- ユーザー語を含めて契約を静的に説明できる。
- NIL と UNKNOWN の流れを実行前に位置付きで診断できる。
- 全語の意味メタデータが一つの typed schema から得られる。
- 値と意味役割の所有権が明確である。
- 同じ内容のプログラムはコンパイル成果物を安全に再利用できる。
- 実行結果に再現性と整合性の receipt を添付できる。
- Tier 2 と論理的 UNKNOWN を実際の語彙から観測できる。
- CLI、プロジェクト、テスト、パッケージ、データ処理が実用的である。
- それでも固定表層、正確数値、Semantic Firewall、参照意味論優先という Ajisai の中核は変わらない。
