# Ajisai 人間向け複雑性隠蔽・構造境界是正 指示書レビューと改訂版

Status: instruction review (2026-06-10)
Authority: non-canonical. 言語意味論の正典は `SPECIFICATION.md` のみ。

## 評価サマリ

提示された指示書の方向性（Human Surface / Machine Surface の二層化、`debug_diagnosis` の配置是正、`interpreter/mod.rs` の責務可視化、`gui-application.ts` の composition root 明示、ブラックボックス契約の文書化）は、現在のコードベースの実態と概ね整合しており、妥当である。

特に以下の主張は実コードで裏が取れた。

1. `rust/src/types/value_operations.rs:5` と `rust/src/semantic/absence.rs:2`（および `semantic/protocol_string_tests.rs`）が `crate::interpreter::debug_diagnosis` に依存しており、意味層・型層から実行器への依存逆転が実在する。
2. `debug_diagnosis.rs`（621 行）自体の依存は `crate::error` と `crate::coreword_registry` のみで、interpreter 内部に依存していない。移動は技術的に容易である。
3. `semantic::absence::AbsenceMetadata` が `diagnosis: Option<DebugDiagnosis>` を直接内包しているため、第一候補 `rust/src/semantic/diagnosis.rs` への移動が構造上最も自然である。
4. `rust/src/interpreter/mod.rs` は 47 個の module を平坦に列挙しており、責務分類が見えない、という指摘は正しい。
5. `gui-application.ts` には composition root であることの明示がなく、かつ表層構文知識の重複（後述）が実際に混入している。
6. `simplify:report` と `check:formalization-coverage` は npm script として実在し、`human:report` は既存 script の合成で実現可能である。

ただし、そのまま実行すると問題になる箇所が 7 点ある。改訂版ではこれらを修正した。

### 問題点 1：完了条件「`types` 層が `crate::interpreter::*` に依存していない」は現状では達成不能

`rust/src/types/mod.rs:662` に、指示書が言及していない第二の types→interpreter 依存が存在する。

```rust
pub execution_plans: Option<Arc<crate::interpreter::execution_plan_set::ExecutionPlanSet>>,
```

これは `WordDefinition` に付随する実行計画キャッシュであり、`debug_diagnosis` 移動とは独立した、より大きな設計判断（キャッシュを interpreter 側 map に出すか、opaque trait object 化するか）を要する。今回の指示書の範囲で同時に解消しようとすると差分が危険になる。

改訂版では、完了条件を「`debug_diagnosis` 起因の依存逆転の解消」に限定し、`execution_plans` は既知の例外として `BLACKBOX_REGISTRY.md` の依存例外節に記録し、別指示書で扱う。

### 問題点 2：`check-semantic-firewall.sh` がファイルパスをハードコードしている

`scripts/check-semantic-firewall.sh` は Debug 書式の漏出検査の対象として
`rust/src/interpreter/debug_diagnosis.rs` をパス直書きで指定している。
ファイルを移動してこの script を更新しないと、`rg` が存在しないファイルに対して
エラー終了（exit 2）し、`if` 条件が偽になるため**検査が静かに無効化されたまま成功扱いになる**。

改訂版では、ファイル移動と同一コミットでの script 更新を必須とし、検証コマンドに `npm run check:semantic-firewall` を追加した。

### 問題点 3：`debug_diagnosis` は半分プロトコル層であり、文言凍結の範囲指定が足りない

`debug_diagnosis.rs` は `as_protocol_str()`（`"parseStructure"` 等の camelCase プロトコル文字列）と `AiDiagnosticPayload`（serde Serialize、`diagnosis.agreedPrefix` 等の外部公開フィールド）を含む。これらは WASM/TS 境界と AI 向け出力の契約であり、semantic firewall の保護対象でもある。

指示書の禁止事項「エラー表示やデバッグ出力の文言を不要に変更しないこと」では弱い。改訂版では「プロトコル文字列・serialize されるフィールド名は凍結。1 バイトも変えない」と明示した。

### 問題点 4：移動後の旧パス互換方針が未指定

`debug_diagnosis` の参照元は Rust 側 8 ファイル（types 2、semantic 2、interpreter 3、wasm_interpreter_bindings 2）と script 1 件である。単一 crate 内で参照数が少ないため、互換 re-export（shim）を残すより全参照を新パスへ更新し切る方が、改修対象 2 の禁止事項「`interpreter/mod.rs` を巨大 re-export 集にしない」とも整合する。改訂版で明記した。

### 問題点 5：改修対象 2 の Phase 2/3（中間 module 導入・物理移動）はリスクに比して益が薄い

