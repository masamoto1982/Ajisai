# Ajisai 意味論的混在の排除 改修チェックリスト（改訂版）

## この改訂版の位置づけ

この文書は、`description` / `category` / `formerly_module` / `Capabilities::PURE` / WASM 境界タプルなどに残る「1 つの値が複数の意味を兼ねる」設計を、段階的に解消するための実装指示書である。

元の指示書の問題意識は妥当である。一方で、現在のコードには既に一部の分離が入っているため、以下のように改修してから使うべきである。

- `listed_in_modules` は既に存在するため、「まず real module listing と documentation category を分ける」という観点は一部達成済みとして扱う。
- `WordPurity` は既に存在するため、`Capabilities::PURE` の排除は「purity 軸を新設する」ではなく「既存の `WordPurity` を権威にして PURE bit を消す」作業として扱う。
- `description` から map/form/fold 接頭辞を取り除く P0 は最優先で正しいが、分類が UI に必要ないなら分類フィールドを公開境界へ足さない。
- `P1 Coreword contract` は大きすぎるため、いきなり全メタデータを統合せず、まず現状の重複・上書き関係をテストで固定してから導入する。
- `P6 description の用途分割` は P0/P4 と重なるため、最初は module/core/user の境界名だけを分け、全 `description` 改名は後段にする。
- 検索コマンドはリポジトリ標準として `grep -R` ではなく `rg` を使う。

## 全体原則

### 禁止する設計

- 自然文フィールドから、分類・実行契約・表示制御・安全性をパースしない。
- `description`, `summary`, `category` に機械可読タグを埋め込まない。
- 表示用分類と、言語意味上の canonical source / module origin を同じフィールドで表さない。
- UI/CSS のクラス名を、内部 enum の `Debug` 表示や未定義の文字列から直接生成しない。
- WASM/TypeScript 境界に新しい位置依存タプルを追加しない。
- `format!("{:?}", value)` を外部 payload として使わない。
- 有限集合を未検証の `&'static str` / `String` だけで表さない。
- 同じ意味のメタデータを複数箇所で手入力しない。
- compatibility alias / listing surface / canonical implementation を混ぜない。

### 推奨する設計

- 自然文は自然文として残し、機械可読値は enum / typed struct に分離する。
- UI に渡す payload は、外部契約として安定させる値だけに限定する。
- 内部分類は Rust 内部で使い、UI 表示が本当に必要な場合だけ protocol string を明示定義する。
- 境界 payload は object / discriminated union にする。
- 既存利用者を壊しやすい変更は、旧型を internal adapter として短期間だけ残し、受け入れ条件で削除予定を明記する。

## 優先順位と依存関係

| Phase | 対象 | 目的 | 備考 |
| --- | --- | --- | --- |
| Phase 1 | P0 | 自然文への map/form/fold 接頭辞埋め込み排除 | 最優先。小さく完結できる。 |
| Phase 2 | P2, P3 | 名前と意味のズレを解消 | `formerly_module` と category 系を先に整理する。 |
| Phase 3 | P4, P5, P10 | WASM/TS 境界を堅くする | UI 破壊を避けるため object payload と型を同時更新する。 |
| Phase 4 | P1, P7, P8 | Coreword 契約メタデータを単一ソース化 | 影響範囲が大きいので後段。 |
| Phase 5 | P6, P9 | 説明文・値表示境界の残りを整理 | P0/P4 の後に実施する。 |

---

# P0: module word の `description` から map/form/fold 接頭辞を排除する

## 現状評価

妥当性: 高い。現在の module word には map/form/fold 接頭辞が自然文説明に残っており、さらに旧 prefix stripping helper で自然文をパースしている。これは最初に直すべき意味混在である。

注意点:

- `ModuleWord` にはまだ `word_shape` がない。
- 既に `purity`, `effects`, `safe_preview`, `capabilities` は存在するため、`word_shape` は実行契約ではなく「評価形状」だけを表す名前にする。
- UI でボタン背景色などに使う必要がないなら、`word_shape` は WASM/TS payload に含めない。

## 方針

`ModuleWord` に typed field を追加し、説明文は自然文だけにする。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordShape {
    Form,
    Map,
    Fold,
}

