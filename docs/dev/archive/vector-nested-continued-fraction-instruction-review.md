# Ajisai Vector ネスト連分数内部表現移行 指示書レビューと改訂版

> **Status (2026-05-13): Superseded — non-canonical.**
> SPECIFICATION.md §4.2 ("Scalar: exact-real continued-fraction arithmetic") is the canonical source for the continued-fraction representation, including lazy CF for irrationals, Gosper bihomographic arithmetic, the nested `( a0 ( a1 ... ))` serialization form, the comparison-budget Bubble/NIL rule, and the removal of `(...)` as a code-block delimiter (§3.1, §3.4). This document predates that decision and covered only finite-CF migration for rationals; treat it as historical context only. Where this document conflicts with SPECIFICATION.md, SPECIFICATION.md wins.

## 1. 結論

提示された指示書の大きな方向性、すなわち **ユーザー表示と内部表現を分離し、丸め誤差のない有限数値表現を将来の周期連分数・Lazy 実数へ拡張できる形へ寄せる** という目的は妥当です。

ただし、現在の Ajisai 実装へそのまま適用すると、型設計の循環、既存 Tensor 最適化との不整合、連分数正規形の曖昧さ、比較例の誤り、実装フェーズの粒度過大により、コンパイル不能または仕様の二重化を招く可能性が高いです。

したがって本書では、元指示書を次の方針へ改訂します。

- **Phase 1 では `ValueData::Scalar(Fraction)` を即時削除しない。** まず `continued_fraction` API と `NumberValue` facade を導入し、外部境界を差し替える。
- **「連分数の実体を右ネスト Vector そのものとして保存する」設計は採用しない。** 再帰的な `NumberValue -> Value -> NumberValue` 循環と、通常 Vector との意味衝突を避けるため、正規ストレージは係数列または専用 Number payload とし、右ネスト Vector は `CF` などの明示ワードで生成する投影表現とする。
- **将来 Tensor 統合は維持するが、現行 `DenseTensor` / `SparseTensor` の small i64 SoA とは別フェーズに分ける。** 現行 Tensor は `numerators: Vec<i64>` / `denominators: Vec<i64>` に依存しており、BigInt 係数の連分数表現へ直ちに置換できない。
- **有限単純連分数の正規形を数学的に明確化する。** `a0` は任意整数、`a1..an` は正整数、長さ 2 以上では末尾 `an > 1` とする。負数は floor division に基づく Euclidean algorithm で生成する。
- **表示・演算・比較の移行は `Fraction` カーネルを一時利用してよいが、境界 API を先に作る。** 結果値を長期的に `Fraction` として保存しない方針は後続 Phase の完了条件にする。

## 2. 現行実装との照合

現在のリポジトリでは、数値値は `ValueData::Scalar(Fraction)` として表現されています。`ValueData` には `Scalar(Fraction)`、`Vector`、`Tensor` が並立しています。

また、`DenseTensor` は `numerators: Vec<i64>` と `denominators: Vec<i64>`、`valid_mask`、`shape` を持つ small fraction 向け SoA 表現です。このため、BigInt 係数を前提にした連分数を Tensor 全体へ即時統合するには、BigInt 対応 Tensor storage または一般 cell storage が必要です。

表示層では `ValueData::Scalar(f)` が `format_fraction(f)` により自然な数値表示へ変換され、Vector/Tensor は再帰的に表示されています。これは元指示書の「NumberFiniteCf は通常 Vector として表示してはいけない」という要件と整合しますが、実装上は display が数値意味タグを識別できる新しい入口を先に必要とします。

算術演算は `Fraction` を受け取る `apply_binary_arithmetic` と `apply_binary_broadcast_with_metrics` を中心に動作しています。したがって、最初から演算経路全体を右ネスト Vector ベースへ置き換えるより、`NumberValue <-> Fraction/ratio` 変換境界を先に作り、既存 kernel を段階的に差し替える方が安全です。

## 3. 元指示書の主な問題点

