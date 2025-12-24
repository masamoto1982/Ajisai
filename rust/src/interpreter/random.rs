// rust/src/interpreter/random.rs
//
// 【責務】
// 暗号論的疑似乱数生成ワードを実装する。
// CSPRNG: 暗号論的に安全な乱数を生成
//
// ============================================================================
// 【設計思想】暗号論的疑似乱数生成器の選択
// ============================================================================
//
// この実装は、WebAssembly環境で動作する暗号論的に安全な乱数生成器を使用する。
//
// ## 背景：PRNGとCSPRNGの違い
//
// **PRNG (Pseudo-Random Number Generator)**:
//   - シード値から決定論的に乱数列を生成
//   - 高速だが、予測可能な場合がある
//   - 例: Mersenne Twister, Xorshift, LCG
//   - 用途: シミュレーション、ゲーム、テスト
//
// **CSPRNG (Cryptographically Secure PRNG)**:
//   - OS提供のエントロピーソースを使用
//   - 予測不可能で、過去の出力から将来の出力を推測不可
//   - 例: /dev/urandom, CryptGenRandom, Web Crypto API
//   - 用途: 暗号鍵生成、セキュリティトークン、セッションID
//
// ## WebAssembly環境でのCSPRNG
//
// `getrandom` クレートを使用することで、以下のソースから乱数を取得：
//   - ブラウザ環境: Web Crypto API (crypto.getRandomValues)
//   - Node.js: crypto.randomFillSync
//   - その他の環境: 適切なOS提供のエントロピーソース
//
// ## 出力形式
//
// 乱数は分数（Fraction）として表現される：
//   - 分子: 0 ～ 2^64-1 の一様乱数
//   - 分母: 2^64
//   - 結果: [0, 1) の範囲の分数
//
// この設計により、Ajisaiの分数システムと完全に親和し、
// 情報損失なく乱数を表現できる。
//
// ============================================================================

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::ToPrimitive;

/// 64ビットの暗号論的乱数を生成し、[0, 1) の分数として返す
fn generate_random_fraction() -> Result<Fraction> {
    let mut buf = [0u8; 8];
    getrandom::getrandom(&mut buf)
        .map_err(|e| AjisaiError::from(format!("CSPRNG: failed to generate random bytes: {}", e)))?;

    // リトルエンディアンでu64に変換
    let random_u64 = u64::from_le_bytes(buf);

    // 分子: 乱数値、分母: 2^64
    let numerator = BigInt::from(random_u64);
    let denominator = BigInt::from(1u128 << 64);

    Ok(Fraction::new(numerator, denominator))
}

/// CSPRNG - 暗号論的疑似乱数を生成
///
/// 【責務】
/// - 暗号論的に安全な乱数を [0, 1) の範囲の分数として生成
/// - 生成個数を指定可能
///
/// 【使用法】
/// ```ajisai
/// CSPRNG                  → [ 0.7234... ] (1個の乱数)
/// [ 5 ] CSPRNG            → [ 0.123 0.456 0.789 0.234 0.567 ] (5個の乱数)
/// ```
///
/// 【引数】
/// - オプション: 生成個数（正の整数）
///   - 指定なし（空スタック or スタックトップが数値でない）: 1個生成
///   - 整数N: N個生成
///
/// 【戻り値】
/// - 1個の場合: 単一要素Vector [ random ]
/// - N個の場合: N要素Vector [ random1 random2 ... randomN ]
///
/// 【エラー】
/// - 生成個数が0以下
/// - 乱数生成に失敗（OSエントロピー不足など、極めて稀）
pub fn op_csprng(interp: &mut Interpreter) -> Result<()> {
    // CSPRNGはStackモード(..)をサポートしない
    // 理由: 乱数生成は新しいデータを生成する操作であり、
    //       既存のスタック要素に対する操作ではない
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("CSPRNG does not support Stack mode (..)"));
    }

    // スタックから生成個数を取得（オプション）
    let count = if let Some(top) = interp.stack.last() {
        // スタックトップが単一要素の数値Vectorかチェック
        match &top.val_type {
            ValueType::Vector(v) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::Number(n) => {
                        // 整数かチェック
                        if n.denominator != BigInt::from(1) {
                            // 分数の場合は個数指定とみなさず、1個生成
                            1usize
                        } else {
                            // 整数の場合はpopして個数として使用
                            let count_val = n.numerator.to_i64()
                                .ok_or_else(|| AjisaiError::from("CSPRNG: count too large"))?;

                            if count_val <= 0 {
                                return Err(AjisaiError::from("CSPRNG: count must be positive"));
                            }

                            // スタックからpop
                            interp.stack.pop();

                            count_val as usize
                        }
                    }
                    _ => 1usize, // 数値でない場合は1個生成
                }
            }
            _ => 1usize, // 単一要素Vectorでない場合は1個生成
        }
    } else {
        1usize // スタックが空の場合も1個生成
    };

    // 乱数を生成
    let mut result_vec = Vec::with_capacity(count);
    for _ in 0..count {
        let random_frac = generate_random_fraction()?;
        result_vec.push(Value::from_number(random_frac));
    }

    // 結果をスタックにプッシュ
    interp.stack.push(Value::from_vector(result_vec));

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_csprng_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        // Stackモード（..）でCSPRNGを呼び出した場合はエラー
        let result = interp.execute(".. CSPRNG").await;
        assert!(result.is_err(), "CSPRNG should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("CSPRNG") && err_msg.contains("Stack mode"),
                "Expected Stack mode error for CSPRNG, got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_csprng_generates_single_value() {
        let mut interp = Interpreter::new();

        // 引数なしで1個の乱数を生成
        let result = interp.execute("CSPRNG").await;
        assert!(result.is_ok(), "CSPRNG should succeed: {:?}", result);

        // スタックに1要素のVectorがあることを確認
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_csprng_generates_multiple_values() {
        let mut interp = Interpreter::new();

        // 5個の乱数を生成
        let result = interp.execute("[ 5 ] CSPRNG").await;
        assert!(result.is_ok(), "CSPRNG with count should succeed: {:?}", result);

        // スタックに1つのVectorがあることを確認
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_csprng_rejects_zero_count() {
        let mut interp = Interpreter::new();

        // 0個は無効
        let result = interp.execute("[ 0 ] CSPRNG").await;
        assert!(result.is_err(), "CSPRNG with count 0 should fail");
    }

    #[tokio::test]
    async fn test_csprng_rejects_negative_count() {
        let mut interp = Interpreter::new();

        // 負数は無効
        let result = interp.execute("[ -1 ] CSPRNG").await;
        assert!(result.is_err(), "CSPRNG with negative count should fail");
    }
}