pub(super) struct ModuleWord {
    pub short_name: &'static str,
    pub description: &'static str,
    pub word_shape: WordShape,
    // ...
}
```

既に同等の型が導入されている場合は、それを再利用する。Core builtin と module word で同じ概念を共有する場合は、`coreword_registry` か `types` 配下に移す。ただし、Core builtin 側の分類と完全に同義であることを確認するまで安易に共有しない。

## 実装チェックリスト

- [ ] `ModuleWord` に `word_shape` を追加する。
- [ ] `module_word!` macro に `word_shape` 引数を追加する。
- [ ] map/form/fold 接頭辞を削除し、対応する `WordShape` を明示する。
- [ ] 旧 prefix stripping helper を削除する。
- [ ] `module_registry` / `module_builtins` の説明文整形を、パースではなく `description` の直接参照に変える。
- [ ] ユーザー向け module word 説明に分類タグが表示されないことを確認する。
- [ ] `word_shape` を WASM/TS/GUI payload に含めていないことを確認する。
- [ ] UI ボタン背景色や CSS class に `word_shape` を使っていないことを確認する。

## 残存確認

```bash
rg -n '(Map|Form|Fold):[[:space:]]' rust/src docs src
rg -n '(strip|parse)_signature_prefix|(module_word)_signature_type' rust/src docs src
```

## 受け入れ条件

- [ ] module word の `description` に map/form/fold 接頭辞が含まれない。
- [ ] module word の分類は typed field だけに存在する。
- [ ] 説明文をパースして分類を取り出す処理が存在しない。
- [ ] UI 境界に module word の分類値が渡されない。
- [ ] module word の表示、`IMPORT`、`IMPORT-ONLY` が壊れていない。

---

# P2: `formerly_module` を削除する

## 現状評価

妥当性: 高い。`formerly_module` は「過去に module だった」という履歴情報に見えるが、実体は `canonical_home == Module(m)` の派生値である。`CorewordMetadata::canonical_module()` が既に存在するため、削除を第一候補にする。

## 方針

案 A を採用する。`formerly_module` は削除し、必要な箇所は `canonical_home` / `canonical_module()` から派生する。

互換のため一時的に残す必要がある場合だけ、deprecated コメント付きの adapter に隔離する。ただし最終受け入れ条件では名前を残さない。

## 実装チェックリスト

- [ ] `formerly_module` の全参照を確認する。
- [ ] `CorewordMetadata` から `formerly_module` を削除する。
- [ ] default / 初期化箇所から `formerly_module: None` を削除する。
- [ ] module import 時の代入箇所を `canonical_home` のみへ寄せる。
- [ ] テスト名を `canonical_home` ベースに変更する。
- [ ] JSON / WASM / docs に `formerlyModule` 互換が必要か確認し、不要なら削除する。

## 残存確認

```bash
rg -n "formerly_module|formerlyModule" rust/src docs src
```

## 受け入れ条件

- [ ] `formerly_module` / `formerlyModule` という名前が残っていない。
- [ ] module origin は `canonical_home` から一貫して判定できる。
- [ ] 履歴情報と canonical home の意味が混ざっていない。
- [ ] coreword listing 関連テストが通る。

---

# P3: `category` / listing metadata の意味を分離する

## 現状評価

妥当性: 中〜高。元の指示は方向性として正しいが、現在のコードには既に `listed_in_modules` と `listed_in_categories` が分離されている。そのため、最初の作業は `listed_in_categories` を「documentation-only group」として改名・型付けすること、次に `category` を `functional_group` へ改名することである。

## 方針

- `category`: Core builtin の機能分類なら `functional_group` に改名する。
- `listed_in_modules`: real module の listing surface として維持する。
- `listed_in_categories`: documentation-only grouping なら `documentation_groups` に改名する。
- `canonical_home`: canonical implementation origin として維持する。

推奨最終形:

```rust
pub struct CorewordMetadata {
    pub name: String,
    pub functional_group: FunctionalGroup,
    pub canonical_home: CanonicalHome,
    pub listed_in_core: bool,
    pub listed_in_modules: Vec<ModuleName>,
    pub documentation_groups: Vec<DocumentationGroup>,
    // ...
}
```

段階移行では、まず `String` / `Vec<String>` のまま名前だけを明確化してよい。

## 実装チェックリスト

- [ ] `BuiltinSpec.category` が実際に functional group だけを表しているか確認する。
- [ ] `CorewordMetadata.category` を `functional_group` へ改名する。
- [ ] `listed_in_categories` を `documentation_groups` または `documentation_listings` へ改名する。
- [ ] `get_words_by_category` / `get_category_listed_words` の関数名を用途に合わせて改名する。
- [ ] real module listing と documentation group listing の検索関数を分ける。
- [ ] UI 表示用 grouping と言語意味上の canonical home を混ぜない。
- [ ] テスト名・コメントから曖昧な `category` を削除する。

## 残存確認

```bash
rg -n "listed_in_categories|category listed|category listing|get_words_by_category|get_category_listed_words" rust/src docs src
rg -n "\bcategory\b" rust/src/coreword_registry.rs rust/src/builtins/builtin-word-definitions.rs docs
```

## 受け入れ条件

- [ ] `category` が機能分類だけを意味する、または `functional_group` に改名されている。
- [ ] documentation group と module origin が混ざっていない。
- [ ] listing 用メタデータと言語意味メタデータが分離されている。
- [ ] コメントに曖昧な「category」説明が残っていない。

---

# P4: WASM/TypeScript 境界の word info タプルを object payload に置換する

## 現状評価

妥当性: 高い。`Array<[...]>` と `wordData[N]` は実在し、境界 payload の事故要因になる。特に user/core/module word info は object 化の効果が大きい。

## 方針

Rust 側で JS object を構築し、TS 側は明示的 interface で受ける。

```ts
export interface CoreWordInfo {
    name: string;
    hoverSummary: string;
    hoverSyntax: string;
}

