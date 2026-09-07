# 結末空間の全単射：改修指示書（2026-09）

Status: **非正典（`[設計根拠]`）**。この文書は Ajisai の意味論を定義しない。
正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。
本書と正典が矛盾したら正典が勝つ。

対象実装者: Claude（またはそれに準ずるエージェント）
作業ブランチ: 各 Phase ごとに新しいブランチを切る

---

## 0. この文書の読み方（実装者向け・最初に必ず読む）

### 0.1 到達したい主張

**この言語のプログラムが取りうる結末は完全に列挙されており、宣言されたすべての結末に
実プログラムの目撃者があり、宣言の外に出るプログラムは存在しない。**

これを散文の主張ではなく **CI ゲート**にするのが本書の全体である。方向は2つあり、
両方を満たして初めて「レジストリは結末空間そのものである」と言える。

| 方向 | 主張 | 現状 |
| --- | --- | --- |
| 健全性 | 観測されたすべての結末が宣言済みである | `docs/semantics-table.json` に `error:unknown` が **34 セル** |
| 非空虚性 | 宣言されたすべての結末に目撃者がある | 宣言 NIL 理由 8 種のうち表が目撃するのは **3 種**のみ |

この主張が価値を持つ根拠は、Ajisai が**全域言語**であること——ループ語を持たず、
`find_reference_cycle`（`rust/src/interpreter/execute_def.rs:242`）が DEF 時点で
相互再帰を含むすべての循環を拒否するため、入力面が有限であり、全数で言い切れる。
チューリング完全な言語はこの主張を述べることができない。

### 0.2 現状の測定値（本書の出発点。すべて実測）

`docs/semantics-table.json`（1,678 セル、6 ドメイン代表値）を集計した結果:

| 結末 | セル数 |
| --- | --- |
| `error:valueShape` | 678 |
| `error:sourceForm` | 539 |
| `nil:literal` | 217 |
| `value` | 156 |
| `error:typoOrUnknownName` | 45 |
| **`error:unknown`** | **34** |
| `nil:missingField` / `nil:notAvailable` / `nil:invalidEncoding` | 4 / 4 / 1 |

`spec/words.json` が `projection.reason` で宣言する 8 理由の目撃状況:

| 理由 | 目撃 |
| --- | --- |
| `invalidEncoding` / `missingField` / `notAvailable` | あり |
| `divisionByZero` / `domainMiss` / `indexOutOfBounds` / `spaceExhausted` / `undecidable` | **なし** |

**原因はドメイン代表値である。** `scalar` の代表が `1` の 1 点しかないため、
`1 1 DIV` は表に載るが `1 0 DIV` は載らない。現在の表は「型の表」であって
「結末の表」ではない。

### 0.3 全 Phase 共通の禁止事項

- ❌ **`SPECIFICATION.html` を直接編集しない。** 生成物である
  （`npm run specification:generate`）。
- ❌ **既存テストを削除・スキップ・`#[ignore]` しない。** 落ちたら実装を直す。
- ❌ **`rust/src/` に 500 行を超える新規ファイルを作らない**
  （`npm run check:file-size` が落ちる）。
- ❌ **`unsafe` を書かない。**
- ❌ **`docs/dev/` 以外に散文ドキュメントを新設しない。**
- ❌ **結末カテゴリの判定に人間向け `message` を使わない。** 文言は変わりうる。
  安定 ID のみを表に入れる（既存 `classifyOutcome` の規律を保つ）。

**本書では後方互換性の維持を求めない。** 所有者の明示指示により、プロトコルの
破壊的変更・`SCHEMA_VERSION` の引き上げ・既存フィールドの削除・移行記録の省略が
すべて許可されている。「互換のために残す」を理由に死んだ表現を延命しないこと。

### 0.4 停止条件（詰まったら黙って続行せず、ここで止めて報告する）

- Phase 1 の重複 ID 裁定（§1.4 落とし穴 A）で、`divisionByZero` /
  `stackUnderflow` / `indexOutOfBounds` / `unknownWord` のいずれかについて
  「NIL でも ERROR でもありうる」以外の第三の答えが必要になった場合
- Phase 2 で、宣言済みカテゴリに分類できない `Custom` 生成点が見つかった場合
  （新しいカテゴリの追加は語彙の変更であり、所有者判断）
