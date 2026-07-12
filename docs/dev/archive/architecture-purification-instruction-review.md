# Ajisai アーキテクチャ純化指示書レビューと改訂案

## 評価サマリ

提示された指示書の方向性（Dense Tensor の SoA 化、No-Rebuild、静的な質量保存、VTU Hint に基づく決定的スケジューリング）は、現在の `SPECIFICATION.md` と実装が目指している方向と概ね整合している。ただし、そのまま実行するとコンパイル不能または仕様・実装の二重化を招く箇所があるため、段階的な「削除前提条件」を明示した指示書へ改修する必要がある。

特に問題となる点は次のとおり。

1. `SPECIFICATION.md` の Step 1 相当はすでに一部反映済みであるため、単純な置換指示では重複・逆戻りが起きる。
2. `DenseTensor` は現在 `rust/src/types/mod.rs` に定義されており、`rust/src/types/value-operations.rs` ではない。また SoA フィールドは導入済みだが、互換用の `fractions: Vec<Fraction>` キャッシュが残っている。
3. `FlowToken` と flow-level error はまだ `rust/src/error.rs`、`rust/src/types/mod.rs`、`rust/src/types/flow-token.rs` に公開 API として残っている。仕様だけ先に削除すると、Safe mode の `NilReason::SafeCaught(ErrorCategory)` や既存テストとの整合性が崩れる。
4. `rust/src/elastic/hedged_executor.rs` は実行ルーティング本体ではなく、hedged winner の検証ヘルパーである。履歴・予算削除の主対象は `rust/src/interpreter/redundancy-budget.rs`、`rust/src/interpreter/redundancy-layer.rs`、それらを re-export する `rust/src/interpreter/mod.rs` である。
5. `VtuHint` は `rust/src/interpreter/quantized-block.rs` に存在するが、現状コメント上は「観測用で実行セマンティクスに影響しない」と明記されている。決定的ルーティングへ使うなら、この不変条件を仕様・コメント・テストで同時に更新する必要がある。
6. `Fraction` は `Small(i64, i64)` と `Big(BigInt, BigInt)` の二表現を持つ。`DenseTensor { numerators: Vec<i64>, denominators: Vec<i64> }` だけに寄せる指示は BigInt exactness を破壊するため、Big Fraction の扱いを「dense 非対応として nested/Scalar のままにする」のか、「BigInt SoA バッファを追加する」のかを先に決める必要がある。
7. 「旧テストは修正せず削除」は危険である。削除対象の挙動テストは消してよいが、代替不変条件（mask、静的 contract rejection、Scalar 隠蔽）を検証するテストを同じコミットまたは直後のコミットで追加する必要がある。

## 改訂した実行方針

このリファクタリングは、次の 4 フェーズに分けて実行する。

### Phase A: 仕様と実装の現状差分を閉じる

- `SPECIFICATION.md` は既存の Static Mass Conservation / No-Rebuild 記述を正とし、未反映部分だけを補強する。
- `Fraction` の BigInt exactness と Dense Tensor の i64 SoA の関係を明記する。
- `DenseTensor` の shape 二重保持（`DenseTensor.shape` と `ValueData::Tensor.shape`）をどちらへ一本化するか決める。推奨は `DenseTensor.shape` へ一本化し、`ValueData::Tensor` の `shape` フィールドを削除すること。

### Phase B: DenseTensor を真の SoA + Mask 表現へ寄せる

- `fractions: Vec<Fraction>` キャッシュを削除し、読み出し時に small Fraction を再構築する。
- invalid lane の reason は `DenseTensor` に入れず、実行コンテキスト側の `NilReasonRegistry` に寄せる。
- BigInt Fraction を含む入力は Phase B では DenseTensor 化しない。BigInt 対応 SoA は別フェーズで設計する。

### Phase C: FlowToken と redundancy budget の削除前提を作る

- `Coreword Contract` の静的検証パスを先に追加し、FlowToken が担っていた検証を compile/JIT/load 時に置換できる状態にする。
- その後に `FlowToken`、flow-level `AjisaiError` variant、`ErrorCategory` variant、関連テストを削除する。
- redundancy budget は、`select_degradation_policy` の利用箇所を決定的 policy に置換してからファイル削除する。

### Phase D: VTU Hint による決定的ルーティングと arithmetic の統合