### 3.1 文書が重複しており、末尾が破損している

元指示書は同一内容が二度貼り付けられており、最後も「ことであ」で途切れています。実装者がどちらを正とすべきか迷うため、重複を削除した単一の改訂版へ統合する必要があります。

### 3.2 `NumberValue { repr: Box<Value> }` は循環的で危険

元指示書は次のような形を提案しています。

```rust
pub struct NumberValue {
    pub semantic: NumberSemantic,
    pub repr: Box<Value>, // 実体は右ネストVector
}
```

しかし、`Value` 側が `Scalar(NumberValue)` を持ち、`NumberValue` が `Value` を持つ設計にすると、数値係数を表すために再び `NumberValue` が必要になりやすく、設計上の循環が発生します。

さらに、`repr` が通常 `Value` である限り、連分数内部係数とユーザーが作った通常 Vector の境界が弱くなります。表示器や比較器が誤って内部表現を通常 Vector として扱うリスクがあります。

改訂方針:

- `NumberValue` の正規ストレージは、Phase 1 では `FiniteCf { terms: Rc<Vec<BigInt>> }` とする。
- 右ネスト Vector は canonical storage ではなく、`CF` ワードや debug API が生成する **plain projection** とする。
- 将来 Tensor 統合時は `TensorSemantic::NumberFiniteCf` と `TensorStorage<AjisaiCell>` を導入し、係数 cell を整数 BigInt として格納する。`NumberValue` が `Value` を内包する循環は避ける。

### 3.3 「数値の実体は Tensor」と「Phase 1 で DenseTensor を残す」が未整理

元指示書は「数値 = Vector の右ネスト」「Vector = Tensor の一形態」「したがって数値の実体は Tensor」と述べています。一方で、Phase 1 では現行 `DenseTensor` を残してよいとも述べています。

現行の `DenseTensor` は small i64 numerator/denominator lane を持つ設計であり、BigInt 連分数係数列を表す Tensor ではありません。したがって、ここを一気に置換すると次の問題が起きます。

- BigInt `Fraction` は exact だが small i64 Tensor に入らない。
- `SparseTensor` も numerator/denominator SoA を前提にしている。
- SIMD/VTU 最適化経路が small fraction lane 前提で動作している。

改訂方針:

- Phase 1-3 では `DenseTensor` を「small rational tensor optimization」として維持する。
- 新しい数値抽象は `NumberValue` API で導入し、Tensor 統合は Phase 5 以降の独立移行とする。
- `DenseTensor::from_fractions` をすぐ deprecated にするのではなく、「新規の scalar number 表現には直接依存しない」と限定する。

### 3.4 有限連分数の正規形が不足している

元指示書は「末尾係数が 1 になる表現は避ける」と述べていますが、係数の符号条件が明記されていません。有限単純連分数として扱うなら、正規形は次のように定義する必要があります。

- `a0` は任意の整数。
- `a1..an` は正整数。
- 長さ 1 の場合は `[a0]`。
- 長さ 2 以上の場合は `an > 1`。
- 分母は常に正に正規化する。
- 負の有理数は truncating division ではなく floor division による Euclidean algorithm で係数列を生成する。

この定義により、代表例は次になります。

| 有理数 | 正規有限連分数 | 右ネスト投影 |
| --- | --- | --- |
| `3` | `[3]` | `[3]` |
| `3/2` | `[1; 2]` | `[1 [2]]` |
| `1/2` | `[0; 2]` | `[0 [2]]` |
| `-3/2` | `[-2; 2]` | `[-2 [2]]` |
| `-1/2` | `[-1; 2]` | `[-1 [2]]` |
| `355/113` | `[3; 7, 16]` | `[3 [7 [16]]]` |

### 3.5 比較例に数学的な誤りがある

元指示書には次の例があります。

```text
[1 [2]] EXACT_EQ [0 [2]]
```

しかし、`[1 [2]] = 1 + 1/2 = 3/2`、`[0 [2]] = 1/2` なので同値ではありません。

