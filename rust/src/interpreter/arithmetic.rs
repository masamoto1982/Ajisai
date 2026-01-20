// rust/src/interpreter/arithmetic.rs
//
// 統一Value宇宙アーキテクチャ版の算術演算
//
// 型チェックは存在しない。すべて分数演算として実行する。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;

// ============================================================================
// ヘルパー関数
// ============================================================================

/// 値から単一スカラーを抽出（スカラーまたは単一要素ベクタから）
fn extract_single_scalar(val: &Value) -> Option<&Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Some(f),
        ValueData::Vector(children) if children.len() == 1 => {
            extract_single_scalar(&children[0])
        }
        _ => None
    }
}

/// 値が単一スカラーとして扱えるかチェック
fn is_single_scalar(val: &Value) -> bool {
    extract_single_scalar(val).is_some()
}

// ============================================================================
// ブロードキャスト付き二項演算
// ============================================================================

/// ブロードキャスト付き二項演算（再帰的Value構造対応）
fn broadcast_binary_op<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match (&a.data, &b.data) {
        // 両方NIL
        (ValueData::Nil, ValueData::Nil) => Ok(Value::nil()),

        // 片方がNIL
        (ValueData::Nil, _) | (_, ValueData::Nil) => {
            Err(AjisaiError::from("Cannot operate with NIL"))
        }

        // 両方スカラー
        (ValueData::Scalar(fa), ValueData::Scalar(fb)) => {
            Ok(Value::from_fraction(op(fa, fb)?))
        }

        // aがスカラー、bがベクター（ブロードキャスト）
        (ValueData::Scalar(fa), ValueData::Vector(vb)) => {
            let result: Result<Vec<Value>> = vb.iter()
                .map(|bi| broadcast_binary_op(&Value::from_fraction(fa.clone()), bi, op))
                .collect();
            Ok(Value::from_children(result?))
        }

        // aがベクター、bがスカラー（ブロードキャスト）
        (ValueData::Vector(va), ValueData::Scalar(fb)) => {
            let result: Result<Vec<Value>> = va.iter()
                .map(|ai| broadcast_binary_op(ai, &Value::from_fraction(fb.clone()), op))
                .collect();
            Ok(Value::from_children(result?))
        }

        // 両方ベクター
        (ValueData::Vector(va), ValueData::Vector(vb)) => {
            if va.len() != vb.len() {
                return Err(AjisaiError::VectorLengthMismatch { len1: va.len(), len2: vb.len() });
            }
            let result: Result<Vec<Value>> = va.iter().zip(vb.iter())
                .map(|(ai, bi)| broadcast_binary_op(ai, bi, op))
                .collect();
            Ok(Value::from_children(result?))
        }

        // Block関連
        (ValueData::Block(_), _) | (_, ValueData::Block(_)) => {
            Err(AjisaiError::from("Cannot perform arithmetic on Block"))
        }
    }
}

// ============================================================================
// 二項演算の汎用実装
// ============================================================================

/// 二項算術演算の汎用ハンドラ
fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match interp.operation_target {
        // StackTopモード: ベクタ間の要素ごと演算
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let result = match broadcast_binary_op(&a_val, &b_val, op) {
                Ok(r) => r,
                Err(e) => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(e);
                }
            };

            // "No change is an error" 原則のチェック（REDUCE等では無効化）
            if !interp.disable_no_change_check && (result == a_val || result == b_val) {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Arithmetic operation resulted in no change"));
            }

            interp.stack.push(result);
        },

        // Stackモード: N個の要素を畳み込む
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            // カウント0はエラー（"No change is an error"原則）
            if count == 0 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 0 results in no change"));
            }

            // カウント1もエラー（1要素の畳み込みは変化なし）
            if count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count ..).collect();

            // 各要素が単一スカラーとして扱えることを確認（単一要素ベクタも許容）
            if items.iter().any(|v| !is_single_scalar(v)) {
                interp.stack.extend(items);
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK mode requires single-element values"));
            }

            let first_scalar = extract_single_scalar(&items[0]).unwrap().clone();
            let mut acc = first_scalar.clone();
            let original_first = acc.clone();

            for item in items.iter().skip(1) {
                if let Some(f) = extract_single_scalar(item) {
                    acc = op(&acc, f)?;
                }
            }

            // "No change is an error" 原則のチェック（REDUCE等では無効化）
            if !interp.disable_no_change_check && acc == original_first {
                interp.stack.extend(items);
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation resulted in no change"));
            }

            interp.stack.push(Value::from_fraction(acc));
        }
    }
    Ok(())
}