export interface CoreWordAliasInfo {
    alias: string;
    canonicalName: string;
    hoverSummary: string;
    hoverSyntax: string;
}

export interface InputHelperWordInfo {
    name: string;
    description: string;
}

export interface ModuleWordInfo {
    name: string;
    description: string | null;
}

export interface UserWordInfo {
    dictionary: string;
    name: string;
    description: string | null;
    protected: boolean;
}

export interface DictionaryDependencyInfo {
    dictionary: string;
    imports: string[];
    dependencies: string[];
}
```

## 実装チェックリスト

- [ ] TS 側に word info object 型を追加する。
- [ ] Rust 側で item array を `push` する構築をやめ、`set_js_prop` などで object を返す。
- [ ] `wordData[0]`, `wordData[1]`, `wordData[2]`, `wordData[3]` を object property 参照に置換する。
- [ ] generated wasm files を再生成する必要があるか確認する。
- [ ] tuple shape コメントを削除する。
- [ ] docs の tuple shape 記述を object shape に更新する。

## 残存確認

```bash
rg -n "wordData\[|Tuple shape|Array<\[" src docs rust/src
```

## 受け入れ条件

- [ ] GUI 側に word info の `wordData[N]` 参照が残っていない。
- [ ] WASM/TS 境界の word info payload が object になっている。
- [ ] tuple shape コメントが残っていない。
- [ ] core/module/user word 表示が壊れていない。

---

# P5: `format!("{:?}")` を WASM/GUI 境界に出さない

## 現状評価

妥当性: 高い。ただしテスト・ログ・cache key など内部用途の `Debug` 表示まで禁止しない。境界 payload だけを対象にする。

## 方針

境界に出る enum には `as_protocol_str()` を実装する。既に `serde` を使う型は `rename_all` / explicit rename を検討する。

```rust
impl ErrorFlowWhen {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorFlowWhen::SafeProjection => "safeProjection",
            ErrorFlowWhen::ExecuteWord => "executeWord",
        }
    }
}
```

## 実装チェックリスト

- [ ] `format!("{:?}")` のうち WASM/GUI payload に流れる箇所を特定する。
- [ ] テスト専用・ログ専用・cache key 専用の `Debug` 表示は別扱いにする。
- [ ] 境界出力用 enum に `as_protocol_str()` を追加する。
- [ ] TS 側の `string` 型を protocol string の union type に寄せる。
- [ ] Debug variant 名に依存するテストを protocol string 検証へ変更する。

## 残存確認

```bash
rg -n "format!\(\"\{:\?\}\"" rust/src
rg -n "kind: string|why: string|when: string|errorCategory\?: string|nilReason\?: string" src/wasm-interpreter-types.ts
```

## 受け入れ条件

- [ ] WASM/GUI 境界に `Debug` 文字列が出ない。
- [ ] protocol string が明示定義されている。
- [ ] enum variant 名を変えても外部 payload が不用意に変わらない。
- [ ] 内部ログ・テスト用の `Debug` 使用は、境界出力ではないことが分かる。

---

# P10: 実行 status の生文字列を構造化する

## 現状評価

妥当性: 中。`status: 'OK' | 'ERROR'` 自体は TS 側では許容できるが、Rust 側で生文字列を何度も書くのは typo に弱い。P4/P5 と同じ境界整理 phase で実施するのがよい。

## 方針

Rust 側は helper / enum 経由で status を設定する。TS 側は discriminated union にする。

```ts
export type ExecuteResult =
    | { status: 'OK'; stack?: Value[]; output?: string; hasMore?: boolean }
    | { status: 'ERROR'; message: string; error?: true; stack?: Value[] };
