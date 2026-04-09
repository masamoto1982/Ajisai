# Ajisai 言語仕様書

<!-- SPECIFICATION-INDEX-BEGIN
セクションID体系: domain-subdomain 形式。全セクションを一意に識別する機械可読ラベル。
相互参照は [§SECTION-ID] 形式で統一する。

ARCH-CANONICAL        アーキテクチャ原則（正準仕様）
  ARCH-PARADIGM       パラダイム宣言
  ARCH-TWO-LAYER      二層アーキテクチャ定義
    ARCH-DATA-PLANE   データプレーン（純粋計算層）
    ARCH-SEMANTIC-PLANE セマンティックプレーン（メタデータ層）
    ARCH-LAZY-RESOLUTION 遅延セマンティック解決
  ARCH-DATAFLOW       Fractional Dataflow 意味論
    ARCH-DATAFLOW-BASIC 基本原則
    ARCH-DATAFLOW-NO-INTERMEDIATE 中間状態保存の最小化
    ARCH-DATAFLOW-REMAINDER 残余分数の連鎖
    ARCH-DATAFLOW-CONSERVATION 分数保存則
  ARCH-CONSUMPTION    値消費モード
    ARCH-BIFURCATION  分流（Bifurcation）
    ARCH-CONSUME-VS-BIFURCATE 消費と分流の対比
  ARCH-EXECUTION      実行モデル
  ARCH-TYPE-DISPLAY   型・表示・形状への影響
  ARCH-ERROR          エラーモデル
  ARCH-COMPAT         互換性方針
  ARCH-ACCEPTANCE     受け入れ基準
  ARCH-DOC-POLICY     ドキュメント記述ポリシー
  ARCH-ADOPTION       採用方針

LANG-OVERVIEW         Ajisaiとは
CORE-CHARACTERISTICS  コア特性
  CORE-DESIGN-PRINCIPLES 設計原則
  CORE-VECTOR-ORIENTED Vector指向
  CORE-FRACTION-ARCH  統一分数アーキテクチャ
  CORE-STRUCTURAL-LIMITS 構造的制限
  CORE-DIMENSION-MODEL 次元モデル
  CORE-BROADCAST      ブロードキャスト
  CORE-NO-CHANGE-ERROR 変化なし許容原則
  CORE-NIL            NIL
  CORE-COMMENT        コメント

MODIFIER-TARGET       操作対象モード
MODIFIER-CONSUME      消費モード
MODIFIER-DEFAULT-CONSUME デフォルト消費原則
MODIFIER-SAFE         セーフモード

CONTROL-CODEBLOCK     コードブロック
CONTROL-PIPELINE      パイプライン演算子
CONTROL-NIL-COALESCE  Nil Coalescing演算子
CONTROL-COND          条件分岐

SIG-TYPE              ワードシグネチャ型
  SIG-MAP             Map型（写像）
  SIG-FORM            Form型（構造）
  SIG-FOLD            Fold型（還元）
  SIG-NONE            シグネチャ型を持たないワード

WORD-CLASSIFICATION   組み込みワードの分類原則
WORD-LIST             コア組み込みワード一覧
WORD-IMPORT           モジュール読み込み
WORD-NAMESPACE        名前空間規則
WORD-USER             ユーザーワード
WORD-GUI-DISPLAY      GUI辞書表示

NIL-SAFETY            NIL安全性と三値論理
  NIL-BY-SIGNATURE    シグネチャ型別NIL挙動
  NIL-MAP             Map型のNIL伝播
  NIL-FORM            Form型のNIL処理
  NIL-FOLD-ARITH      Fold型（算術・比較）のNIL伝播
  NIL-FOLD-LOGIC      Fold型（論理）のKleene三値規則
  NIL-COALESCE        Nil Coalescing演算子
  NIL-SAFE-MODE       セーフモードとNIL
  NIL-ERROR-CATEGORY  エラーカテゴリ

DESIGN-PROHIBITIONS   設計上の禁止事項

DEV-PRINCIPLES        開発原則
  DEV-CONSISTENCY     実装とメタ情報の一貫性
  DEV-CODE-QUALITY    ソースコード品質
  DEV-AI-FIRST        AIファースト実装規約
  DEV-NAMING-INDEX    命名インデックス規約
  DEV-NO-BACKWARD-COMPAT 後方互換性の破棄

MODULE-MUSIC          標準ライブラリモジュール: music
MODULE-JSON-IO        標準ライブラリモジュール: json / io
SPECIFICATION-INDEX-END -->

## 0. Ajisaiアーキテクチャ原則（正準仕様） {#ARCH-CANONICAL}

> **目的**: Ajisaiの実装と運用における正準アーキテクチャを定義する。
> 実装は本章を唯一の正準仕様として扱う。

### 0.1 パラダイム宣言（Manifesto） {#ARCH-PARADIGM}

Ajisaiでは、データを「水」にたとえる。
- データは流れる水であり、実体は常に `Fraction` である。
- 演算は水の一部を使い、残りを次の演算に渡す。
- `DisplayHint` は水面の波紋であり、計算の実体ではない。
- `NIL` は泡、`CodeBlock` は水の注ぎ方であり、どちらも分数そのものではない。

この比喩は理解の入口であり、以降の章では実装仕様として厳密に記述する。

**データプレーン/セマンティックプレーン分離の核心:**

| 層 | 対象 | アクセスタイミング |
|---|---|---|
| データプレーン | 純粋な分数（Fraction）のテンソル | 全演算で常時 |
| セマンティックプレーン | 表示ヒント・ドメインメタデータ | PRINT・副作用の発火時のみ（遅延問い合わせ） |

計算エンジンが触れるのはデータプレーンのみ。意味論的情報は物理的に別のメモリ空間で管理され、計算パスには一切介入しない。

### 0.2 二層アーキテクチャの定義 {#ARCH-TWO-LAYER}

#### 0.2.1 データプレーン（純粋計算層） {#ARCH-DATA-PLANE}

実行スタックおよびパイプラインには、**混じり気のない純粋な Fraction の連続配列・テンソル**のみが配置される。

- `Value` 構造体から `display_hint` と `ext` を**完全に排除**する。
- データプレーン上の値は `ValueData` のみで構成される: `Scalar(Fraction)`, `Vector(Vec<...>)`, `Nil`, `CodeBlock(Vec<Token>)`。
- 計算エンジンはメタデータの存在を意識せず、分数テンソルの数学的演算（加減乗除・比較・構造変換）のみを超高速で実行する。
- TPUのシストリックアレイやSIMD命令との親和性を最大化するため、Fraction の連続メモリレイアウトを前提とする。

```rust
// データプレーン上の値: 純粋なデータのみ
pub struct Value {
    pub data: ValueData,
    // display_hint: なし（セマンティックプレーンで管理）
    // ext: なし（セマンティックプレーンで管理）
}

pub enum ValueData {
    Scalar(Fraction),
    Vector(Rc<Vec<Value>>),
    Record { pairs: Rc<Vec<Value>>, index: HashMap<String, usize> },
    Nil,
    CodeBlock(Vec<Token>),
}
```

#### 0.2.2 セマンティックプレーン（意味論・メタデータ層） {#ARCH-SEMANTIC-PLANE}

`DisplayHint`、`ValueExt`（音楽DSLの付加情報等）、およびその他の表示・ドメイン固有メタデータは、
データプレーンとは**物理的に別のメモリ空間**で管理される。

**SemanticRegistry（セマンティック・レジストリ）:**

```rust
pub struct SemanticRegistry {
    /// FlowToken ID → DisplayHint のマッピング
    hints: HashMap<u64, DisplayHint>,
    /// FlowToken ID → モジュール固有メタデータのマッピング
    extensions: HashMap<u64, Box<dyn ValueExt>>,
    /// スタックインデックス → DisplayHint（FlowTokenが未割当の値用）
    stack_hints: Vec<Option<DisplayHint>>,
}
```

- データのフローID（`FlowToken.id`）またはスタックインデックスをキーとして、「ある分数の塊が人間にとってどういう意味を持つのか」を外部からオブザーバーとして追跡する。
- `FlowToken` が分流（bifurcation）した場合、子フローは親のメタデータを継承する。
- メタデータの参照は**遅延評価**で行われる: PRINT, STR, GUI表示など、人間向け出力の瞬間にのみセマンティックプレーンに問い合わせる。

#### 0.2.3 二層間の接点（遅延セマンティック解決） {#ARCH-LAZY-RESOLUTION}

データプレーンとセマンティックプレーンの接触は、以下の**限定された境界点**でのみ発生する|

| 境界点 | 動作 | 例 |
|--------|------|-----|
| 値の生成 | パーサーがリテラルを生成する際、データプレーンに値を、セマンティックプレーンにヒントを同時登録 | `'hello'` → データ: 文字コード配列、セマンティック: `String` ヒント |
| 表示出力 | PRINT/GUI表示時にセマンティックプレーンを参照し、DisplayHint に基づいてフォーマット | `PRINT` → `hints.get(flow_id)` で表示形式を決定 |
| 形式変換 | STR/NUM/BOOL等の変換ワードはセマンティックプレーンのヒントのみを更新 | `STR` → `hints.insert(flow_id, DisplayHint::String)` |
| モジュール副作用 | MUSIC@PLAY等のモジュールワードがextメタデータを参照・更新 | `MUSIC@PLAY` → `extensions.get(flow_id)` |
| 分流（`,,`） | 子フローが親のメタデータを継承 | 親ヒント `String` → 子フローにも `String` を登録 |

**重要原則:** 算術演算（`+`, `-`, `*`, `/`）、比較演算（`=`, `<`）、構造操作（`GET`, `SORT`, `REVERSE`）など、
純粋な計算ワードはセマンティックプレーンに一切アクセスしない。

### 0.3 Fractional Dataflow の意味論 {#ARCH-DATAFLOW}

#### 0.3.1 基本原則 {#ARCH-DATAFLOW-BASIC}

- すべての計算データは `Fraction` を基礎単位とする。
- 実行器は「値のコピー」ではなく「分数フローの移送」を行う。
- 各命令は `入力フロー -> (消費分, 残余フロー)` を返す純粋変換として定義される。

#### 0.3.2 中間状態保存の最小化 {#ARCH-DATAFLOW-NO-INTERMEDIATE}

- 実行パイプラインの原則は **no materialized intermediates**。
- ベクトル演算の各段は、可能な限りその場で入力を消費し、結果のみを下流へ押し出す。
- 実装上どうしても一時領域が必要な場合も、意味論上は「ストリーム継手（join）」として扱い、ユーザー観測可能な中間コレクションを作らない。

#### 0.3.3 残余分数の連鎖 {#ARCH-DATAFLOW-REMAINDER}

- 各値は「総量」を持つ。
- 操作が `c` を消費したら、直後の残余は `r = total - c`。
- 次操作の初期入力は常にこの `r`（未消費分）である。
- `r < 0` になる操作は禁止（過剰消費エラー）。
- パイプライン終端では、仕様上 `r = 0` を目指す（完全消費）。

#### 0.3.4 分数保存則（Conservation of Fraction） {#ARCH-DATAFLOW-CONSERVATION}

