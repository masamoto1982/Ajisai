# Ajisai Semantic Firewall / 診断付き NIL 移行指示書レビュー（改訂版）

## 1. 判定

元の指示書の中心方針は妥当である。

特に、次の方針は Ajisai の拡張耐性を高めるうえで採用すべきである。

- Rust enum variant 名、`Debug` 表示、GUI 表示文字列を外部仕様にしない。
- WASM / TypeScript / GUI / AI 診断へ渡す値は、明示された protocol string または structured object に限定する。
- NIL の「不在値としての同一性」と「なぜ NIL になったか」を分離する。
- 三層デバッグ診断を NIL の診断メタデータへ接続する。
- 後方互換を前提にせず、旧 `nilReason` / `errorCategory` / `Debug` 由来文字列を外部境界から除去する。

ただし、元の指示書はそのまま実装指示として使うには問題がある。現在のリポジトリ状態に対して過大で、いくつかの命名・境界・テスト方針が曖昧または矛盾しているため、下記の改訂版に置き換える。

## 2. 元指示書の主な問題点

### 2.1 文書構造の問題

- 同じ指示書本文が二重に貼られているため、レビュー・実装時に差分管理しづらい。
- 冒頭の `text` コードフェンスが閉じられておらず、以降の章立てが Markdown として崩れる。
- チェックリスト記法と本文命令が混在しており、「仕様」「実装」「検査」「受け入れ条件」の境界が不明瞭である。

### 2.2 protocol string 方針の矛盾

元指示書は「lower camel case を採用する」としつつ、`shape` の初期値に `code_block`、`capabilities` に `exact_numeric`、`nil_passthrough`、`origin` に `safe_projection` など snake_case を列挙している。

改訂方針:

- 外部 protocol string は **lower camel case に統一**する。
- Rust enum variant 名と protocol string の対応は `as_protocol_str()` だけを canonical source にする。
- 仕様本文では Rust enum variant 名ではなく protocol string を使う。

例:

| 軸 | Rust 内部名例 | protocol string |
| --- | --- | --- |
| shape | `CodeBlock` | `codeBlock` |
| capability | `ExactNumeric` | `exactNumeric` |
| capability | `NilPassthrough` | `nilPassthrough` |
| origin | `SafeProjection` | `safeProjection` |
| origin | `HostEnvironment` | `hostEnvironment` |

### 2.3 NIL reason と error category の混同

現在の仕様書は `NilReason` を内部診断状態として説明し、`SafeCaught` が error category を保持するとしている。一方で、元指示書のテスト例では `absence.reason = safeCaught` と `absence.caughtCategory = divisionByZero` を要求しており、これは良い方向である。

ただし、元指示書は次の点が曖昧である。

- `NilReason` 自体に `DivisionByZero` を残すのか、SAFE 捕捉は常に `SafeCaught(ErrorCategory)` に寄せるのか。
- `literalNil` を protocol string として追加するのか、literal NIL の `reason` は `undefined` / `None` にするのか。
- `absence.reason` と `diagnosis.why` の粒度差をどう保つのか。

改訂方針:

- `absence.reason` は「NIL になった直接理由」を表す。
- `diagnosis.why` は「原因分類」を表す。
- SAFE 捕捉は `reason: "safeCaught"` とし、捕捉した error category は `caughtCategory` に構造化して出す。
- literal NIL は `reason` を持たない。`literalNil` は初期導入しない。

### 2.4 `ValueData::Nil` 直接 match 禁止が強すぎる

内部表現の実装、等価性、シリアライズ、低レベル演算では `ValueData::Nil` を直接扱う必要がある。問題は `ValueData::Nil` の存在そのものではなく、それを外部観測仕様・WASM payload・GUI 判定・AI 診断の canonical source にすることである。

改訂方針:

- **外部境界・ユーザー向け表示・AI 診断・TypeScript 判定では** `Value::is_absent()` と `Value::semantic_kind()` を使う。
- **Rust core 内部の表現処理では** `ValueData::Nil` 直接 match を許可する。
- `rg "ValueData::Nil"` はゼロ件を要求する検査ではなく、外部境界へ漏れていないかを確認するレビュー用検査とする。

### 2.5 `ValueOrigin` は現時点で完全には導出できない

元指示書は `Value::origin(&self) -> ValueOrigin` を要求しているが、現在の `Value` は生成元を一般値に保持していない。全値の origin を正確に返すには、各生成箇所へ origin metadata を伝播する設計が必要になる。

