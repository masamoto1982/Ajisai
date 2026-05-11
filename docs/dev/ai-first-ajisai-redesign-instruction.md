# AIファースト Ajisai 再設計・改修指示書

## 0. この文書の目的

この文書は、Claude Code へ渡すための **新型 Ajisai** 改修指示書である。

現行 Ajisai は、Stack / Vector / Tensor / DisplayHint / Semantic Firewall / VTU / NIL metadata など、多くの有益な実験を含む。ただし、現行の表層言語は Vector を作成・分離・結合・再配置する操作体系に寄りすぎており、学習曲線が高い。

新型 Ajisai では、次を最優先する。

1. **AI が安全かつ機械的に扱いやすいこと。**
2. **人間にとっても最小の学習曲線で使えること。**
3. **丸め誤差のない計算モデルを中核にすること。**
4. **有理数だけでなく、無理数・計算可能実数を扱える設計へ進むこと。**
5. **内部表現と表示を明確に分離する DisplayHint / Semantic Firewall の思想を維持すること。**

Forth 2012 conformance は目的ではない。Forth は学習曲線を下げるための知見として利用する。Ajisai は Forth 互換言語ではなく、**Forth-inspired AI-first exact tensor language** として再設計する。

---

## 1. 設計スローガン

> Human surface: Forth-like.  
> AI surface: structured protocol.  
> Runtime soul: Tensor.  
> Numeric core: continued fraction / computable real.  
> Display: semantic boundary.

日本語では次のように定義する。

> Ajisai は Forth に着想を得た AI ファーストな正確 Tensor 言語である。  
> 人間向け表層は小さな word と固定 stack effect で構成する。  
> AI 向けには構造化 IR / protocol / metadata を canonical とする。  
> すべての値は Tensor substrate 上にあり、数値 cell は連分数または計算可能実数として表現する。  
> 表示は DisplayHint により内部表現から分離する。

---

## 2. 最重要方針

### 2.1 AI canonical protocol を第一級にする

AI が扱う canonical form は Forth-like source text ではない。

Claude Code は、次の三層を明確に分離して実装すること。

1. **Human Surface**
   - Forth-like なテキスト構文。
   - 人間が読む・書く。
   - 例: `355 113 / AS-CF .`

2. **Canonical Stack IR**
   - AI / tooling が主に扱う構造化表現。
   - word sequence、literal、stack effect、metadata を lossless に持つ。
   - Human Surface から parse 可能であり、Human Surface へ pretty-print 可能であること。

3. **Tensor Dataflow IR / TensorPlan**
   - VTU / optimizer / exact numeric engine が扱う内部計画。
   - stack manipulation は aliasing / dependency graph へ lower される。
   - pure tensor computation を fusion / classification できること。

### 2.2 Human Surface は Forth-inspired に寄せる

新型 Ajisai の人間向け表層は、Forth の知見を借りる。

- word は空白区切り。
- word は固定 stack effect を持つ。
- mode によって word の意味が変わってはならない。
- stack 操作は `DUP` / `DROP` / `SWAP` / `OVER` / `ROT` など明示 word で行う。
- quotation / code block は `{ ... }` を使う。
- `(...)` は comment / stack effect annotation として使う。
- `.` は Forth 風の print word として使う。現行 Ajisai の `TOP` modifier ではない。

### 2.3 現行の操作対象モード・消費モードは廃止する

次は新型 Ajisai の core から除去する。

- `OperationTargetMode`
- `ConsumptionMode`
- `TOP` / `STAK`
- `EAT` / `KEEP`
- `.` / `..` / `,` / `,,` による modifier algebra
- `..,,` や `~..` のような mode combination

これらは現行 Ajisai の Vector 工作のための機能であり、新型 Ajisai の AI-first / Forth-inspired 方針には合わない。

代替は明示 stack words と固定 stack effect である。

例:

```forth
\ old keep-like intent
A B 2DUP +

\ old stack-target-like intent
DEPTH COLLECT-N SORT
```

### 2.4 Stack は二本を基本にする。ただし Forth conformance には縛られない

新型 Ajisai は、意味論として次の二本 stack を持つ。

1. **Data stack**
   - 通常の値を運ぶ stack。

2. **Flow stack**
   - return stack / continuation stack / loop context / temporary control storage を統合する内部 stack。
   - Forth の return stack に着想を得るが、Forth 2012 の細部には縛られない。

Control-flow stack は compile-time abstraction として別途持ってよいが、ユーザー可視の第三 stack にしてはならない。

### 2.5 Stack も Tensor で実装してよい

Stack の意味論は LIFO だが、内部 storage は rank-1 Tensor として実装してよい。