- `VtuHint::suitability == StrongCandidate` を SIMD/SoA path へルーティングする唯一条件にする。
- `WeakCandidate` は Phase D では Plain path とし、履歴・cooldown・auto-degrade を使わない。
- arithmetic はまず binary small-fraction Tensor 同士に限定して bulk path を追加し、Scalar は shape `[1]` の Tensor へ内部昇格する。ただし user-facing validation では Scalar と Vector/Tensor の区別を維持する。

## 改訂指示書: Step 1（仕様書更新）

### 対象

- `SPECIFICATION.md`

### 改修指示

既存の `4.3.1 Internal representation classes` と `13. Fractional-Dataflow Internal Invariants` は全面置換ではなく、次の内容を満たすように差分更新する。

```markdown
#### 4.3.1 Internal representation classes

A Vector value is internally represented in one of two classes:

- **nested** — a tree of `Value` elements (`Vec<Value>`). Any element type may appear.
- **dense** — a SIMD-oriented `DenseTensor` backed by Structure-of-Arrays numerator and denominator buffers, a shape, and a validity mask. Every valid lane is an exact small Fraction. An invalid lane represents NIL occupancy without rebuilding the dense representation into nested `Vec<Value>` form.

Dense Tensor exactness rule:
- Small fractions are stored as normalized `(i64 numerator, i64 denominator)` lanes.
- Fractions that require BigInt storage are exact values, but they are not admitted to the small-lane DenseTensor representation until a BigInt-capable SoA representation is introduced.

No-Rebuild Principle: a dense Vector never degrades to a nested Vector solely because a lane becomes NIL. NIL occupancy is represented by clearing the corresponding validity-mask bit. Diagnostic NIL reasons are stored outside the dense payload in an execution-context sparse registry keyed by tensor identity and lane index.
```

```markdown
### 13.1 Static Mass Conservation

Ajisai treats flow mass conservation as a compile/JIT/load-time property. A Coreword Contract declares arity, consumption, production, bifurcation, and NIL-projection behavior. Optimized execution paths may be entered only after those contracts have been validated for the surrounding flow.

The ordinary runtime must not maintain per-value `FlowToken` objects or perform step-by-step mass accounting. Flow-accounting failures such as over-consumption, unconsumed leaks, flow breaks, and bifurcation-ratio violations are contract-validation failures and must be reported before the optimized path executes.
```

### Step 1 の受け入れ条件

- `SPECIFICATION.md` に runtime `FlowToken` 監視を必須とする記述が残っていない。
- `OverConsumption`、`UnconsumedLeak`、`FlowBreak`、`BifurcationViolation` が user-level runtime error catalog に載っていない。
- Dense Tensor の BigInt 非対応範囲が明示されている。

## 改訂指示書: Step 2（データ構造の再定義）

### 対象

- `rust/src/types/mod.rs`
- `rust/src/types/value-operations.rs`
- `rust/src/wasm-value-conversion.rs`
- `rust/src/types/display.rs`
- Tensor を pattern match している各 interpreter module

### 重要な前提

現在の `DenseTensor` はすでに SoA フィールドを持つが、`fractions: Vec<Fraction>` を保持しているため、真の SoA ではない。まずは small-fraction dense tensor として整理し、BigInt fraction は dense 化しない。

### 置き換え後の中核構造

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenseTensor {
    pub numerators: Vec<i64>,
    pub denominators: Vec<i64>,
    pub valid_mask: Vec<u64>,
    pub shape: Vec<usize>,
    pub is_pure_integer: bool,
}

impl DenseTensor {
    pub fn from_fractions(data: Vec<Fraction>, shape: Vec<usize>) -> Option<Self> {
        let expected_len = shape.iter().product::<usize>();
        if expected_len != data.len() {
            return None;
        }

        let mut numerators = Vec::with_capacity(data.len());
        let mut denominators = Vec::with_capacity(data.len());
        let mut is_pure_integer = true;

        for fraction in data {
            let (numerator, denominator) = fraction.extract_i64_pair()?;
            numerators.push(numerator);
            denominators.push(denominator);
            is_pure_integer &= denominator == 1;
        }

        let mut valid_mask = vec![u64::MAX; numerators.len().div_ceil(64)];
        if let Some(last) = valid_mask.last_mut() {
            let live_bits = numerators.len() % 64;
            if live_bits != 0 {
                *last = (1u64 << live_bits) - 1;
            }
        }

        Some(Self {
            numerators,
            denominators,
            valid_mask,
            shape,
            is_pure_integer,
        })
    }

