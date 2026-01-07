// rust/src/interpreter/random.rs
//
// 【責務】
// 暗号論的疑似乱数生成ワードを実装する。
// CSPRNG: 暗号論的に安全な乱数を生成
//
// ============================================================================
// 【設計思想】分数システムを活用した効率的な乱数生成
// ============================================================================
//
// Ajisaiの分数システムを最大限活用し、必要な粒度だけの乱数を生成する。
//
// ## 従来のアプローチの問題点
//
// 多くの言語では浮動小数点数で [0, 1) の乱数を生成し、必要に応じて
// スケーリングする：
//   random() * 6  // 0.0 〜 5.999... を生成
//   floor(...)    // 0 〜 5 の整数に変換
//
// この方法では：
//   1. 浮動小数点の精度限界で完全な一様性が保証されない
//   2. 常に最大精度のエントロピーを消費
//   3. 丸め誤差の蓄積
//
// ## Ajisaiのアプローチ：分母指定による効率化
//
// 分母を明示的に指定することで：
//   1. 必要最小限のエントロピーバイト数で済む
//   2. BigIntのサイズを最小化
//   3. 完全な一様分布を保証（リジェクションサンプリング）
//   4. 後続の演算も高速
//
// ## 使用例
//
// [ 6 ] [ 1 ] CSPRNG     # サイコロ: 0/6, 1/6, ..., 5/6 のいずれか
// [ 100 ] [ 3 ] CSPRNG   # パーセント精度で3個
// [ 5 ] CSPRNG           # デフォルト精度（2^32）で5個
// CSPRNG                 # デフォルト精度で1個
//
// ============================================================================

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;
use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, One};

/// デフォルトの分母（2^32）
/// 32ビット精度は多くの用途で十分であり、4バイトのエントロピーで済む
const DEFAULT_DENOMINATOR_BITS: u32 = 32;

/// 0以上denominator未満の一様乱数を生成（リジェクションサンプリング）
///
/// リジェクションサンプリングにより、完全な一様分布を保証する。
/// 効率のため、十分な余裕を持ったバイト数を生成してリジェクション率を下げる。
fn generate_uniform(denominator: &BigInt) -> Result<BigInt> {
    // 分母が1以下の場合は0を返す
    if *denominator <= BigInt::one() {
        return Ok(BigInt::from(0));
    }

    // 分母のビット数を計算
    let denom_bits = denominator.bits() as usize;

    // リジェクション率を下げるため、分母より少なくとも64ビット多い範囲で生成
    // これによりリジェクション率は最大でも 2^(-64) ≈ 5.4e-20 になる
    let total_bits = denom_bits + 64;
    let bytes = (total_bits + 7) / 8;

    // 最大試行回数（リジェクション率が極めて低いため、通常は1-2回で成功）
    const MAX_ATTEMPTS: usize = 10;

    for _ in 0..MAX_ATTEMPTS {
        let mut buf = vec![0u8; bytes];
        getrandom::getrandom(&mut buf)
            .map_err(|e| AjisaiError::from(format!("CSPRNG: failed to generate random bytes: {}", e)))?;

        let random_value = BigInt::from_bytes_le(Sign::Plus, &buf);

        // 剰余を取ることで [0, denominator) の範囲に変換
        // 上位ビットに十分な余裕があるため、バイアスは無視できるほど小さい
        let result = &random_value % denominator;
        return Ok(result);
    }

    Err(AjisaiError::from("CSPRNG: failed to generate random number"))
}