- Phase 3 のドメイン拡張後、セル生成が CI 予算（§3.6）を超えた場合
- 正典（`spec/`）の記述と実装が食い違っていることを発見した場合
  （→ `docs/dev/spec-impl-alignment-methodology.md` の裁定手順に載せる案件）

### 0.5 よく使う検証コマンド

```sh
# Rust（作業ディレクトリは rust/）
cd rust && cargo fmt --check
cd rust && cargo clippy --all-targets -- -D warnings
cd rust && cargo test --all-targets

# CLI をビルド（Node 側ジェネレータが使う）
cargo build --bin ajisai --manifest-path rust/Cargo.toml

# リポジトリ全体のゲート（ルートで）
npm run check:file-size
npm run check:semantic-firewall
npm run check:unreachable-contract
npm run check:docs-dev-drift
npm run semantics:table:check
npm run specification:check
```

### 0.6 用語

| 用語 | 意味 |
| --- | --- |
| 結末（outcome） | 1 プログラムの実行が到達する分類。`value` / `nil:<reason>` / `error:<category>` |
| 三分法 | 値 / NIL（理由付き）/ ERROR（`LANG.FAILURE.TRICHOTOMY`） |
| 目撃者（witness） | ある結末 ID を実際に生成する実行可能なプログラム |
| 健全性 | 観測された結末がすべて宣言済みであること |
| 非空虚性 | 宣言された結末がすべて目撃されること |

---

## Phase 1 — 結末レジストリの単一化

### 1.1 目的

結末の語彙は現在 **4 か所**に分散しており、相互にクロスチェックされていない。

| 場所 | 内容 |
| --- | --- |
| `rust/src/error.rs:6` `NilReason` | 14 variant + プロトコル文字列 |
| `rust/src/error.rs:169` `ErrorCategory` | 17 variant（`Declared(&'static str)` と `Custom` を含む） |
| `rust/src/semantic/absence.rs:5` `AbsenceOrigin` | 理由から `absence_origin_for_reason` で導出される |
| `spec/words.json` | `projection.reason`（8 種）と `errorWhen`（31 種） |

これは本リポジトリが `valid_mask` でも F-1 でも繰り返し潰してきた
**「同じ事実が二か所にあり、クロスチェックされていない」**型そのものである。
正典側に単一の列挙を置き、実装をそこから検査する。

方向は `check:runtime-metadata`（"Runtime Core Word metadata is projected, not
authored"）と同じ——**正典が宣言し、実装が射影する**。逆にしないこと。

### 1.2 触ってよいファイル（ホワイトリスト）

```
新規: spec/outcomes.json
新規: spec/outcomes.schema.json
新規: scripts/check-outcome-registry.mjs
編集: spec/README.md                          （正典ソース表に 1 行）
編集: rust/src/error.rs                       （EmptySequence の削除）
編集: package.json                            （scripts に 1 行）
編集: .github/workflows/test.yml              （ゲート 1 ステップ）
編集: docs/dev/INDEX.md
```

### 1.3 事前に読むファイル

| ファイル | 読む理由 |
| --- | --- |
| `rust/src/error.rs` の `NilReason`（6 行付近）と `ErrorCategory`（169 行付近） | 列挙の現物。doc コメントに各 variant の意味が書かれている |
| `rust/src/semantic/absence.rs` の `AbsenceOrigin` | `absence_origin_for_reason` が唯一の導出であることの確認 |
| `spec/README.md` | 正典ソース表の形式 |
| `scripts/check-word-schema-migration.mjs` | 正典 JSON をスキーマ検査するゲートの既存形 |
| `scripts/check-runtime-metadata-source.mjs` | 「正典が宣言し実装が射影する」ゲートの既存形 |

### 1.4 ⚠️ 落とし穴

#### 落とし穴 A：同じ ID が両レジストリに存在する（**所有者判断が要る**）

`NilReason` と `ErrorCategory` のプロトコル文字列を突き合わせると、
**4 つの ID が両方に現れる**:

```
divisionByZero  stackUnderflow  indexOutOfBounds  unknownWord
```

三分法は「値を作れない演算は NIL、壊れた演算は ERROR」と述べているので、
同じ ID が両側にあること自体は必ずしも矛盾ではない——F-3 で確認したとおり、
同じ条件が文脈によって射影にも失敗にもなりうる。だが**それが意図なのか事故なのかは
現状のどこにも書かれていない**。