改訂例:

```text
[1 [1]] EXACT_EQ [2]
```

または、正規形でない入力も許容するなら次も同値例として使えます。

```text
[0 [1 [1]]] EXACT_EQ [0 [2]]
```

### 3.6 Parser 要件の `1/2` 例が曖昧

元指示書は `1/2` について「`[0 [2]]` または `[1 [2]]` など正しい有限連分数」と述べていますが、`[1 [2]]` は `3/2` です。`1/2` の正規形は `[0 [2]]` です。

改訂方針:

- `1/2` は必ず `[0 [2]]` 相当の `FiniteCf([0, 2])` へ正規化する。
- `-1/2` は `[-1 [2]]` 相当の `FiniteCf([-1, 2])` へ正規化する。
- 小数リテラルは 10 進有理数として parse し、同じ正規化 API を通す。

### 3.7 `CF` の仕様は「内部値」ではなく「plain 投影」と明記すべき

`CF` は内部表現確認用のワードとして妥当です。ただし、出力を `Plain Vector` にするなら、それは canonical storage そのものではなく表示用・検証用の投影です。

改訂方針:

- `CF` は `NumberValue::FiniteCf` から plain nested vector を生成する。
- 生成された plain vector には `NumberFiniteCf` semantic を付けない。
- その結果、通常 display は `[ 3 [ 7 [ 16 ] ] ]` のように Vector として表示する。
- 数値として再解釈したい場合は、別途 `FROM-CF` のような明示ワードを将来追加する。

### 3.8 `CONVERGENT` の index 定義が必要

`355/113 / 2 CONVERGENT -> 22/7` は、0-based index なら正しいです。`[3; 7, 16]` の convergent は次の通りです。

- index `0`: `3`
- index `1`: `22/7`
- index `2`: `355/113`

したがって、元例の `2 CONVERGENT -> 22/7` は 1-based index の場合だけ正しいです。

改訂方針:

- Ajisai の index 操作と揃えるため、`CONVERGENT` は **0-based** とする。
- `355 113 / 1 CONVERGENT` が `22/7`、`2 CONVERGENT` が `355/113` を返す。
- 1-based を採用する場合は、word 名を `CONVERGENT1` にするなど明示する。

## 4. 改訂後の実装指示書

## 4.1 目的

Ajisai の数値表現を、現行の `Fraction` 直接保持から、連分数 semantic を持つ `NumberValue` へ段階移行する。

ユーザー入力・通常表示では従来通り自然な数値として扱い、内部確認用 word を使った場合のみ、右ネスト Vector 投影を表示する。

最終的には数値・Vector・Tensor の統合を目指すが、現行 `DenseTensor` / `SparseTensor` の small fraction 最適化とは段階的に統合する。

## 4.2 非目標

この改修では、次を完了条件に含めない。

- `DenseTensor` / `SparseTensor` の即時全面置換。
- `ValueData::Scalar(Fraction)` の Phase 1 即時削除。
- π、e、三角関数、log などの Lazy 連分数完全実装。
- 周期連分数の完全演算。
- CAS 的な式変形。
- 小数を内部保存形式にすること。

## 4.3 新しい有限連分数正規形

有限単純連分数を、係数列 `terms = [a0, a1, ..., an]` として定義する。

不変条件:

1. `terms` は空でない。
2. `a0` は任意の整数。
3. `a1..an` は正整数。
4. `terms.len() >= 2` の場合、末尾 `an > 1`。
5. 同じ有理数は一意な正規係数列へ正規化する。
6. 分母は正に正規化する。

右ネスト Vector は保存形式ではなく、次の投影形式とする。

```text
[a0]                  -> [a0]
[a0, a1]              -> [a0 [a1]]
[a0, a1, a2]          -> [a0 [a1 [a2]]]
[a0, a1, ..., an]     -> [a0 [a1 [... [an]]]]
```

Ajisai 表層構文では、連分数の区切りに `,` を導入しない。Rust 内部の `Vec<BigInt>` は実装補助であり、Ajisai 構文ではない。