    pub fn len(&self) -> usize {
        self.numerators.len()
    }

    pub fn is_valid(&self, index: usize) -> bool {
        index < self.len() && ((self.valid_mask[index / 64] >> (index % 64)) & 1) == 1
    }

    pub fn get_small_fraction(&self, index: usize) -> Option<Fraction> {
        if !self.is_valid(index) {
            return None;
        }
        Some(Fraction::from_i64_pair(
            self.numerators[index],
            self.denominators[index],
        ))
    }

    pub fn clear_valid(&mut self, index: usize) {
        if index < self.len() {
            self.valid_mask[index / 64] &= !(1u64 << (index % 64));
        }
    }
}
```

### `ValueData::Tensor` の推奨形

`shape` は `DenseTensor` 側に一本化する。移行中に大きな差分を避けたい場合のみ一時的に二重保持を許すが、最終形は次を推奨する。

```rust
#[derive(Debug, Clone)]
pub enum ValueData {
    Scalar(Fraction),
    Vector(Rc<Vec<Value>>),
    Tensor(Rc<DenseTensor>),
    Record {
        pairs: Rc<Vec<Value>>,
        index: HashMap<String, usize>,
    },
    Nil,
    CodeBlock(Vec<Token>),
    ProcessHandle(u64),
    SupervisorHandle(u64),
}
```

### NIL reason registry

`NilReasonRegistry` は lane index だけでなく tensor identity を含める。lane index のみだと複数 Tensor 間で衝突する。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TensorLaneId {
    pub tensor_id: u64,
    pub lane: usize,
}

pub type NilReasonRegistry = HashMap<TensorLaneId, NilReason>;
```

### Step 2 の受け入れ条件

- `DenseTensor` から `fractions: Vec<Fraction>` が削除されている。
- Tensor 表示・等価性・indexing は `get_small_fraction` または lane accessor 経由で動く。
- invalid lane は `ValueData::Nil` への rebuild ではなく `valid_mask` で表現される。
- BigInt Fraction を誤って i64 lane に切り詰めない。

## 以降の Step 3〜5 への修正指示

### Step 3: Scheduler Purification

- `hedged_executor.rs` を削除対象にしない。ここは winner validation であり、budget policy 本体ではない。
- 先に `redundancy_layer.rs` の公開関数を `VtuHint` 入力の決定的 policy に置換する。
- `interpreter/mod.rs` の `pub use redundancy_budget::{...};` を削除するのは、利用箇所がなくなってからにする。

### Step 4: Runtime Diet

- `FlowToken` 削除は Coreword Contract validator の導入後に行う。
- `AjisaiError` と `ErrorCategory` から flow-level variant を削除する際は、`NilReason::SafeCaught` の serialization/debug 表示も同時に更新する。
- 「テスト削除」は最後の手段とし、旧挙動のテストを新不変条件のテストへ置換する。

### Step 5: Fraction-as-Tensor

- まず small Fraction Tensor 同士の `ADD/SUB/MUL/DIV` bulk path を実装する。
- overflow の扱いを明示する。i64 cross product が overflow する場合は BigInt scalar path にフォールバックするか、BigInt SoA phase へ送る。
- ユーザー向け型エラーには `tensor`、`lane`、`SoA` といった内部語を出さない。

## 最終的な改訂版 Output Request

LLM へ渡す最終指示は次の形にする。

```markdown
Step 1 と Step 2 だけを最初の PR で実施してください。既存コードを確認し、すでに反映済みの仕様文は重複更新しないでください。

必須条件:
- `DenseTensor` の実体定義は `rust/src/types/mod.rs` を正とする。
- `fractions: Vec<Fraction>` キャッシュ削除に伴うコンパイルエラーをすべて修正する。
- BigInt Fraction を i64 lane へ切り詰めてはならない。
- `FlowToken`、redundancy budget、arithmetic 統合はこの PR では触らず、後続 PR の前提条件として TODO ではなく issue/checklist に分離してください。
- 旧挙動テストを削除する場合は、同じ PR で新不変条件テストを追加してください。

提出物:
1. 仕様差分
2. Rust 実装差分
3. 追加/更新テスト
4. `cargo test` の結果
```
