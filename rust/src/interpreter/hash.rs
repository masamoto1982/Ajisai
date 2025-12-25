// rust/src/interpreter/hash.rs
//
// 【責務】
// 分数システムを活用した強力なハッシュ関数を実装する。
// HASH: 任意のAjisai値を決定論的にハッシュ化
//
// ============================================================================
// 【設計思想】分数システムを活用した効率的かつ強力なハッシュ
// ============================================================================
//
// Ajisaiの分数システムの特性を最大限活用したハッシュ関数：
//
// ## 従来のハッシュアプローチ
//
// 多くの言語では固定長の整数（32bit/64bit）をハッシュ値として返す：
//   hash("hello") → 0x1234ABCD
//
// ## Ajisaiのアプローチ：分数ハッシュ
//
// 分数として結果を返すことで：
//   1. ハッシュ値が [0, 1) の範囲に正規化される
//   2. 任意精度の出力ビット数を指定可能
//   3. 他の数学演算とシームレスに統合
//   4. 正規化された分数（1/2 = 2/4）は同じハッシュを生成
//
// ## アルゴリズム: 多項式モジュラーハッシュ
//
// 複数の大きな素数を使用し、中国剰余定理風の混合で強度を確保：
//   1. 入力値を正規バイト列にシリアライズ
//   2. バイト列を多項式の係数として解釈
//   3. 複数の素数で評価し、結果を混合
//   4. 分数（hash / 2^bits）として返す
//
// ## 使用例
//
// 'hello' HASH               # デフォルト256ビットハッシュ
// [ 1 2 3 ] HASH             # ベクタのハッシュ
// [ 1/2 ] HASH               # 分数のハッシュ（正規形を使用）
// [ 128 ] 'hello' HASH       # 128ビット出力
// [ 512 ] 'hello' HASH       # 512ビット出力
//
// ============================================================================

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive};

/// デフォルトのハッシュビット数
const DEFAULT_HASH_BITS: u32 = 256;

/// ハッシュ計算に使用する大きな素数群
/// これらの素数は互いに素で、十分に大きいため衝突耐性が高い
const PRIME_BITS: u32 = 127;

lazy_static::lazy_static! {
    /// 第1素数: 2^127 - 1 (メルセンヌ素数)
    static ref PRIME1: BigInt = BigInt::parse_bytes(
        b"170141183460469231731687303715884105727", 10
    ).unwrap();

    /// 第2素数: 2^127 - 73 (別の大きな素数)
    static ref PRIME2: BigInt = BigInt::parse_bytes(
        b"170141183460469231731687303715884105655", 10
    ).unwrap();

    /// 第3素数: 2^127 - 735 (さらに別の大きな素数)
    static ref PRIME3: BigInt = BigInt::parse_bytes(
        b"170141183460469231731687303715884104993", 10
    ).unwrap();

    /// 多項式ハッシュの基数
    static ref HASH_BASE: BigInt = BigInt::from(257u32);
}

/// 値を正規バイト列にシリアライズ
///
/// 分数の正規形を使用するため、1/2と2/4は同じバイト列を生成
fn serialize_value(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    serialize_value_inner(&value.val_type, &mut bytes);
    bytes
}