任意のパイプライン `P` について|

**単一路の保存則:**

`initial_total = Σ(consumed_i) + final_remainder`

**分流路の保存則（`,,` による分流時）:**

`parent_mass = branch_a_mass + branch_b_mass`

分流によって生じた各枝は独立した保存則チェーンを持ち、それぞれの枝内で上記の単一路保存則が適用される。

- 実行器はこの保存則を破ってはならない。
- デバッグモードでは保存則検査を必須化できる。
- 分流の追跡にはフローID・親子関係・質量比がメタデータとして記録される。

### 0.4 値消費モード（Consumption Mode） {#ARCH-CONSUMPTION}

値の消費モードは以下のとおり定義する。

- **対象選択**: 先頭（または指定位置）のフロー要素を対象化する。
- **消費実行**: 命令は必要量のみを消費する。
- **残余継承**: 未消費分は同一ID系列の後続入力として自動継承する。
- **再生成禁止**: 消費済み分を暗黙に復元・複製してはならない。

#### 0.4.1 分流（Bifurcation） {#ARCH-BIFURCATION}

`,,`（分流モード）は「値のコピー」ではなく、**フロー質量の分流**として定義される。

分流操作は、親フローの質量を2つの子フローに分割する|

```
parent_mass = branch_retained + branch_result
```

- **分流比**: MVP実装では均等分割 `1/2 1/2` とする。
- **分流元ID**: 子フローは親フローIDへの参照を保持し、追跡可能性を維持する。
- **値の共有**: 分流後の2枝は同一の値実体を共有する（`Rc<Vec<Value>>` による共有参照）。質量メタデータのみが分割される。
- **セマンティック継承**: 分流時、子フローは親フローのセマンティックプレーン上のメタデータ（DisplayHint, ValueExt）を自動継承する。
- **保存則の拡張**: 分流を含むパイプラインでは、以下が成立する|
  - 単一路: `initial = consumed + remainder`
  - 分流路: `parent_mass = branch_a_mass + branch_b_mass`

これにより、`,,` は「値を残す」操作ではなく「流れを2方向へ分ける」操作として、Fractional Dataflowの意味論と整合する。

#### 0.4.2 `,` と `,,` の対比 {#ARCH-CONSUME-VS-BIFURCATE}

| 観点 | `,`（消費モード） | `,,`（分流モード） |
|------|-------------------|-------------------|
| フロー動作 | 全質量を下流へ移送 | 質量を2枝に分割 |
| 保存則 | `initial = consumed + remainder` | `parent = branch_a + branch_b` |
| 値の実体 | 消費・変換されて新値を生成 | 値実体は共有、質量メタデータのみ分割 |
| セマンティック | 結果に新ヒントが付与される | 子フローが親のヒントを継承 |
| 用途 | 通常のデータ変換 | 中間結果を保持しつつ計算を続行 |
| フロー動作の要約 | 入力フローを消費・変換して新値を生成 | 入力フローを2枝に分割して両方を保持 |

### 0.5 実行モデル（インタプリタ要件） {#ARCH-EXECUTION}

`rust/src/interpreter` の評価ループは、概念上以下を満たすこと。

1. **入力をフローとして受理**（スタック/Vectorはフロー境界の表現）
2. **各命令で consumed/remainder を計算**（データプレーンのみで完結）
3. **remainder を次命令へ直結**
4. **終端で保存則を検証**

**インタプリタの二層構造:**

```rust
pub struct Interpreter {
    // ── データプレーン ──
    stack: Stack,                              // 純粋な Value（メタデータなし）のスタック
    // ── セマンティックプレーン ──
    semantic_registry: SemanticRegistry,        // メタデータのレジストリ
    // ── フロー追跡 ──
    active_flows: Vec<FlowToken>,
    flow_consumed_log: Vec<(u64, Fraction)>,
    flow_tracking: bool,
    // ── その他（辞書、モジュール状態等） ──
    dictionary: HashMap<String, Arc<WordDefinition>>,
    // ...
}
```

- 演算実行時、インタプリタはデータプレーン（`stack`）のみを操作する。
- セマンティックプレーン（`semantic_registry`）は、PRINT/STR/GUI表示などの境界点でのみ参照・更新される。
- `FlowToken` はデータプレーンとセマンティックプレーンの**橋渡し**として機能する: FlowToken の `id` がセマンティックレジストリのキーとなる。

推奨内部モデル|

- `FlowToken { id, total, remaining, shape }` — `hint` フィールドは廃止（セマンティックプレーンに移動）
- 命令シグネチャ例: `fn exec(op, input: FlowToken) -> (Output, FlowToken)`

> 注: 上記は仕様上の意味モデル。実際のRust型は最適化のため変更可。

### 0.6 型・表示・形状への影響 {#ARCH-TYPE-DISPLAY}

- `DisplayHint` はデータプレーンから排除され、セマンティックプレーンで管理される。表示専用という性質は変わらない。
- `shape` は「格納構造」ではなく「フロー束の論理形状」として解釈する。データプレーン上の `ValueData` のネスト構造から動的に導出される。
- `NIL` は「消費対象外の空イベント」として伝播可能だが、保存則計算には寄与しない。
- `ValueExt` トレイトは維持されるが、`Value` 構造体から分離され、セマンティックプレーンの `extensions` マップで管理される。

### 0.7 エラーモデル {#ARCH-ERROR}

| エラー種別 | 発生条件 |
|---|---|
| `OverConsumption` | 要求消費量が残余を超えた |
| `UnconsumedLeak` | 完全消費が要求されるコンテキストで残余が残った |
| `FlowBreak` | 連鎖IDが途切れ、残余継承が不可能になった |
| `BifurcationViolation` | 分流保存則の違反 |

### 0.8 互換性方針 {#ARCH-COMPAT}

- 後方互換を理由に現行設計を歪めない。
- 互換性維持のための分岐・フラグ・レガシーAPIは禁止。
- 既存テストは破棄・再定義を許可し、新仕様の保存則を合格基準にする。

### 0.9 受け入れ基準（実装着手前の合意ポイント） {#ARCH-ACCEPTANCE}

| ID | 基準 | 検証対象 |
|---|---|---|
| AC-01 | `Value` 構造体から `display_hint` と `ext` が排除されている | [§ARCH-DATA-PLANE] |
| AC-02 | `SemanticRegistry` がインタプリタに導入され、メタデータを管理している | [§ARCH-SEMANTIC-PLANE] |
| AC-03 | すべての主要演算がデータプレーンのみで完結し、セマンティックプレーンにアクセスしない | [§ARCH-LAZY-RESOLUTION] |
| AC-04 | PRINT/STR等の表示ワードが遅延セマンティック解決を行っている | [§ARCH-LAZY-RESOLUTION] |
| AC-05 | すべての主要演算が consumed/remainder を返す設計で記述されている | [§ARCH-DATAFLOW-BASIC] |
| AC-06 | ベクトル処理に「観測可能な中間配列生成」がない | [§ARCH-DATAFLOW-NO-INTERMEDIATE] |
| AC-07 | 保存則違反がテストで検出できる | [§ARCH-DATAFLOW-CONSERVATION] |
| AC-08 | 分流（`,,`）時にセマンティックメタデータが子フローに正しく継承される | [§ARCH-BIFURCATION] |
| AC-09 | README / public/docs が同じ用語体系で統一されている | [§DEV-CONSISTENCY] |

### 0.9.1 ドキュメント記述ポリシー {#ARCH-DOC-POLICY}

本仕様書自体にもAIファースト原則を適用する。

| 原則 | 内容 |
|---|---|
| 現行仕様のみ | 仕様書・READMEは現行仕様のみを記述対象とする。旧モデル・移行経緯・比較表は含めない |
| 比喩の集約 | 抽象的な比喩は [§ARCH-PARADIGM] に集約し、以降の章は実装仕様を記述する |
| セクションID必須 | すべてのセクションに `{#SECTION-ID}` 形式の機械可読アンカーを付与する |
| 相互参照の統一 | セクション参照は `[§SECTION-ID]` 形式で統一する。数値セクション番号のみの参照は禁止 |
| 定義の一意性 | 同一概念の正準定義は1箇所のみ。他箇所からは正準定義への参照で代替する |
| 散文より構造 | 列挙・分類・対比は表またはリスト形式で記述する。散文は文脈説明に限定する |

### 0.10 採用方針（実装決定事項） {#ARCH-ADOPTION}

以下を**採用済みの実装方針**として扱う。以降の実装は本節に従う。

1. **テンソル内部表現の正準化（Flat Buffer + Shape + Stride）**
   - `ValueData::Vector` の木構造は外部表現（言語I/O）として維持してよい。
   - ただし `RESHAPE` / `TRANSPOSE` / ブロードキャスト演算などのテンソル計算は、内部で必ず
     `FlatTensor { data: Vec<Fraction>, shape: Vec<usize>, strides: Vec<usize> }` 相当へ正規化して処理する。
   - 目的はキャッシュ効率向上・SIMD/TPU最適化準備・AIによるコード変換容易性である。

2. **Fractional Dataflow の線形消費最適化フック**
   - `FlowToken` は線形型検証の中間表現として扱う。
   - 演算器に「安全なインプレース更新候補」を判断できるフックを導入する。
   - フックは挙動変更ではなく、`remaining == total` かつエイリアスなしのケースを検出するための内部判定APIとする。
   - 実装: `interpreter/optimization-hooks.rs` の `InPlaceJudgment` enum と `check_in_place_candidate` 関数。
     `interpreter/arithmetic.rs` の二項演算器がフックを呼び出す。結果は現時点では `_in_place_candidates` として保持（将来の最適化パスで利用予定）。

3. **Fraction Small Value Optimization**
   - **演算ホットパスをヒープ非依存に保つ**ことを必須とする。
   - 特にテンソル演算経路では不要な `Value` 再帰クローンを禁止し、`Fraction` 配列の走査を優先する。
   - 実装:
     - `Fraction::is_small()` メソッドを追加（`FractionRepr::Small` 判定）。
     - `apply_unary_flat`: `iter()` → `into_iter()` に変更し、`Big` Fraction の二重保持を排除。
     - `apply_binary_broadcast`: 同一shape fast path を追加。同形状オペランド時に `unravel_index` / `project_broadcast_index` / `ravel_index`（各呼び出しで `Vec<usize>` をヒープ確保）を省略する。

4. **AIファースト実装規約（Ajisai全体に適用開始）**
   - 「人間向けの技巧」よりも「生成AIが局所解析しやすい構造」を優先する。
   - 具体的には以下を推奨する。
     - 単機能ヘルパーを分離し、入出力型を固定する。
     - テンソル処理は `flatten -> shape/stride計算 -> index変換 -> rebuild` の4段に統一する。
     - 再帰より反復を優先し、エラー文言を機械判定しやすい定型にする。
     - 暗黙仕様を避け、演算前提条件（rank, shape, total_size）を明示検証する。


---

## 1. Ajisaiとは {#LANG-OVERVIEW}

Ajisaiは**Vector指向**かつ**Fractional Dataflow**のプログラミング言語である。