`interpreter/mod.rs` の実態は「巨大な玄関」というより「分類コメントのない平坦な module 列挙 + 少数の re-export」であり、ロジックは持っていない。47 module の物理移動は import 更新が crate 全域・wasm bindings・テストに波及する。一方、人間可読性の目的は Phase 1（分類コメント）+ 可視性の棚卸しでほぼ達成できる。

改訂版では今回の範囲を Phase 1 と可視性監査に限定し、Phase 2/3 は安定後の別指示書に分離した。`pub use interpreter_core::*;`（glob re-export）は公開面が見えない点で問題だが、展開は別途とし、今回は棚卸し対象としてのみ扱う。

### 問題点 6：docs の配置・Authority 慣行・既存文書との統合先が未指定

本リポジトリでは `docs/` 直下は生成物 JSON（`word-manifest.json` 等）置き場であり、人間向け開発文書は `docs/dev/`、品質文書は `docs/quality/` に置かれ、各文書は冒頭で Authority（non-canonical であること、正典は `SPECIFICATION.md`）を宣言する慣行がある。また Human Surface 構想と重なる既存文書として `docs/dev/three-layer-documentation-model.md`（利用者向け文書の三層モデル）、`PORTABILITY.md`（移植性ポリシー、日本語）、`docs/quality/QUALITY_POLICY.md` が既に存在する。

改訂版では新規文書の配置を `docs/HUMAN_SURFACE.md` / `docs/BLACKBOX_REGISTRY.md` ではなく `docs/dev/` 配下とし、Authority ヘッダ必須・既存文書への相互参照必須とした。記述言語は開発文書の主流に合わせ日本語（識別子・契約名は英語のまま）とする。

### 問題点 7：改修対象 3 は「確認する」で止まっており、既に見えている具体物を挙げていない

`gui-application.ts`（386 行）には、composition root 明示コメントの欠如に加えて、以下の表層構文知識の重複が現に存在する。

```text
- MODULE_IMPORT_PATTERN（gui-application.ts:49）:
  IMPORT / IMPORT-ONLY の表層構文を正規表現で再パースしている。
  tokenizer / interpreter が持つ構文知識の GUI 側複製である。
- HIDDEN_AUTOCOMPLETE_ALIASES（gui-application.ts:40）:
  Core word の alias 一覧を手書きで複製している。
  rust/src/core_word_aliases.rs および word-manifest.json と二重管理になっている。
```

改訂版では、この 2 点を確認対象として名指しし、対応方針（即時移動は必須とせず、出所の明示と二重管理リスクの記録を必須とする）を定めた。

---

# 改訂版指示書：人間向け複雑性の隠蔽と構造境界の是正

## 目的

Ajisai は AI ファースト言語として、語彙メタデータ、数式化カバレッジ、
Primitive / Derived / Sugar / HostedEffect / Exploratory の分類、
Core / Hosted / Platform の分離など、多層的な構造を備えている。

この構造は AI にとっては有効だが、人間の開発者には全体像の把握が難しい。
本改修は、AI ファースト性を損なわずに、人間が理解すべき面だけを明示し、
実装詳細や機械可読メタデータを契約付きでブラックボックス化する。
併せて、以下の構造上の違和感を是正する。

1. `debug_diagnosis` が `interpreter` 配下にあり、`types` / `semantic` から実行器への依存逆転が生じている（実在を確認済み）
2. `interpreter/mod.rs` が 47 module を分類なしで平坦に列挙している
3. `gui-application.ts` が composition root であることが明示されておらず、表層構文知識の複製が混入している

本改修の目的はコード量削減ではない。人間が全内部構造を読まなくても、
設計判断・安全性・移植性・数式化状態を把握できるようにすることである。

## 基本方針

構造を二層に分ける。

```text
Human Surface   人間が設計判断のために見る層
Machine Surface AI、テスト、検査スクリプト、生成ツールが見る層
```

人間が理解すべきもの：言語思想、Core / Hosted / Platform の境界、
語彙分類、仕様上の振る舞い、数式化・移植性・安全性の判定結果、
ブラックボックスの契約。

人間が通常読まなくてよいもの：`word-manifest.json` /
`formalization-coverage.json` の内部構造、生成・検査 script の実装、
exact numeric engine の内部表現、interpreter の個別 dispatch / word 実装、
WASM / Tauri / Platform adapter の接続、GUI wiring / worker / persistence。

ブラックボックス化は「隠して終わり」ではなく、各ブラックボックスについて
「何を保証するか／何を定義してはいけないか／どの検査で守られているか／
人間が読むべき代替文書は何か」を明示する。

