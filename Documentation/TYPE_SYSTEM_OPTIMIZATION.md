# 型システム最適化の設計提案

## 現状の分析

### 問題点: 過剰なラッピング

現在の実装では、すべてのスカラー値が単一要素ベクタでラップされています：

```rust
// rust/src/interpreter/mod.rs:231
Token::Number(n) => {
    let val = Value { val_type: ValueType::Number(...) };
    self.stack.push(Value { val_type: ValueType::Vector(vec![val], ...) });
    //                                                  ^^^^^^^^  過剰なラッピング
}
```

**影響:**
- すべての値操作で wrap/unwrap が必要
- 不要なメモリアロケーション (Vec::new() が頻繁に発生)
- パフォーマンスのオーバーヘッド

## 設計思想の考察

**なぜこの設計なのか？**

1. **統一的な処理**: すべてをベクタとして扱うことで、演算子が統一的に動作
2. **ベクタ演算の簡潔性**: 要素ごとの演算とブロードキャストが自然に表現できる
3. **FORTH的な設計**: スタック上のすべてが同じ型（ベクタ）

**トレードオフ:**
- メモリ効率 vs コードの簡潔性
- パフォーマンス vs 設計の一貫性

## 最適化アプローチ

### アプローチ1: 内部最適化（推奨・安全）

言語仕様を変えず、内部実装を最適化：

#### 1.1 スモールベクタ最適化 (Small Vec Optimization)

```rust
pub enum ValueType {
    Number(Fraction),
    String(String),
    Boolean(bool),
    Symbol(String),
    // 単一要素ベクタの最適化版
    SingletonVector(Box<Value>),  // Box で1要素のみ保持
    Vector(Vec<Value>, BracketType),
    Nil,
}
```

**利点:**
- `Vec::new()` の代わりに `Box::new()` (1アロケーション削減)
- メモリ使用量が約50%削減
- API互換性を保持

**実装例:**
```rust
// ヘルパー関数
pub fn wrap_in_square_vector(value: Value) -> Value {
    Value { val_type: ValueType::SingletonVector(Box::new(value)) }
}
```

#### 1.2 Copy-on-Write (COW) の活用

```rust
use std::rc::Rc;

pub enum ValueType {
    Number(Fraction),
    String(Rc<String>),  // 文字列の共有
    // ...
    Vector(Rc<Vec<Value>>, BracketType),  // ベクタの共有
}
```

**利点:**
- イミュータブルな操作でのコピー削減
- 大きなベクタの処理が高速化

**注意:**
- 変更時のクローンコストが発生
- Rustの所有権モデルとの相性を検討

#### 1.3 参照の活用

```rust
// 現状: 値をコピー
pub fn extract_number(val: &Value) -> Result<&Fraction> { ... }

// 改善: 参照を返す
pub fn extract_number_ref(val: &Value) -> Result<&Fraction> { ... }
```

**利点:**
- 読み取り専用操作でのパフォーマンス向上
- メモリコピーの削減

---

### アプローチ2: 設計変更（影響大・慎重に検討）

#### 2.1 スカラー値の直接保持

```rust
pub enum StackValue {
    Scalar(Value),      // スカラーは直接保持
    Vector(Vec<Value>), // ベクタのみVec
}

pub type Stack = Vec<StackValue>;
```

**利点:**
- メモリ使用量が大幅削減 (50-70%)
- パフォーマンス向上

**欠点:**
- 言語仕様の変更が必要
- すべての演算子の実装を見直し
- 後方互換性の喪失

#### 2.2 タグ付きユニオン (Tagged Union)

```rust
pub struct Value {
    // 最適化: 単一要素フラグ
    is_singleton: bool,
    val_type: ValueType,
}
```

**利点:**
- 既存のAPIを大部分保持
- 内部で最適化を切り替え可能

---

## 推奨実装計画

### Phase 1: 測定とベンチマーク (1-2週間)

```rust
// ベンチマーク追加
#[bench]
fn bench_value_creation(b: &mut Bencher) {
    b.iter(|| {
        let val = Value { val_type: ValueType::Number(Fraction::from(42)) };
        wrap_in_square_vector(val)
    });
}
```

**目標:**
- 現状のボトルネック特定
- 最適化の効果を定量化

### Phase 2: SingletonVector の導入 (2-3週間)

1. `ValueType::SingletonVector` を追加
2. `wrap_in_square_vector()` を `SingletonVector` 使用に変更
3. `unwrap_single_element()` を最適化
4. 全テストで動作確認

**期待効果:**
- メモリ使用量: -40%
- 値作成速度: +30%

### Phase 3: 参照最適化 (1-2週間)

1. 読み取り専用ヘルパー関数を参照版に変更
2. 演算子で参照を活用
3. パフォーマンス測定

**期待効果:**
- 比較演算: +20%
- 算術演算: +15%

---

## 実装上の注意点

### 1. 後方互換性

- WASM APIは変更しない
- JavaScript側の影響を最小化
- 段階的なロールアウト

### 2. テストの充実

```rust
#[test]
fn test_singleton_optimization() {
    let val = wrap_in_square_vector(Value::number(42));
    assert!(matches!(val.val_type, ValueType::SingletonVector(_)));
}
```

### 3. パフォーマンス監視

- CI/CDにベンチマークを統合
- リグレッションの早期検出

---

## 代替案: 他の言語の事例

### Julia言語のアプローチ

```julia
# スカラーとベクタを明確に区別
x = 5        # スカラー
y = [5]      # 1要素ベクタ
z = [1, 2]   # ベクタ
```

**Ajisaiへの適用:**
- 現在の設計に近い
- ユーザーは明示的にベクタを作成

### APL/J言語のアプローチ

```apl
5 ←→ ,5  ⍝ スカラーとラベルされた配列は異なる
```

**Ajisaiへの適用:**
- 内部的に区別、外部的に統一
- 現在の設計思想と一致

---

## 結論

**短期的推奨:**
- **Phase 1 (測定)** を即座に実施
- **Phase 2 (SingletonVector)** を次のメジャーバージョンで導入

**長期的検討:**
- ユーザーフィードバックを収集
- パフォーマンスプロファイルを継続的に監視
- 必要に応じて Phase 3 を実施

**リスク評価:**
- Phase 1-2: リスク低、効果中
- Phase 3: リスク中、効果高

---

## 参考文献

- [Small Vec Optimization in Rust](https://doc.rust-lang.org/std/vec/struct.Vec.html)
- [Copy-on-Write Pattern](https://en.wikipedia.org/wiki/Copy-on-write)
- [Rc vs Arc in Rust](https://doc.rust-lang.org/std/rc/struct.Rc.html)

---

**文書作成日:** 2025-11-16
**作成者:** Claude (設計レビュー)
**ステータス:** 提案 (未実装)