`spec/outcomes.json` は各 ID について `manifestsAs` を
`["nil"]` / `["error"]` / `["nil","error"]` のいずれかで**明示的に宣言する**。
4 つを機械的に `["nil","error"]` にしてはならない。1 件ずつ実測で確かめ、
両方が実際に起きる ID だけが両方を宣言する。片方しか起きない ID は片方だけを宣言し、
使われていない側の variant は削除する（後方互換は不要）。

判定に迷う ID が出たら §0.4 の停止条件。

#### 落とし穴 B：`EmptySequence` は死んでいる。互換のためだけに生きている

`rust/src/error.rs:8-20` の doc コメントが自ら述べている:

> No longer produced. ... Retained, unlike the retired `LogicallyUnknown`, because
> it *is* reverse-decoded: `value_persist::decode_value` hard-errors on a reason
> string it does not know, so dropping the variant would make a session snapshot
> taken before this change fail to restore rather than degrade.

**残す唯一の理由が「古いセッションスナップショットの復元」であり、本書では
後方互換が不要とされている。** よって `EmptySequence` を削除する。
`value_persist::decode_value` 側の分岐も同時に消すこと。残したまま
`spec/outcomes.json` に載せると、非空虚性ゲート（Phase 4）が
「目撃者のいない宣言」として必ず落とす。

#### 落とし穴 C：`literal` は理由であって射影理由ではない

`NilReason::Literal`（プロトコル文字列 `literal`）は `NIL` リテラルが積む理由で、
表に 217 セル現れる。しかし `spec/words.json` の `projection.reason` には現れない。
**これは欠陥ではなく層の違いである。** `spec/outcomes.json` は
「全 NIL 理由」を列挙し、そのうち「Word の射影によって到達しうるもの」を
`projectable: true` で区別する。この区別を潰すと、Phase 4 の非空虚性ゲートが
`literal` に射影目撃者を要求して永久に落ちる。

#### 落とし穴 D：`ErrorCategory::Declared(&'static str)` は ID ではない

`Declared` は「失敗した Word の `errorWhen` が宣言する条件名をそのまま
プロトコル綴りにする」ための可変 variant（`rust/src/error.rs:189-193`）。
レジストリに `declared` という ID を作ってはならない。
`spec/outcomes.json` は `errorWhen` の 31 条件を**個別の ID として**列挙し、
`Declared` はそれらへの参照機構として扱う。

### 1.5 手順

#### Step 1.1 — `spec/outcomes.json` を書く

形式（この形をそのまま使う）:

```json
{
  "schemaVersion": 1,
  "nilReasons": [
    {
      "id": "divisionByZero",
      "projectable": true,
      "manifestsAs": ["nil"],
      "documentation": "A divisor equal to zero, or indistinguishable from zero within the comparison budget."
    }
  ],
  "errorCategories": [
    {
      "id": "stackUnderflow",
      "kind": "structural",
      "documentation": "A Word was applied with fewer operands than its declared arity."
    },
    {
      "id": "nonNumeric",
      "kind": "declared",
      "documentation": "..."
    }
  ]
}
```

- `nilReasons` は `NilReason` のプロトコル文字列と 1:1（`EmptySequence` 削除後）
- `errorCategories` の `kind` は `structural`（`ErrorCategory` の固定 variant 由来）
  または `declared`（`spec/words.json` の `errorWhen` 由来）
- 配列順は ID の昇順に固定する。`--check` は文字列一致で比較する

`spec/outcomes.schema.json` を `spec/words.schema.json` に倣って書き、
`spec/README.md` の正典ソース表に 1 行足す（`Defines` 欄は
"The complete outcome space — every NIL reason and every error category"）。

**完了条件**: `spec/outcomes.json` がスキーマに適合する。

#### Step 1.2 — `EmptySequence` を削除する

落とし穴 B のとおり。`NilReason` から variant を消し、
`value_persist::decode_value` の分岐、プロトコル文字列の写像、
`AbsenceOrigin::EmptySequence` を同時に消す。コンパイルエラーが作業リストになる。

**完了条件**: `cd rust && cargo test --all-targets` が緑。

#### Step 1.3 — 射影ゲート `check:outcome-registry`

`scripts/check-outcome-registry.mjs` が以下を検査し、1 件でも破れたら非ゼロ終了:

1. `spec/outcomes.json` の `nilReasons[].id` の集合 ≡ Rust `NilReason` の
   プロトコル文字列の集合（過不足なし）