// ============================================================================
// 算術演算の実装
// ============================================================================

/// + 演算子 - 加算（型チェックなし）
pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.add(b)))
}

/// - 演算子 - 減算（型チェックなし）
pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.sub(b)))
}

/// * 演算子 - 乗算（型チェックなし）
pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    // StackTopモードでスカラー×ベクター の場合、整数スカラー最適化を試みる
    if interp.operation_target == OperationTarget::StackTop {
        if let (Some(b_val), Some(a_val)) = (interp.stack.pop(), interp.stack.pop()) {
            // NILチェック
            if a_val.is_nil() || b_val.is_nil() {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Cannot operate with NIL"));
            }

            // 整数スカラー最適化の適用を試みる
            let result = apply_optimized_mul(&a_val, &b_val);

            match result {
                Ok(r) => {
                    // "No change is an error" 原則のチェック
                    if !interp.disable_no_change_check && (r == a_val || r == b_val) {
                        interp.stack.push(a_val);
                        interp.stack.push(b_val);
                        return Err(AjisaiError::from("Arithmetic operation resulted in no change"));
                    }
                    interp.stack.push(r);
                    return Ok(());
                }
                Err(e) => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(e);
                }
            }
        } else {
            return Err(AjisaiError::StackUnderflow);
        }
    }

    // Stackモードは汎用ハンドラを使用
    binary_arithmetic_op(interp, |a, b| Ok(a.mul(b)))
}

/// 整数スカラー最適化を適用した乗算
fn apply_optimized_mul(a: &Value, b: &Value) -> Result<Value> {
    match (&a.data, &b.data) {
        (ValueData::Scalar(fa), ValueData::Scalar(fb)) => {
            Ok(Value::from_fraction(fa.mul(fb)))
        }
        (ValueData::Scalar(scalar), ValueData::Vector(vec)) => {
            if scalar.is_integer() {
                let result: Vec<Value> = vec.iter()
                    .map(|v| apply_scalar_mul_to_value(scalar, v))
                    .collect();
                Ok(Value::from_children(result))
            } else {
                broadcast_binary_op(a, b, |x, y| Ok(x.mul(y)))
            }
        }
        (ValueData::Vector(vec), ValueData::Scalar(scalar)) => {
            if scalar.is_integer() {
                let result: Vec<Value> = vec.iter()
                    .map(|v| apply_scalar_mul_to_value(scalar, v))
                    .collect();
                Ok(Value::from_children(result))
            } else {
                broadcast_binary_op(a, b, |x, y| Ok(x.mul(y)))
            }
        }
        (ValueData::Vector(va), ValueData::Vector(vb)) => {
            if va.len() != vb.len() {
                return Err(AjisaiError::VectorLengthMismatch { len1: va.len(), len2: vb.len() });
            }
            let result: Result<Vec<Value>> = va.iter().zip(vb.iter())
                .map(|(ai, bi)| apply_optimized_mul(ai, bi))
                .collect();
            Ok(Value::from_children(result?))
        }
        _ => Err(AjisaiError::from("Cannot multiply NIL")),
    }
}

/// スカラーを再帰的に値に乗算（整数最適化）
fn apply_scalar_mul_to_value(scalar: &Fraction, val: &Value) -> Value {
    match &val.data {
        ValueData::Scalar(f) => {
            if scalar.is_integer() {
                Value::from_fraction(f.mul_by_integer(scalar))
            } else {
                Value::from_fraction(f.mul(scalar))
            }
        }
        ValueData::Vector(children) => {
            let new_children: Vec<Value> = children.iter()
                .map(|c| apply_scalar_mul_to_value(scalar, c))
                .collect();
            Value::from_children(new_children)
        }
        ValueData::Nil => val.clone(),
        ValueData::Block(_) => val.clone(),
    }
}