- 計算対象は `Fraction` ベースのデータ構造（`Scalar` / `Vector` / `Record`）である。
- 実行は consumed/remainder モデルで進み、各演算は入力フローを消費して残余を次段へ渡す。
- 表示・ドメインメタデータはセマンティックプレーンで管理し、計算パスから分離する。

FORTHから継承したのは**後置記法**と**辞書システム**のみである。データ構造の中心はスタックではなく**Vector**であり、FORTH的なスタック操作（DUP, SWAP, ROT, OVER等）は存在しない。

---

## 2. コア特性 {#CORE-CHARACTERISTICS}

### 2.0 Ajisai設計原則 {#CORE-DESIGN-PRINCIPLES}

以下はAjisaiの設計を貫く基本原則である。各原則の詳細は参照先セクションに記述する。

**変化なし許容原則（[§CORE-NO-CHANGE-ERROR]）**
ワードの実行結果が入力と同一である場合でも、標準モードでは成功として扱う。no-opはエラーではない。

**デフォルト消費原則（[§MODIFIER-DEFAULT-CONSUME]）**
すべてのワードはデフォルトでオペランドを消費する。分流モード `,,` でのみ元の値を残せる。

**組み込みワードの分類原則（[§WORD-CLASSIFICATION]）**
組み込みワードはVector指向の基本計算に必須な最小集合に限定する。ドメイン機能は標準ライブラリモジュールとして提供する。

**データプレーン/セマンティックプレーン分離（[§ARCH-TWO-LAYER]）**
計算エンジンは純粋な分数テンソルのみを扱う。表示・ドメインメタデータはセマンティックプレーンで管理し、計算パスから分離する。

**分数保存則（[§ARCH-DATAFLOW-CONSERVATION]）**
任意のパイプラインにおいて、初期総量 = 消費総量 + 最終残余 が成立する。分流時は親質量 = 枝A質量 + 枝B質量。

**中間状態保存の最小化（[§ARCH-DATAFLOW-NO-INTERMEDIATE]）**
実行パイプラインの原則は no materialized intermediates。ベクトル演算の各段は、可能な限りその場で入力を消費し、結果のみを下流へ押し出す。

**後方互換性の破棄（[§DESIGN-PROHIBITIONS], [§DEV-NO-BACKWARD-COMPAT]）**
Ajisaiはプレリリース段階であり、後方互換性は一切保証しない。非推奨パスや互換レイヤーは導入しない。

### 2.1 Vector指向 {#CORE-VECTOR-ORIENTED}

すべてのデータ構造の中心はVectorである。スタックは「暗黙の0次元Vector」として機能するが、言語の設計思想はスタック操作ではなく、Vectorに対する変換と写像に基づく。

### 2.2 統一分数アーキテクチャ（Unified Fraction Architecture） {#CORE-FRACTION-ARCH}

すべての計算データは内部的に分数（Fraction）として表現される。型システムは存在しない。`NIL` と `CodeBlock` は演算データとは別種の値として扱う。

#### 実装構造（データプレーン / セマンティックプレーン分離）

> **正準定義:** `Value`/`ValueData` の正準定義は [§ARCH-DATA-PLANE] を、`SemanticRegistry` の正準定義は [§ARCH-SEMANTIC-PLANE] を参照。本節ではリテラル表現との対応関係を補足する。

- `ValueData` は再帰的定義であり、`Vector(Vec<Value>)` がValueを含むことで任意深度のネスト構造を自然に表現する。形状（shape）は明示的なフィールドではなく、ネスト構造から動的に計算される。
- 実装はメモリ最適化のために、文字列・トークン・ワード定義などで共有表現（例: 参照カウントスライス）を内部的に採用してよい。ただし言語仕様上の観測可能な意味論は「文字列は分数列として扱える」「Dataの本質はFractionである」を維持しなければならない。
- `CodeBlock` はコードブロック（`{ ... }` / `( ... )`）をスタック上の第一級値として保持するためのバリアントであり、DEFによるワード定義やMAP/FILTER/FOLD/CONDへの引数として使用される。

#### 内部表現

| ユーザーから見える姿 | 内部表現 | 説明 |
|---------------------|---------|------|
| `42` | `Scalar(42/1)` | 整数は分数 |
| `1/3` | `Scalar(1/3)` | 分数リテラル |
| `0.5` | `Scalar(1/2)` | 小数は分数に変換 |
| `TRUE` | `Scalar(1/1)` + Boolean hint | 1が真、0が偽 |
| `'A'` | `Scalar(65/1)` + String hint | 文字コード（Unicode） |
| `'Hello'` | `Vector([Scalar(72/1), ...])` + String hint | 文字コードの配列 |
| `NIL` | `Nil` | 値の不在（独立した表現） |
| `[ ]` | --- | エラー（空ブラケットは禁止） |
| `''` | `Nil` + String hint | 空文字列はNIL（String hintを保持） |

#### Fraction型

```rust
pub struct Fraction {
    pub numerator: BigInt,   // 分子（任意精度）
    pub denominator: BigInt, // 分母（任意精度）
}
```

- 任意精度: `num-bigint` クレートによる無制限精度
- 自動簡約: GCD計算により常に最簡形を維持
- 符号の正規化: 分母は常に正の整数。負の分数は分子で符号を表現する（例: `-1/3`）。リテラル `1/-3` は分数として解釈されない
- サポートする数値形式: 整数（`42`）、小数（`1.5`）、分数（`1/3`）、指数（`1e10`）

#### DisplayHint

`DisplayHint` は表示専用の情報であり、演算には一切使用しない。

```rust
pub enum DisplayHint {
    Auto,      // 自動判定
    Number,    // 数値として表示
    String,    // 文字列として表示
    Boolean,   // 真偽値として表示
    DateTime,  // 日時として表示
    Nil,       // NILとして表示
}
```

重要な原則|

1. 演算は `data` のみを参照する。`display_hint` は無視される
2. 表示時のみ `display_hint` を参照し、フォーマットを決定する
3. 形式変換ワード（STR, NUM, BOOL等）は `display_hint` を変更する。`data` は必要に応じて変換される

### 2.3 構造的制限 {#CORE-STRUCTURAL-LIMITS}

Ajisaiは認知的負荷を制御するために、以下の制限を設けている。

| 制限 | 値 | 説明 |
|------|-----|------|
| ネスト次元 | 制限なし | スタック（暗黙の1次元）と `[]` Vectorネストは実装上の固定上限を設けない |
| 実行ステップ上限 | 100000（既定） | 1回の `execute` 呼び出し内で評価可能なステップ数の上限 |

固定の呼び出し深度制限は採用しない。暴走再帰や無限評価は実行ステップ上限で制御する。

#### 実行ステップガード

名前付きワード（ユーザーワード）の呼び出しチェーンは深さで制限しない。
代わりに、1回の `execute` 実行で評価できる総ステップ数を上限で管理する。

```ajisai
# 深いチェーン: 許可される
{ B } 'A' DEF
{ C } 'B' DEF
{ D } 'C' DEF
{ E } 'D' DEF
{ [ 1 ] } 'E' DEF
A
```

ガードの対象|

- ユーザーワード・組み込みワードを含む実行評価ステップ全体
- 高階関数内の評価も同一カウンタで計測される
- 直接再帰・間接再帰もステップ上限に達すると停止する

実行ステップ上限超過時は `ExecutionLimitExceeded` エラーを返す。

### 2.4 次元モデル {#CORE-DIMENSION-MODEL}

#### 括弧の役割

Ajisaiの括弧は以下のように役割が固定されている。

| 括弧 | 役割 | 用途 |
|------|------|------|
| `[]` | Vector区切り | データ構造の入力・表示 |
| `{}` `()` | コードブロック区切り | DEF、MAP/FILTER/FOLD/CONDの引数 |

入力時に `{}` や `()` でVectorを記述することはできない。`[]` のみがVector区切りである。

#### ネスト次元

GUIのスタックエリアは暗黙的に第1次元に相当する。`[]` によるVectorネストについて、Ajisaiは固定の次元上限を設けない。

深いネストは入力・表示の可読性に影響するため、実利用では適切な粒度に分割することを推奨する。

#### 表示規則

Vectorの表示は常に `[]` を使用する。ネストの深さはGUI上で色によって区別される（9段階の色パレット）。

```ajisai
[ 1 2 3 ]                          # -> [ 1 2 3 ]
[ [ 1 2 ] [ 3 4 ] ]                # -> [ [ 1 2 ] [ 3 4 ] ]
[ [ [ 1 ] [ 2 ] ] [ [ 3 ] [ 4 ] ] ]
                                    # -> [ [ [ 1 ] [ 2 ] ] [ [ 3 ] [ 4 ] ] ]
```

複数のVectorがスタック上にある場合、各Vectorは独立して表示される|

```ajisai
[ 1 2 ] [ 3 4 ]                    # -> [ 1 2 ]  [ 3 4 ]
```

#### 形状（Shape）

各次元のサイズを表すVectorである。

```ajisai
[ 1 2 3 ]                          # 形状: [ 3 ]
[ [ 1 2 3 ] [ 4 5 6 ] ]            # 形状: [ 2 3 ]
```

#### 矩形制約

同一次元内の要素は同じ形状でなければならない。

```ajisai
[ [ 1 2 ] [ 3 4 ] ]                # 正当: 各行が長さ2
[ [ 1 2 ] [ 3 4 5 ] ]              # エラー: 行の長さが不一致
```

### 2.5 ブロードキャスト {#CORE-BROADCAST}

NumPy/APL と同様のブロードキャスト規則を採用する。形状の異なるテンソル間の演算を自動的な形状調整により可能にする。

#### 規則

1. **形状の比較は右から行う**
2. **各次元は以下の場合に互換**: サイズが同じ、またはどちらかが1
3. **足りない次元は左に1を追加して補う**
4. **サイズが1の次元は、必要に応じて拡張される**

```
形状 [2, 3] と [3] の比較|
  [2, 3]
     [3]    <- [1, 3] として扱う
  ------
  [2, 3]   <- 結果の形状
```

#### 具体例

```ajisai
# スカラーとVector
[ 5 ] [ 1 2 3 ] +                  # -> [ 6 7 8 ]

# Vectorと行列（行方向ブロードキャスト）
[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +
                                    # -> [ [ 11 22 33 ] [ 14 25 36 ] ]

# 列Vectorと行列（列方向ブロードキャスト）
[ [ 1 2 3 ] [ 4 5 6 ] ] [ [ 100 ] [ 200 ] ] +
                                    # -> [ [ 101 102 103 ] [ 204 205 206 ] ]
```

#### エラーケース

互換性のない形状ではブロードキャストできずエラーとなる|

```ajisai
[ 1 2 3 ] [ 1 2 ] +                # エラー: [3] と [2] は互換性なし
```

### 2.6 「変化なしは成功」原則 {#CORE-NO-CHANGE-ERROR}

Ajisaiでは、ワードの実行結果が入力と同一である場合でも、標準モードでは成功として扱う。  
「何も変わらない操作（no-op）」は許容され、ランタイムエラーにしない。