2. `spec/outcomes.json` の `kind: "structural"` な `errorCategories[].id` の集合
   ≡ Rust `ErrorCategory` の固定 variant のプロトコル文字列の集合
3. `spec/words.json` に現れるすべての `projection.reason` が
   `nilReasons[].id` に存在し、かつ `projectable: true` である
4. `spec/words.json` に現れるすべての `errorWhen` 条件が
   `kind: "declared"` な `errorCategories[].id` に存在する

Rust 側の読み取りは、`error.rs` の `as_protocol_str` の
`Variant => "string"` 対を正規表現で抽出する方式でよい
（`scripts/check-runtime-metadata-source.mjs` と同じ性格の解析）。
抽出が 0 件になったら**黙って緑にせず失敗させる**こと——
実装のリファクタで抽出が壊れたとき、ゲートが空集合同士を比較して
通ってしまうのが、この種のスクリプトの典型的な死に方である。

`package.json` に `"outcome-registry:check": "node scripts/check-outcome-registry.mjs"`
を追加し、`.github/workflows/test.yml` の
"Exhaustive semantics table is in sync"（131 行付近）の**直前**にステップを足す。

**完了条件**: `npm run outcome-registry:check` が緑。

### 1.6 受け入れ条件

- [ ] `spec/outcomes.json` と `spec/outcomes.schema.json` が存在し、`spec/README.md` の表に載っている
- [ ] `NilReason::EmptySequence` と `AbsenceOrigin::EmptySequence` が存在しない
- [ ] 両レジストリに現れる 4 ID それぞれについて `manifestsAs` が実測に基づいて宣言されている
- [ ] `npm run outcome-registry:check` が緑で、CI に登録されている
- [ ] 抽出が空集合になったときゲートが失敗することを、テストまたは手動で確認した
- [ ] `cargo test --all-targets` / `clippy -D warnings` が緑

### 1.7 コミット

```
Declare the outcome space once, in spec/, and project the implementation from it

Every NIL reason and every error category was spelled in four places — NilReason,
ErrorCategory, AbsenceOrigin and words.json's projection.reason/errorWhen — with
nothing comparing them. That is the same shape as the valid_mask defect: one fact
in two representations, cross-checked by nobody.

spec/outcomes.json is now the single normative enumeration and the gate projects
the Rust enums from it. EmptySequence goes: its own doc comment said it is no
longer produced and was retained only so pre-existing session snapshots would
still decode, which is not a reason to keep a reason nothing can witness.
```

---

## Phase 2 — 未宣言の結末を表現不可能にする

### 2.1 目的

`docs/semantics-table.json` の 34 セルの `error:unknown` を 0 にする。
ただし**個別に潰すのではなく、生成機構を断つ**。

因果は確認済みである:

```
impl From<String> for AjisaiError   (rust/src/error.rs:550)
impl From<&str>  for AjisaiError    (rust/src/error.rs:556)
        ↓  どちらも AjisaiError::Custom(String) を作る
AjisaiError::Custom(_) => ErrorCategory::Custom     (rust/src/error.rs:243)
        ↓
ErrorCategory::Custom => CauseClass::Unknown        (debug_diagnosis.rs:294)
        ↓
diagnosis.why == "unknown", nextChecks は「メッセージを直接読め」1 件
```

つまり **`?` 演算子や `.into()` で文字列がエラーになる経路が開いている限り、
未宣言の結末は静かに作られ続ける**。この 2 つの `From` 実装を削除すれば、
コンパイラが**分類すべき全地点の一覧を出す**。

### 2.2 触ってよいファイル（ホワイトリスト）

```
編集: rust/src/error.rs
編集: コンパイルエラーが出たすべての rust/src/ 配下のファイル
編集: rust/src/interpreter/execution_loop.rs      （136 行の Literal 写像）
編集: rust/src/interpreter/debug_diagnosis.rs     （Custom 分岐の削除）
編集: docs/semantics-table.json                   （再生成物）
```

`spec/` は触らない。新しいカテゴリが必要になったら §0.4 の停止条件。

### 2.3 ⚠️ 落とし穴

#### 落とし穴 A：`Custom` の生成点は `error.rs` の中にしか見えない