```

Rust 側の protocol 表示は一箇所に閉じ込める。

```rust
enum WasmStatus {
    Ok,
    Error,
}

impl WasmStatus {
    fn as_protocol_str(&self) -> &'static str {
        match self {
            WasmStatus::Ok => "OK",
            WasmStatus::Error => "ERROR",
        }
    }
}
```

## 実装チェックリスト

- [ ] Rust 側の `"OK"` / `"ERROR"` 直接指定を helper に置換する。
- [ ] TS 側の `ExecuteResult` を discriminated union にする。
- [ ] GUI 側で `status` による narrowing が効く形へ分岐を整理する。
- [ ] `push_json_string` など ExecuteResult 以外の status payload も同じ helper を使うか判断する。

## 残存確認

```bash
rg -n '"OK"|"ERROR"' rust/src src
```

## 受け入れ条件

- [ ] Rust 側で status 生文字列を直接書かない。
- [ ] TS 側で status による型 narrowing が効く。
- [ ] 実行成功・失敗表示が壊れていない。

---

# P1: Coreword contract metadata を単一ソース化する

## 現状評価

妥当性: 高いが、範囲が大きい。`BuiltinSpec`, `CorewordMetadata`, `WordDefinition`, `core_builtin_capabilities`, `apply_contract_overrides` の関係を一気に変えると regressions が出やすい。Phase 4 で、まず監査・テスト固定を行ってから導入する。

## 方針

`CorewordContract` を権威にする。ただし導出可能な値を手入力しない。

```rust
pub struct CorewordContract {
    pub purity: WordPurity,
    pub effects: &'static [Effect],
    pub deterministic: bool,
    pub partiality: Partiality,
    pub nil_policy: NilPolicy,
    pub safety_level: SafetyLevel,
    pub capabilities: Capabilities,
    pub lifecycle: LifecycleStatus,
}