改訂方針:

- Phase 1 では `origin()` は conservative に `Unknown` または NIL の `AbsenceOrigin` から導出できる範囲に限定する。
- 非 NIL 値の origin 完全追跡は Phase 2 以降の別作業に分離する。
- 仕様では「origin axis は存在するが、初期実装で全値の由来を完全追跡するとは限らない」と明記する。

### 2.6 `pub` フィールドが Semantic Firewall を弱める

元指示書は `Value` に `pub absence: Option<AbsenceMetadata>` を追加する案を示しているが、外部境界で意味層メソッド経由を徹底したいなら、直接フィールドアクセスを増やすのは望ましくない。

改訂方針:

- 既存構造との整合上 `Value` フィールドをすぐ private にできない場合でも、外部境界コードは `absence_metadata()` / `nil_reason()` / `nil_diagnosis()` を使う。
- 新規コードでは `Value { data: ValueData::Nil, ... }` の直接生成を避け、NIL コンストラクタを使う。
- 将来の Phase では `Value` フィールドの private 化を検討する。

### 2.7 `nil_reason()` メソッド名の扱い

元指示書は後方互換不要としつつ `Value::nil_reason()` メソッドを残す。これは矛盾ではないが、意図を明確にする必要がある。

改訂方針:

- `nil_reason()` は外部 API 互換ではなく、Rust 内部の convenience accessor としてのみ残してよい。
- WASM / TypeScript payload には `nilReason` を出さない。
- 新規境界コードは `absence_metadata()` を優先する。

### 2.8 Debug 表示禁止の範囲が曖昧

`format!("{:?}", ...)` をすべて禁止すると、内部テストや人間向け debug log まで不必要に制限する。

改訂方針:

- 禁止対象は **observable protocol payload** と **機械判定に使われる文字列** に限定する。
- 内部ログ、panic message、テスト失敗時の補助表示では `Debug` を許可する。
- WASM / API / GUI / AI 診断 payload では `as_protocol_str()` または structured object を必須にする。

### 2.9 検索コマンドがリポジトリ標準に合わない

リポジトリ作業では `grep -R` ではなく `rg` を使う。

改訂方針:

- すべての残存確認コマンドを `rg` に置き換える。

## 3. 改訂後の実装指示書

## 3.1 目的

Ajisai に `Semantic Firewall` を導入し、内部実装と外部観測仕様を分離する。

- 内部 enum variant 名を外部 protocol にしない。
- Rust `Debug` 出力を WASM / TypeScript / GUI / AI 診断の機械可読値にしない。
- NIL を `diagnostic absence value` として扱う。
- NIL の同一性、直接理由、発生経路、回復可能性、三層診断を分離する。
- 後方互換のためだけの旧 `nilReason` / `errorCategory` / Debug 由来文字列は残さない。

## 3.2 仕様変更

`SPECIFICATION.md` に `Semantic Firewall` 章を追加し、次を明記する。

```markdown
## Semantic Firewall

Ajisai separates internal representation from observable semantics.
Internal representation may change freely. Observable semantics must be accessed through semantic axes and protocol fields.

The following are not part of Ajisai's observable semantics:

- Rust enum variant names
- Rust Debug output
- internal value representation
- display strings
- GUI colors
- CSS class names
- dictionary storage layout
- module file layout

Machine-readable consumers must use protocol fields only. Human-readable strings are non-canonical and may change.
```

仕様上の意味軸を次のように定義する。

| Axis | protocol field | 初期 protocol string |
| --- | --- | --- |
| semantic kind | `semanticKind` | `number`, `collection`, `record`, `code`, `process`, `supervisor`, `absence`, `unknown` |
| shape | `shape` | `scalar`, `vector`, `tensor`, `record`, `codeBlock`, `handle`, `absence`, `unknown` |
| capabilities | `capabilities` | `numeric`, `exactNumeric`, `iterable`, `indexable`, `callable`, `stackItem`, `nilPassthrough`, `diagnosable`, `serializable`, `displayable`, `userEditable`, `moduleOwned`, `coreOwned`, `aiExplainable` |
| origin | `origin` | `literal`, `computed`, `coreWord`, `builtinWord`, `moduleWord`, `userWord`, `safeProjection`, `nilPropagation`, `hostEnvironment`, `optimizer`, `unknown` |
| absence metadata | `absence` | structured object |
| diagnosis | `diagnosis` | structured object |
| display | `display` | human-readable only; non-canonical |
| serialization | `serialization` | explicit format contract only |