fn serialize_value_inner(val_type: &ValueType, bytes: &mut Vec<u8>) {
    match val_type {
        ValueType::Number(frac) => {
            // 型タグ: 数値
            bytes.push(0x01);
            // 符号
            if frac.numerator < BigInt::zero() {
                bytes.push(0x00); // 負
            } else {
                bytes.push(0x01); // 非負
            }
            // 分子の絶対値（正規形なので分母との共通因子はない）
            let num_bytes = if frac.numerator < BigInt::zero() {
                (-&frac.numerator).to_bytes_le().1
            } else {
                frac.numerator.to_bytes_le().1
            };
            bytes.extend_from_slice(&(num_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&num_bytes);
            // 分母（正規形では常に正）
            let den_bytes = frac.denominator.to_bytes_le().1;
            bytes.extend_from_slice(&(den_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&den_bytes);
        }
        ValueType::String(s) => {
            // 型タグ: 文字列
            bytes.push(0x02);
            bytes.extend_from_slice(&(s.len() as u32).to_le_bytes());
            bytes.extend_from_slice(s.as_bytes());
        }
        ValueType::Boolean(b) => {
            // 型タグ: 真偽値
            bytes.push(0x03);
            bytes.push(if *b { 0x01 } else { 0x00 });
        }
        ValueType::Vector(v) => {
            // 型タグ: ベクタ
            bytes.push(0x04);
            bytes.extend_from_slice(&(v.len() as u32).to_le_bytes());
            for elem in v {
                serialize_value_inner(&elem.val_type, bytes);
            }
        }
        ValueType::Symbol(s) => {
            // 型タグ: シンボル
            bytes.push(0x05);
            bytes.extend_from_slice(&(s.len() as u32).to_le_bytes());
            bytes.extend_from_slice(s.as_bytes());
        }
        ValueType::Nil => {
            // 型タグ: Nil
            bytes.push(0x06);
        }
        ValueType::DateTime(frac) => {
            // 型タグ: DateTime（内部的に分数として保存）
            bytes.push(0x07);
            // タイムスタンプ分数をシリアライズ
            let num_bytes = frac.numerator.to_bytes_le().1;
            bytes.extend_from_slice(&(num_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&num_bytes);
            let den_bytes = frac.denominator.to_bytes_le().1;
            bytes.extend_from_slice(&(den_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&den_bytes);
        }
    }
}

/// 多項式ハッシュを計算
///
/// bytes を多項式の係数として解釈し、HASH_BASE を変数として
/// 指定された素数でモジュロ評価する
fn polynomial_hash(bytes: &[u8], prime: &BigInt) -> BigInt {
    let mut hash = BigInt::zero();
    let mut power = BigInt::one();

    for &byte in bytes {
        // hash += byte * power (mod prime)
        hash = (&hash + &power * BigInt::from(byte)) % prime;
        // power *= HASH_BASE (mod prime)
        power = (&power * &*HASH_BASE) % prime;
    }

    hash
}

/// 複数の素数でハッシュを計算し、混合する
///
/// 中国剰余定理風の混合により、各素数のハッシュを結合して
/// より大きなハッシュ空間を生成
fn multi_prime_hash(bytes: &[u8], output_bits: u32) -> BigInt {
    let h1 = polynomial_hash(bytes, &PRIME1);
    let h2 = polynomial_hash(bytes, &PRIME2);
    let h3 = polynomial_hash(bytes, &PRIME3);

    // 各ハッシュを結合（ビットシフトと加算）
    let combined = &h1 + (&h2 << PRIME_BITS as usize) + (&h3 << (2 * PRIME_BITS) as usize);

    // 出力ビット数に調整
    let output_modulus = BigInt::one() << output_bits as usize;

    // 追加の混合: combined を output_modulus で割った余りを取る前に
    // さらにビット拡散を行う
    let mut result = combined.clone();

    // 自己フィードバック混合（より均一な分布のため）
    let shift1 = output_bits / 3;
    let shift2 = output_bits * 2 / 3;
    result = &result ^ (&result >> shift1 as usize);
    result = &result ^ (&result >> shift2 as usize);

    // 最終的に output_bits に収める
    result % output_modulus
}

/// スタックから整数を抽出（単一要素Vectorの数値）
fn extract_positive_integer(val: &Value) -> Option<u32> {
    match &val.val_type {
        ValueType::Vector(v) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) => {
                    // 整数かつ正数かチェック
                    if n.denominator == BigInt::one() && n.numerator > BigInt::from(0) {
                        n.numerator.to_u32()
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// HASH - 任意のAjisai値を決定論的にハッシュ化
///
/// 【責務】
/// - 任意のAjisai値（数値、文字列、ベクタ、真偽値など）をハッシュ化
/// - 同じ値は常に同じハッシュを生成（決定論的）
/// - 分数は正規形でハッシュ化（1/2と2/4は同じハッシュ）
///
/// 【使用法】
/// ```ajisai
/// 'hello' HASH              # デフォルト256ビットハッシュ
/// [ 1 2 3 ] HASH            # ベクタのハッシュ
/// [ 1/2 ] HASH              # 分数のハッシュ
/// [ 128 ] 'hello' HASH      # 128ビット出力
/// [ 512 ] [ 1 2 3 ] HASH    # 512ビット出力
/// ```
///
/// 【引数】
/// - 必須: ハッシュ対象の値（スタックトップ）
/// - オプション: [ ビット数 ] 出力ビット数（32～1024、デフォルト256）
///
/// 【戻り値】
/// - 単一要素のVector: [ ハッシュ値 / 2^bits ]
/// - ハッシュ値は [0, 1) の範囲の分数
///
/// 【エラー】
/// - スタックが空
/// - ビット数が32未満または1024超
pub fn op_hash(interp: &mut Interpreter) -> Result<()> {
    // HASHはStackモード(..)をサポートしない
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("HASH does not support Stack mode (..)"));
    }

    if interp.stack.is_empty() {
        return Err(AjisaiError::from("HASH requires a value to hash"));
    }

    // スタックから引数を解析
    // パターン1: 値のみ → デフォルト256ビット
    // パターン2: [ bits ] 値 → 指定ビット数
    let (output_bits, target_value) = parse_hash_args(interp)?;

    // ビット数の検証
    if output_bits < 32 || output_bits > 1024 {
        return Err(AjisaiError::from(
            "HASH: output bits must be between 32 and 1024"
        ));
    }

    // 値をシリアライズ
    let bytes = serialize_value(&target_value);

    // ハッシュを計算
    let hash_value = multi_prime_hash(&bytes, output_bits);

    // 分数として結果を構築: hash_value / 2^output_bits
    let denominator = BigInt::one() << output_bits as usize;
    let result_fraction = Fraction::new(hash_value, denominator);

    // 結果をスタックにプッシュ
    interp.stack.push(Value::from_vector(vec![
        Value::from_number(result_fraction)
    ]));

    Ok(())
}

/// HASHの引数を解析
fn parse_hash_args(interp: &mut Interpreter) -> Result<(u32, Value)> {
    // スタックトップを確認
    let target = interp.stack.pop().unwrap();

    // スタックが空なら、targetがハッシュ対象
    if interp.stack.is_empty() {
        return Ok((DEFAULT_HASH_BITS, target));
    }

    // 次の要素が整数（ビット数指定）かチェック
    if let Some(bits_val) = interp.stack.last() {
        if let Some(bits) = extract_positive_integer(bits_val) {
            // ビット数指定あり
            interp.stack.pop();
            return Ok((bits, target));
        }
    }

    // 整数でなければ、ビット数指定なし
    Ok((DEFAULT_HASH_BITS, target))
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueType;
    use crate::types::fraction::Fraction;
    use num_bigint::BigInt;
    use num_traits::One;

    #[tokio::test]
    async fn test_hash_rejects_stack_mode() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'hello' .. HASH").await;
        assert!(result.is_err(), "HASH should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("HASH") && err_msg.contains("Stack mode"),
                "Expected Stack mode error for HASH, got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_hash_string() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'hello' HASH").await;
        assert!(result.is_ok(), "HASH should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 結果が[0, 1)の範囲の分数であることを確認
        if let ValueType::Vector(v) = &interp.stack[0].val_type {
            assert_eq!(v.len(), 1);
            if let ValueType::Number(frac) = &v[0].val_type {
                let zero = Fraction::new(BigInt::from(0), BigInt::one());
                let one = Fraction::new(BigInt::one(), BigInt::one());
                assert!(*frac >= zero && *frac < one, "Hash should be in [0, 1)");
            } else {
                panic!("Expected Number");
            }
        } else {
            panic!("Expected Vector");
        }
    }

    #[tokio::test]
    async fn test_hash_deterministic() {
        let mut interp = Interpreter::new();

        // 同じ入力は同じハッシュを生成
        interp.execute("'hello' HASH").await.unwrap();
        let hash1 = interp.stack.pop().unwrap();

        interp.execute("'hello' HASH").await.unwrap();
        let hash2 = interp.stack.pop().unwrap();

        assert_eq!(hash1.val_type, hash2.val_type, "Same input should produce same hash");
    }

    #[tokio::test]
    async fn test_hash_different_inputs() {
        let mut interp = Interpreter::new();

        interp.execute("'hello' HASH").await.unwrap();
        let hash1 = interp.stack.pop().unwrap();

        interp.execute("'world' HASH").await.unwrap();
        let hash2 = interp.stack.pop().unwrap();

        assert_ne!(hash1.val_type, hash2.val_type, "Different inputs should produce different hashes");
    }

    #[tokio::test]
    async fn test_hash_vector() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] HASH").await;
        assert!(result.is_ok(), "HASH on vector should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_fraction_normalization() {
        let mut interp = Interpreter::new();

        // 1/2 と 2/4 は同じ正規形なので同じハッシュ
        interp.execute("[ 1/2 ] HASH").await.unwrap();
        let hash1 = interp.stack.pop().unwrap();

        interp.execute("[ 2/4 ] HASH").await.unwrap();
        let hash2 = interp.stack.pop().unwrap();

        assert_eq!(hash1.val_type, hash2.val_type,
                   "Equivalent fractions should produce same hash (1/2 = 2/4)");
    }

    #[tokio::test]
    async fn test_hash_with_bit_specification() {
        let mut interp = Interpreter::new();

        // 128ビット出力
        let result = interp.execute("[ 128 ] 'hello' HASH").await;
        assert!(result.is_ok(), "HASH with bit spec should succeed: {:?}", result);

        // 結果が[0, 1)の範囲の分数であることを確認
        // 注: Fraction::new()は自動約分するため、分母が正確に2^128とは限らない
        if let ValueType::Vector(v) = &interp.stack[0].val_type {
            assert_eq!(v.len(), 1);
            if let ValueType::Number(frac) = &v[0].val_type {
                let zero = Fraction::new(BigInt::from(0), BigInt::one());
                let one = Fraction::new(BigInt::one(), BigInt::one());
                assert!(*frac >= zero && *frac < one, "Hash should be in [0, 1)");
                // 分母が2^128の約数であることを確認
                let max_denom = BigInt::one() << 128usize;
                assert!(&max_denom % &frac.denominator == BigInt::from(0),
                        "Denominator should divide 2^128");
            } else {
                panic!("Expected Number");
            }
        } else {
            panic!("Expected Vector");
        }
    }

    #[tokio::test]
    async fn test_hash_boolean() {
        let mut interp = Interpreter::new();

        interp.execute("[ TRUE ] HASH").await.unwrap();
        let hash_true = interp.stack.pop().unwrap();

        interp.execute("[ FALSE ] HASH").await.unwrap();
        let hash_false = interp.stack.pop().unwrap();

        assert_ne!(hash_true.val_type, hash_false.val_type,
                   "TRUE and FALSE should have different hashes");
    }

    #[tokio::test]
    async fn test_hash_nested_vector() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ 1 2 ] [ 3 4 ] ] HASH").await;
        assert!(result.is_ok(), "HASH on nested vector should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_empty_string() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'' HASH").await;
        assert!(result.is_ok(), "HASH on empty string should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_empty_vector() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ ] HASH").await;
        assert!(result.is_ok(), "HASH on empty vector should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_preserves_stack() {
        let mut interp = Interpreter::new();
        // 非整数値がスタックにあっても影響しない（整数はビット数として解釈される可能性がある）
        interp.execute("[ 1/2 ] 'hello' HASH").await.unwrap();

        // スタックには [1/2] と ハッシュ結果 の2つ
        assert_eq!(interp.stack.len(), 2);
    }

    #[tokio::test]
    async fn test_hash_bits_consumed() {
        let mut interp = Interpreter::new();
        // 整数はビット数として消費される
        interp.execute("[ 128 ] 'hello' HASH").await.unwrap();

        // スタックにはハッシュ結果のみ（[128]は消費された）
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_invalid_bits() {
        let mut interp = Interpreter::new();

        // 32未満はエラー
        let result = interp.execute("[ 16 ] 'hello' HASH").await;
        assert!(result.is_err(), "Bits < 32 should error");

        // 1024超もエラー
        let result = interp.execute("[ 2048 ] 'hello' HASH").await;
        assert!(result.is_err(), "Bits > 1024 should error");
    }

    #[tokio::test]
    async fn test_hash_distribution() {
        let mut interp = Interpreter::new();

        // 複数の異なる入力をハッシュし、すべて異なることを確認
        let inputs = ["a", "b", "c", "aa", "ab", "abc"];
        let mut hashes = Vec::new();

        for input in inputs {
            interp.execute(&format!("'{}' HASH", input)).await.unwrap();
            hashes.push(interp.stack.pop().unwrap());
        }

        // すべて異なることを確認
        for i in 0..hashes.len() {
            for j in (i+1)..hashes.len() {
                assert_ne!(hashes[i].val_type, hashes[j].val_type,
                           "Different inputs should have different hashes");
            }
        }
    }
}