---

## 改修対象 1：`debug_diagnosis` の配置是正

### 確認済みの現状

- `rust/src/types/value_operations.rs:5` — `use crate::interpreter::debug_diagnosis::DebugDiagnosis;`
- `rust/src/semantic/absence.rs:2` — 同上。`AbsenceMetadata` が `diagnosis: Option<DebugDiagnosis>` を内包する
- `rust/src/semantic/protocol_string_tests.rs:5` — `CauseClass` / `ErrorLocusKind` / `ErrorPhase` を参照
- `debug_diagnosis.rs` 自身の依存は `crate::error` と `crate::coreword_registry` のみ

### 移動先

`rust/src/semantic/diagnosis.rs` とする（候補比較は不要。`AbsenceMetadata` が
`DebugDiagnosis` を内包しており、semantic 配下が構造上最も自然であることを確認済み）。

### 実施内容

1. `rust/src/interpreter/debug_diagnosis.rs` を `rust/src/semantic/diagnosis.rs` へ移動する
2. `semantic/mod.rs` に `pub mod diagnosis;` を追加し、必要な型を re-export する
3. 全参照元（types 2 ファイル、semantic 2 ファイル、interpreter 内
   `mod.rs` / `execution_loop.rs` / `error_flow_trace.rs`、
   `wasm_interpreter_bindings` 2 ファイル）を新パスへ更新する。
   旧パスの互換 re-export（shim）は残さない
4. **同一コミットで** `scripts/check-semantic-firewall.sh` 内の
   `rust/src/interpreter/debug_diagnosis.rs` を新パスへ更新する。
   更新を忘れると `rg` が存在しないファイルでエラー終了し、
   検査が静かに無効化されたまま成功扱いになる
5. `types` / `semantic` / `interpreter` の依存方向を確認する

### 凍結事項（禁止事項より強い）

- `as_protocol_str()` が返すプロトコル文字列（`"parseStructure"`、`"executeWord"` 等）と、
  `AiDiagnosticPayload` / `DebugCheck` の serialize されるフィールド名
  （`diagnosis.agreedPrefix` を含む）は外部契約である。1 バイトも変更しない
- 診断情報の意味、enum variant の集合を変更しない
- `types` / `semantic` から `interpreter` への依存を新たに追加しない
- 循環依存を導入しない

### 既知の例外（本改修の範囲外）

`rust/src/types/mod.rs:662` の
`WordDefinition.execution_plans: Option<Arc<crate::interpreter::execution_plan_set::ExecutionPlanSet>>`
は本改修では解消しない。`BLACKBOX_REGISTRY.md` の依存例外節に記録し、
解消（interpreter 側キャッシュ map への移設、または opaque 化）は別指示書とする。

### 完了条件

```text
- debug_diagnosis 起因の types -> interpreter / semantic -> interpreter 依存が消えている
- types 層の crate::interpreter::* 参照が execution_plans の既知例外 1 件のみである
- check-semantic-firewall.sh が新パスを検査しており、かつ通る
- プロトコル文字列・serialize フィールド名に差分がない
- cargo test が通る
```

---

## 改修対象 2：`interpreter/mod.rs` の責務可視化（Phase 1 のみ）

### 範囲の限定

原案の Phase 2（中間 module 導入）・Phase 3（物理移動）は本改修では行わない。
47 module の物理移動は import 更新が crate 全域・WASM bindings・テストへ波及し、
差分リスクに対して可読性の利得が薄い。Phase 1 と可視性監査で目的の大半を達成し、
Phase 2/3 は本改修が安定した後の別指示書とする。

### 実施内容

1. `interpreter/mod.rs` の module 宣言を意味別に並べ替え、分類コメントを付す。
   分類は以下を基準とする（実ファイルとの整合で調整可）：

```rust
// Runtime state and evaluation context
//   (interpreter_core, execution_loop, resolve_word, resolve_cache, epoch, ...)
// Word implementations (Core / Derived / builtins)
//   (arithmetic, comparison, logic, math_ops, tensor_ops, sort, ...)
// Module system builtins
//   (modules, ...)
// Optimization and execution strategy
//   (compiled_plan, quantized_block, redundancy_layer, optimization_hooks, ...)
// Hosted effects and platform-facing operations
//   (host, io, serial, audio, random, datetime, ...)
// Interpreter-local diagnostics and validation
//   (error_flow_trace, naming_convention_checker, shadow_validation, ...)
```