```text
data stack = Tensor<StackCell, rank=1>
flow stack = Tensor<StackCell, rank=1>
```

ただし、ユーザーに「stack は tensor なので自由に shape 操作できる」と見せてはならない。Stack Tensor は実装詳細または debug/introspection 対象であり、通常の stack semantics は Forth-like に保つ。

---

## 3. 値モデル

### 3.1 すべての値は Tensor substrate 上に置く

新型 Ajisai の値は次の形へ向かう。

```rust
pub struct AjisaiValue {
    pub tensor: Tensor<ValueCell>,
    pub semantics: ValueSemantics,
    pub display_hint: DisplayHint,
}
```

`ValueCell` は少なくとも以下を表せること。

```rust
pub enum ValueCell {
    Number(NumberValue),
    Bool(bool),
    Text(TextAtom),
    Nil(AbsenceMetadata),
    Record(RecordCell),
    Code(CodeRef),
    Handle(HandleId),
}
```

「すべてが連分数」ではない。正しくは次である。

> すべての Ajisai 値は Tensor substrate 上にある。  
> 数値 cell が continued-fraction / computable-real 表現を持つ。

### 3.2 数値 cell は連分数 / 計算可能実数として表現する

数値表現は `Fraction` 直接保持から、次へ段階移行する。

```rust
pub enum NumberValue {
    FiniteCf(CfNode),
    PeriodicCf {
        prefix: Option<Box<CfNode>>,
        period: Box<CfNode>,
    },
    LazyCf(LazyCf),
    ComputableReal(RealProgram),
}
```

有限連分数の canonical internal tree は flat list ではなく、右ネスト node とする。

```rust
pub enum CfNode {
    Last(BigInt),
    Cons(BigInt, Box<CfNode>),
}
```

### 3.3 連分数の内部表示は `[]` 右ネストにする

丸括弧は stack effect / comment に使うため、連分数の内部表示には使わない。

有限連分数の debug/internal display は次の形式とする。

```text
[3]
[1 [2]]
[1 [2 [3]]]
[3 [7 [16]]]
```

対応例:

```text
[3]            = 3
[1 [2]]        = 3/2
[0 [2]]        = 1/2
[-1 [2]]       = -1/2
[3 [7 [16]]]   = 355/113
```

この表示は ordinary Tensor/List literal ではない。`NumberValue::FiniteCf(CfNode)` の表示 projection である。

AI canonical protocol では文字列ではなく、構造化 `CfNode` として扱うこと。

例:

```json
{
  "kind": "finiteContinuedFraction",
  "node": {
    "head": "3",
    "tail": {
      "head": "7",
      "tail": { "last": "16" }
    }
  }
}
```

### 3.4 有限連分数の正規形

`CfNode` は次を満たす。

1. 空 node は存在しない。
2. 先頭係数 `a0` は任意整数。
3. `a1..an` は正整数。
4. 長さ 2 以上なら末尾 `an > 1`。
5. 同じ有理数は一意な `CfNode` へ正規化する。
6. 分母は正に正規化する。
7. 負の有理数は floor division ベースの Euclidean algorithm で係数化する。

---

## 4. DisplayHint / Semantic Firewall

### 4.1 維持するもの

次の現行思想は維持・強化する。

- 内部表現と observable semantics の分離。
- Rust enum variant 名や Debug 表示を外部 protocol にしない。
- display string を machine-readable decision に使わない。
- `DisplayHint` による表示制御。
- structured `absence` metadata。
- structured diagnosis。

### 4.2 DisplayHint は計算に影響してはならない

DisplayHint は表示境界の情報であり、計算結果や word dispatch に影響してはならない。

現行コードに DisplayHint を実行判定へ使う箇所がある場合、新型 Ajisai では semantic tag / ValueCell kind / protocol axes に置き換えること。

### 4.3 推奨 DisplayHint

```rust
pub enum DisplayHint {
    Auto,
    Fraction,
    Decimal { digits: usize },
    ContinuedFractionNested,
    Tensor,
    String,
    Boolean,
    Internal,
}
```

例:

```forth
355 113 / .
\ 355/113

355 113 / AS-CF .
\ [3 [7 [16]]]

355 113 / 20 AS-DECIMAL .
\ 3.14159292035398230088
```

---

## 5. 構文方針

### 5.1 予約記号

新型 Ajisai の表層では、次を原則とする。

```text
(...)  comment / stack effect annotation
{...}  quotation / code block
[...]  tensor/list literal または internal CF display projection
'...'  string literal
```

`()` を code block として扱う現行仕様は廃止する。

### 5.2 Stack effect annotation

丸括弧は Forth 風の stack effect annotation に使う。