impl CorewordContract {
    pub fn safe_preview(&self) -> bool {
        self.purity == WordPurity::Pure
            && self.effects.is_empty()
            && self.deterministic
            && self.safety_level.allows_preview()
            && self.capabilities.is_empty()
    }
}
```

`safe_preview` は原則導出にする。例外的に手入力する場合は `preview_policy_override` のような名前で、理由コメントと validation test を必須にする。

## 実装チェックリスト

- [ ] `CorewordMetadata` の契約系フィールドを一覧化する。
- [ ] `BuiltinSpec` の契約系フィールドを一覧化する。
- [ ] `core_builtin_capabilities(...)` の現在の mapping をテストで固定する。
- [ ] `apply_contract_overrides(...)` の上書き内容をテストで固定する。
- [ ] 手入力値と導出値を分類する。
- [ ] `CorewordContract` を導入する。
- [ ] `BuiltinSpec` から `contract` を参照できるようにする。
- [ ] `CorewordMetadata` は `contract` から生成する。
- [ ] `WordDefinition.capabilities` は `contract.capabilities` 由来にする。
- [ ] `WordDefinition.stability` は `contract.lifecycle` 由来にする。
- [ ] `apply_contract_overrides` を削除するか、移行用 adapter として期限付きで限定する。
- [ ] AQ-REQ-007 系テストを更新し、安全性・純粋性の整合性テストを維持する。

## 残存確認

```bash
rg -n "apply_contract_overrides|core_builtin_capabilities|safe_preview|deterministic|safety_level" rust/src docs
```

## 受け入れ条件

- [ ] Coreword の安全性・純粋性・能力・ライフサイクルの権威が明確である。
- [ ] 同じ意味の値を複数箇所で手入力していない。
- [ ] `apply_contract_overrides` による暗黙上書きがない、または移行用として明確に限定されている。
- [ ] `safe_preview` が副作用あり word に対して `true` にならない。
- [ ] AQ-REQ-007 系テストが通る。

---

# P7: `Capabilities::PURE` を capability から分離する

## 現状評価

妥当性: 高い。`WordPurity` は既に存在するため、新設ではなく既存の purity 軸を権威にする。`Capabilities::PURE.union(Capabilities::INPUT_HELPER)` のような状態は意味的に危うい。

## 方針

- `Capabilities` は具体的な能力・効果可能性だけを表す。
- pure/effectful/observable は `WordPurity` だけで表す。
- pure な word の capability は原則 `Capabilities::empty()` とする。

## 実装チェックリスト

- [ ] `Capabilities::PURE` の全参照を確認する。
- [ ] `Capabilities::PURE.union(...)` を重点的に確認する。
- [ ] pure 判定を `capabilities.contains(Capabilities::PURE)` から `purity == WordPurity::Pure` に置換する。
- [ ] `Capabilities::PURE` bit を削除する。
- [ ] default capabilities を `Capabilities::empty()` に寄せる。
- [ ] `WordDefinition` / `CorewordMetadata` / module word の purity と capabilities の整合性 validation を追加する。
- [ ] dictionary tier tests を更新する。

## 残存確認

```bash
rg -n "Capabilities::PURE|contains\(Capabilities::PURE\)|PURE\.union" rust/src docs
```

## 受け入れ条件

- [ ] `Capabilities::PURE` が存在しない。
- [ ] pure/effectful/observable は `WordPurity` で表される。
- [ ] capabilities は具体的な能力・副作用可能性だけを表す。
- [ ] `PURE | IO` のような矛盾状態を作れない。

---

# P8: `Stability` を `LifecycleStatus` に改名し、`SafetyLevel` と分離する

## 現状評価

妥当性: 中〜高。現在の `Stability` は成熟度を表しているように見えるが、`SafetyLevel` と併存するため名前の曖昧さが残る。P1 の契約統合時に合わせて実施するのが安全である。

## 方針

- API/機能としての成熟度は `LifecycleStatus`。
- 実行安全性は `SafetyLevel`。
- 片方からもう片方を暗黙に導出しない。

```rust
pub enum LifecycleStatus {
    Stable,
    Experimental,
    Deprecated,
}
```

## 実装チェックリスト

- [ ] `Stability` / `stability` の参照を確認する。
- [ ] 成熟度を表す箇所を `LifecycleStatus` / `lifecycle` に改名する。
- [ ] `safety_level` から lifecycle を暗黙に導出していないか確認する。
- [ ] 両者の関係をコメントで明記する。
- [ ] 必要なら validation test を追加する。
- [ ] docs を更新する。

## 残存確認

```bash
rg -n "\bStability\b|\bstability\b" rust/src docs
```

## 受け入れ条件

- [ ] API 成熟度は lifecycle で表される。
- [ ] 実行安全性は safety level で表される。
- [ ] stable が「安全」という意味で使われていない。
- [ ] safety level と lifecycle の関係が明文化されている。

---

# P6: `description` フィールドの用途を分割する

## 現状評価

妥当性: 中。問題意識は正しいが、全 `description` の一括改名は影響範囲が広い。P0 で module word の機械タグを消し、P4 で境界 payload を object 化した後に実施する。

## 方針

用途が違う境界から順に名前を分ける。

- core builtin hover: `hover_summary`, `hover_syntax`
- module word natural language: `module_description`
- user word persisted text: `user_description`
- lookup 表示: `lookup_summary`
- docs 用文章: `documentation_summary`

## 実装チェックリスト

- [ ] `description` の全参照を用途別に分類する。
- [ ] user word の保存用説明を `user_description` に寄せる。
- [ ] module word の自然文説明を `module_description` に寄せる。
- [ ] builtin hover summary は既存の `hover_summary` / `hover_syntax` と整合させる。
- [ ] LOOKUP 表示用説明がある場合は `lookup_summary` として分離する。
- [ ] UI 側の型名・表示名も用途に合わせる。
- [ ] docs を更新する。

## 残存確認

```bash
rg -n "\bdescription\b" rust/src src docs
```

## 受け入れ条件

- [ ] 機械可読タグが `description` に入らない。
- [ ] builtin / module / user word の説明用途がコード上で区別されている。
- [ ] hover 用、lookup 用、保存用の意味が混ざっていない。

---

# P9: `Value.type` と `displayHint` の責務を明確化する

## 現状評価

妥当性: 中。`Value.type` は構造種別と表示種別を混ぜやすい。変更は GUI 全体に影響するため、P4/P10 の境界整理後に実施する。

## 方針

- `type` を `kind` に改名する。
- `kind` は構造上の値種別だけを表す。
- `displayHint` は表示上の意図だけを表す。
- `string` / `datetime` / `boolean` は実体型なのか display hint なのかを明文化する。

```ts
export type WasmValue =
    | { kind: 'scalar'; value: Fraction; displayHint?: DisplayHint }
    | { kind: 'vector'; value: WasmValue[]; displayHint?: DisplayHint }
    | { kind: 'record'; value: Record<string, WasmValue>; displayHint?: DisplayHint }
    | { kind: 'nil'; reason?: string; displayHint?: 'nil' };