```ajisai
[ 1 ] REVERSE                     # -> [ 1 ]   （成功）
[ 1 2 3 ] SORT                    # -> [ 1 2 3 ] （成功）
'hello' STR                       # -> 'hello' （成功）
123 NUM                           # -> 123     （成功）
TRUE BOOL                         # -> TRUE    （成功）
```

この方針により、外部入力を含むパイプラインで「たまたま既に目的状態だった」ケースでも停止せず、堅牢に継続できる。

### 2.7 NIL {#CORE-NIL}

NILはAjisaiにおける「値の不在」を表すセンチネル値である。

#### 内部表現

```
data: ValueData::Nil      （独立した表現）
display_hint: Nil
```

NILは `ValueData::Nil` として独立したバリアントで表現される。`Fraction(0/0)` はNIL判定の互換メソッドとして存在するが、主表現ではない。

#### 空ブラケットの禁止

空ブラケット `[ ]` はエラーとなる。NILが必要な場合は `NIL` キーワードを使用する。

```ajisai
[ ]                                 # エラー: Empty vector is not allowed
NIL                                 # NIL（センチネル値 0/0）
[ 1 NIL 3 ]                        # NILを要素として持つVector
```

#### NILに対する操作

| 操作 | 結果 | 説明 |
|------|------|------|
| `NIL LENGTH` | `[ 0 ]` | 長さ0 |
| `NIL 'WORD' MAP` | `NIL` | NILをMAPしてもNIL |
| `NIL [ 42 ] { + } FOLD` | `[ 42 ]` | 初期値がそのまま返る |
| `NIL => [ 0 ]` | `[ 0 ]` | Nil Coalescingで代替値 |
| `[ 42 ] => [ 0 ]` | `[ 42 ]` | 非NILはそのまま |

#### FILTERとNIL

FILTERで該当要素がない場合はNILを返す。Nil Coalescing演算子 `=>` と組み合わせてデフォルト値を設定できる。

```ajisai
[ 1 2 3 ] { [ 10 ] < NOT } FILTER  # -> NIL（該当なし）
[ 1 2 3 ] { [ 10 ] < NOT } FILTER => [ 0 ]
                                    # -> [ 0 ]（デフォルト値）
```

### 2.8 コメント {#CORE-COMMENT}

`#` 以降は行末までコメントとして扱われる。

```ajisai
[ 1 2 3 ] + # これはコメント
123#数値の直後でもコメントになる   # -> [ 123 ]
1/3#分数の後のコメント             # -> [ 1/3 ]
'#文字列内は保護される'            # -> '#文字列内は保護される'
```

`#` はトークン境界として機能するため、数値やリテラルの読み取りは `#` の直前で停止する。文字列リテラル内の `#` はコメントとして解釈されない。

---

## 3. 操作修飾子 {#MODIFIER}

### 3.1 操作対象モード {#MODIFIER-TARGET}

| 修飾子 | 名称 | 動作 |
|--------|------|------|
| `.` | スタックトップモード | スタック最上位の1要素に対して操作を適用（デフォルト） |
| `..` | スタック全体モード | スタック全体（0次元Vector）に対して操作を適用 |

ワード実行後、自動的にスタックトップモード（`.`）にリセットされる。

### 3.2 消費モード {#MODIFIER-CONSUME}

| 修飾子 | 名称 | 動作 |
|--------|------|------|
| `,` | 消費モード | オペランドをスタックから消費する（デフォルト） |
| `,,` | 分流モード（Bifurcation） | フロー質量を2枝に分割し、元のオペランドと結果の両方をスタックに配置する |

ワード実行後、自動的に消費モード（`,`）にリセットされる。

修飾子は順序非依存である: `.. ,,` と `,, ..` は同じ動作をする。

`,,` は値の「コピー」ではなく、フロー質量の**分流**である。分流後の2枝（元値と結果）は同一の値実体を共有するが、それぞれが親質量の半分の質量を持つ。詳細は [§ARCH-DATAFLOW-BASIC] を参照。

### 3.3 デフォルト消費原則 {#MODIFIER-DEFAULT-CONSUME}

**すべてのワードはデフォルトで対象（オペランド）を消費する。** これはAjisaiにおける消費性の均一原則である。

フロー質量を分流して元の値を残したい場合は、分流モード `,,` で明示的に指定する。ワードごとに消費性が異なるという暗黙の例外は設けない。

```ajisai
# GET: 対象Vectorと引数Vectorを消費し、取得した要素を返す
[ 10 20 30 ] [ 0 ] GET        # -> [ 10 ]
[ 10 20 30 ] [ 0 ] ,, GET     # -> [ 10 20 30 ] [ 0 ] [ 10 ]  （分流モード: 質量が3枝に分割）

# LENGTH: 対象Vectorを消費し、要素数を返す
[ 1 2 3 4 5 ] LENGTH           # -> [ 5 ]
[ 1 2 3 4 5 ] ,, LENGTH       # -> [ 1 2 3 4 5 ] [ 5 ]  （分流モード: 質量が2枝に分割）

# 算術: 両オペランドを消費し、結果を返す
[ 1 2 3 ] [ 10 ] +            # -> [ 11 12 13 ]
[ 1 2 3 ] [ 10 ] ,, +         # -> [ 1 2 3 ] [ 10 ] [ 11 12 13 ]  （分流モード: 質量が3枝に分割）
```

この原則により、プログラマは `,,` の有無だけで消費性を判断できる。ワードごとの暗記は不要になる。

**分流時の質量分配:** `,,` 使用時、元の各オペランドと結果は親フローの質量を均等に分割して受け取る。スタック上の観測可能な値は変わらないが、各値に紐づくフロー質量が分割される。

#### スタック全体モード（`..`）での消費性

`..` モードでは操作対象が「スタック全体」になる。デフォルト消費原則はここでも同様に適用される。

```ajisai
# GET: スタック全体を消費し、取得した要素を返す
a b c [ 1 ] .. GET            # -> [ b ]
a b c [ 1 ] ,, .. GET         # -> a b c [ b ]  （分流モード）

# LENGTH: スタック全体を消費し、要素数を返す
1 2 3 4 5 .. LENGTH            # -> [ 5 ]
1 2 3 4 5 ,, .. LENGTH         # -> 1 2 3 4 5 [ 5 ]  （分流モード）

# REVERSE: スタック全体を消費し、反転結果を再配置する
a b c .. REVERSE               # -> c b a
```

#### 分流モード（`,,`）の範囲

`,,` はワードが消費するすべてのオペランドに対して分流を適用する。対象Vectorのみ、あるいは引数Vectorのみを選択的に分流することはできない。

```ajisai
# TAKE: 対象Vectorと引数Vectorの両方を消費/分流する
[ 1 2 3 4 5 ] [ 3 ] TAKE      # -> [ 1 2 3 ]                      （両方消費）
[ 1 2 3 4 5 ] [ 3 ] ,, TAKE   # -> [ 1 2 3 4 5 ] [ 3 ] [ 1 2 3 ]  （分流: 質量が3枝に分割）

# INSERT: 同上
[ 1 3 ] [ 1 2 ] INSERT         # -> [ 1 2 3 ]                       （両方消費）
[ 1 3 ] [ 1 2 ] ,, INSERT      # -> [ 1 3 ] [ 1 2 ] [ 1 2 3 ]       （分流: 質量が3枝に分割）
```

### 3.4 セーフモード {#MODIFIER-SAFE}

| 修飾子 | 名称 | 動作 |
|--------|------|------|
| `~` | セーフモード | ワード実行中にエラーが発生した場合、エラーを抑制しNILを返す |

セーフモードはワードの実行を「失敗してもよい」ものとして扱う。エラーが発生した場合、スタックはワード実行前の状態に復元され、結果としてNILがプッシュされる。エラーが発生しなかった場合は通常通りの結果を返す。

```ajisai
# 通常: エラーで停止
[ 1 2 3 ] [ 10 ] GET              # エラー: Index 10 out of bounds

# セーフモード: エラー時にNILを返す
[ 1 2 3 ] [ 10 ] ~ GET            # -> NIL

# => と組み合わせてデフォルト値を提供
[ 1 2 3 ] [ 10 ] ~ GET => [ 0 ]   # -> [ 0 ]

# 正常時はそのまま結果を返す
[ 1 2 3 ] [ 1 ] ~ GET             # -> [ 20 ]
```

ワード実行後、自動的にセーフモードは解除される（他の修飾子と同様）。

#### 他の修飾子との組み合わせ

`~` は `.`/`..` および `,`/`,,` と組み合わせ可能である。修飾子は順序非依存。

```ajisai
# セーフ + 分流 + スタック全体
a b c [ 10 ] ~ ,, .. GET          # -> a b c NIL（エラー時）
```

#### 適用対象

`~` はすべての組み込みワードに対して使用可能である。ただし以下のワードには適用しても意味がない（エラーを発生させないため）。

- 定数: `TRUE`, `FALSE`, `NIL`
- 修飾子自身: `.`, `..`, `,`, `,,`, `!`, `==`
- 制御フロー: `:`, `;`（構文要素のため修飾子が適用されない）

#### 設計上の注意

- `~` は「エラーを握りつぶす」のではなく「エラーをNILに変換する」ものである。これはAjisaiのNIL安全性モデル（[§NIL-SAFETY]）と一貫している。
- no-opは標準で成功扱いのため、`~` は主に構造エラー・範囲外アクセス・ゼロ除算などの実エラーに対して使用する。
- `~` を使用するかどうかはプログラマの明示的な判断であり、Ajisaiの「trust the programmer」精神を維持する。

---

## 4. 制御構造 {#CONTROL}

### 4.1 コードブロック {#CONTROL-CODEBLOCK}

`{ コード }` または `( コード )` でコードブロック（遅延評価されるコード）を定義する。DEFでのワード定義やMAP/FILTER/FOLDの引数に使用する。コードブロック内の改行は禁止。

### 4.2 パイプライン演算子 {#CONTROL-PIPELINE}

`==` はデータフローを視覚的に明示するためのno-opマーカーである。

```ajisai
[ 1 2 3 4 5 ]
  == { [ 2 ] * } MAP
  == { [ 5 ] < NOT } FILTER
  == { [ 0 ] + } FOLD
```

### 4.3 Nil Coalescing演算子 {#CONTROL-NIL-COALESCE}

`=>` は値がNILの場合に代替値を返す。

```ajisai
NIL => [ 0 ]       # -> [ 0 ]
[ 42 ] => [ 0 ]    # -> [ 42 ]
```

### 4.4 条件分岐 {#CONTROL-COND}

`COND` は複数のガード・本体ペアを順に評価し、最初にTRUEを返したガードの本体を実行する。

```ajisai
[ 42 ]
  { [ 0 ] < }   { 'negative' }
  { IDLE }      { 'positive' }
  COND
```

- ガードと本体は必ずペアで指定する
- `{ IDLE }` ガードは else 節として扱う
- どのガードも一致せず else もない場合は `CondExhausted` エラー
- `COND` は Form 型として扱う

---

## 5. ワードシグネチャ型 {#SIG-TYPE}