```forth
: SQUARE ( number -- number ) DUP * ;
: HYPOT  ( number number -- number ) DUP * SWAP DUP * + SQRT ;
```

AI-first 方針では、これは単なる comment ではなく contract source である。

実装要件:

1. 通常実行 parser は `(...)` を comment として無視してよい。
2. documentation / contract parser は `(...)` を stack effect annotation として取得する。
3. 取得した stack effect と word body の推論 stack effect を照合できるようにする。
4. 不一致時は diagnostic を出す。

### 5.3 Word は固定 stack effect を持つ

全 core word は固定 stack effect を持つ。

例:

```text
DUP      ( x -- x x )
DROP     ( x -- )
SWAP     ( x y -- y x )
OVER     ( x y -- x y x )
ROT      ( x y z -- y z x )
ADD      ( number number -- number )
DIV      ( number number -- number )
AS-CF    ( value -- value )
.        ( value -- )
```

Word の意味は hidden mode に依存してはならない。

---

## 6. AI canonical Stack IR

### 6.1 目的

Human Surface は人間向けであり、AI が最も使いやすい canonical form ではない。

Claude Code は、Human Surface と独立した canonical Stack IR を導入すること。

### 6.2 最小 IR 例

```json
{
  "kind": "definition",
  "name": "SQUARE",
  "declaredStackEffect": {
    "inputs": [{ "name": "x", "semanticKind": "number" }],
    "outputs": [{ "name": "y", "semanticKind": "number" }]
  },
  "body": [
    { "op": "word", "name": "DUP" },
    { "op": "word", "name": "MUL" }
  ]
}
```

### 6.3 IR 要件

Stack IR は次を持つ。

- token / word sequence
- literal values
- declared stack effect
- inferred stack effect
- word references
- source spans
- diagnostics
- semantic metadata hooks

AI / tooling は Stack IR を canonical として扱う。

Human Surface は Stack IR の pretty-print として生成できることが望ましい。

---

## 7. Tensor Dataflow IR / VTU

### 7.1 VTU を殺さないための再定義

VTU は Vector 操作高速化ではない。

新型 Ajisai では、VTU を次のように再定義する。

> VTU は、Stack IR から lower された pure Tensor Dataflow IR / TensorPlan に対して働く分類・最適化・実行計画層である。

### 7.2 lowering pipeline

```text
Human Surface
  ↓ parse
Canonical Stack IR
  ↓ stack effect analysis
Stack-resolved IR
  ↓ alias / dependency lowering
Tensor Dataflow IR
  ↓ shape / purity / exactness analysis
VTU classification
  ↓ optional execution planning
Exact Tensor runtime
```

### 7.3 Stack manipulation は VTU 前に消す

`DUP` / `SWAP` / `OVER` / `ROT` は VTU kernel ではない。

これらは dataflow aliasing / rewiring として lower する。

例:

```forth
: SQUARE ( number -- number ) DUP * ;
```

Tensor Dataflow IR:

```text
%0 = input number
%1 = MUL %0 %0
return %1
```

### 7.4 VTU metadata

各 word は、VTU に必要な metadata を持つこと。

- stack effect
- semantic input/output kinds
- shape rule
- broadcast policy
- purity
- exactness policy
- nil policy
- display effect
- candidate kernel kind
- fusion eligibility

### 7.5 exactness boundary

近似 backend は明示境界なしに使ってはならない。

- exact computation が default。
- `TO-F32` / `TO-F64` / `APPROX` / `DECIMAL` のような明示 word の後だけ approximate backend を許可する。
- VTU は exact backend と approximate backend を区別する。

---

## 8. NIL / absence / diagnostics

現行の diagnostic NIL の思想は維持する。

新型 Ajisai でも、NIL は単なる null ではなく structured absence value である。

保持する protocol:

```ts
type ProtocolAbsence = {
  reason?: string;
  origin: string;
  recoverability: string;
  caughtCategory?: string;
  diagnosis?: ProtocolDiagnosis;
};
```

Stack / Tensor / NumberValue の再設計後も、NIL metadata を失ってはならない。

AI 向け protocol では、display text `NIL` を判定に使わず、semanticKind / shape / capabilities / absence を使うこと。

---

## 9. 移行計画

### Phase 0: 仕様分岐の明文化

- 現行 Ajisai を v1 として扱う。
- 新型 Ajisai を v2 / next として設計する。
- `SPECIFICATION.md` をいきなり破壊せず、まず `docs/dev/` に v2 仕様草案を置く。
- 既存テストが v1 前提であることを明記する。

### Phase 1: Core model document

