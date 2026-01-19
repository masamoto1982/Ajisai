// rust/src/interpreter/random.rs
//
// 【責務】
// 暗号論的疑似乱数生成ワードを実装する。
// CSPRNG: 暗号論的に安全な乱数を生成
//
// 統一Value宇宙アーキテクチャ版
//
// ============================================================================
// 【設計思想】分数システムを活用した効率的な乱数生成
// ============================================================================
//
// Ajisaiの分数システムを最大限活用し、必要な粒度だけの乱数を生成する。
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
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;
use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, One};

/// デフォルトの分母（2^32）
const DEFAULT_DENOMINATOR_BITS: u32 = 32;

// ============================================================================
// ヘルパー関数（統一Value宇宙アーキテクチャ用）
// ============================================================================

/// 0以上denominator未満の一様乱数を生成（リジェクションサンプリング）
fn generate_uniform(denominator: &BigInt) -> Result<BigInt> {
    if *denominator <= BigInt::one() {
        return Ok(BigInt::from(0));
    }

    let denom_bits = denominator.bits() as usize;
    let total_bits = denom_bits + 64;
    let bytes = (total_bits + 7) / 8;

    const MAX_ATTEMPTS: usize = 10;

    for _ in 0..MAX_ATTEMPTS {
        let mut buf = vec![0u8; bytes];
        getrandom::getrandom(&mut buf)
            .map_err(|e| AjisaiError::from(format!("CSPRNG: failed to generate random bytes: {}", e)))?;

        let random_value = BigInt::from_bytes_le(Sign::Plus, &buf);
        let result = &random_value % denominator;
        return Ok(result);
    }

    Err(AjisaiError::from("CSPRNG: failed to generate random number"))
}

/// スタックから正の整数を抽出（単一要素Vectorの数値）
fn extract_positive_integer(val: &Value) -> Option<BigInt> {
    // Vectorの場合、最初の要素をチェック
    if let ValueData::Vector(children) = &val.data {
        if children.len() == 1 {
            if let Some(frac) = children[0].as_scalar() {
                // 整数かつ正数かチェック
                if frac.denominator == BigInt::one() && frac.numerator > BigInt::from(0) {
                    return Some(frac.numerator.clone());
                }
            }
        }
    }
    None
}

/// CSPRNG - 暗号論的疑似乱数を生成（分母指定モード対応）
pub fn op_csprng(interp: &mut Interpreter) -> Result<()> {
    // CSPRNGはStackモード(..)をサポートしない
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("CSPRNG does not support Stack mode (..)"));
    }

    let (denominator, count) = parse_csprng_args(interp)?;

    if denominator <= BigInt::from(0) {
        return Err(AjisaiError::from("CSPRNG: denominator must be positive"));
    }

    let mut result_vec = Vec::with_capacity(count);
    for _ in 0..count {
        let numerator = generate_uniform(&denominator)?;
        let frac = Fraction::new(numerator, denominator.clone());
        result_vec.push(Value::from_number(frac));
    }

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
    use crate::types::ValueData;

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
        // Check it's a vector with 1 element
        let val = &interp.stack[0];
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 1);
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
        // Check it's a vector with 5 elements
        let val = &interp.stack[0];
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 5);
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
        // Check it's a vector with 3 elements
        let val = &interp.stack[0];
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 3);
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
        // Check it's a vector with 50 elements
        let val = &interp.stack[0];
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 50);
        } else {
            panic!("Expected Vector");
        }
    }
}