すべてのデータ操作ワードは、以下の3つのシグネチャ型のいずれかに属する。シグネチャ型が操作修飾子（`.`/`..`、`,`/`,,`）の挙動を決定する。個々のワードごとに修飾子の挙動を暗記する必要はない。

```
Map型（写像）    — 各要素に独立して変換を射す
Form型（構造）   — 集合の構造に作用する
Fold型（還元）   — 複数の値を一つに畳み込む
```

### 5.1 Map型（写像） {#SIG-MAP}

個々の値を独立に変換する。入力と出力の要素数は同じ。

| 修飾子 | 動作 |
|--------|------|
| `. ,` | スタックトップを消費し、変換結果をプッシュ |
| `. ,,` | スタックトップの質量を分流し、変換結果を追加 |
| `.. ,` | スタック上の各要素を消費し、各変換結果をプッシュ |
| `.. ,,` | スタック上の各要素の質量を分流し、各変換結果を追加 |

```ajisai
[ 1 ] STR                         # -> '1'
[ 1 ] ,, STR                      # -> [ 1 ] '1'  （質量を分流: 各枝が元の1/2の質量を持つ）
[ 1 ] [ 2 ] [ 3 ] .. STR          # -> '1' '2' '3'
[ 1 ] [ 2 ] [ 3 ] ,, .. STR       # -> [ 1 ] [ 2 ] [ 3 ] '1' '2' '3'  （分流）
```

### 5.2 Form型（構造） {#SIG-FORM}

集合の構造に対して操作する。`..` ではスタック全体を一つの集合として扱う。

| 修飾子 | 動作 |
|--------|------|
| `. ,` | スタックトップの集合を消費し、操作結果をプッシュ |
| `. ,,` | スタックトップの集合の質量を分流し、操作結果を追加 |
| `.. ,` | スタック全体を一つの集合として消費し、操作結果をプッシュ |
| `.. ,,` | スタック全体の質量を分流し、操作結果を追加 |

```ajisai
[ 3 1 2 ] SORT                    # -> [ 1 2 3 ]
[ 3 1 2 ] ,, SORT                 # -> [ 3 1 2 ] [ 1 2 3 ]  （分流: 質量が2枝に分割）
[ 3 ] [ 1 ] [ 2 ] .. SORT         # -> [ 1 ] [ 2 ] [ 3 ]
[ 3 ] [ 1 ] [ 2 ] ,, .. SORT      # -> [ 3 ] [ 1 ] [ 2 ] [ 1 ] [ 2 ] [ 3 ]  （分流）
```

### 5.3 Fold型（還元） {#SIG-FOLD}

2つの値を1つに結合する。`..` ではN個の値を左から順に畳み込む。

| 修飾子 | 動作 |
|--------|------|
| `. ,` | 上位2つのオペランドを消費し、結果をプッシュ |
| `. ,,` | 上位2つのオペランドの質量を分流し、結果を追加 |
| `.. ,` | N個のオペランドを消費し、左から順に畳み込んだ結果をプッシュ |
| `.. ,,` | N個のオペランドの質量を分流し、畳み込み結果を追加 |

```ajisai
[ 3 ] [ 4 ] +                     # -> [ 7 ]
[ 3 ] [ 4 ] ,, +                  # -> [ 3 ] [ 4 ] [ 7 ]  （分流: 質量が3枝に分割）
[ 1 ] [ 2 ] [ 3 ] [ 3 ] .. +      # -> [ 6 ]
[ 1 ] [ 2 ] [ 3 ] [ 3 ] ,, .. +   # -> [ 1 ] [ 2 ] [ 3 ] [ 6 ]  （分流）
```

### 5.4 シグネチャ型を持たないワード {#SIG-NONE}

以下のワードはデータ操作ワードではないため、シグネチャ型を持たない。

| 分類 | ワード |
|------|--------|
| 定数 | `TRUE` `FALSE` `NIL` |
| 生成 | `NOW` `CSPRNG` `DATETIME` `TIMESTAMP` |
| 修飾子 | `.` `..` `,` `,,` `~` `!` `==` |
| 制御フロー | `IDLE` `EXEC` `EVAL` |
| ワード管理 | `DEF` `DEL` `?` |
| 入力支援 | `'` `FRAME` |
| スタック操作 | `COLLECT` |
| メタプログラミング | `EXEC` `EVAL` `HASH` |

#### EXEC

ベクタをコードとして解釈し実行する（Vector Duality）。

```ajisai
# StackTopモード（デフォルト）: スタックトップのベクタをコードとして実行
[ [ 2 ] [ 3 ] * ] EXEC             # -> [ 6 ]

# Stackモード: スタック全体をベクタとみなしてコードとして実行
[ 1 ] [ 1 ] '+' .. EXEC            # -> [ 2 ]
```

- ベクタ内の要素はコードとして再解釈される: 数値はリテラル、文字列はワード名、ネストベクタはベクタリテラル
- `.`/`..` モードに対応

#### EVAL

文字列をパースしてコードとして実行する。

```ajisai
# StackTopモード（デフォルト）: スタックトップの文字列をパースして実行
'[ 2 ] [ 3 ] *' EVAL               # -> [ 6 ]

# Stackモード: スタック全体を文字コード列として結合し、パースして実行
[ 49 ] [ 32 ] [ 50 ] [ 32 ] [ 43 ] .. EVAL  # -> 3（"1 2 +" を実行）
```

- `.`/`..` モードに対応
- ユーザーワードも文字列内で使用可能

#### COLLECT

スタックからN個の要素を収集してベクタを作成する。

```ajisai
1 2 3 3 COLLECT                     # -> [ 1 2 3 ]
[ 1 2 ] [ 3 4 ] 2 COLLECT          # -> [ [ 1 2 ] [ 3 4 ] ]（フラット化しない）
```

- `[ ]` リテラル構文ではスタック上の計算結果を動的にベクタにまとめることができないため、この操作は組み込みワードの組み合わせでは再現できない
- 引数の整数値はベクタに包まず、スカラーとしてスタックに直接置く（`3 COLLECT` であり `[ 3 ] COLLECT` ではない）
- CONCATとの違い: COLLECTは要素をそのまま保持し、CONCATはフラット化して結合する

#### !（強制フラグ）

被依存ユーザーワードのDEL/DEFを許可する。

```ajisai
[ 2 ] * | 'DOUBLE' DEF
DOUBLE DOUBLE | 'QUAD' DEF

# DOUBLEはQUADから参照されているため通常は削除/再定義できない
'DOUBLE' DEL                        # エラー: DOUBLE is referenced by: QUAD

# ! を付けると強制的に削除/再定義が可能（警告メッセージが出力される）
! 'DOUBLE' DEL                      # 成功（Warning出力あり）
! [ 3 ] * | 'DOUBLE' DEF          # 成功（Warning出力あり）
```

- 組み込みワードに対しては `!` があっても削除・上書き不可（BuiltinProtectionエラー）
- `!` はDELまたはDEFの直前でのみ有効。他のワードを実行するとフラグがリセットされる

---

## 6. ワード一覧 {#WORD}

### 6.1 組み込みワードの分類原則 {#WORD-CLASSIFICATION}

Ajisaiの組み込みワードは、**言語コアとして常に利用可能であり、Vector指向の基本計算に必須な最小集合**に限定する。組み込みワードは削除・上書きできない。

以下のドメイン機能はコアから除外し、標準ライブラリモジュールとして提供する。

- 音楽DSL（`SEQ`, `SIM`, `PLAY`, `CHORD`, `SLOT`, `GAIN`, `PAN`, `ADSR` 等）
- JSON/外部連携（`PARSE`, `STRINGIFY`, `INPUT`, `OUTPUT`, `JSON-GET` 等）

#### 辞書の命名体系

| 層 | GUI タブ名 | パス識別子 | 解決優先度 |
|---|---|---|---|
| 組み込みワード | **Core word** | `CORE` | 1（最高・保護対象） |
| モジュールワード | **MUSIC word**, **JSON word** 等 | `MUSIC`, `JSON` 等 | 2 |
| ユーザー定義ワード | **User word** | `USER` | 3 |

#### 短縮名の解決優先順位

| 優先度 | 層 | 例 |
|---|---|---|
| 1（最高） | 組み込みワード | `GET`, `SORT` |
| 2 | モジュールワード | `PLAY`（MUSIC IMPORT後） |
| 3 | ユーザー定義ワード | `SAY-HELLO` |

短縮名が複数の辞書に存在する場合（モジュールとユーザー定義の間の衝突）、
AmbiguousWordエラーを返し、パス記法での一意指定を要求する。
組み込みワードは常に最高優先で解決され、衝突検出の対象外となる。

#### DEF 時の衝突

ユーザーがモジュールサンプルワードと同名のユーザーワードを `DEF` した場合、
定義は許可されるが、衝突の警告が出力される。

```ajisai
'music' IMPORT
999 | 'C4' DEF
# → Warning: 'C4' now exists in both MUSIC@C4 and DEMO@C4.
#   Use a qualified path when calling this word.
```

衝突後、短縮名 `C4` は曖昧となりエラーとなるため、
`MUSIC@C4` または `DEMO@C4` のようにパス記法で指定する必要がある。

#### IMPORT 時の衝突

モジュールのサンプルワードがユーザーワードと同名の場合、
両方が共存し、警告メッセージが出力される。ユーザー定義は自動削除されない。

```ajisai
100 | 'C4' DEF
'music' IMPORT
# → Warning: 'C4' now exists in both MUSIC@C4 and DEMO@C4.
#   Use a qualified path when calling this word.
```

#### コア組み込みワードの保護

コア組み込みワード（基礎語彙）は、モジュールワード・ユーザー定義を問わず
上書き・削除ができない（BuiltinProtection エラー）。

### 6.2 コア組み込みワード一覧 {#WORD-LIST}

以下に機能カテゴリとシグネチャ型の二軸で分類する。

| 機能カテゴリ | ワード | シグネチャ |
|-------------|--------|:----------:|
| 位置操作（0オリジン） | `GET` | Form |
| | `INSERT` | Form |
| | `REPLACE` | Form |
| | `REMOVE` | Form |
| 量操作 | `LENGTH` | Form |
| | `TAKE` | Form |
| Vector操作 | `SPLIT` | Form |
| | `CONCAT` | Form |
| | `REVERSE` | Form |
| | `RANGE` | Form |
| | `REORDER` | Form |
| | `COLLECT` | --- |
| | `SORT` | Form |
| 文字列操作 | `CHARS` | Map |
| | `JOIN` | Map |
| 形式変換 | `NUM` | Map |
| | `STR` | Map |
| | `BOOL` | Map |
| | `CHR` | Map |
| 算術 | `FLOOR` `CEIL` `ROUND` | Map |
| | `+` `-` `*` `/` `MOD` | Fold |
| 比較 | `=` `<` `<=` | Fold |
| 論理 | `NOT` | Map |
| | `AND` `OR` | Fold |
| 高階関数 | `MAP` `FILTER` `FOLD` | Form |
| 条件分岐 | `COND` | Form |
| メタプログラミング | `EXEC` | --- |
| | `EVAL` | --- |
| | `HASH` | --- |
| 生成 | `CSPRNG` | --- |
| 日時 | `NOW` | --- |
| | `DATETIME` | --- |
| | `TIMESTAMP` | --- |
| 形状操作 | `SHAPE` `RANK` | Map |
| | `RESHAPE` `TRANSPOSE` `FILL` | Form |
| 入力支援 | `PRINT` | Map |
| モジュールシステム | `IMPORT` | --- |

