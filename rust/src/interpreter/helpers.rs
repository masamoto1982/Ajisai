// rust/src/interpreter/helpers.rs
//
// 【責務】
// インタプリタ内で頻繁に使用される共通ヘルパー関数を提供する。
// 型変換、値の抽出、エラーハンドリングなどの定型処理を一元化し、
// コードの重複を排除して保守性を向上させる。
//
// Vector指向型システム対応版

use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

// ============================================================================
// 整数・インデックス抽出関数
// ============================================================================

/// 単一要素ベクタから整数値（i64）を抽出する
///
/// 【責務】
/// - 値が単一要素ベクタであることを検証
/// - 内部の数値が整数（分母が1）であることを検証
/// - i64範囲内に収まることを検証
///
/// 【用途】
/// - カウント引数の取得（MAP/FILTER/算術演算のSTACKモード）
/// - 整数パラメータの取得
///
/// 【エラー】
/// - 単一要素ベクタでない場合
/// - 内部が数値でない、または分数の場合
/// - i64範囲を超える場合
pub fn get_integer_from_value(value: &Value) -> Result<i64> {
    match &value.val_type {
        ValueType::Vector(v) if v.len() == 1 => {
            if let ValueType::Number(n) = &v[0].val_type {
                if n.denominator == BigInt::one() {
                    n.numerator.to_i64().ok_or_else(|| AjisaiError::from("Integer value is too large for i64"))
                } else {
                    Err(AjisaiError::type_error("integer", "fraction"))
                }
            } else {
                Err(AjisaiError::type_error("integer", "other type"))
            }
        },
        ValueType::Number(n) => {
            // 直接Number型の場合も処理
            if n.denominator == BigInt::one() {
                n.numerator.to_i64().ok_or_else(|| AjisaiError::from("Integer value is too large for i64"))
            } else {
                Err(AjisaiError::type_error("integer", "fraction"))
            }
        },
        _ => Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    }
}

/// 単一要素ベクタからBigInt整数値を抽出する
///
/// 【責務】
/// - 値が単一要素ベクタであることを検証
/// - 内部の数値が整数（分母が1）であることを検証
/// - BigIntとして返す（サイズ制限なし）
///
/// 【用途】
/// - インデックス指定（GET/INSERT/REPLACE/REMOVE）
/// - 大きな整数値の取得
///
/// 【エラー】
/// - 単一要素ベクタでない場合
/// - 内部が数値でない、または分数の場合
pub fn get_bigint_from_value(value: &Value) -> Result<BigInt> {
    match &value.val_type {
        ValueType::Vector(ref v) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == BigInt::one() => Ok(n.numerator.clone()),
                ValueType::Number(_) => Err(AjisaiError::type_error("integer", "fraction")),
                _ => Err(AjisaiError::type_error("integer", "other type")),
            }
        },
        ValueType::Number(n) => {
            // 直接Number型の場合も処理
            if n.denominator == BigInt::one() {
                Ok(n.numerator.clone())
            } else {
                Err(AjisaiError::type_error("integer", "fraction"))
            }
        },
        _ => Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    }
}

// ============================================================================
// 値抽出・アンラップ関数
// ============================================================================

/// 単一要素ベクタから内部の値への参照を取得する
///
/// 【責務】
/// - ベクタが単一要素であることを検証
/// - 内部の値への参照を返す
///
/// 【用途】
/// - 比較演算・論理演算での値取得
/// - ベクタラップされた値の参照取得
///
/// 【エラー】
/// - ベクタでない場合
/// - 複数要素のベクタの場合
pub fn extract_single_element(vector_val: &Value) -> Result<&Value> {
    match &vector_val.val_type {
        ValueType::Vector(v) if v.len() == 1 => Ok(&v[0]),
        ValueType::Vector(_) => Err(AjisaiError::from("Multi-element vector not supported in this context")),
        _ => Err(AjisaiError::type_error("single-element vector", "other type")),
    }
}