- この文書をもとに v2 canonical design document を作る。
- Human Surface / Stack IR / TensorPlan の三層を定義する。
- Stack effect notation の grammar を定義する。
- DisplayHint / Semantic Firewall の継承方針を定義する。

### Phase 2: NumberValue / CfNode prototype

- `CfNode` を追加する。
- `FiniteCf` 正規化を実装する。
- ratio → CfNode、CfNode → ratio を実装する。
- debug display `[3 [7 [16]]]` を実装する。
- flat terms list を canonical storage にしない。

### Phase 3: Stack IR prototype

- Human Surface parser から Stack IR を生成する。
- `(...)` stack effect annotation を取得する。
- stack effect inference を実装する。
- mode-free fixed stack effect word set を定義する。

### Phase 4: Forth-inspired core word set

- `DUP`, `DROP`, `SWAP`, `OVER`, `ROT`
- arithmetic words: `ADD`, `SUB`, `MUL`, `DIV`
- display words: `.`, `AS-CF`, `AS-FRACTION`, `AS-DECIMAL`
- quotation words: `{ ... }` based execution

この Phase で `TOP/STAK/EAT/KEEP` を新型 core から除外する。

### Phase 5: Tensor substrate unification

- `AjisaiValue = Tensor<ValueCell> + semantics + display_hint` へ向かう設計を始める。
- Stack storage を rank-1 Tensor として実装する。ただし semantics は LIFO。
- ordinary user-facing stack operation は Tensor shape 操作にしない。

### Phase 6: Tensor Dataflow IR / VTU integration

- Stack IR から Tensor Dataflow IR へ lower する。
- stack manipulation を aliasing に変換する。
- pure numeric blocks を VTU candidate として分類する。
- fusion / exactness / approximate boundary を実装する。

### Phase 7: AI protocol first-class support

- Stack IR / TensorPlan / Value semantics / diagnostics を JSON-compatible protocol として出せるようにする。
- GUI / API / AI tooling は display string ではなく protocol を読む。
- Human Surface は protocol から pretty-print 可能にする。

---

## 10. 明示的にやってはいけないこと

1. DisplayHint を計算分岐に使わない。
2. Rust enum variant 名や Debug string を external protocol にしない。
3. `TOP/STAK/EAT/KEEP` 相当の hidden mode を復活させない。
4. Stack をユーザー可視 Tensor として自由に reshape 可能にしない。
5. Flat `[1 2 3]` terms list を finite CF の canonical storage にしない。
6. 明示 approximation boundary なしに f32/f64/BFloat16/GPU approximate backend へ落とさない。
7. AI 向け canonical form を source text だけにしない。
8. NIL の reason / origin / diagnosis を display text に畳み込まない。

---

## 11. Definition of Done

新型 Ajisai の最初の PoC は、次を満たせばよい。

1. `: SQUARE ( number -- number ) DUP MUL ;` を parse できる。
2. stack effect annotation を取得できる。
3. body から inferred stack effect を計算できる。
4. `355 113 DIV AS-CF .` を Stack IR に変換できる。
5. `355/113` を `CfNode = [3 [7 [16]]]` として表現できる。
6. 通常表示 `355/113` と CF 表示 `[3 [7 [16]]]` を DisplayHint で切り替えられる。
7. Stack IR から simple Tensor Dataflow IR へ lower できる。
8. `DUP MUL` を `%1 = MUL %0 %0` へ lower できる。
9. Display string ではなく structured protocol で value semantics を取得できる。
10. 既存 v1 と衝突する場合、v2 namespace / feature flag / branch を分ける。

---

## 12. Claude Code への実装上の注意

- いきなり全既存実装を破壊しない。
- まず v2 用の module / feature flag / documentation を追加する。
- v1 の既存テストを破壊する変更は、v2 方針が固まってから行う。
- 大改修は小さな PR に分割する。
- 各 PR では必ず machine-readable protocol と tests を追加する。
- GUI 表示や WASM boundary を触る場合、display string と protocol field を混同しない。
- VTU は source syntax ではなく TensorPlan を見る設計へ移す。

---

## 13. 最終判断

現行 Ajisai は実験として成功しているが、学習曲線と表層概念が重い。

新型 Ajisai では、Forth の単純な stack-oriented 表層を借りつつ、AI にとっては structured protocol / Stack IR / TensorPlan を canonical とする。

Ajisai の独自性は Forth 互換性ではなく、次にある。

- Tensor substrate
- continued-fraction / computable-real numeric core
- DisplayHint による表示分離
- structured absence / diagnosis
- machine-readable word contracts
- VTU による pure tensor dataflow classification
- AI が source text ではなく semantic protocol を扱えること

この方向を新型 Ajisai の中核として実装すること。
