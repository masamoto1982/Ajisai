# Ajisai パフォーマンスベースライン測定レポート

**作成日:** 2025-11-16
**ステータス:** Phase 1 完了 (ベンチマーク基盤整備)
**次のステップ:** WASM対応のベンチマーク手法の導入

---

## 📊 目的

型システム最適化の効果を定量的に測定するため、現状のパフォーマンスベースラインを確立する。

---

## 🔧 実施内容

### 1. ベンチマークツールの導入

**追加したツール:**
- `criterion` v0.5 - Rustの標準的なベンチマークフレームワーク
- HTML形式のレポート生成機能を有効化

**Cargo.toml の変更:**
```toml
[dev-dependencies]
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "value_operations"
harness = false

[[bench]]
name = "interpreter_operations"
harness = false
```

### 2. ベンチマークスイートの作成

#### A. `value_operations.rs` - 値操作のベンチマーク

測定対象：
1. **number_creation** - 数値リテラルの作成とスタックプッシュ
2. **vector_creation** - ベクタの作成（サイズ: 1, 10, 100要素）
3. **string_creation** - 文字列リテラルの作成
4. **mixed_values** - 異なる型の値を混在
5. **nested_vectors** - ネストされたベクタ（深さ: 1, 3, 5）
6. **stack_push_pop** - スタック操作のオーバーヘッド（50回）

**重点測定項目:**
- `wrap_in_square_vector()` のアロケーション回数
- ベクタの作成コスト
- メモリ使用量パターン

#### B. `interpreter_operations.rs` - インタプリタ操作のベンチマーク

測定対象：
1. **vector_get** - GET操作の性能
2. **vector_concat** - CONCAT操作の性能
3. **arithmetic_add** - 加算演算（要素ごと/ブロードキャスト）
4. **map** - MAP高階関数の性能
5. **custom_word_execution** - カスタムワード実行

**重点測定項目:**
- ベクタ操作の時間計算量
- 高階関数のオーバーヘッド
- カスタムワードの呼び出しコスト

---

## ⚠️ 技術的課題

### WASMターゲットの制約

**問題:**
```
cannot call wasm-bindgen imported functions on non-wasm targets
```

**原因:**
- Ajisaiは `wasm-bindgen` を使用してWASM向けにコンパイル
- ネイティブ環境では `js-sys` などの関数が利用できない
- `criterion` はネイティブ環境で実行されるため不整合

**影響:**
- 現在のベンチマークは実行不可
- 代替手法が必要

---

## 🔍 現状分析（静的解析ベース）

ベンチマークは実行できませんでしたが、コードレビューから以下のボトルネックを特定：

### 1. 過剰なアロケーション

**問題箇所:** `rust/src/interpreter/mod.rs:231`
```rust
Token::Number(n) => {
    let val = Value { val_type: ValueType::Number(...) };  // アロケーション1
    self.stack.push(Value {
        val_type: ValueType::Vector(vec![val], ...)       // アロケーション2 (Vec)
    });
}
```

**問題点:**
- すべてのスカラー値が `Vec` でラップされる
- 1つの数値リテラルで **2回のヒープアロケーション**
- `Vec::new() + push()` のオーバーヘッド

**推定コスト:**
- メモリ: 24バイト（Vecのヘッダ） + 要素サイズ
- 時間: ~50-100ns/値（環境依存）

### 2. 重複するラッピング/アンラッピング

**該当箇所:** `rust/src/interpreter/helpers.rs`

頻繁に呼び出される関数：
```rust
pub fn wrap_in_square_vector(value: Value) -> Value { ... }
pub fn unwrap_single_element(value: Value) -> Value { ... }
pub fn extract_single_element(vector_val: &Value) -> Result<&Value> { ... }
```

**使用頻度の推定:**
- 1つの演算で平均 **2-4回** の wrap/unwrap
- 100要素のベクタ演算で **200-400回**
- ネストが深いと exponential に増加

### 3. ベクタ操作のコピーコスト

**該当箇所:** `rust/src/interpreter/vector_ops.rs`

例: CONCAT操作
```rust
pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    // ... 各ベクタを Vec からコピー
    let mut result = Vec::new();
    for item in vec1 {
        result.push(item.clone());  // Deep copy
    }
    // ...
}
```