```

## 実装チェックリスト

- [ ] `Value.type` の全参照を確認する。
- [ ] `displayHint` の全参照を確認する。
- [ ] `type` を構造種別として使っている箇所を分類する。
- [ ] `type` を表示種別として使っている箇所を分類する。
- [ ] TS 型を discriminated union にする。
- [ ] Rust → WASM 変換を更新する。
- [ ] GUI renderer を更新する。
- [ ] roundtrip test を更新する。

## 残存確認

```bash
rg -n "displayHint|\.type\b|type: string" rust/src src docs
```

## 受け入れ条件

- [ ] 値の構造種別と表示ヒントが混ざっていない。
- [ ] `displayHint` は表示専用である。
- [ ] `kind` は構造専用である。
- [ ] string/datetime/boolean 表示が壊れていない。

---

# 横断チェック

各 phase 完了後に実行する。

```bash
rg -n "signature_type|signature-" rust/src src docs
rg -n '(Map|Form|Fold):[[:space:]]' rust/src src docs
rg -n '(strip|parse)_signature_prefix|(module_word)_signature_type' rust/src src docs
rg -n "formerly_module|formerlyModule" rust/src src docs
rg -n "listed_in_categories|\bcategory\b" rust/src/coreword_registry.rs rust/src/builtins/builtin-word-definitions.rs docs
rg -n "format!\(\"\{:\?\}\"" rust/src
rg -n "wordData\[|Tuple shape|Array<\[" src docs rust/src
rg -n "Capabilities::PURE" rust/src docs
```

注意: この横断チェックは「0 件でなければ必ず失敗」ではない。テスト・内部ログ・互換 adapter などの許容例がある場合は、許容理由をコメントまたは追跡 issue に残す。

# 標準テストコマンド

変更内容に応じて、少なくとも該当領域のテストを実行する。

```bash
cd rust && cargo test
npm run check
npm test
npm run build
```

WASM 境界や generated files を触った場合は追加で実行する。

```bash
npm run build:wasm
npm run check
npm test
```

# 最終受け入れ条件

- [ ] 自然文フィールドに機械可読タグが埋め込まれていない。
- [ ] `description` をパースして分類・契約・表示制御を得る処理がない。
- [ ] functional group / documentation group / listing surface / canonical home が分離されている。
- [ ] `formerly_module` のような名前と実体がズレたフィールドがない。
- [ ] Coreword の安全性・純粋性・能力・ライフサイクル情報の権威が明確である。
- [ ] WASM/TS 境界で位置依存タプルが増えていない。
- [ ] GUI 側に word info の `wordData[N]` 参照が残っていない。
- [ ] `format!("{:?}")` が外部 payload に使われていない。
- [ ] 有限集合が未検証の `&'static str` / `String` だけで表されていない。
- [ ] `Capabilities::PURE` のような「能力」と「性質」の混在がない。
- [ ] 値の構造種別と表示ヒントが分離されている。
- [ ] 関連テスト・ビルドが通る。