NIL 仕様は次に置き換える。

```markdown
NIL is a diagnostic absence value.

- `semanticKind(NIL) = "absence"`
- `shape(NIL) = "absence"`
- `capabilities(NIL)` includes `"diagnosable"`
- NIL identity is independent from its reason.
- NIL display text such as `"NIL"` is human-readable and non-canonical.
- `NIL?` only checks whether a value is absent. It must not branch on the absence reason.
```

NIL metadata は次を持つ。

```ts
type ProtocolAbsence = {
  reason?: string;
  origin: string;
  recoverability: string;
  caughtCategory?: string;
  diagnosis?: ProtocolDiagnosis;
};
```

literal NIL の `reason` は当面 `undefined` / absent とする。

## 3.3 Rust core 実装

### 3.3.1 semantic module

`rust/src/semantic/` を追加する。

推奨分割:

- `rust/src/semantic/mod.rs`
- `rust/src/semantic/protocol.rs`
- `rust/src/semantic/value_axes.rs`
- `rust/src/semantic/absence.rs`
- `rust/src/semantic/capability.rs`

`rust/src/lib.rs` または crate root の適切な場所に `semantic` module を追加する。

### 3.3.2 意味層 enum

次の enum を追加する。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticKind {
    Number,
    Collection,
    Record,
    Code,
    Process,
    Supervisor,
    Absence,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueShape {
    Scalar,
    Vector,
    Tensor,
    Record,
    CodeBlock,
    Handle,
    Absence,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    Numeric,
    ExactNumeric,
    Iterable,
    Indexable,
    Callable,
    StackItem,
    NilPassthrough,
    Diagnosable,
    Serializable,
    Displayable,
    UserEditable,
    ModuleOwned,
    CoreOwned,
    AiExplainable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueOrigin {
    Literal,
    Computed,
    CoreWord,
    BuiltinWord,
    ModuleWord { module: Option<String> },
    UserWord,
    SafeProjection,
    NilPropagation,
    HostEnvironment,
    Optimizer,
    Unknown,
}
```

各 enum へ `as_protocol_str()` を実装する。protocol string は lower camel case に統一する。

### 3.3.3 absence metadata

次を追加する。

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbsenceOrigin {
    Literal,
    SafeProjection,
    NilPropagation,
    EmptySequence,
    MissingField,
    InvalidEncoding,
    InvalidLens,
    StackUnderflow,
    IndexOutOfBounds,
    UnknownWord,
    ExecutionFailure,
    HostEnvironment,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recoverability {
    Recoverable,
    Retryable,
    Fatal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsenceMetadata {
    pub reason: Option<NilReason>,
    pub origin: AbsenceOrigin,
    pub recoverability: Recoverability,
    pub diagnosis: Option<DebugDiagnosis>,
}
```

`NilReason::as_protocol_str()` と `ErrorCategory::as_protocol_str()` を実装する。

`NilReason::SafeCaught(Box<ErrorCategory>)` は、外部出力時に次のように構造化する。

```json
{
  "reason": "safeCaught",
  "caughtCategory": "divisionByZero"
}
```

### 3.3.4 Value の移行

`Value` から `nil_reason: Option<NilReason>` を削除し、`absence: Option<AbsenceMetadata>` を追加する。

NIL 生成は次のコンストラクタへ集約する。

```rust
impl Value {
    pub fn nil_literal() -> Self;
    pub fn nil_with_absence(absence: AbsenceMetadata) -> Self;
    pub fn nil_with_reason(
        reason: NilReason,
        origin: AbsenceOrigin,
        recoverability: Recoverability,
    ) -> Self;
    pub fn nil_from_diagnosis(
        reason: NilReason,
        origin: AbsenceOrigin,
        recoverability: Recoverability,
        diagnosis: DebugDiagnosis,
    ) -> Self;
}
```

`Value::nil()` を残す場合は `Value::nil_literal()` の alias とし、外部 protocol には露出しない。

`Value` へ次の accessor を追加する。

```rust
impl Value {
    pub fn semantic_kind(&self) -> SemanticKind;
    pub fn shape_kind(&self) -> ValueShape;
    pub fn capabilities(&self) -> Vec<Capability>;
    pub fn has_capability(&self, capability: Capability) -> bool;
    pub fn origin(&self) -> ValueOrigin;

    pub fn is_absent(&self) -> bool;
    pub fn is_nil(&self) -> bool;
    pub fn absence_metadata(&self) -> Option<&AbsenceMetadata>;
    pub fn nil_reason(&self) -> Option<&NilReason>;
    pub fn nil_diagnosis(&self) -> Option<&DebugDiagnosis>;
}
```

`nil_reason()` は Rust 内部 convenience accessor とし、WASM / TS payload 名には使わない。

## 3.4 DebugDiagnosis の protocol 化

`ErrorPhase`、`ErrorLocusKind`、`CauseClass`、`ErrorCategory` に `as_protocol_str()` を実装する。

必須 protocol string 例:

| 型 | variant | protocol string |
| --- | --- | --- |
| `ErrorPhase` | `ParseStructure` | `parseStructure` |
| `ErrorPhase` | `ResolveWord` | `resolveWord` |
| `ErrorPhase` | `SafeProjection` | `safeProjection` |
| `ErrorLocusKind` | `CoreWord` | `coreWord` |
| `CauseClass` | `TypoOrUnknownName` | `typoOrUnknownName` |
| `ErrorCategory` | `DivisionByZero` | `divisionByZero` |
| `ErrorCategory` | `IndexOutOfBounds` | `indexOutOfBounds` |

`summary` は人間向け文であり、機械判定に使わない。`evidence` は当面 `Vec<String>` のままでよいが、外部 consumer が `evidence` 文字列を parse することは禁止する。

## 3.5 WASM 境界

WASM から外部へ出す診断 payload は次の shape にする。

```ts
type ProtocolDiagnosis = {
  when: string;
  where: {
    kind: string;
    word?: string;
    module?: string;
    dictionary?: string;
  };
  why: string;
  summary: string;
  evidence: string[];
  nextChecks: Array<{
    label: string;
    detail: string;
  }>;
};

type ProtocolValueSemantics = {
  semanticKind: string;
  shape: string;
  capabilities: string[];
  origin: string;
  absence?: ProtocolAbsence;
};
```

`Value` payload には `semantics?: ProtocolValueSemantics` を追加する。

旧フィールドは削除する。

- `nilReason?: string`
- top-level `errorCategory?: string`

error flow trace event で error category が必要な場合は、`diagnosis.why` と、NIL の場合は `semantics.absence.caughtCategory` / `semantics.absence.reason` に寄せる。イベント固有の分類が必要なら `diagnosis` 配下に構造化して追加し、top-level Debug 由来文字列には戻さない。

## 3.6 TypeScript / GUI

`src/wasm-interpreter-types.ts` を更新する。

```ts
export interface ProtocolDiagnosis { ... }
export interface ProtocolAbsence { ... }
export interface ProtocolValueSemantics { ... }

export interface Value {
    type: string;
    value: any | Fraction | Value[];
    displayHint?: 'auto' | 'number' | 'string' | 'boolean' | 'datetime' | 'nil';
    semantics?: ProtocolValueSemantics;
}
```

GUI 判定は次に統一する。

- NIL 判定: `value.semantics?.semanticKind === "absence"`
- NIL 理由表示: `value.semantics?.absence?.reason`
- SAFE 捕捉分類: `value.semantics?.absence?.caughtCategory`
- 三層診断: `value.semantics?.absence?.diagnosis` または event の `diagnosis`

禁止:

- `value.value === "NIL"` による機械判定
- CSS class / 色による意味判定
- Rust Debug 由来の `"DivisionByZero"`、`"SafeCaught"`、`"ExecuteWord"` などへの依存

## 3.7 テスト

### Rust unit tests

追加・更新するテスト:

- `rust/src/semantic/protocol-string-tests.rs`
- `rust/src/semantic/absence-metadata-tests.rs`
- `rust/src/interpreter/nil-reason-tests.rs`
- `rust/src/interpreter/error-flow-trace-tests.rs`
- `rust/src/interpreter/debug-diagnosis.rs` 関連テスト

必須ケース:

- literal NIL
  - `semantic_kind = absence`
  - `shape = absence`
  - `absence.origin = literal`
  - `absence.reason = None`
- division by zero の SAFE projection
  - `absence.reason = safeCaught`
  - `absence.caughtCategory = divisionByZero`
  - `diagnosis.when = safeProjection` または `executeWord`
  - `diagnosis.why = domain`
- unknown word
  - `absence.reason = safeCaught` または直接 NIL 化する経路では `unknownWord`
  - `caughtCategory = unknownWord` が存在する場合は protocol string で検証する
  - `diagnosis.when = resolveWord`
  - `diagnosis.why = typoOrUnknownName`
- NIL passthrough
  - 元の `AbsenceMetadata` が失われない。

### WASM / TypeScript tests

- WASM payload に `nilReason` が存在しない。
- WASM payload に `semantics.semanticKind` が存在する。
- NIL では `semantics.absence` が存在する。
- `diagnosis.when`、`diagnosis.where.kind`、`diagnosis.why` が lower camel case protocol string である。
- TypeScript に旧 `nilReason` 参照がない。

## 3.8 残存確認コマンド

`grep -R` ではなく `rg` を使う。

```bash
rg -n "nil_reason" rust/src
rg -n "nilReason" rust/src src
rg -n "errorCategory" rust/src src
rg -n 'format!\("\{:?' rust/src
rg -n "ValueData::Nil" rust/src
rg -n '"NIL"' src
rg -n 'DivisionByZero|SafeCaught|ExecuteWord|ParseStructure|ResolveWord' src rust/src/wasm-interpreter-state.rs
```

判定基準:

- `nilReason` は外部 payload / TS 型 / GUI 判定からゼロ件にする。
- `errorCategory` は top-level payload からゼロ件にする。構造化された `caughtCategory` は許可する。
- `format!("{:?}", ...)` は WASM / API / GUI / AI 診断 payload でゼロ件にする。
- `ValueData::Nil` は Rust 内部処理では許可するが、外部境界の canonical 判定には使わない。

## 3.9 実装順序

1. `SPECIFICATION.md` に `Semantic Firewall` と診断付き NIL を追加し、旧 `NilReason` 仕様を置き換える。
2. `rust/src/semantic/` を追加し、意味軸 enum と protocol string を実装する。
3. `NilReason` / `ErrorCategory` / `ErrorPhase` / `ErrorLocusKind` / `CauseClass` に `as_protocol_str()` を追加する。
4. `AbsenceMetadata` / `AbsenceOrigin` / `Recoverability` を追加する。
5. `Value.nil_reason` フィールドを `Value.absence` に移行する。
6. NIL 生成を `Value::nil_literal()` / `nil_with_reason()` / `nil_from_diagnosis()` へ集約する。
7. DebugDiagnosis と SAFE projection の結果を `AbsenceMetadata` に接続する。
8. WASM payload を protocol string / structured object に変更する。
9. TypeScript 型と GUI 判定を `semantics` 経由へ変更する。
10. テストを新仕様へ更新する。
11. `rg` 検査で旧 external payload と Debug 由来文字列依存を除去する。
12. `cargo test` と TypeScript 型チェックを通す。

## 3.10 受け入れ条件

- `SPECIFICATION.md` に `Semantic Firewall` と診断付き NIL が記載されている。
- `Value` から `nil_reason: Option<NilReason>` が削除され、`absence: Option<AbsenceMetadata>` が追加されている。
- external payload に `nilReason` がない。
- top-level external payload に `errorCategory` がない。
- `caughtCategory` は `absence` 配下の structured field としてのみ出る。
- protocol string が lower camel case に統一されている。
- `Debug` 出力は WASM / API / GUI / AI 診断 payload に使われていない。
- GUI は NIL を `semanticKind === "absence"` で判定している。
- `summary` / `evidence` / 表示文字列を機械判定に使っていない。
- literal NIL、SAFE projection、unknown word、NIL passthrough のテストが新仕様で通る。
- `cargo test` が通る。
- TypeScript 型チェックが通る。

## 4. 最終コメント

この改修の本質は、`nil_reason` を `absence` に名前変更することではない。

本質は、Ajisai の observable semantics を次のように定義し直すことである。

- 内部表現は自由に変えられる。
- 外部境界は protocol string / structured object だけを見る。
- 人間向け表示と機械可読値を混ぜない。
- NIL の存在、理由、由来、回復可能性、診断を別軸として扱う。

この方針を守れば、将来 `ValueData`、dense tensor、診断分類、GUI 表示、AI 説明が変わっても、ユーザーコードと外部 tool は壊れにくくなる。
