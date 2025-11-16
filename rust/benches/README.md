# Ajisai ベンチマークスイート

このディレクトリには、Ajisai言語インタプリタのパフォーマンス測定用ベンチマークが含まれています。

## 📁 構成

```
benches/
├── value_operations.rs        # 値操作のベンチマーク
├── interpreter_operations.rs  # インタプリタ実行のベンチマーク
└── README.md                   # このファイル
```

## 🚨 重要な注意事項

**現在、これらのベンチマークは直接実行できません。**

### 理由

Ajisaiは WASM (WebAssembly) ターゲット用にコンパイルされており、`wasm-bindgen` を使用しています。`criterion` などの標準的なRustベンチマークツールはネイティブ環境で実行されるため、以下のエラーが発生します：

```
cannot call wasm-bindgen imported functions on non-wasm targets
```

## 🎯 目的

このベンチマークスイートは、将来の最適化のための **設計ドキュメント** として機能します：

1. **測定すべき項目の特定** - どの操作を測定すべきかを明確化
2. **ベースライン確立** - 最適化前の参照ポイント
3. **回帰テスト** - 最適化後のパフォーマンス比較

## 📊 ベンチマーク項目

### 1. value_operations.rs

値の作成と操作に関するベンチマーク：

| ベンチマーク | 測定内容 | 重要性 |
|------------|---------|--------|
| `number_creation` | 数値リテラルの作成 | ★★★ 最頻出操作 |
| `vector_creation` | ベクタの作成（1-100要素） | ★★★ スケーラビリティ |
| `string_creation` | 文字列リテラルの作成 | ★★☆ |
| `mixed_values` | 異なる型の値の混在 | ★★☆ リアルワールド |
| `nested_vectors` | ネストされたベクタ（深さ1-5） | ★★★ 再帰的処理 |
| `stack_push_pop` | スタック操作（50回） | ★★★ オーバーヘッド測定 |

**重点測定:**
- `wrap_in_square_vector()` のアロケーション回数
- メモリ使用量パターン

### 2. interpreter_operations.rs

実際のコード実行のベンチマーク：

| ベンチマーク | 測定内容 | 重要性 |
|------------|---------|--------|
| `vector_get` | GET操作 | ★★★ 頻出操作 |
| `vector_concat` | CONCAT操作 | ★★☆ |
| `arithmetic_add` | 加算（要素ごと/ブロードキャスト） | ★★★ 算術演算 |
| `map` | MAP高階関数 | ★★★ 関数型プログラミング |
| `custom_word_execution` | カスタムワード実行 | ★★☆ ユーザー定義 |

**重点測定:**
- ベクタ操作の時間計算量
- 高階関数のオーバーヘッド

## 🔧 代替測定手法

ベンチマークが直接実行できないため、以下の代替手法を推奨します：

### A. プロファイリングツール

```bash
# Flamegraphの生成（要: flamegraph crate）
cargo flamegraph --dev --bin example_program

# Chrome DevTools でWASMプロファイリング
# 1. npm run dev でアプリを起動
# 2. Chrome DevToolsの Performanceタブで記録
```

### B. メモリプロファイリング

```bash
# ヒープ使用量の測定（Linux）
valgrind --tool=massif cargo test

# メモリ割り当ての追跡
cargo build --release
heaptrack ./target/release/example
```

### C. ユニットベンチマーク

WASM非依存の内部関数のみをベンチマーク：

```rust
// 例: Fraction演算のベンチマーク
#[bench]
fn bench_fraction_add(b: &mut test::Bencher) {
    use ajisai_core::types::fraction::Fraction;
    let f1 = Fraction::from(42);
    let f2 = Fraction::from(10);
    b.iter(|| f1.add(&f2));
}
```

## 📈 期待される結果

最適化後の目標値：

| 項目 | 現状（推定） | 目標 | 改善率 |
|-----|------------|------|--------|
| メモリ使用量 | 100% | 60-70% | -30~40% |
| 値作成速度 | 100% | 120-130% | +20~30% |
| ベクタ操作 | 100% | 115-125% | +15~25% |
| 算術演算 | 100% | 110-120% | +10~20% |

## 🚀 今後の計画

### Phase 2: 実行可能なベンチマークの導入

1. **WASMベンチマークフレームワーク**
   - `wasm-pack test` を使用
   - `web-sys::Performance::now()` で時間測定

2. **ユニットベンチマークの実装**
   - WASM非依存の関数を分離
   - `criterion` で測定可能に

3. **CI/CD統合**
   - GitHub Actionsでベンチマークを自動実行
   - パフォーマンスリグレッションの検出

## 📚 参考資料

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [WebAssembly Performance](https://rustwasm.github.io/book/reference/code-size.html)
- [型システム最適化設計](../../Documentation/TYPE_SYSTEM_OPTIMIZATION.md)
- [パフォーマンスベースラインレポート](../../Documentation/PERFORMANCE_BASELINE_REPORT.md)

## 📝 まとめ

このベンチマークスイートは、**将来の最適化作業のための青写真**です。実際の測定は代替手法を使用してください。

最適化の実装後、このベンチマークを参考にパフォーマンス向上を検証します。