`grep AjisaiError::Custom(` は `rust/src/error.rs` しか返さない。
**これを「生成点は 4 つだけ」と読んではならない。** 実際の生成は
`From<String>` / `From<&str>` 経由の暗黙変換であり、`?` や `.into()` の形で
散在している（`.into()` / `from(` の出現は非テストコードで 175 箇所、
`Err(format!(...))` 形は 17 箇所ある。すべてが Custom 経路とは限らないが、
**候補の規模はこの桁である**）。

正しい手順は grep ではなく**削除してコンパイラに数えさせる**こと。

#### 落とし穴 B：分類先を `Declared` に逃がしすぎない

`ErrorCategory::Declared(&'static str)` は `spec/words.json` の `errorWhen` が
その条件を宣言している Word の失敗にのみ使う。宣言していない条件名を
`Declared` に渡すと、Phase 1 のゲート 4 が落ちる（レジストリに無い ID になる）。
条件が本当に新しいなら、それは語彙の変更であり所有者判断（§0.4）。

#### 落とし穴 C：`NilReason::Literal` の写像先

`rust/src/interpreter/execution_loop.rs:136` は
`| NilReason::Literal => Some(ErrorCategory::Custom)` を含む。
`Custom` の削除でここが壊れる。**`Literal` は失敗ではない**ので、
エラーカテゴリを与えるのではなく、この写像から外すのが正しい
（写像の戻り値は `Option` なので `None` に落とす）。
安易に別のカテゴリを当てると、NIL リテラルが失敗として診断されるようになる。

#### 落とし穴 D：`error:valueShape` / `error:sourceForm` は `why` であって category ではない

表の `outcome` は `diagnosis.why`（`classifyOutcome`、
`scripts/generate-semantics-table.mjs:127`）から作られる。`why` は
`ErrorCategory` そのものではなく `CauseClass` 由来の綴りである
（`debug_diagnosis.rs:256` 付近）。Phase 4 のゲートはこの対応を経由するので、
**`why` の綴りと `spec/outcomes.json` の ID の対応関係を、
Phase 2 の時点で 1 か所に書いておくこと**。2 つの語彙が暗黙に対応している状態を
残すと、Phase 1 で潰したはずの「同じ事実が二か所」を作り直すことになる。

### 2.4 手順

1. `impl From<String> for AjisaiError` と `impl From<&str> for AjisaiError` を削除する。
2. `cargo build` のエラー一覧を作業リストにし、1 件ずつ宣言済みカテゴリへ分類する。
   分類できないものが出たら止めて報告する（落とし穴 B）。
3. `AjisaiError::Custom(String)` と `ErrorCategory::Custom` を削除する。
   `debug_diagnosis.rs:294` / `:517` の `Custom` 分岐、
   `execution_loop.rs:136` の `Literal` 写像（落とし穴 C）も直す。
4. `npm run semantics:table` で表を再生成し、`error:unknown` が 0 であることを確認する。

**完了条件**: `docs/semantics-table.json` に `unknown` を含む outcome が 1 件も無い。

### 2.5 受け入れ条件

- [ ] `AjisaiError::Custom` と `ErrorCategory::Custom` が存在しない
- [ ] `From<String>` / `From<&str>` for `AjisaiError` が存在しない
- [ ] 再生成した `docs/semantics-table.json` の `error:unknown` が **0 セル**
- [ ] `NIL` リテラルがエラーカテゴリを持たない（落とし穴 C の回帰テスト）
- [ ] `why` の綴りと `spec/outcomes.json` の ID の対応が 1 か所に書かれている
- [ ] `cargo test --all-targets` / `clippy -D warnings` が緑

### 2.6 コミット

```
Make an undeclared error unrepresentable

34 of the exhaustive table's 1,678 cells answered `error:unknown`, and the cause
was not 34 oversights: `From<String> for AjisaiError` turned any string into
Custom, so every `?` on a string-shaped Result minted an outcome outside the
registry silently. Deleting the two conversions makes the compiler enumerate
every site that relied on them, and deleting Custom itself makes the class of
defect impossible rather than merely absent today.
```

---

## Phase 3 — ドメインを型から結末条件へ拡張する

### 3.1 目的

§0.2 のとおり、現在の 6 ドメインは型の代表であって結末の代表ではない。
宣言された結末を実際に到達させるドメイン集合に置き換える。

### 3.2 触ってよいファイル（ホワイトリスト）

```
編集: scripts/generate-semantics-table.mjs
編集: docs/semantics-table.json     （再生成物）
```

`rust/` は触らない。実装のバグを見つけても Phase 3 では直さず報告する。

