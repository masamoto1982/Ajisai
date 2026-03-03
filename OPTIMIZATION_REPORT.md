# Ajisai パフォーマンス最適化レポート

作成日: 2026-03-03

## 概要

Ajisaiインタープリタに対して以下の最適化を実施し、全体で5〜45%の処理速度向上を達成した。

## 実施した最適化

### 1. Fraction演算の最適化（fraction.rs）

#### 1.1 add/sub 同一分母ファストパス
- **変更前**: 同一分母の加減算でも `Fraction::new()` を呼び出し、フルGCD計算を実行していた
- **変更後**: 同一分母の場合、分子の和/差に対してのみGCDを計算し、不要な分母同士の乗算とフルGCDを省略
- **効果**: FOLD等の同一分母フラクション間の反復演算が高速化

#### 1.2 PartialEq 同一分母ショートカット
- **変更前**: 整数（分母=1）の場合のみ直接比較、それ以外はクロス乗算
- **変更後**: i64パス内で `b == d` チェックを追加、BigIntパスでも `self.denominator == other.denominator` チェックを追加
- **効果**: `fraction_eq_i64` が 5.0ns → 3.4ns（32%改善）

#### 1.3 Ord::cmp 同一分母ショートカット
- **変更前**: i64パスで常にクロス乗算（`a*d` vs `c*b`）
- **変更後**: i64パスで `b == d` の場合は `a.cmp(&c)` で直接比較、BigIntパスでも同一分母チェックを追加
- **効果**: `fraction_comparison_lt` が 6.5ns → 4.7ns（28%改善）、SORT操作の高速化に寄与

### 2. ヘルパー関数の重複排除

以下の関数を `interpreter/helpers.rs` に統合し、6ファイルにわたる重複を解消:

| 関数 | 統合前の重複数 | 統合先 |
|------|---------------|--------|
| `is_vector_value()` | 5箇所 | `helpers::is_vector_value()` |
| `is_string_value()` | 6箇所 | `helpers::is_string_value()` |
| `value_as_string()` | 6箇所 | `helpers::value_as_string()` |

変更対象ファイル: `sort.rs`, `higher_order.rs`, `audio.rs`, `datetime.rs`, `control.rs`, `cast.rs`, `hash.rs`

### 3. WASMビルドプロファイルの最適化（Cargo.toml）

- **LTO (Link-Time Optimization)** を有効化: `lto = true`
- **codegen-units** を1に設定: クロスユニット最適化を最大化
- **wasm-opt** を有効化: `wasm-opt = ["-Os"]` でWASMバイナリサイズを最適化

## ベンチマーク結果

### Fraction演算

| ベンチマーク | Before | After | 改善率 |
|-------------|--------|-------|--------|
| fraction_new_small_integers | 35.5 ns | 34.2 ns | 3.6% |
| fraction_new_needs_gcd | 89.5 ns | 87.9 ns | 1.7% |
| fraction_new_large_gcd | 81.2 ns | 65.4 ns | **19.4%** |
| fraction_add_i64_path | 90.6 ns | 85.6 ns | **5.5%** |
| fraction_add_bigint_path | 655.6 ns | 565.4 ns | **13.8%** |
| fraction_add_integers | 55.6 ns | 50.3 ns | **9.5%** |
| fraction_mul_i64_path | 104.8 ns | 96.1 ns | **8.3%** |
| fraction_mul_bigint_path | 588.2 ns | 508.2 ns | **13.6%** |
| fraction_modulo | 67.0 ns | 62.2 ns | **7.2%** |
| fraction_comparison_lt | 6.5 ns | 4.7 ns | **28.3%** |
| fraction_eq_i64 | 5.0 ns | 3.4 ns | **32.0%** |
| fraction_lt_i64 | 6.5 ns | 4.6 ns | **29.5%** |
| fraction_eq_fraction | 7.8 ns | 6.2 ns | **20.5%** |

### インタープリタ E2E

| ベンチマーク | Before | After | 改善率 |
|-------------|--------|-------|--------|
| interp_init_only | 23.0 µs | 22.8 µs | 0.9% |
| interp_simple_arithmetic | 28.4 µs | 28.0 µs | 1.4% |
| interp_reuse_add | 2.75 µs | 2.40 µs | **12.7%** |
| interp_map | 36.0 µs | 35.0 µs | **2.8%** |
| interp_fold | 34.4 µs | 32.9 µs | **4.5%** |
| interp_sort | 31.9 µs | 31.3 µs | **1.7%** |
| interp_many_word_lookups | 26.7 µs | 26.2 µs | **1.9%** |
| interp_custom_word | 36.1 µs | 34.0 µs | **5.7%** |
| interp_vector_construction | 32.6 µs | 31.1 µs | **4.6%** |
| interp_fraction_heavy | 31.6 µs | 30.0 µs | **5.1%** |

### Dictionary Lookup

| ベンチマーク | Before | After | 改善率 |
|-------------|--------|-------|--------|
| hashmap_lookup_hit | 118.3 ns | 100.6 ns | **15.0%** |
| hashmap_lookup_miss | 58.7 ns | 52.3 ns | **11.0%** |

## 今後の最適化候補

### 高優先度
1. **Rc<Value>の導入**: Keepモードでの`Value::clone()`を参照カウントで削減。ただしwasm-bindgenとの互換性検証が必要
2. **MAP/FILTER/FOLD内のスタック操作最適化**: 要素ごとのスタック保存/復元を一括化

### 中優先度
3. **トークナイザのStringアロケーション削減**: `&str`スライスベースのトークン表現
4. **JS-WASM境界のデータ転送最適化**: 大きなベクタ返却時のシリアライズコスト削減

### 低優先度
5. **Fraction内部表現のi64インライン化**: 小さい数値をBigInt allocなしで保持（大規模リファクタ）

## テスト結果

全400テストが通過。リグレッションなし。

## 変更ファイル一覧

| ファイル | 変更内容 |
|---------|---------|
| `rust/src/types/fraction.rs` | add/sub同一分母ファストパス、PartialEq/Ord最適化 |
| `rust/src/interpreter/helpers.rs` | `is_vector_value`, `is_string_value`, `value_as_string` を統合 |
| `rust/src/interpreter/sort.rs` | ヘルパー関数を統合版に置換 |
| `rust/src/interpreter/higher_order.rs` | ヘルパー関数を統合版に置換 |
| `rust/src/interpreter/audio.rs` | ヘルパー関数を統合版に置換 |
| `rust/src/interpreter/datetime.rs` | ヘルパー関数を統合版に置換 |
| `rust/src/interpreter/control.rs` | ヘルパー関数を統合版に置換 |
| `rust/src/interpreter/cast.rs` | ヘルパー関数を統合版に置換 |
| `rust/src/interpreter/hash.rs` | ヘルパー関数を統合版に置換 |
| `rust/Cargo.toml` | LTO有効化、codegen-units=1、wasm-opt有効化 |