**注記:** `>` と `>=` は提供しない。`<` と `<=` のみを使用する（オペランドの順序で代替可能）。

### 6.3 モジュール読み込み（IMPORT） {#WORD-IMPORT}

`IMPORT` は標準ライブラリモジュールを現在の実行コンテキストに読み込むコアワードである。

- 入力: モジュール名文字列（例: `'music'`, `'json'`, `'io'`）
- 出力: なし（スタック効果なし）
- 副作用: モジュールが公開するワード群を辞書に登録する
- 冪等性: 同一モジュールの再IMPORTは成功し、二重登録は行わない
- エラー: 未知モジュール名は `UnknownModule` として扱う

```ajisai
'music' IMPORT
'json' IMPORT
'io' IMPORT
```

### 6.4 名前空間規則 {#WORD-NAMESPACE}

モジュールワードは `MODULE@WORD` 形式の**パス記法**で参照できる。

- 例: `MUSIC@PLAY`, `JSON@PARSE`, `IO@INPUT`
- 目的: コアワードとの衝突回避、およびワードの出自の可視化
- 大文字化規則: ワード名およびパス各セグメントは内部的に大文字へ正規化する

#### 辞書パス体系

| 完全修飾パス | 対象 |
|---|---|
| `DICT@CORE@WORD` | コア組み込みワード |
| `DICT@MODULE@WORD` | モジュールワード（例: `DICT@MUSIC@PLAY`） |
| `DICT@USER@DICTNAME@WORD` | ユーザー定義ワード（例: `DICT@USER@DEMO@SAY-HELLO`） |

省略形（左セグメントから省略可能）|

| 記法 | 意味 |
|---|---|
| `MUSIC@PLAY` | MUSICモジュールのPLAY |
| `DEMO@SAY-HELLO` | DEMO辞書のSAY-HELLO |
| `SAY-HELLO` | 名前衝突がない場合の省略形 |

### 6.5 ユーザーワード {#WORD-USER}

`{ コード } '名前' DEF` でユーザーが定義するワード（ユーザーワード）。ワード名は自動的に大文字に変換される。固定の呼び出し深度制限は適用されず、実行ステップ上限の対象となる。

#### description付き定義

DEFの直前にdescription文字列をスタックに積むことで、ワードの説明文を付与できる。

```ajisai
{ コード } '名前' 'description' DEF
```

descriptionはGUIの辞書ホバー表示および `?` ワードで参照される。

AIファーストなコードでは、descriptionの記述を強く推奨する。
シグネチャ型・責務・入出力を記述することで、AIによるコード解析のコンテキストが向上する。

推奨フォーマット:

```ajisai
{ [ 2 ] * } 'DOUBLE' 'Map型: 各要素を2倍にする。入力: Numeric Vector, 出力: Numeric Vector' DEF
{ [ 2 ] MOD [ 0 ] = } 'IS-EVEN' 'Map型: 偶数判定。入力: Scalar, 出力: Boolean' DEF
```

### 6.6 GUI辞書表示 {#WORD-GUI-DISPLAY}

GUIの辞書エリアでは、以下を別セクションで表示する。

- コア組み込みワード
- IMPORT済みモジュールワード
- ユーザーワード

**表示順序:**
- 記号ワード（先頭文字がアルファベットでないもの）を先に、記号の文字コード順で表示
- アルファベットワードを後に、アルファベット昇順で表示

各ワードボタンにはホバー時にdescription（組み込み/モジュールワード）またはワード定義（ユーザーワード）が表示される。

#### 6.6.1 ワードボタンの色エンコーディング {#WORD-GUI-COLOR}

ワードボタンは**2つの独立した色チャンネル**で情報を伝える。

**チャンネル1: 文字色・枠線色（ワードの性質）**

| 分類 | 文字色・枠線色 | 意味 |
|------|---------------|------|
| コア組み込みワード（基礎語彙） | `--color-core` | コアとして常時利用可能・上書き不可 |
| モジュールワード | `--color-module` | IMPORTにより追加される |
| 被依存ユーザーワード | `--color-dependency` | 他のユーザーワードから参照されている |
| 非依存ユーザーワード | `--color-non-dependency` | 他のユーザーワードから参照されていない |

**チャンネル2: 背景色（シグネチャ型）**

| シグネチャ型 | CSS変数名 | 意味 |
|-------------|-----------|------|
| Map型（写像） | `--color-signature-map` | 各要素に独立して変換を射す |
| Form型（構造） | `--color-signature-form` | 集合の構造に作用する |
| Fold型（還元） | `--color-signature-fold` | 複数の値を一つに畳み込む |
| None（型なし） | なし（白背景を維持） | シグネチャ型を持たないワード |

#### 6.6.2 各コアワードのシグネチャ型分類 {#WORD-GUI-SIGNATURE-MAP}

```
Map型:  CHARS, JOIN, NUM, STR, BOOL, CHR, FLOOR, CEIL, ROUND, NOT, SHAPE, RANK, PRINT
Form型: GET, INSERT, REPLACE, REMOVE, LENGTH, TAKE, SPLIT, CONCAT, REVERSE, RANGE,
        REORDER, SORT, MAP, FILTER, FOLD, COND, RESHAPE, TRANSPOSE, FILL
Fold型: +, -, *, /, MOD, =, <, <=, AND, OR
None:   上記以外の全コアワード（COLLECT, IDLE, EXEC, EVAL, HASH,
        CSPRNG, NOW, DATETIME, TIMESTAMP, IMPORT 等）
```

---

## 7. NIL安全性と三値論理 {#NIL-SAFETY}

Ajisaiは Kleene の強三値論理に基づくNIL伝播モデルを採用する。基本原則は**「結果が論理的に確定できるならNILを吸収し、できないなら伝播する」**である。

### 7.1 シグネチャ型別のNIL挙動 {#NIL-BY-SIGNATURE}

| シグネチャ型 | NIL挙動 | 根拠 |
|---|---|---|
| Map型 | NIL伝播 | 不明な値の変換は不明 |
| Form型 | NIL = 空集合 | 値の不在 = 空の集合 |
| Fold型（算術・比較） | NIL伝播 | 不明との演算は不明 |
| Fold型（論理） | Kleene三値規則 | 確定可能なら吸収 |

### 7.2 Map型のNIL伝播 {#NIL-MAP}

不明な値に変換を射しても不明である。

```ajisai
NIL STR                            # -> NIL
NIL FLOOR                          # -> NIL
NIL CHARS                          # -> NIL
NIL NOT                            # -> NIL
```

### 7.3 Form型のNIL処理 {#NIL-FORM}

NILは「値の不在」であり、空の集合として扱う。

```ajisai
NIL LENGTH                         # -> [ 0 ]
NIL SORT                           # -> NIL
NIL REVERSE                        # -> NIL
NIL 'WORD' MAP                     # -> NIL
NIL 'WORD' FILTER                  # -> NIL
NIL [ 42 ] + | FOLD              # -> [ 42 ]（初期値がそのまま返る）
```

### 7.4 Fold型（算術・比較）のNIL伝播 {#NIL-FOLD-ARITH}

不明な値との演算結果は不明である。

```ajisai
NIL [ 1 ] +                       # -> NIL
NIL [ 1 ] *                       # -> NIL
NIL [ 1 ] =                       # -> NIL
NIL [ 1 ] <                       # -> NIL
```

### 7.5 Fold型（論理）のKleene三値規則 {#NIL-FOLD-LOGIC}

論理演算では、一方のオペランドだけで結果が確定する場合、NILは吸収される。

#### AND 真理値表

| 左 \ 右 | TRUE | FALSE | NIL |
|---------|------|-------|-----|
| **TRUE** | TRUE | FALSE | NIL |
| **FALSE** | FALSE | FALSE | FALSE |
| **NIL** | NIL | FALSE | NIL |

#### OR 真理値表

| 左 \ 右 | TRUE | FALSE | NIL |
|---------|------|-------|-----|
| **TRUE** | TRUE | TRUE | TRUE |
| **FALSE** | TRUE | FALSE | NIL |
| **NIL** | TRUE | NIL | NIL |

```ajisai
FALSE NIL AND                      # -> FALSE  （FALSE AND x = 常にFALSE）
TRUE NIL AND                       # -> NIL    （不確定）
TRUE NIL OR                        # -> TRUE   （TRUE OR x = 常にTRUE）
FALSE NIL OR                       # -> NIL    （不確定）
NIL NOT                            # -> NIL    （不明の否定は不明）
```

### 7.6 Nil Coalescing演算子 {#NIL-COALESCE}

`=>` はNILに対する明示的な代替値を提供する。パイプラインの末端でNILを安全に処理するための標準的な手段である。

```ajisai
NIL => [ 0 ]                      # -> [ 0 ]（NILの場合は代替値）
[ 42 ] => [ 0 ]                   # -> [ 42 ]（非NILはそのまま）

# パイプラインでの使用例
[ 1 2 3 ]
  == [ 10 ] < NOT | FILTER
  == => [ 0 ]                      # FILTERがNILを返した場合の安全策
```

### 7.7 セーフモードとNIL {#NIL-SAFE-MODE}

セーフモード修飾子 `~` （[§MODIFIER-SAFE]）は、エラーをNILに変換する。これにより、エラーが発生しうる操作をNIL安全パイプラインに組み込むことができる。

```ajisai
# パイプラインでの安全なGET
[ 1 2 3 ]
  == [ 10 ] ~ GET          # インデックス超過 → NIL
  == => [ 0 ]              # NILをデフォルト値で補完
# -> [ 0 ]

# MAP内での安全な操作
[ 1 0 2 ] [ 10 ] SWAP ~ / | MAP
# 0での除算 → そのイテレーションの結果がNILになる
# -> [ 10 NIL 5 ]

# FILTERと組み合わせた堅牢なパイプライン
[ 'hello' 42 'world' ]
  == ~ NUM | MAP          # 数値変換できない要素はNIL
  == NIL = NOT | FILTER   # NILを除外
  == => [ 0 ]               # 全部NILだった場合の安全策
```

セーフモードは「エラーの可能性を認識した上で意図的にNILとして処理する」ための仕組みである。無条件にすべてのワードに `~` を付けるような使い方は推奨しない。

### 7.8 エラーカテゴリ {#NIL-ERROR-CATEGORY}

Ajisaiの実行時エラーは以下のカテゴリに分類される。セーフモード修飾子 `~` はすべてのカテゴリのエラーをNILに変換する。

