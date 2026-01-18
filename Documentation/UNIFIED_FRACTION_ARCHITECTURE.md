# 統一分数アーキテクチャ (Unified Fraction Architecture)

## 概要

Ajisaiは**統一分数アーキテクチャ**を採用しています。これは、すべての値を内部的に `Vec<Fraction>` として統一表現し、従来の「型」という概念を完全に廃止した設計です。

```
設計フィロソフィー:
従来の言語：データ → 型チェック → 演算
Ajisai：データ → 演算（型チェックなし）→ 表示時のみ解釈
```

---

## Value構造体

```rust
pub struct Value {
    pub data: Vec<Fraction>,       // 純粋な分数の配列（唯一の真実）
    pub display_hint: DisplayHint, // 表示ヒント（演算には使用しない）
    pub shape: Vec<usize>,         // 多次元配列の形状情報
}
```

### フィールドの役割

| フィールド | 役割 | 演算への影響 |
|-----------|------|-------------|
| `data` | 実際のデータ（分数配列） | **唯一の計算対象** |
| `display_hint` | 表示形式のヒント | なし（表示時のみ参照） |
| `shape` | 多次元配列の形状 | RESHAPE等の形状操作のみ |

---

## 内部表現

すべてのユーザー入力は `Vec<Fraction>` に変換されます：

| ユーザー入力 | 内部表現 (`data`) | 表示 |
|-------------|-------------------|------|
| `42` | `[42/1]` | `[ 42 ]` |
| `1/3` | `[1/3]` | `[ 1/3 ]` |
| `TRUE` | `[1/1]` | `TRUE` |
| `FALSE` | `[0/1]` | `FALSE` |
| `'A'` | `[65/1]` | `'A'` |
| `'Hello'` | `[72/1, 101/1, 108/1, 108/1, 111/1]` | `'Hello'` |
| `[ 1 2 3 ]` | `[1/1, 2/1, 3/1]` | `[ 1 2 3 ]` |
| `NIL` | `[0/0]` (センチネル値) | `NIL` |
| `[ ]` | **エラー** | — |

---

## DisplayHint

`DisplayHint` は**表示専用**の情報であり、演算には一切使用しません。

```rust
pub enum DisplayHint {
    Auto,      // 自動判定
    Number,    // 数値として表示
    String,    // 文字列として表示
    Boolean,   // 真偽値として表示
    DateTime,  // 日時として表示
}
```

### 重要な原則

1. **演算は `data` のみを参照** - `display_hint` は無視
2. **表示時のみ `display_hint` を参照** - フォーマットを決定
3. **形式変換ワード（STR, NUM等）は `display_hint` を変更** - `data` は必要に応じて変換

---

## Fraction型

すべての数値は `Fraction` として表現されます：

```rust
pub struct Fraction {
    pub numerator: BigInt,   // 分子（任意精度）
    pub denominator: BigInt, // 分母（任意精度）
}
```

### 特徴

- **任意精度**: `num-bigint` クレートによる無制限精度
- **自動簡約**: GCD計算により常に最簡形を維持
- **交差簡約**: 乗算時の事前約分による最適化

### サポートする数値形式

```
整数:     42, -10
小数:     1.5, .5, 123.456
分数:     1/3, -5/7
指数:     1e10, 1.5e-3
```

---

## エラーハンドリング

統一分数アーキテクチャでは「型エラー」は存在しません。代わりに**構造エラー**を使用します：

```rust
pub enum AjisaiError {
    StackUnderflow,
    StructureError { expected: String, got: String },  // 旧: TypeError
    UnknownWord(String),
    DivisionByZero,
    IndexOutOfBounds { index: i64, length: usize },
    VectorLengthMismatch { len1: usize, len2: usize },
    Custom(String),
}
```

### エラーメッセージ例

```
# 旧（型ベース）
Type error: expected vector, got other type

# 新（構造ベース）
Structure error: expected vector, got other format
```

---

## 形式変換ワード

「型変換」ではなく「形式変換」として機能します：

| ワード | 機能 | エラー条件 |
|--------|------|-----------|
| `STR` | 文字列形式に変換 | 既に文字列形式の場合 |
| `NUM` | 数値形式に変換 | 既に数値形式の場合 |
| `BOOL` | 真偽値形式に変換 | 既に真偽値形式の場合 |
| `NIL` | NILに変換 | 既にNILの場合 |
| `CHARS` | 文字列を文字配列に分解 | 文字列形式でない場合 |
| `JOIN` | 文字配列を文字列に結合 | ベクタでない場合 |

### エラーメッセージ

```
STR: value is already in string format
NUM: value is already in number format
BOOL: value is already in boolean format
NIL: value is already nil
```

---

## 設計思想

### FORTH精神の継承

FORTHは「プログラマーを信頼する」哲学を持ちます。Ajisaiはこれを継承し：

1. **型チェックの廃止** - プログラマーが責任を持つ
2. **自由な演算** - すべての値に対してすべての演算が試行可能
3. **軽量な実行** - 型チェックのオーバーヘッドなし

### データの真実

```
唯一の真実: Vec<Fraction>
すべての解釈: 表示時に決定
```

これにより：
- メモリ効率の向上
- 演算の単純化
- 柔軟なデータ操作

---

## コメントシステムとの調和

`#` によるコメントシステムは、統一分数アーキテクチャと完全に調和しています：

```ajisai
123#これはコメント     # → [ 123 ]
1/3#分数の後のコメント  # → [ 1/3 ]
'#文字列内は保護'       # → '#文字列内は保護'
```

### トークナイザーの設計

`#` は `is_special_char()` でトークン境界として定義されているため：
1. 数値読み取り時に `#` で停止
2. 分数リテラル `1/3` も正しく認識
3. 文字列内の `#` は保護される

---

## 将来の拡張

### 行列演算（計画中）

`tensor.rs` に以下の関数が準備されています：

- `infer_shape` - 形状推論
- `transpose` - 転置
- `reshape` - 形状変更
- `flatten_to_numbers` - 平坦化
- `rank` - 次元数取得

これらは `shape` フィールドを活用した行列演算ワード（TRANSPOSE, RESHAPE等）で使用予定です。

---

## 参考

- [FORTH言語](https://en.wikipedia.org/wiki/Forth_(programming_language))
- [APL言語の配列モデル](https://en.wikipedia.org/wiki/APL_(programming_language))
- [num-bigint クレート](https://docs.rs/num-bigint/)

---

**文書作成日:** 2025-01-13
**アーキテクチャバージョン:** 2.0 (統一分数アーキテクチャ)
**ステータス:** 実装完了