## 4.4 型設計

### Phase 1-3 の推奨型

```rust
pub enum NumberSemantic {
    FiniteCf,
    PeriodicCf,
    LazyCf,
}

pub enum NumberValue {
    FiniteCf { terms: Rc<Vec<BigInt>> },
    PeriodicCf { prefix: Rc<Vec<BigInt>>, period: Rc<Vec<BigInt>> },
    LazyCf { generator: LazyNumberGeneratorId, cache: Rc<Vec<BigInt>> },
}
```

移行中は次のいずれかを採用する。

#### A. 低リスク案

現行 `ValueData::Scalar(Fraction)` は残しつつ、`NumberValue` API を追加する。

- Parser は当面 `Fraction` を生成してよい。
- `Value::as_number_value()` または `NumberValue::from_fraction()` を追加する。
- `CF` / `TERMS` / `CONVERGENT` は `Fraction -> NumberValue::FiniteCf` 変換を通して実装する。
- この Phase では保存形式の完全置換を要求しない。

#### B. 中リスク案

`ValueData::Scalar(NumberValue)` へ置換する。ただし `NumberValue` は `Box<Value>` を持たず、係数列を持つ。

```rust
pub enum ValueData {
    Scalar(NumberValue),
    Vector(Rc<Vec<Value>>),
    Tensor {
        data: Rc<DenseTensor>,
        shape: Rc<Vec<usize>>,
    },
    Record { ... },
    Nil,
    CodeBlock(Vec<Token>),
    ProcessHandle(u64),
    SupervisorHandle(u64),
}
```

#### C. 最終形

将来 `TensorStorage<AjisaiCell>` を導入した後、数値 scalar も rank-0 または shape `[1]` の semantic tagged tensor として統合する。

```rust
pub enum TensorSemantic {
    Plain,
    NumberFiniteCf,
    NumberPeriodicCf,
    NumberLazyCf,
    String,
    Boolean,
    Interval,
}

pub enum TensorStorage<T> {
    Dense(Vec<T>),
    Sparse {
        indices: Vec<usize>,
        values: Vec<T>,
        default: T,
        len: usize,
    },
    Lazy {
        shape: Vec<usize>,
        source: TensorGeneratorId,
    },
}

pub enum AjisaiCell {
    Integer(BigInt),
    Number(NumberValue),
    Value(Box<Value>),
    Nil,
}
```

この最終形は Phase 5 以降の目標であり、Phase 1 の完了条件にはしない。

## 4.5 新規モジュール

追加対象:

```text
rust/src/types/continued_fraction.rs
```

責務:

- 有理数から有限連分数係数列への変換。
- 有限連分数係数列から有理数への復元。
- 正規形チェック。
- 正規化。
- 右ネスト Vector 投影の生成。
- plain nested vector から係数列を抽出する debug/import helper。
- convergent 計算。
- 表示・比較・演算用の補助関数。

## 4.6 必須 API

### 係数列生成

```rust
pub fn terms_from_ratio(
    numerator: BigInt,
    denominator: BigInt,
) -> Result<Vec<BigInt>, NilReason>;

pub fn terms_from_i64(n: i64) -> Vec<BigInt>;
```

要件:

- `denominator == 0` は `NilReason::DivisionByZero` 相当の bubble へ変換できるよう `Err` を返す。
- 分母は正へ正規化する。
- gcd で約分する。
- floor division ベースの Euclidean algorithm を使う。
- 長さ 2 以上の場合、末尾 `1` を残さない。

### NumberValue 生成

```rust
pub fn finite_cf_from_i64(n: i64) -> NumberValue;

pub fn finite_cf_from_ratio(
    numerator: BigInt,
    denominator: BigInt,
) -> Result<NumberValue, NilReason>;
```

### 右ネスト Vector 投影

```rust
pub fn nested_vector_from_terms(terms: &[BigInt]) -> Result<Value, CfError>;
```