2. `#[cfg(test)]` module 群は分類コメント不要。末尾にまとめる
3. 公開面の棚卸しを行う：`pub` / `pub(crate)` / private の現状を確認し、
   crate 外（WASM bindings 等）から参照されていない `pub` を `pub(crate)` に
   降格できるものは降格する。ただし降格はコンパイルとテストが通る範囲に限る
4. `pub use interpreter_core::*;` の glob re-export は公開面が読めないため
   棚卸し対象として記録する。明示列挙への展開は参照箇所が多い場合は本改修では行わず、
   `BLACKBOX_REGISTRY.md` に課題として記録する

### 禁止事項

- 仕様上の振る舞い・word の意味を変更しない
- Core word と HostedEffect の境界を曖昧にしない
- `pub` を増やさない
- ファイルの物理移動・rename を行わない（Phase 1 の範囲外）

### 完了条件

```text
- interpreter/mod.rs を読めば module の大分類（runtime / words / modules /
  optimization / hosted / diagnostics）が分かる
- HostedEffect 系 module と Core 実行系 module がコメント上区別されている
- public API が拡大していない（縮小は可）
- cargo test が通る
```

---

## 改修対象 3：`gui-application.ts` を composition root として明示

### 確認済みの現状

`src/gui/gui-application.ts`（386 行）は GUI 部品・サービスの結線中心であり、
composition root として概ね健全である。ただし以下が混入している。

```text
1. MODULE_IMPORT_PATTERN（gui-application.ts:49 付近）:
   IMPORT / IMPORT-ONLY の表層構文を正規表現で再パースしている
2. HIDDEN_AUTOCOMPLETE_ALIASES（gui-application.ts:40 付近）:
   Core word alias の手書き複製。rust/src/core_word_aliases.rs および
   word-manifest.json と二重管理
```

### 実施内容

1. ファイル冒頭に以下のコメントを追加する：

```ts
/**
 * GUI composition root.
 *
 * This module wires together GUI services, components, workers, persistence,
 * and rendering. It is intentionally allowed to depend on many GUI-facing
 * modules because its role is composition, not domain logic.
 *
 * Ajisai language semantics, Core word behavior, formalization rules, and
 * portability policy must not be defined here.
 */
```

2. `MODULE_IMPORT_PATTERN` と `HIDDEN_AUTOCOMPLETE_ALIASES` について、
   即時の移設は必須としない。ただし最低限、各定義のコメントで
   「これは表層構文知識 / alias 一覧の GUI 側複製であり、正典は
   tokenizer / core_word_aliases.rs / word-manifest.json である」ことを明示し、
   `BLACKBOX_REGISTRY.md` の gui_shell 項の must_not_define / 既知の重複として記録する。
   可能であれば alias 一覧は word-manifest.json 由来の生成データへ置換してよい
3. 他に言語意味論・platform 依存の混入がないか確認し、あれば適切な層に移す
4. import が多いこと自体は composition root として許容する

### 完了条件

```text
- gui-application.ts が composition root として明示されている
- 表層構文知識の複製 2 件が、出所と正典の明示付きで管理されている
- 既存 UI 動作が変わっていない
- npm run check / npm test が通る
```

---

## 改修対象 4：Human Surface とブラックボックス境界の文書化

### 配置と慣行

本リポジトリの慣行に合わせ、以下とする。

```text
docs/dev/HUMAN_SURFACE.md
docs/dev/BLACKBOX_REGISTRY.md
```

（`docs/` 直下は生成物 JSON 置き場のため使わない。）

両文書とも冒頭に Authority 宣言を置く：
non-canonical であり、言語意味論の正典は `SPECIFICATION.md` のみであること。

記述言語は開発文書の主流に合わせ日本語とする（識別子・契約 ID は英語）。

### 既存文書との統合

新規作成の前に、以下と内容が重複しないよう相互参照で接続する。

```text
- docs/dev/three-layer-documentation-model.md（利用者向け文書の三層モデル。
  HUMAN_SURFACE は開発者向けの層定義であり、役割が異なることを明記）
- PORTABILITY.md（移植性ポリシー。Portability status の正典として参照）
- docs/quality/QUALITY_POLICY.md / TRACEABILITY_POLICY.md（検証側の正典として参照）
```

### `docs/dev/HUMAN_SURFACE.md` の内容

原案の構成（Humans should understand / are not expected to understand / Rule）を踏襲し、
opaque な subsystem については「契約・テスト・生成レポート・仕様」を読むという
規則を明記する。加えて、人間の入口となるコマンド（改修対象 5 の `human:report`）と
正典文書群（`SPECIFICATION.md`、`PORTABILITY.md`）への導線を載せる。