### 3.3 ⚠️ 落とし穴

#### 落とし穴 A：ドメインは恣意的に増やさない。1 つ 1 つが条件に紐づく

新しいドメイン代表値は「**それが無いと到達できない宣言済み条件がある**」場合にのみ
足す。生成物の `domains` セクションは、各ドメインについて
`motivatedBy`（そのドメインが到達させる条件 ID）を記録すること。
記録できないドメインは足さない。これがドメイン集合を監査可能にする唯一の方法である。

#### 落とし穴 B：セル数は掛け算で増える

アリティ 3 の Word は `|DOMAINS|^3` セルになる。16 ドメインなら 1 語あたり 4,096 セル。
全体の見積もりは **約 20,000 セル**（現行 1,678 の約 12 倍）。
現在の実装は 1 セルにつき CLI プロセスを 1 つ spawn する
（`runCell`、`scripts/generate-semantics-table.mjs:147`）ので、
**逐次のままだと CI 予算を超える**。ワーカープール（並列度 = CPU 数）で実行し、
**結果は必ず元の順序に再構成する**こと。`--check` は文字列一致なので、
順序が実行完了順に漏れた瞬間にゲートが不安定になる。

#### 落とし穴 C：`spaceExhausted` はプロファイル依存である

`spaceExhausted` を目撃させるドメイン（巨大スカラー）は、実行時の
materialization ceiling に依存する。CLI の既定プロファイルは MCP や
playground のそれと**異なりうる**（`tools/mcp-server/README.md` の
"The playground applies a different, looser profile" を参照）。

したがって:

1. 巨大スカラーの値は、CLI の実際の既定 ceiling を**実測してから**決める。
   README の 100001 は MCP プロファイルの数字であり、そのまま流用しない。
2. 生成物に `profile` セクションを足し、**表がどのプロファイルの下で生成されたかを
   記録する**。これを省くと、同じソースが別の結末を持つ環境で表が壊れ、
   原因が追えなくなる。

#### 落とし穴 D：コストの高いドメインを組み合わせない

巨大スカラー × 巨大スカラー × 巨大スカラーの組が生成語（`RANGE` / `FILL`）に
当たると、実行時間とメモリが跳ねる。ceiling が守るので**落ちはしない**が、
CI 時間は食う。§3.6 の予算を超えたら、巨大スカラーだけをアリティ 1 の位置に
限定する規則を入れ、その制限を生成物に明記すること（黙って落とすと
「全数である」という主張が嘘になる）。

#### 落とし穴 E：除外リストは現行の語彙に追随させる

現行の除外は `COLLECT`（variableArity）/ `OR-NIL`（controlArity）/
`KEEP`（modifierNotWord）である。2026-08 の指示書が挙げた `COND` / `VENT` は
現在の `spec/words.json` の状態と一致しない。**除外はハードコードせず、
`stack.inputs` が数値でない語を機械的に落とす現行ロジックを保つ。**

### 3.4 ドメイン集合（設計確定。勝手に変えない）

```js
const DOMAINS = [
  { id: 'scalarOne',       source: '1',                 motivatedBy: [] },
  { id: 'scalarZero',      source: '0',                 motivatedBy: ['divisionByZero'] },
  { id: 'scalarNegative',  source: '1 NEG',             motivatedBy: ['domainMiss'] },
  { id: 'scalarFraction',  source: '1 2 /',             motivatedBy: ['nonInteger'] },
  { id: 'scalarLarge',     source: '<§3.3-C で実測',    motivatedBy: ['indexOutOfBounds', 'spaceExhausted'] },
  { id: 'booleanTrue',     source: 'TRUE',              motivatedBy: [] },
  { id: 'textShort',       source: "'a'",               motivatedBy: [] },
  { id: 'textEmpty',       source: "''",                motivatedBy: [] },
  { id: 'textNumeric',     source: "'12'",              motivatedBy: [] },
  { id: 'vectorPair',      source: '[ 1 2 ]',           motivatedBy: [] },
  { id: 'vectorEmpty',     source: '[ ]',               motivatedBy: [] },
  { id: 'vectorRagged',    source: '[ [ 1 ] [ 2 3 ] ]', motivatedBy: ['shapeMismatch'] },
  { id: 'vectorWithNil',   source: '[ 1 NIL ]',         motivatedBy: [] },
  { id: 'nilLiteral',      source: 'NIL',               motivatedBy: ['literal'] },
  { id: 'codeBlock',       source: '{ 1 }',             motivatedBy: [] },
  { id: 'codeBlockFails',  source: '{ 1 0 / }',         motivatedBy: ['executionFailure'] },
];
```