要件:

- `terms` が空なら `CfError::EmptyTerms`。
- 生成する値は plain Vector であり、Number semantic を持たない。
- 係数は整数数値として表現する。

### 係数列抽出

```rust
pub fn terms_from_nested_vector(value: &Value) -> Result<Vec<BigInt>, CfError>;
```

受け入れる形:

```text
[a0]
[a0 [a1]]
[a0 [a1 [a2]]]
```

拒否する形:

```text
[]
[a0 a1 a2]
[a0 []]
[a0 [a1 a2 a3]]
[non_integer]
```

抽出時の扱い:

- plain nested vector は debug/import 入力として扱う。
- 抽出後、`normalize_terms` で canonical terms へ寄せる。
- 通常 Vector を暗黙に数値として演算へ渡すことはしない。

### 有理数への復元

```rust
pub fn ratio_from_terms(terms: &[BigInt]) -> Result<(BigInt, BigInt), CfError>;

pub fn ratio_from_finite_cf(value: &NumberValue) -> Result<(BigInt, BigInt), CfError>;
```

例:

```text
[3, 7, 16] -> (355, 113)
```

### 正規化

```rust
pub fn normalize_terms(terms: &[BigInt]) -> Result<Vec<BigInt>, CfError>;

pub fn normalize_nested_finite_cf(value: &Value) -> Result<NumberValue, CfError>;
```

要件:

- 構造を検証する。
- 一度有理数へ復元する。
- 再度 `terms_from_ratio` で正規係数列へ変換する。

### Convergent

```rust
pub fn convergent_terms(
    terms: &[BigInt],
    index: usize,
) -> Result<(BigInt, BigInt), CfError>;
```

index は 0-based とする。

## 4.7 既存 `Fraction` の扱い

Phase 1-3 では `Fraction` を計算カーネルとして残してよい。

許可:

```text
NumberValue::FiniteCf
-> ratio / Fraction へ復元
-> 既存 Fraction arithmetic で計算
-> NumberValue::FiniteCf へ戻す
```

禁止:

- 新規の長期保存表現として `Fraction` 依存を増やすこと。
- 新しい public semantic API が `Fraction` を canonical representation として露出すること。
- `NumberFiniteCf` semantic を持つ値を display で plain Vector として表示すること。

## 4.8 算術演算

有限連分数同士の四則演算は、Phase 1-3 では ratio へ復元して正確な有理数演算を行い、結果を有限連分数へ戻す。

加算:

```text
a b +
```

処理:

1. `a` と `b` を数値 semantic として取得する。
2. 有限連分数なら `(num, den)` へ復元する。
3. 有理数として加算する。
4. 結果を `NumberValue::FiniteCf` へ変換する。
5. 通常表示では整数または `numerator/denominator` として表示する。

除算:

- 右辺が 0 なら bubble にする。
- 通常 Vector や Record を数値演算へ渡した場合は structure error にする。

## 4.9 比較演算

### EXACT_EQ

数学的同値性を比較する。

```text
[1 [1]] EXACT_EQ [2] -> true
```

ただし、plain Vector は通常演算で暗黙に数値化しない。debug/import context で plain nested vector を受け付ける場合のみ、正規化後に比較してよい。

### STRUCT_EQ

内部構造または plain 投影構造が同一かを比較する。

```text
[3 [7 [16]]] STRUCT_EQ [3 [7 [16]]] -> true
[1 [1]] STRUCT_EQ [2] -> false
```

`STRUCT_EQ` は user-facing の主比較ではなく、デバッグ・検証向けとする。

### LT / GT / LE / GE

有限連分数は ratio へ復元して正確に比較する。

Lazy/Periodic で未対応の場合:

- 必要精度へ到達できない場合は bubble。
- 型不整合は error。

## 4.10 表示

### 通常表示

`NumberValue::FiniteCf` は ratio へ復元して表示する。

- 整数なら `3`。
- 非整数なら `355/113`。
- 内部右ネスト投影は通常表示しない。