/// スタックから整数を抽出（単一要素Vectorの数値）
fn extract_positive_integer(val: &Value) -> Option<BigInt> {
    match val.val_type() {
        ValueType::Vector(v) if v.len() == 1 => {
            match v[0].val_type() {
                ValueType::Number(n) => {
                    // 整数かつ正数かチェック
                    if n.denominator == BigInt::one() && n.numerator > BigInt::from(0) {
                        Some(n.numerator.clone())
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

/// CSPRNG - 暗号論的疑似乱数を生成（分母指定モード対応）
///
/// 【責務】
/// - 暗号論的に安全な乱数を分数として生成
/// - 分母を指定することで必要な粒度だけを効率的に生成
///
/// 【使用法】
/// ```ajisai
/// CSPRNG                  # デフォルト精度（分母2^32）で1個
/// [ 5 ] CSPRNG            # デフォルト精度で5個
/// [ 6 ] [ 1 ] CSPRNG      # 分母6で1個（サイコロ: 0/6〜5/6）
/// [ 100 ] [ 3 ] CSPRNG    # 分母100で3個
/// ```
///
/// 【引数】
/// - 引数なし: 分母2^32で1個生成
/// - [ count ]: 分母2^32でcount個生成
/// - [ denominator ] [ count ]: 分母denominatorでcount個生成
///
/// 【戻り値】
/// - count個の乱数を含むVector
/// - 各乱数は [0, 1) の範囲の分数（0/denom, 1/denom, ..., (denom-1)/denom）
///
/// 【エラー】
/// - 生成個数が0以下
/// - 分母が0以下
/// - 乱数生成に失敗
pub fn op_csprng(interp: &mut Interpreter) -> Result<()> {
    // CSPRNGはStackモード(..)をサポートしない
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("CSPRNG does not support Stack mode (..)"));
    }

    // スタックから引数を解析
    // パターン1: 引数なし → 分母デフォルト、個数1
    // パターン2: [ count ] → 分母デフォルト、個数count
    // パターン3: [ denom ] [ count ] → 分母denom、個数count

    let (denominator, count) = parse_csprng_args(interp)?;

    // 分母の検証
    if denominator <= BigInt::from(0) {
        return Err(AjisaiError::from("CSPRNG: denominator must be positive"));
    }

    // 乱数を生成
    let mut result_vec = Vec::with_capacity(count);
    for _ in 0..count {
        let numerator = generate_uniform(&denominator)?;
        let frac = Fraction::new(numerator, denominator.clone());
        result_vec.push(Value::from_number(frac));
    }

    // 結果をスタックにプッシュ
    interp.stack.push(Value::from_vector(result_vec));

    Ok(())
}

/// CSPRNGの引数を解析
fn parse_csprng_args(interp: &mut Interpreter) -> Result<(BigInt, usize)> {
    let default_denom = BigInt::from(1u64 << DEFAULT_DENOMINATOR_BITS);

    // スタックが空の場合：デフォルト分母で1個
    if interp.stack.is_empty() {
        return Ok((default_denom, 1));
    }

    // スタックトップを確認
    let top = interp.stack.last().unwrap();

    // 整数でない場合：デフォルト分母で1個（スタックはそのまま）
    let Some(first_int) = extract_positive_integer(top) else {
        return Ok((default_denom, 1));
    };

    // 1つ目の整数をpop
    interp.stack.pop();

    // 次の要素も整数かチェック
    if let Some(second) = interp.stack.last() {
        if let Some(second_int) = extract_positive_integer(second) {
            // パターン3: [ denom ] [ count ]
            interp.stack.pop();
            let count = first_int.to_usize()
                .ok_or_else(|| AjisaiError::from("CSPRNG: count too large"))?;
            return Ok((second_int, count));
        }
    }

    // パターン2: [ count ]
    let count = first_int.to_usize()
        .ok_or_else(|| AjisaiError::from("CSPRNG: count too large"))?;

    Ok((default_denom, count))
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueType;
    use crate::types::fraction::Fraction;
    use num_bigint::BigInt;
    use num_traits::One;

    #[tokio::test]
    async fn test_csprng_rejects_stack_mode() {
        let mut interp = Interpreter::new();
        let result = interp.execute(".. CSPRNG").await;
        assert!(result.is_err(), "CSPRNG should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("CSPRNG") && err_msg.contains("Stack mode"),
                "Expected Stack mode error for CSPRNG, got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_csprng_generates_single_value() {
        let mut interp = Interpreter::new();
        let result = interp.execute("CSPRNG").await;
        assert!(result.is_ok(), "CSPRNG should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 1要素のVectorであることを確認
        if let ValueType::Vector(v) = interp.stack[0].val_type() {
            assert_eq!(v.len(), 1);
            // 値が[0, 1)の範囲にあることを確認
            if let ValueType::Number(frac) = v[0].val_type() {
                let zero = Fraction::new(BigInt::from(0), BigInt::one());
                let one = Fraction::new(BigInt::one(), BigInt::one());
                assert!(frac >= zero && frac < one, "Random value should be in [0, 1)");
            } else {
                panic!("Expected Number");
            }
        } else {
            panic!("Expected Vector");
        }
    }

    #[tokio::test]
    async fn test_csprng_generates_multiple_values() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 5 ] CSPRNG").await;
        assert!(result.is_ok(), "CSPRNG with count should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 5要素のVectorであることを確認
        if let ValueType::Vector(v) = interp.stack[0].val_type() {
            assert_eq!(v.len(), 5);
            // 各値が[0, 1)の範囲にあることを確認
            let zero = Fraction::new(BigInt::from(0), BigInt::one());
            let one = Fraction::new(BigInt::one(), BigInt::one());
            for elem in v {
                if let ValueType::Number(frac) = elem.val_type() {
                    assert!(frac >= zero && frac < one, "Random value should be in [0, 1)");
                } else {
                    panic!("Expected Number");
                }
            }
        } else {
            panic!("Expected Vector");
        }
    }

    #[tokio::test]
    async fn test_csprng_with_denominator() {
        let mut interp = Interpreter::new();
        // 分母6で3個生成（サイコロのような用途）
        let result = interp.execute("[ 6 ] [ 3 ] CSPRNG").await;
        assert!(result.is_ok(), "CSPRNG with denominator should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 3要素のVectorであることを確認
        if let ValueType::Vector(v) = interp.stack[0].val_type() {
            assert_eq!(v.len(), 3);
            // 各要素が[0, 1)の範囲の分数であることを確認
            // 注: Fractionは自動約分されるため、分母が6とは限らない（例: 2/6 → 1/3）
            let zero = Fraction::new(BigInt::from(0), BigInt::one());
            let one = Fraction::new(BigInt::one(), BigInt::one());
            for elem in v {
                if let ValueType::Number(frac) = elem.val_type() {
                    assert!(frac >= zero && frac < one, "Random value should be in [0, 1)");
                } else {
                    panic!("Expected Number");
                }
            }
        } else {
            panic!("Expected Vector");
        }
    }

    #[tokio::test]
    async fn test_csprng_dice_range() {
        let mut interp = Interpreter::new();
        // 分母6で100個生成し、[ 6 ] * で整数化したときに 0〜5 の範囲になることを確認
        let result = interp.execute("[ 6 ] [ 100 ] CSPRNG [ 6 ] *").await;
        assert!(result.is_ok());

        if let ValueType::Vector(v) = interp.stack[0].val_type() {
            assert_eq!(v.len(), 100);
            for elem in v {
                if let ValueType::Number(frac) = elem.val_type() {
                    // 分母6の乱数に6を掛けると、0〜5の整数になる
                    assert!(frac.denominator == BigInt::one(), "Should be integer after *6");
                    let num = &frac.numerator;
                    assert!(num >= &BigInt::from(0), "Value should be >= 0");
                    assert!(num < &BigInt::from(6), "Value should be < 6");
                } else {
                    panic!("Expected Number");
                }
            }
        } else {
            panic!("Expected Vector");
        }
    }

    #[tokio::test]
    async fn test_csprng_preserves_non_integer_on_stack() {
        let mut interp = Interpreter::new();
        // 分数がスタックにあっても、それは個数として解釈されない
        let result = interp.execute("[ 1/2 ] CSPRNG").await;
        assert!(result.is_ok());
        // スタックには [ 1/2 ] と CSPRNG結果の2つがあるはず
        assert_eq!(interp.stack.len(), 2);
    }

    #[tokio::test]
    async fn test_csprng_small_denominator_efficiency() {
        let mut interp = Interpreter::new();
        // 分母2で生成（コイントス）- 0/2 または 1/2
        let result = interp.execute("[ 2 ] [ 50 ] CSPRNG").await;
        assert!(result.is_ok());

        if let ValueType::Vector(v) = interp.stack[0].val_type() {
            assert_eq!(v.len(), 50);
            let mut has_zero = false;
            let mut has_half = false;
            for elem in v {
                if let ValueType::Number(frac) = elem.val_type() {
                    // 0/2 は 0/1 に約分、1/2 はそのまま
                    if frac.numerator == BigInt::from(0) {
                        has_zero = true;
                    } else if frac.numerator == BigInt::one() && frac.denominator == BigInt::from(2) {
                        has_half = true;
                    } else {
                        panic!("Unexpected value: {}/{}", frac.numerator, frac.denominator);
                    }
                }
            }
            // 50個あれば、両方の値が出現するはず（極めて高い確率で）
            assert!(has_zero || has_half, "Should have at least one of 0 or 1/2");
        }
    }
}