**問題点:**
- イミュータブルなベクタでも毎回クローン
- 大きなベクタでは O(n) のコピーコスト

---

## 📈 期待される改善効果（推定値）

### SingletonVector 最適化を導入した場合

**Before (現状):**
```rust
ValueType::Vector(vec![value], BracketType::Square)
// メモリ: 24 bytes (Vec) + 16 bytes (Value) = 40 bytes
// アロケーション: 2回
```

**After (最適化後):**
```rust
ValueType::SingletonVector(Box::new(value))
// メモリ: 8 bytes (Box pointer) + 16 bytes (Value) = 24 bytes
// アロケーション: 1回
```

**効果:**
- メモリ使用量: **-40%** (40 → 24 bytes)
- アロケーション回数: **-50%** (2 → 1回)
- 推定性能向上: **+20-30%** (値作成)

### Copy-on-Write (Rc) を導入した場合

**Before:**
```rust
Vec<Value>  // 常にクローン
```

**After:**
```rust
Rc<Vec<Value>>  // 参照カウント
```

**効果:**
- 読み取り専用操作: **+50-70%** (コピー不要)
- 変更操作: **-10%** (Rc::make_mut のオーバーヘッド)
- 平均: **+30-40%** (読み取りが多い想定)

---

## 🎯 代替ベンチマーク手法の提案

### アプローチ1: WASM環境でのベンチマーク

**ツール:**
- `wasm-pack test --headless --firefox`
- `web-sys::Performance::now()` で時間測定

**利点:**
- 実際の実行環境での測定
- WASM特有の最適化を反映

**欠点:**
- セットアップが複雑
- CI/CDへの統合が困難

### アプローチ2: ユニットベンチマーク

**方法:**
- WASMに依存しない純粋なRustコードのみベンチマーク
- 内部APIを直接テスト

**例:**
```rust
#[bench]
fn bench_fraction_operations(b: &mut Bencher) {
    let f1 = Fraction::from(42);
    let f2 = Fraction::from(10);
    b.iter(|| f1.add(&f2));
}
```

**利点:**
- criterion で測定可能
- 詳細なプロファイリング

**欠点:**
- エンドツーエンドの性能は測定できない

### アプローチ3: プロファイリングツール

**ツール:**
- `perf` (Linux)
- `flamegraph` - フレームグラフ生成
- Chrome DevTools (WASM)

**利点:**
- ボトルネックの可視化
- 実際のユースケースで測定

---

## 📋 推奨アクション

### 短期（1週間）

1. **ユニットベンチマークの実装**
   - `Fraction` 演算
   - `wrap_in_square_vector` / `unwrap_single_element`
   - ヘルパー関数群

2. **プロファイリングツールのセットアップ**
   - flamegraphの導入
   - サンプルコードでのプロファイリング

### 中期（2-3週間）

3. **SingletonVector プロトタイプ**
   - 新しい ValueType バリアントの実装
   - A/Bテストで効果測定

4. **メモリプロファイリング**
   - `valgrind --tool=massif`
   - ヒープ使用量の可視化

### 長期（1-2ヶ月）

5. **WASM ベンチマークスイート**
   - ブラウザ環境でのベンチマーク
   - CI/CDパイプラインへの統合

6. **継続的パフォーマンス監視**
   - リグレッション検出
   - パフォーマンス履歴の追跡

---

## 🔗 参考リソース

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [WASM Performance Best Practices](https://rustwasm.github.io/book/reference/code-size.html)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)

---

## 📝 結論

**Phase 1 の成果:**
- ✅ ベンチマーク基盤を整備
- ✅ 測定すべき項目を特定
- ✅ 技術的課題を明確化
- ✅ 静的解析で最適化ポイントを特定

**次のステップ:**
- ユニットベンチマークの実装（WASM非依存）
- プロファイリングツールでの実測
- 最適化プロトタイプの作成

**期待される成果:**
- メモリ使用量: **-30~40%**
- 実行速度: **+20~30%**
- 保守性: **向上** (コードの簡潔化)

---

**レポート作成者:** Claude (設計レビュー)
**承認待ち:** Phase 2 実装計画