### `CF`

`CF` は数値から plain nested vector 投影を返す。

```text
355 113 / CF
```

期待表示:

```text
[ 3 [ 7 [ 16 ] ] ]
```

出力は `NumberFiniteCf` semantic を持たない plain Vector とする。

### `TERMS`

`TERMS` は係数列を plain flat vector として返す。

```text
355 113 / TERMS
```

期待表示:

```text
[ 3 7 16 ]
```

### `CONVERGENT`

0-based index を採用する。

```text
355 113 / 0 CONVERGENT -> 3
355 113 / 1 CONVERGENT -> 22/7
355 113 / 2 CONVERGENT -> 355/113
```

### `DECIMAL`

明示精度付きで小数文字列または表示専用値を返す。

```text
355 113 / 20 DECIMAL
```

期待表示:

```text
3.14159292035398230088
```

内部保存形式を小数に変換しない。

## 4.11 Parser 要件

- `123` は `FiniteCf([123])` 相当へ変換する。
- `1/2` は `FiniteCf([0, 2])` 相当へ変換する。
- `-1/2` は `FiniteCf([-1, 2])` 相当へ変換する。
- `3/2` は `FiniteCf([1, 2])` 相当へ変換する。
- 小数リテラルは 10 進有理数として parse し、同じ正規化 API を通す。
- `,` は消費モード専用とし、連分数構文へ導入しない。

## 4.12 Error / Bubble 方針

既存方針に従う。

Bubble:

- 0 除算。
- Lazy 連分数比較で必要精度へ到達できない。
- 未対応の超越関数。
- Convergent index が範囲外で、仕様上「計算不能」と扱う場合。

Error:

- 通常 Vector を数値演算へ渡した。
- plain nested vector を finite CF として import しようとして構造が壊れている。
- `CF` / `TERMS` / `CONVERGENT` に非数値を渡した。
- 3 要素以上の Vector node を finite CF node として解釈しようとした。

## 5. 実装フェーズ

### Phase 1: 連分数ライブラリ導入

対象:

- `rust/src/types/continued_fraction.rs`
- `rust/src/types/mod.rs`

作業:

1. `CfError` を定義する。
2. `terms_from_ratio` / `ratio_from_terms` / `normalize_terms` を実装する。
3. `nested_vector_from_terms` / `terms_from_nested_vector` を実装する。
4. `convergent_terms` を実装する。
5. 単体テストを追加する。

この Phase では `ValueData::Scalar(Fraction)` の削除を要求しない。

### Phase 2: NumberValue facade 導入

作業:

1. `NumberValue` を追加する。
2. `Fraction -> NumberValue::FiniteCf`、`NumberValue::FiniteCf -> Fraction/ratio` 変換を追加する。
3. `Value::as_number_value()` または同等の helper を追加する。
4. 表示器へ NumberValue 経路を追加する。
5. 既存 `Fraction` 表示との互換を保ちながら、新規テストを追加する。

### Phase 3: Debug/学習用 word 追加

作業:

1. `CF` を追加する。
2. `TERMS` を追加する。
3. `CONVERGENT` を追加する。
4. `DECIMAL` は明示精度必須で追加する。実装負荷が大きい場合は Phase 3b に分ける。

### Phase 4: 算術・比較境界の NumberValue 化

作業:

1. `+ - * /` の scalar path を `NumberValue` API 経由にする。
2. 結果は `NumberValue::FiniteCf` へ戻す。
3. Tensor broadcast path は既存 `Fraction` path を維持してよいが、新規依存は helper 経由に寄せる。
4. `EXACT_EQ` と `STRUCT_EQ` の意味差をテストで固定する。

### Phase 5: Tensor 統合設計

作業:

1. `TensorSemantic` と `TensorStorage<AjisaiCell>` の設計メモを追加する。
2. 現行 `DenseTensor` small fraction SoA と新 `NumberValue` の責務境界を明記する。
3. BigInt 対応 Tensor storage の導入前に `DenseTensor` を削除しない。
4. Vector を Tensor の特殊形へ統合する移行計画を別文書または dev doc に残す。