/// 値から数値（Fraction）への参照を抽出する
///
/// 【責務】
/// - 直接数値の場合はそのまま返す
/// - 単一要素ベクタの場合は内部の数値を返す
///
/// 【用途】
/// - 算術演算での数値取得
/// - 数値が必要な演算全般
///
/// 【エラー】
/// - 数値でもベクタでもない場合
/// - ベクタ内が数値でない場合
pub fn extract_number(val: &Value) -> Result<&Fraction> {
    match &val.val_type {
        ValueType::Number(n) => Ok(n),
        ValueType::Vector(v) if v.len() == 1 => {
            if let ValueType::Number(n) = &v[0].val_type {
                Ok(n)
            } else {
                Err(AjisaiError::type_error("number", "other type in inner vector"))
            }
        },
        _ => Err(AjisaiError::type_error("number or single-element number vector", "other type")),
    }
}

/// 値から複数の数値を抽出する（ベクタの全要素）
///
/// 【責務】
/// - ベクタ内のすべての数値を抽出
/// - 直接数値の場合は1要素のベクタとして返す
///
/// 【用途】
/// - ベクタ全体の数値演算
/// - 配列データの取得
///
/// 【エラー】
/// - 数値以外の要素が含まれる場合
pub fn extract_numbers(val: &Value) -> Result<Vec<&Fraction>> {
    match &val.val_type {
        ValueType::Number(n) => Ok(vec![n]),
        ValueType::Vector(v) => {
            let mut result = Vec::with_capacity(v.len());
            for elem in v {
                if let ValueType::Number(n) = &elem.val_type {
                    result.push(n);
                } else {
                    return Err(AjisaiError::type_error("number", "other type in vector"));
                }
            }
            Ok(result)
        },
        _ => Err(AjisaiError::type_error("number or number vector", "other type")),
    }
}

/// 単一要素ベクタから文字列を抽出する
///
/// 【責務】
/// - 単一要素ベクタであることを検証
/// - 内部が文字列であることを検証
/// - 文字列を大文字に変換して返す
///
/// 【用途】
/// - ワード名の取得（MAP/FILTER）
/// - 文字列パラメータの取得
///
/// 【エラー】
/// - 単一要素ベクタでない場合
/// - 内部が文字列でない場合
pub fn get_word_name_from_value(value: &Value) -> Result<String> {
    match &value.val_type {
        ValueType::Vector(v) if v.len() == 1 => {
            if let ValueType::String(s) = &v[0].val_type {
                Ok(s.to_uppercase())
            } else {
                Err(AjisaiError::type_error("string for word name", "other type"))
            }
        },
        ValueType::String(s) => {
            // 直接String型の場合も処理
            Ok(s.to_uppercase())
        },
        _ => Err(AjisaiError::type_error("single-element vector with string", "other type")),
    }
}

// ============================================================================
// インデックス正規化関数
// ============================================================================

/// 負数インデックスを正規化し、範囲チェックを行う
///
/// 【責務】
/// - 負数インデックス（-1 = 末尾）を正のインデックスに変換
/// - 範囲外の場合はNoneを返す
///
/// 【用途】
/// - GET/REPLACE/REMOVE操作でのインデックス計算
/// - すべてのベクタ・スタック操作でのインデックス処理
///
/// 【引数】
/// - index: 指定されたインデックス（負数可）
/// - length: 対象の長さ
///
/// 【戻り値】
/// - Some(usize): 正規化された有効なインデックス
/// - None: 範囲外
pub fn normalize_index(index: i64, length: usize) -> Option<usize> {
    let actual_index = if index < 0 {
        // 負数インデックス: -1は末尾、-2は末尾の1つ前
        let offset = (length as i64) + index;
        if offset < 0 {
            return None;
        }
        offset as usize
    } else {
        index as usize
    };

    if actual_index < length {
        Some(actual_index)
    } else {
        None
    }
}

// ============================================================================
// ベクタラッピング関数
// ============================================================================