| カテゴリ | 説明 | 例 |
|---------|------|-----|
| StackUnderflow | スタックに十分な要素がない | `+`（スタックが空） |
| StructureError | 期待される構造と実際の構造が不一致 | `[ 'hello' ] [ 1 ] +` |
| UnknownWord | 辞書に存在しないワード | `UNKNOWNWORD` |
| DivisionByZero | ゼロ除算 | `[ 1 ] [ 0 ] /` |
| IndexOutOfBounds | インデックスが範囲外 | `[ 1 2 3 ] [ 10 ] GET` |
| LengthMismatch | ベクタ長の不一致（ブロードキャスト不可） | `[ 1 2 3 ] [ 1 2 ] +` |
| ExecutionLimitExceeded | 実行ステップ上限超過 | 直接再帰・無限ループ相当の実行 |
| ModeUnsupported | ワードが対応しないモードの使用 | `'NAME' .. DEF` |
| BuiltinProtection | 組み込みワードの削除・上書き | `[ [ 1 ] ] 'GET' DEF` |
| CondExhausted | CONDで全ガードが不一致かつelse節なし | `[ 42 ] { [ 0 ] < } { 'neg' } COND` |

エラーメッセージは人間が読める形式で、エラーカテゴリ、期待された状態、実際の状態、および可能な場合はエラー発生箇所を含む。

---

## 8. 設計上の禁止事項 {#DESIGN-PROHIBITIONS}

以下はAjisaiの設計に反するため、仕様として明確に禁止する|

| ID | 禁止事項 | 理由・代替手段 |
|---|---|---|
| PROHIBIT-01 | スタック操作ワード（DUP, SWAP, ROT, OVER等）の導入 | `REORDER` や `.. GET` 等のVector操作で表現する |
| PROHIBIT-02 | 実行ステップ上限を無効化した無制限実行の許容 | [§CORE-STRUCTURAL-LIMITS] で定義された上限 |
| PROHIBIT-03 | 型システムの導入 | すべての値は分数であり、型チェックは存在しない |
| PROHIBIT-04 | 後方互換性の維持 | フォールバック・非推奨パス・互換シムを導入しない |

---

## 9. 開発原則 {#DEV-PRINCIPLES}

### 9.1 実装とメタ情報の一貫性 {#DEV-CONSISTENCY}

本仕様書（SPECIFICATION.md）、README.md、リファレンスページ（language-reference-playground.html および public/docs/ 配下のHTMLファイル）、およびソースコード実装は、常に互いに整合していなければならない。いずれかに乖離が見つかった場合、速やかに修正する。

修正の方向は固定ではない|

- **仕様→実装**: 仕様の意図が正しく、実装が追いついていない場合
- **実装→仕様**: 実装上の発見や制約により、仕様側を更新すべき場合

重要なのは「仕様が常に正」ではなく、**実装とメタ情報（仕様・README・リファレンス）が乖離しないこと**である。

### 9.2 ソースコード品質 {#DEV-CODE-QUALITY}

#### コメント

ソースコード内のコメントは極力省く。コードが自明であることを優先し、コメントが必要な箇所はコード自体の改善を検討する。やむを得ず記述する場合は「なぜ」を書き、「何を」は書かない。

#### デッドコードの禁止

使われていないコード（デッドコード）は即座に削除する。`#[allow(dead_code)]` や `#[allow(unused_variables)]` 等のコンパイラ警告抑制は、デッドコードの温存手段であり使用しない。未使用のパラメータ、到達不能なコードパス、呼び出し元のない関数は、将来の使用可能性に関わらず削除する。

#### 不要な間接層の禁止

メソッド呼び出しをそのまま委譲するだけのラッパー関数は作成しない。`val.is_vector()` で済む処理を `is_vector_value(val)` のような関数で包むことは、コードの可読性を下げる不要な間接層である。

### 9.3 AIファースト実装規約 {#DEV-AI-FIRST}

「人間向けの技巧」よりも「生成AIが局所解析しやすい構造」を優先する。

#### テンソル処理の4段構成

テンソル演算（算術・論理・単項演算を含む）は以下の4段パイプラインに統一する|

1. **Flatten**: `FlatTensor::from_value()` でネスト構造を `Vec<Fraction>` + shape + strides に正規化
2. **Shape/Stride計算**: `compute_strides()` / `broadcast_shape()` でブロードキャスト形状を決定
3. **Index変換**: `unravel_index()` / `project_broadcast_index()` / `ravel_index()` で要素ごとに演算
4. **Rebuild**: `FlatTensor::to_value()` / `build_nested_value()` でネスト構造を復元

この4段構成により、SIMD/TPU最適化の挿入点が明確になり、AIによるコード変換が容易になる。

#### 再帰より反復

要素ごとの演算（element-wise operations）には再帰的な木構造走査を使わず、フラットバッファ上の反復処理を使用する。

- `apply_binary_broadcast()`: 二項演算のブロードキャスト付きフラット反復
- `apply_unary_flat()`: 単項演算のフラット反復

再帰が許容される場面: `build_nested_value()` のようなデータ構築、`value_to_code()` のようなシリアライズ。

#### エラー文言の機械判定対応

エラーメッセージは定型パターンに統一し、機械的なパース・分類を可能にする|

- `"Tensor shape/data mismatch: data_len={}, required={}, shape={:?}"`
- `"Cannot broadcast shapes {:?} and {:?}"`
- `"RESHAPE failed: data length {} doesn't match shape {:?} (requires {})"`

#### 前提条件の明示検証

演算開始前に rank / shape / total_size 等の前提条件を明示的に検証し、暗黙の仕様依存を排除する。

#### コーディングスタイル原則 {#DEV-AI-FIRST-STYLE}

コード自体の構造を「生成AIが局所解析しやすい形」に統一する。関数型的なイディオム（イテレータチェーンの過剰結合、クロージャのネスト、暗黙的な型推論への依存）を排除し、命令的・段階的な記述を基本とする。

| ID | 原則 | 禁止パターン | 推奨パターン |
|---|---|---|---|
| STYLE-01 | 命令的・段階的な記述 | イテレータチェーンの過剰な結合 | 中間変数による段階分離 |
| STYLE-02 | 型アノテーションの明示 | 推論依存の暗黙的束縛 | ローカル変数への型注釈 |
| STYLE-03 | クロージャの単機能化 | 複合処理クロージャ | 名前付き関数への分離 |
| STYLE-04 | ガード節による早期リターン | 深いネスト構造 | 前提条件を先頭で除去 |
| STYLE-05 | matchの網羅的記述 | ワイルドカード `_` の乱用 | 全バリアントの明示 |
| STYLE-06 | エラー文言の定型化 | 自由形式のエラーメッセージ | `WORD: expected X, got Y` 形式 |

**STYLE-01: 命令的・段階的な記述（Explicit Stages）**

3段以上のイテレータチェーンを禁止する。各段を独立した中間変数に束縛し、入出力型を明示する。

```rust
// 禁止: イテレータチェーンで全処理を一行に圧縮
let result = values.iter().filter(|v| !v.is_nil()).map(|v| v.as_scalar().unwrap()).fold(Fraction::zero(), |acc, f| acc + f);

// 推奨: 各段を独立した束縛として明示
let non_nil_values: Vec<&Value> = values.iter().filter(|v| !v.is_nil()).collect();
let scalars: Vec<Fraction> = non_nil_values.iter().map(|v| v.as_scalar().unwrap()).collect();
let result: Fraction = scalars.iter().fold(Fraction::zero(), |acc, f| acc.clone() + f.clone());
```

**STYLE-02: 型アノテーションの明示（Explicit Types）**

`collect()`・`unwrap()`・関数呼び出しの返り値を束縛するローカル変数には型注釈を付与する。Rustが確実に推論できる自明な場合（`let i: usize = 0` 等）は不要。

```rust
// 禁止
let mut result = Vec::new();
let x = compute_something();

// 推奨
let mut result: Vec<Fraction> = Vec::new();
let x: FlatTensor = compute_something();
```

**STYLE-03: クロージャの単機能化（Single-Purpose Closures）**

クロージャ本体が5行を超える場合は名前付き関数に分離する。クロージャは単一の変換のみを担う。

```rust
// 禁止: クロージャ内に複数の処理が混在
values.iter().map(|v| {
    let s = v.as_scalar().unwrap_or_default();
    let normalized = s.normalize();
    if normalized.is_zero() { Fraction::one() } else { normalized }
})

// 推奨: 名前付き関数に分離
fn normalize_or_one(v: &Value) -> Fraction {
    let s = v.as_scalar().unwrap_or_default();
    let normalized = s.normalize();
    if normalized.is_zero() { Fraction::one() } else { normalized }
}
values.iter().map(normalize_or_one)
```

**STYLE-04: ガード節による早期リターン（Guard Clauses First）**

関数冒頭でガード節（早期リターン）により前提条件を除去し、本処理のインデント深度を浅く保つ。

```rust
// 禁止: 深いネスト
fn op_some_word(interp: &mut Interpreter) -> Result<()> {
    if let Some(val) = interp.stack.pop() {
        if val.is_vector() {
            // ... 本処理 ...
        } else { Err(AjisaiError::from("...")) }
    } else { Err(AjisaiError::StackUnderflow) }
}

// 推奨: ガード節で早期リターン
fn op_some_word(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    if !val.is_vector() { return Err(AjisaiError::from("..."))| }
    // 本処理（インデント深度が浅い）
    Ok(())
}
```

**STYLE-05: matchの網羅的記述（Exhaustive Match）**

`match` 式でワイルドカード `_` を使わず、全バリアントを明示する。新バリアント追加時にコンパイルエラーで変更箇所を検出可能にする。

**STYLE-06: エラー文言の定型化（Structured Error Messages）**

エラーメッセージは以下の定型パターンに統一する|

- `"WORD_NAME: expected TYPE, got ACTUAL_TYPE"`
- `"WORD_NAME: stack underflow, requires N arguments"`
- `"TENSOR: shape mismatch: expected SHAPE, got SHAPE"`

### 9.4 AIファーストファイルサイズ制約 {#DEV-AI-FIRST-FILE-SIZE}

すべての Rust ソースファイル（`rust/src/**/*.rs`）は **500行以下** に収める。これはAIコーディングエージェントのコンテキストウィンドウ効率を最大化するための制約である。

#### 規則

- 1ファイルあたりの上限: **500行**（空行・コメント含む）
- 上限を超えた場合は、テストの分離またはロジックの分割により対処する
- テスト分離: `#[cfg(test)] mod tests { ... }` を別ファイル（例: `hash-tests.rs`）に移動し、親モジュールの `mod.rs` で `#[cfg(test)] #[path = "hash-tests.rs"] mod hash_tests;` として登録する
- ロジック分割: 関連する操作群を別ファイルに分離し、`pub(crate)` で必要なシンボルを公開する
- `mod.rs` ファイルは `pub mod` 宣言と `pub use` 再エクスポートのみを含み、実装コードを置かない

#### 目標範囲

- **300〜500行**: 1ファイルの推奨範囲
- **500行超**: 分割が必要

### 9.5 命名インデックス規約（Naming-as-Index Convention） {#DEV-NAMING-INDEX}

Ajisaiの関数名・メソッド名は、自然な英語表現ではなく、**機械的に検索・分類・推論可能な構造化ラベル**として設計する。名前は軽量メタデータ（インデックス）として機能し、AIによる探索・横断編集・自動推論を第一級の設計要件とする。