## 6. テスト計画

### 6.1 係数列生成

- `3 -> [3]`
- `0 -> [0]`
- `-3 -> [-3]`
- `1/2 -> [0, 2]`
- `-1/2 -> [-1, 2]`
- `3/2 -> [1, 2]`
- `-3/2 -> [-2, 2]`
- `355/113 -> [3, 7, 16]`
- 末尾 `1` が残らないこと。
- 分母負値が正規化されること。
- `denominator == 0` が DivisionByZero へ変換されること。

### 6.2 復元

- `[3] -> 3/1`
- `[1, 2] -> 3/2`
- `[0, 2] -> 1/2`
- `[-2, 2] -> -3/2`
- `[3, 7, 16] -> 355/113`

### 6.3 右ネスト Vector 投影

- `[3] -> [3]`
- `[1, 2] -> [1 [2]]`
- `[3, 7, 16] -> [3 [7 [16]]]`
- `[]` は拒否。
- `[a0 []]` は拒否。
- `[a0 a1 a2]` は拒否。

### 6.4 表示

- `NumberValue::FiniteCf([3])` は `3`。
- `NumberValue::FiniteCf([1, 2])` は `3/2`。
- `NumberValue::FiniteCf([3, 7, 16])` は `355/113`。
- `CF` の出力は plain Vector として表示される。
- plain Vector `[3 [7 [16]]]` は通常数値表示に潰れない。

### 6.5 演算

- `1/2 + 1/3 = 5/6`
- `3/2 * 2/3 = 1`
- `355/113 - 3 = 16/113`
- `1 / 0` は bubble。
- 非数値への四則演算は error。

### 6.6 比較

- `1/2 < 2/3`
- `355/113 > 3`
- `1/2 EXACT_EQ 2/4`
- `[1 [1]] EXACT_EQ [2]` は debug/import context で true。
- `[1 [1]] STRUCT_EQ [2]` は false。

## 7. 受け入れ条件

### Phase 1 の Definition of Done

- `continued_fraction.rs` が追加されている。
- 正規係数列生成・復元・正規化・右ネスト投影・convergent の単体テストが通る。
- 既存 `Fraction` / `DenseTensor` テストが壊れていない。
- `,` を連分数構文として追加していない。

### Phase 2-4 の Definition of Done

- 通常 display は内部 CF 投影を表示しない。
- `CF` だけが plain nested vector 投影を返す。
- 四則演算の scalar result は finite CF semantic を持つ。
- 0 除算は bubble、型不整合は error になる。
- `EXACT_EQ` と `STRUCT_EQ` の差がテストで固定されている。

### Phase 5 の Definition of Done

- `DenseTensor` の small fraction SoA と新しい Number/Tensor semantic の関係が dev doc に明記されている。
- BigInt 連分数係数を扱える Tensor storage なしに現行 `DenseTensor` を削除していない。
- `TensorStorage<AjisaiCell>` への移行計画が段階化されている。

## 8. 最終仕様の要約

Ajisai の有限数値は、正規化された有限単純連分数 semantic を持つ `NumberValue` として扱う。

概念例:

```text
355/113
```

有限連分数係数列:

```text
[3, 7, 16]
```

`CF` による plain nested vector 投影:

```text
[ 3 [ 7 [ 16 ] ] ]
```

通常表示:

```text
355/113
```

重要な設計判断:

- `,` は Ajisai 表層の連分数構文には使わない。
- 右ネスト Vector は Phase 1-4 では canonical storage ではなく、明示 word による投影とする。
- 通常 Vector と数値 semantic を混同しない。
- `Fraction` は移行期間の計算 kernel として残してよいが、最終的な semantic API の正規表現にはしない。
- Tensor 統合は `TensorStorage<AjisaiCell>` 導入後の別 Phase とする。
- 「できなかった」は bubble、「使い方が違う」は error にする。
