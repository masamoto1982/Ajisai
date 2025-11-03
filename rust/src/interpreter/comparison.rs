// rust/src/interpreter/comparison.rs
//
// 【責務】
// 比較演算子（=、<、<=、>、>=）と論理演算子（AND、OR、NOT）を実装する。
// すべての演算は単一要素ベクタを想定し、結果を単一要素ベクタとして返す。

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::interpreter::helpers::{extract_single_element, wrap_result_value};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;

// ============================================================================
// 二項比較演算の汎用実装
// ============================================================================

/// 二項比較演算の汎用ハンドラ
///
/// 【責務】
/// - 2つの単一要素ベクタから数値を取り出して比較
/// - 比較結果をBoolean値として返す
/// - すべての比較演算（<、<=、>、>=）で共通使用
///
/// 【引数】
/// - op: Fraction同士の比較関数
fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();

    let a_val = extract_single_element(&a_vec)?;
    let b_val = extract_single_element(&b_vec)?;

    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Boolean(op(n1, n2)) }
        },
        _ => return Err(AjisaiError::type_error("number", "other type")),
    };

    interp.stack.push(wrap_result_value(result));
    Ok(())
}

// ============================================================================
// 比較演算子
// ============================================================================

/// < 演算子 - 小なり
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺より小さいか判定
///
/// 【使用法】
/// - `[3] [5] <` → `[true]`
/// - `[5] [3] <` → `[false]`
/// - `[3] [3] <` → `[false]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.lt(b))
}

/// <= 演算子 - 小なりイコール
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺以下か判定
///
/// 【使用法】
/// - `[3] [5] <=` → `[true]`
/// - `[5] [3] <=` → `[false]`
/// - `[3] [3] <=` → `[true]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.le(b))
}

/// > 演算子 - 大なり
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺より大きいか判定
///
/// 【使用法】
/// - `[5] [3] >` → `[true]`
/// - `[3] [5] >` → `[false]`
/// - `[3] [3] >` → `[false]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.gt(b))
}

/// >= 演算子 - 大なりイコール
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺以上か判定
///
/// 【使用法】
/// - `[5] [3] >=` → `[true]`
/// - `[3] [5] >=` → `[false]`
/// - `[3] [3] >=` → `[true]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.ge(b))
}

/// = 演算子 - 等価比較
///
/// 【責務】
/// - 2つの値を比較し、完全に等しいか判定
/// - あらゆる型の値を比較可能（Number、String、Boolean、Vector、Nil）
///
/// 【使用法】
/// - `[3] [3] =` → `[true]`
/// - `[3] [5] =` → `[false]`
/// - `['hello'] ['hello'] =` → `[true]`
/// - `[a b] [a b] =` → `[true]`
///
/// 【引数スタック】
/// - b: 右オペランド（任意の値）
/// - a: 左オペランド（任意の値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - なし（すべての型で比較可能）
pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();

    let result = Value { val_type: ValueType::Boolean(a_vec == b_vec) };
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

// ============================================================================
// 論理演算子
// ============================================================================

/// NOT 演算子 - 論理否定
///
/// 【責務】
/// - Boolean値を反転する
/// - Nilに対してはエラー（"No change is an error" 原則）
///
/// 【使用法】
/// - `[true] NOT` → `[false]`
/// - `[false] NOT` → `[true]`
/// - `[nil] NOT` → エラー（変化なし）
///
/// 【引数スタック】
/// - [value]: 論理値（Boolean or Nil）
///
/// 【戻り値スタック】
/// - [result]: 反転後の論理値
///
/// 【エラー】
/// - Nilの場合（変化がないため）
/// - Boolean/Nil以外の型の場合
pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let val_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let val = extract_single_element(&val_vec)?;

    let result = match &val.val_type {
        ValueType::Boolean(b) => Value { val_type: ValueType::Boolean(!b) },
        ValueType::Nil => {
            // "No change is an error" principle
            interp.stack.push(val_vec);
            return Err(AjisaiError::from("NOT on NIL resulted in no change"));
        },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other type")),
    };

    interp.stack.push(wrap_result_value(result));
    Ok(())
}

/// AND 演算子 - 論理積
///
/// 【責務】
/// - 2つの論理値の AND を計算
/// - Boolean と Nil の組み合わせをサポート
///
/// 【真理値表】
/// ```
/// | A     | B     | Result |
/// |-------|-------|--------|
/// | true  | true  | true   |
/// | true  | false | false  |
/// | false | true  | false  |
/// | false | false | false  |
/// | true  | nil   | nil    |
/// | false | nil   | false  |
/// | nil   | true  | nil    |
/// | nil   | false | false  |
/// | nil   | nil   | nil    |
/// ```
///
/// 【使用法】
/// - `[true] [true] AND` → `[true]`
/// - `[true] [false] AND` → `[false]`
/// - `[true] [nil] AND` → `[nil]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（Boolean or Nil）
/// - [a]: 左オペランド（Boolean or Nil）
///
/// 【戻り値スタック】
/// - [result]: AND の結果（Boolean or Nil）
///
/// 【エラー】
/// - オペランドがBoolean/Nilでない場合
pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    let a_val = extract_single_element(&a_vec)?;
    let b_val = extract_single_element(&b_vec)?;

    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => {
            Value { val_type: ValueType::Boolean(*a && *b) }
        },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) => {
            Value { val_type: ValueType::Boolean(false) }
        },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) | (ValueType::Nil, ValueType::Nil) => {
            Value { val_type: ValueType::Nil }
        },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

/// OR 演算子 - 論理和
///
/// 【責務】
/// - 2つの論理値の OR を計算
/// - Boolean と Nil の組み合わせをサポート
///
/// 【真理値表】
/// ```
/// | A     | B     | Result |
/// |-------|-------|--------|
/// | true  | true  | true   |
/// | true  | false | true   |
/// | false | true  | true   |
/// | false | false | false  |
/// | true  | nil   | true   |
/// | false | nil   | nil    |
/// | nil   | true  | true   |
/// | nil   | false | nil    |
/// | nil   | nil   | nil    |
/// ```
///
/// 【使用法】
/// - `[true] [false] OR` → `[true]`
/// - `[false] [false] OR` → `[false]`
/// - `[false] [nil] OR` → `[nil]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（Boolean or Nil）
/// - [a]: 左オペランド（Boolean or Nil）
///
/// 【戻り値スタック】
/// - [result]: OR の結果（Boolean or Nil）
///
/// 【エラー】
/// - オペランドがBoolean/Nilでない場合
pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    let a_val = extract_single_element(&a_vec)?;
    let b_val = extract_single_element(&b_vec)?;

    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => {
            Value { val_type: ValueType::Boolean(*a || *b) }
        },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) => {
            Value { val_type: ValueType::Boolean(true) }
        },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) | (ValueType::Nil, ValueType::Nil) => {
            Value { val_type: ValueType::Nil }
        },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    interp.stack.push(wrap_result_value(result));
    Ok(())
}