### `docs/dev/BLACKBOX_REGISTRY.md` の内容

最低限、以下の subsystem を含める。

```text
- exact_numeric_engine          rust/src/types/continued_fraction.rs, fraction*.rs
- word_manifest                 docs/word-manifest.json, scripts/generate-word-manifest.mjs
- formalization_coverage        docs/formalization-coverage.json, scripts/check-formalization-coverage.mjs
- interpreter_word_implementations  rust/src/interpreter/（word 実装群）
- wasm_bridge                   rust/src/wasm_interpreter_bindings/
- gui_shell                     src/gui/
- platform_adapter              src/platform/, src-tauri/
- script_generated_reports      scripts/*.mjs, docs/*.json
```

各 subsystem について `id / path / human_visibility / human_contract /
verified_by / must_not_define / human_entrypoint` を記述する（原案の記述例に従う）。

加えて、**依存例外節**を設け、以下を記録する。

```text
- types -> interpreter: WordDefinition.execution_plans（rust/src/types/mod.rs）。
  実行計画キャッシュ。解消は別指示書。
- gui_shell 内の表層構文知識複製: MODULE_IMPORT_PATTERN,
  HIDDEN_AUTOCOMPLETE_ALIASES（src/gui/gui-application.ts）。
  正典は tokenizer / core_word_aliases.rs / word-manifest.json。
- interpreter/mod.rs の pub use interpreter_core::*（glob re-export）。
  公開面の明示列挙は別途。
```

---

## 改修対象 5：人間向けレポートの導線

既存 script（`simplify:report` = `scripts/ajisai-simplify-report.mjs`、
`check:formalization-coverage` = `scripts/check-formalization-coverage.mjs`）が
実在することを確認済み。`package.json` に以下を追加する。

```json
{
  "scripts": {
    "human:report": "npm run simplify:report && npm run check:formalization-coverage"
  }
}
```

新規のレポート整形実装は本改修では行わない。目的は完璧なレポートではなく、
人間が見る単一の入口を作ることである。原案の理想形（語彙集計・数式化状態・
opaque subsystem 一覧の統合表示）は、必要になった時点で別指示書とする。
`HUMAN_SURFACE.md` から `npm run human:report` を案内する。

---

## 依存方向の原則

```text
specification / semantic / types
  ↓
interpreter
  ↓
wasm / gui / platform
```

下位層が上位層の意味を実装する。上位層が下位層の都合に依存してはならない。

禁止される依存：`types -> interpreter`、`semantic -> interpreter`、
`core semantics -> platform adapter`、`word classification -> GUI`、
`formalization rule -> WASM bridge`。

許容される依存：`interpreter -> types / semantic`、
`gui -> interpreter facade / wasm facade`、
`platform adapter -> hosted effect interface`、
`scripts -> manifest / coverage files`。

既知の例外（記録の上、別指示書で解消）：
`WordDefinition.execution_plans`（types -> interpreter）。

---

## テスト・検証

以下は全て本リポジトリに実在するコマンドである。改修の各コミット後に実行する。

```bash
# TypeScript / GUI
npm run check
npm run lint
npm test
npm run build

# レポート・検査（改修対象 1 では check:semantic-firewall が特に必須）
npm run check:semantic-firewall
npm run simplify:report
npm run check:formalization-coverage
npm run word:manifest:check

# Rust
cargo test
cargo clippy
cargo fmt --check
```

---

## 期待する最終状態

```text
- 人間が読むべき設計面が docs/dev/HUMAN_SURFACE.md に明示されている
- ブラックボックス化してよい subsystem とその契約が docs/dev/BLACKBOX_REGISTRY.md に
  記載され、既知の依存例外も同所に記録されている
- DebugDiagnosis が rust/src/semantic/diagnosis.rs にあり、
  debug_diagnosis 起因の依存逆転が解消されている
- check-semantic-firewall.sh が移動後のパスを検査している
- interpreter/mod.rs を読めば module の大分類が分かる
- gui-application.ts が composition root として明示され、
  表層構文知識の複製が正典への参照付きで管理されている
- npm run human:report が人間向けの単一入口として機能する
```

## 注意点（原案から維持）

言語仕様・語彙の意味・Core semantics を変更しないこと。
word の意味変更、Primitive / Derived / Sugar 分類の変更、HostedEffect の Core への混入、
数式化状態のごまかし、GUI 都合による仕様変更、interpreter 都合による semantic 層の
汚染は行わない。

最終的な理想：

```text
AI は詳細構造を読む。
人間は契約とレポートを見る。
仕様は常に Core / Hosted / Platform の境界によって守られる。
```