`motivatedBy` が空のドメインは「型としての代表」であり、既存 6 ドメインの
役割を引き継ぐもの。空であること自体は許されるが、**新規に空のドメインを足さない**
（落とし穴 A）。

`vectorEmpty` と `textEmpty` が表現可能であることは確認済み——
`NilReason::EmptySequence` の doc コメントが
"Both are ordinary values now" と述べている。

`scalarNegative` に `-1` ではなく `1 NEG` を使うのは、トークナイザの符号付き
リテラル解釈に表が依存しないようにするため。実測で `-1` が同じ値を作ると確認できれば
どちらでもよいが、**確認せずに `-1` へ変えないこと**。

### 3.5 手順

1. `DOMAINS` を §3.4 に差し替え、`domains` 出力を `{ id, source, motivatedBy }` に拡張する。
2. `scalarLarge` の値を CLI 既定 ceiling の実測から決める（落とし穴 C）。
3. `runCell` をワーカープール化し、結果を元順序に再構成する（落とし穴 B）。
4. 生成物に `profile`（materialization ceiling 等、表が仮定した実行時プロファイル）を足す。
5. `schemaVersion` を 2 に上げる。
6. `npm run semantics:table` で再生成し、`npm run semantics:table:check` が緑になることを確認する。

### 3.6 受け入れ条件

- [ ] `npm run semantics:table` が **5 分以内**に完走する（CI 予算）
- [ ] `npm run semantics:table:check` が 2 回連続で緑（順序が非決定でないことの確認）
- [ ] 生成物に `profile` セクションがある
- [ ] すべてのドメインに `motivatedBy` があり、新規ドメインで空のものが無い
- [ ] 除外リストが `stack.inputs` から機械的に導出されている（ハードコードでない）
- [ ] `rust/` に変更が無い

### 3.7 コミット

```
Choose domain representatives by outcome, not by type

The exhaustive table witnessed 3 of the 8 declared NIL reasons: `scalar` had one
representative, `1`, so `1 1 DIV` was covered and `1 0 DIV` was not. A table built
from type representatives says what the type system does, not what the language
can answer. Each new representative names the condition it exists to reach, so the
domain set stays auditable rather than merely larger.
```

---

## Phase 4 — 全単射ゲート

### 4.1 目的

Phase 1〜3 の成果を、両方向の CI ゲートに変える。

### 4.2 触ってよいファイル（ホワイトリスト）

```
新規: scripts/check-outcome-bijection.mjs
新規: spec/outcome-witnesses.json
編集: package.json
編集: .github/workflows/test.yml
編集: tests/conformance/index.html      （目撃者ケースの追加）
編集: docs/dev/INDEX.md
```

### 4.3 ⚠️ 落とし穴

#### 落とし穴 A：単一 Word のセルでは目撃できない結末がある

全数表は「1 語 × ドメイン組」しか実行しないので、**複数ステップを要する結末は
原理的に載らない**。少なくとも以下がそれに当たる:

| 結末 | 必要なもの |
| --- | --- |
| `undecidable` | 比較予算を尽くす 2 つの近接した代数的数 |
| `executionFailure` | 失敗するブロックを含む実行 |
| `selfReferentialDefinition` | `[ REC ] 'REC' DEF` |

したがって目撃者は **2 つの源**を持つ: 全数表と、手書きの
`spec/outcome-witnesses.json`。ゲートは「**いずれかに目撃者があること**」を要求する。
**片方だけを見るゲートを書かないこと。**

#### 落とし穴 B：目撃者は「書いてある」ではなく「実行した」でなければならない

`spec/outcome-witnesses.json` のプログラムは、ゲート実行時に**実際に走らせて**
宣言どおりの結末を出すことを確認する。ソース文字列を突き合わせるだけの実装にすると、
語彙が変わったときに目撃者が黙って死ぬ。

#### 落とし穴 C：目撃者のいない宣言は、削除の候補であって例外リストの候補ではない

ゲートが落ちたとき、`spec/outcomes.json` から ID を消すか、目撃者を書くかの
**2 択**である。「目撃困難につき除外」のリストを作らないこと。それを認めた瞬間、
このゲートは何も主張しなくなる。到達不能と判断したら削除する（後方互換は不要）。