/// / 演算子 - 除算（型チェックなし）
pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    // StackTopモードで整数スカラーによる除算の最適化
    if interp.operation_target == OperationTarget::StackTop {
        if let (Some(b_val), Some(a_val)) = (interp.stack.pop(), interp.stack.pop()) {
            // NILチェック
            if a_val.is_nil() || b_val.is_nil() {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Cannot operate with NIL"));
            }

            let result = apply_optimized_div(&a_val, &b_val);

            match result {
                Ok(r) => {
                    // "No change is an error" 原則のチェック
                    if !interp.disable_no_change_check && (r == a_val || r == b_val) {
                        interp.stack.push(a_val);
                        interp.stack.push(b_val);
                        return Err(AjisaiError::from("Arithmetic operation resulted in no change"));
                    }
                    interp.stack.push(r);
                    return Ok(());
                }
                Err(e) => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(e);
                }
            }
        } else {
            return Err(AjisaiError::StackUnderflow);
        }
    }

    // Stackモードは汎用ハンドラを使用
    binary_arithmetic_op(interp, |a, b| {
        if b.is_zero() {
            Err(AjisaiError::DivisionByZero)
        } else {
            Ok(a.div(b))
        }
    })
}

/// 整数スカラー最適化を適用した除算
fn apply_optimized_div(a: &Value, b: &Value) -> Result<Value> {
    match (&a.data, &b.data) {
        (ValueData::Scalar(fa), ValueData::Scalar(fb)) => {
            if fb.is_zero() {
                return Err(AjisaiError::DivisionByZero);
            }
            Ok(Value::from_fraction(fa.div(fb)))
        }
        (ValueData::Vector(vec), ValueData::Scalar(scalar)) => {
            // ベクター ÷ スカラー
            if scalar.is_zero() {
                return Err(AjisaiError::DivisionByZero);
            }
            if scalar.is_integer() {
                let result: Result<Vec<Value>> = vec.iter()
                    .map(|v| apply_scalar_div_to_value(v, scalar))
                    .collect();
                Ok(Value::from_children(result?))
            } else {
                broadcast_binary_op(a, b, |x, y| {
                    if y.is_zero() { Err(AjisaiError::DivisionByZero) } else { Ok(x.div(y)) }
                })
            }
        }
        (ValueData::Scalar(scalar), ValueData::Vector(vec)) => {
            // スカラー ÷ ベクター
            let result: Result<Vec<Value>> = vec.iter()
                .map(|v| apply_div_scalar_by_value(scalar, v))
                .collect();
            Ok(Value::from_children(result?))
        }
        (ValueData::Vector(va), ValueData::Vector(vb)) => {
            if va.len() != vb.len() {
                return Err(AjisaiError::VectorLengthMismatch { len1: va.len(), len2: vb.len() });
            }
            let result: Result<Vec<Value>> = va.iter().zip(vb.iter())
                .map(|(ai, bi)| apply_optimized_div(ai, bi))
                .collect();
            Ok(Value::from_children(result?))
        }
        _ => Err(AjisaiError::from("Cannot divide NIL")),
    }
}

/// スカラーで値を除算（整数最適化）
fn apply_scalar_div_to_value(val: &Value, scalar: &Fraction) -> Result<Value> {
    match &val.data {
        ValueData::Scalar(f) => {
            if scalar.is_integer() {
                Ok(Value::from_fraction(f.div_by_integer(scalar)))
            } else {
                Ok(Value::from_fraction(f.div(scalar)))
            }
        }
        ValueData::Vector(children) => {
            let new_children: Result<Vec<Value>> = children.iter()
                .map(|c| apply_scalar_div_to_value(c, scalar))
                .collect();
            Ok(Value::from_children(new_children?))
        }
        ValueData::Nil => Ok(val.clone()),
        ValueData::Block(_) => Ok(val.clone()),
    }
}

/// スカラーを値で除算
fn apply_div_scalar_by_value(scalar: &Fraction, val: &Value) -> Result<Value> {
    match &val.data {
        ValueData::Scalar(f) => {
            if f.is_zero() {
                return Err(AjisaiError::DivisionByZero);
            }
            Ok(Value::from_fraction(scalar.div(f)))
        }
        ValueData::Vector(children) => {
            let new_children: Result<Vec<Value>> = children.iter()
                .map(|c| apply_div_scalar_by_value(scalar, c))
                .collect();
            Ok(Value::from_children(new_children?))
        }
        ValueData::Nil => Ok(val.clone()),
        ValueData::Block(_) => Ok(val.clone()),
    }
}