#### 9.5.1 基本文法テンプレート {#DEV-NAMING-TEMPLATE}

関数名は原則として以下のテンプレートに従う。

| テンプレート | 用途 | 例 |
|---|---|---|
| `action_object` | 基本操作 | `parse_token`, `format_value` |
| `action_object_in_context` | 文脈依存操作 | `lookup_hint_in_registry`, `register_word_in_dictionary` |
| `action_source_to_target` | 変換 | `serialize_value_to_json`, `format_token_to_source` |
| `build_product_from_input` | 複数入力からの構築 | `build_bracket_structure_from_shape` |
| `create_object_from_input` | 単体生成 | `create_value_from_integer_vector` |
| `is_property_object` | 真偽値判定 | `is_nil_value`, `is_scalar_value` |
| `has_property_object` | 保有判定 | `has_display_hint` |

#### 9.5.2 接続語の意味固定 {#DEV-NAMING-CONNECTORS}

| 接続語 | 意味 | 例 |
|---|---|---|
| `_to_` | 変換先 | `serialize_value_to_json` |
| `_from_` | 生成元・構築元 | `build_tensor_from_value` |
| `_in_` | 文脈依存先 | `lookup_hint_in_registry` |

これらの意味は固定であり、他の用途に流用しない。

#### 9.5.3 制御語彙（動詞） {#DEV-NAMING-VERBS}

以下を中核動詞として優先的に用いる。

| 動詞 | 用途 |
|---|---|
| `parse` | 文字列・トークン列からの構造解析 |
| `resolve` | 文脈に基づく名前解決・曖昧性の解消 |
| `check` / `validate` | 条件検査・前提条件の検証 |
| `normalize` | 正規化（分数の既約化、シンボルの大文字化等） |
| `build` | 複数入力からの複合構造の構築 |
| `create` | 単体のオブジェクト生成 |
| `collect` | 複数要素の収集・集約 |
| `extract` | 構造からの部分取得 |
| `lookup` | 辞書・レジストリからの参照 |
| `register` | 辞書・レジストリへの登録 |
| `emit` | 出力の生成・送出 |
| `serialize` / `deserialize` | 外部形式への変換・復元 |
| `format` | 表示用文字列への整形 |
| `render` | UI要素の描画・表示 |
| `update` | 既存状態の変更 |
| `remove` | 要素の削除 |
| `apply` | 演算の適用 |
| `execute` | コードの実行 |
| `compare` | 比較 |
| `compute` | 数値計算・導出 |

新語の追加は例外扱いとし、既存語彙で表現可能な場合は追加しない。

#### 9.5.4 同義語の氾濫禁止 {#DEV-NAMING-NO-SYNONYMS}

同一種の処理に複数の動詞を混在させない。

**禁止パターン:**

- `make` / `build` / `create` / `generate` / `produce` の無秩序な併用
- `get` / `fetch` / `retrieve` / `obtain` の無秩序な併用

**統一規則:**

| 意味 | 採用語 | 非採用語 |
|---|---|---|
| 複合構築 | `build` | `make`, `generate`, `produce` |
| 単体生成 | `create` | `make`, `generate`, `produce` |
| 構造からの取得 | `extract` | `get`（Rust慣用`get`メソッドを除く） |
| 辞書参照 | `lookup` | `get`, `fetch`, `retrieve` |
| 複数収集 | `collect` | `gather`, `fetch` |

#### 9.5.5 曖昧語の禁止 {#DEV-NAMING-NO-AMBIGUOUS}

以下の語を関数名に使用しない。

`do`, `handle`, `process`, `fix`, `manage`, `thing`, `stuff`, `util`, `helper`, `materialize`, `cook`, `settle`, `temp`, `quick`, `better`

`handle` は `resolve` に、`process` は `apply` / `execute` / `render` に置き換える。

#### 9.5.6 略語の共有辞書 {#DEV-NAMING-ABBREVIATIONS}

プロジェクト内で定着した略語のみ許可する。同一概念に複数の略語を使用しない。

| 正式名 | 許可略語 | 禁止略語 |
|---|---|---|
| operation | `op` | `oper` |
| configuration | `config` | `cfg`, `conf` |
| element | `elem` | `el` |

#### 9.5.7 真偽値関数の命名 {#DEV-NAMING-BOOLEAN}

真偽値を返す関数は `is_` または `has_` で始める。

- `is_nil_value`, `is_scalar_value`, `is_string_like`
- `has_display_hint`, `has_definition`

`check_` は副作用を伴う検査、または `Result` を返す検証に使用する。純粋な真偽値判定には `is_` / `has_` を使う。

#### 9.5.8 Rust / TypeScript 間の概念語彙統一 {#DEV-NAMING-CROSS-LANG}

Rust と TypeScript の概念語彙列を一致させる。casing の差のみ許容する。

| Rust | TypeScript |
|---|---|
| `collect_user_words` | `collectUserWords` |
| `serialize_value_to_json` | `serializeValueToJson` |
| `render_execution_result` | `renderExecutionResult` |
| `lookup_hint_in_registry` | `lookupHintInRegistry` |

語彙の不一致（例: Rust `build_*` / TypeScript `make*`）は解消する。

#### 9.5.9 Rust 固有の例外 {#DEV-NAMING-RUST-EXCEPTIONS}

以下の Rust 慣用パターンは命名規約の例外として維持する。

- **トレイト実装メソッド**: `fmt`, `from`, `clone`, `eq`, `cmp`, `partial_cmp`, `default`（言語仕様による制約）
- **`new()` コンストラクタ**: Rust の標準的なコンストラクタパターンとして許容
- **`from_*()` コンストラクタ**: `Value::from_int()`, `Fraction::from_str()` 等（Rust の `From` トレイト慣用に準拠）
- **`as_*()` 変換**: `as_scalar()`, `as_i64()` 等（Rust の軽量参照変換慣用に準拠）
- **`len()`, `is_empty()`**: Rust の標準コレクション慣用

これらの例外は、Rust エコシステムとの整合性維持のために認められる。

#### 9.5.10 `op_*` プレフィックス {#DEV-NAMING-OP-PREFIX}

Ajisai言語ワードの実装関数は `op_` プレフィックスを維持する。これは言語コアの演算実装を機械的に識別するための名前空間である。

- `op_add`, `op_sort`, `op_map`, `op_play`

#### 9.5.11 doc comment との役割分離 {#DEV-NAMING-DOC-SEPARATION}

関数名は機械向けインデックスとして設計する。自然言語による詳細説明は doc comment に記述する。

```rust
/// Converts a nested Value tree into a flat fraction buffer with shape metadata.
/// Used as the first stage of the 4-stage tensor processing pipeline.
pub fn build_flat_tensor_from_value(value: &Value) -> FlatTensor { ... }
```

#### 9.5.12 良い例 / 悪い例 {#DEV-NAMING-EXAMPLES}

**良い例:**

| 関数名 | 理由 |
|---|---|
| `parse_token_from_string` | action_object_from_source |
| `serialize_value_to_json` | action_source_to_target |
| `lookup_hint_in_registry` | action_object_in_context |
| `build_bracket_structure_from_shape` | build_product_from_input |
| `is_nil_value` | is_property_object |
| `collect_builtin_definitions` | action_object |
| `apply_binary_arithmetic` | action_object |
| `render_execution_result` | action_object（UI） |

**悪い例:**

| 関数名 | 問題 | 改善案 |
|---|---|---|
| `handleFlow` | 曖昧語 `handle` | `resolve_flow` |
| `processTensor` | 曖昧語 `process` | `apply_tensor_op` |
| `makeThings` | 曖昧語 `make` + `things` | 具体的な名前に |
| `doSomething` | 曖昧語 `do` | 具体的な動詞に |
| `generateBrackets` | 非採用語 `generate` | `build_brackets` |
| `getThing` | 曖昧な `get` + `thing` | `extract_*` / `lookup_*` |

### 9.6 後方互換性の破棄 {#DEV-NO-BACKWARD-COMPAT}

Ajisaiはプレリリース段階にあり、後方互換性は一切保証しない。より良い設計が見つかった場合、既存の動作を躊躇なく破壊する。非推奨（deprecated）パスや互換レイヤーは導入しない。

---

## 10. 標準ライブラリモジュール: music {#MODULE-MUSIC}

`music` は音楽DSL機能を提供する標準ライブラリモジュールであり、コア言語には含まれない。

### 10.1 読み込み

```ajisai
'music' IMPORT
```

このIMPORT以後、`MUSIC@` 名前空間で音楽ワードが利用可能になる。

### 10.2 公開ワード

- 再生制御: `MUSIC@SEQ`, `MUSIC@SIM`, `MUSIC@PLAY`, `MUSIC@CHORD`
- スロット制御: `MUSIC@SLOT`
- エフェクト: `MUSIC@GAIN`, `MUSIC@GAIN-RESET`, `MUSIC@PAN`, `MUSIC@PAN-RESET`, `MUSIC@FX-RESET`, `MUSIC@ADSR`
- 波形: `MUSIC@SINE`, `MUSIC@SQUARE`, `MUSIC@SAW`, `MUSIC@TRI`

### 10.3 分数の音楽的解釈

| 値 | 解釈 | 例 |
|-----|------|-----|
| `n`（整数） | nHz を 1スロット再生 | `440` → 440Hz, 1スロット |
| `n/d`（分数） | nHz を dスロット再生 | `440/2` → 440Hz, 2スロット |
| `0/d` | dスロット休符 | `0/2` → 2スロット休符 |
| `NIL` | 1スロット休符 | `NIL` → 休符 |
| 文字列 | 歌詞（出力のみ、時間消費なし） | `'Hello'` |

### 10.4 使用例

```ajisai
'music' IMPORT
[ 440 550 660 ] MUSIC@SEQ MUSIC@PLAY
```

---

## 11. 標準ライブラリモジュール: json / io {#MODULE-JSON-IO}

JSON変換およびGUI I/O連携はコア機能ではなく、`json` と `io` モジュールとして提供する。

### 11.1 読み込み

```ajisai
'json' IMPORT
'io' IMPORT
```

### 11.2 json モジュール

#### 公開ワード

- `JSON@PARSE`（Map）
- `JSON@STRINGIFY`（Map）
- `JSON@GET`（Form）
- `JSON@KEYS`（Form）
- `JSON@SET`（Form）

#### JSON ↔ Ajisai データ型マッピング

| JSONの型 | Ajisaiの表現 | 備考 |
|----------|-------------|------|
| `null` | `NIL` | |
| `Number` | `Scalar(Fraction)` | |
| `String` | `Vector<Char>` + `DisplayHint::String` | |
| `Boolean` | `Scalar(1)` / `Scalar(0)` + `DisplayHint::Boolean` | |
| `Array` | `Vector` | |
| `Object` | `Vector<[key, value]>` | |

### 11.3 io モジュール

#### 公開ワード

- `IO@INPUT`
- `IO@OUTPUT`

### 11.4 使用例

```ajisai
'json' IMPORT
'io' IMPORT
IO@INPUT JSON@PARSE
[ 2 ] *
JSON@STRINGIFY IO@OUTPUT
```

---