#### 落とし穴 D：このゲートは `check:unreachable-contract` が降りた問題を引き受ける

`scripts/check-unreachable-contract.mjs` の doc コメントは、
`errorWhen` の条件文字列については live/dead を判別できないので
**意図的に検査対象から外した**と明記している（Rust のエラー文言は散文であり、
camelCase の識別子と一致しないため grep では区別できない）。

本ゲートはその問題を、名前照合ではなく**実行による目撃**で解く。
`check:unreachable-contract` の doc コメントに、
この範囲が本ゲートへ移ったことを 1 文追記すること
（`npm run check:docs-dev-drift` が名前の実在を検査する点にも注意）。

### 4.4 手順

1. `spec/outcome-witnesses.json` を書く。形式:
   `{ "id": "undecidable", "source": "...", "expect": "nil:undecidable" }`
2. `scripts/check-outcome-bijection.mjs`:
   - **健全性**: `docs/semantics-table.json` の全 `outcome` を分解し、
     `nil:<reason>` の `reason` と `error:<why>` の `why` がすべて
     `spec/outcomes.json` の ID に解決することを検査する（`why` → ID の対応は
     Phase 2 落とし穴 D で 1 か所に置いたものを使う）
   - **非空虚性**: `spec/outcomes.json` の全 ID について、全数表または
     目撃者ファイルに少なくとも 1 件の目撃があることを検査する。
     目撃者ファイル側は実際に CLI で実行して結末を確認する（落とし穴 B）
   - どちらの方向も、破れた ID を**すべて列挙**して非ゼロ終了する
     （最初の 1 件で止めない。作業リストとして使えるようにする）
3. `package.json` に `"outcome-bijection:check"` を足し、CI の
   "Exhaustive semantics table is in sync" の**直後**にステップを足す。
4. 目撃者が必要な結末のうち、conformance に載せるべきものは
   `tests/conformance/index.html` にもケースを足す。

### 4.5 受け入れ条件

- [ ] `npm run outcome-bijection:check` が緑で、CI に登録されている
- [ ] 健全性: 表の全結末が `spec/outcomes.json` に解決する
- [ ] 非空虚性: `spec/outcomes.json` の全 ID に目撃者がある（除外リストは無い）
- [ ] 目撃者ファイルのプログラムはゲート実行時に実際に実行されている
- [ ] `spec/outcomes.json` から ID を 1 つ削って走らせると健全性が落ち、
      目撃者を 1 つ削って走らせると非空虚性が落ちることを確認した
- [ ] `check-unreachable-contract.mjs` の doc コメントに移管の 1 文がある

### 4.6 コミット

```
Gate the outcome registry from both sides

Soundness says no program reaches an outcome the registry does not declare;
non-vacuity says no declaration sits there unreachable. Either half alone is
satisfiable by a registry that lies in the other direction, so both are gated
and neither has an exemption list — an outcome with no witness is deleted, not
excused.

This also picks up the question check-unreachable-contract.mjs explicitly
declined: it could not tell a live errorWhen condition from a dead one by name,
because Rust error prose and camelCase identifiers never match. Witnessing by
execution does not have that problem.
```

---

## 5. 完了時に言えるようになること

4 つの Phase がすべて緑になった時点で、次が**機械検査済みの主張**になる:

> Ajisai の結末空間は `spec/outcomes.json` に完全に列挙されている。
> 宣言されたすべての結末には、それを生成する実行可能なプログラムがある。
> 宣言の外に出るプログラムは存在せず、コンパイラがそれを表現不可能にしている。

この主張は全域言語であることに依存している。停止しないプログラムを書ける言語では、
「結末の全数」がそもそも有限集合にならない。

## 6. 本書がやらないこと

- **仕様側から予測表を生成して実行表と突き合わせる**（Phase C 相当）。
  本書は結末の*レジストリ*を閉じるが、`spec/words.json` の宣言が
  各セルの結末を*正しく予告しているか*は検査しない。これは次の指示書の主題である。
- **観測ダイジェストへのプロファイル束縛**。Phase 3 で表に `profile` を記録するのは
  その準備だが、ダイジェスト側の変更は本書の範囲外。
- **MCP リソースとしての公開**。`spec/outcomes.json` と
  `docs/semantics-table.json` をエージェントに配ることは、
  レジストリが閉じてから行う。