/// 値を単一要素Vectorでラップする
///
/// 【責務】
/// - 任意の値を [value] の形式にラップ
///
/// 【用途】
/// - 非数値型のリテラル値のスタックへのプッシュ（String、Boolean、Symbol、Nil）
/// - 非数値型の演算結果の統一形式での返却
///
/// 【引数】
/// - value: ラップする値
///
/// 【戻り値】
/// - [value]形式のベクタ
pub fn wrap_value(value: Value) -> Value {
    Value::from_vector(vec![value])
}

/// 数値を単一要素Vectorでラップする
///
/// 【責務】
/// - 数値（Fraction）を1要素のVectorとしてラップ
///
/// 【用途】
/// - 数値リテラルのスタックへのプッシュ
/// - 数値演算結果の統一形式での返却
///
/// 【引数】
/// - fraction: ラップする数値
///
/// 【戻り値】
/// - [number]形式のベクタ
pub fn wrap_number(fraction: Fraction) -> Value {
    Value::from_vector(vec![Value::from_number(fraction)])
}

/// 単一要素ベクタの場合は内部要素を取り出す
///
/// 【責務】
/// - ベクタが単一要素の場合は内部要素を返す
/// - それ以外の場合は元の値をそのまま返す
///
/// 【用途】
/// - INSERT/REPLACE操作での要素展開
/// - 不要なネストの除去
///
/// 【引数】
/// - value: 処理する値
///
/// 【戻り値】
/// - 単一要素ベクタの場合: 内部要素
/// - それ以外: 元の値
pub fn unwrap_single_element(value: Value) -> Value {
    match value.val_type {
        ValueType::Vector(mut v) if v.len() == 1 => v.remove(0),
        _ => value,
    }
}

/// 非数値型の結果値をラップして返す
///
/// 【責務】
/// - 比較演算・論理演算の結果（Boolean）を単一要素ベクタにラップ
/// - 統一的な結果形式の提供
///
/// 【用途】
/// - 比較演算子の結果返却（Boolean）
/// - 論理演算子の結果返却（Boolean）
///
/// 【引数】
/// - value: ラップする結果値
///
/// 【戻り値】
/// - [value]形式のベクタ
pub fn wrap_result_value(value: Value) -> Value {
    wrap_value(value)
}

// ============================================================================
// テストモジュール
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_index_positive() {
        assert_eq!(normalize_index(0, 5), Some(0));
        assert_eq!(normalize_index(4, 5), Some(4));
        assert_eq!(normalize_index(5, 5), None);
    }

    #[test]
    fn test_normalize_index_negative() {
        assert_eq!(normalize_index(-1, 5), Some(4));
        assert_eq!(normalize_index(-5, 5), Some(0));
        assert_eq!(normalize_index(-6, 5), None);
    }

    #[test]
    fn test_wrap_unwrap() {
        let frac = Fraction::new(BigInt::from(42), BigInt::one());
        let wrapped = wrap_number(frac.clone());
        let unwrapped = unwrap_single_element(wrapped);
        assert_eq!(unwrapped, Value { val_type: ValueType::Number(frac) });
    }

    #[test]
    fn test_extract_number() {
        let frac = Fraction::new(BigInt::from(42), BigInt::one());
        let wrapped = wrap_number(frac.clone());
        let extracted = extract_number(&wrapped).unwrap();
        assert_eq!(extracted, &frac);
    }

    #[test]
    fn test_extract_numbers() {
        let v = Value::from_vector(vec![
            Value::from_number(Fraction::new(BigInt::from(1), BigInt::one())),
            Value::from_number(Fraction::new(BigInt::from(2), BigInt::one())),
            Value::from_number(Fraction::new(BigInt::from(3), BigInt::one())),
        ]);
        let nums = extract_numbers(&v).unwrap();
        assert_eq!(nums.len(), 3);
    }

    #[test]
    fn test_get_integer_from_value() {
        let wrapped = wrap_number(Fraction::new(BigInt::from(42), BigInt::one()));
        let result = get_integer_from_value(&wrapped).unwrap();
        assert_eq!(result, 42);
    }
}
