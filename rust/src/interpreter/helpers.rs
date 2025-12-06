// rust/src/interpreter/helpers.rs
//
// 【責務】
// インタプリタ内で頻繁に使用される共通ヘルパー関数を提供する。
// 型変換、値の抽出、エラーハンドリングなどの定型処理を一元化し、
// コードの重複を排除して保守性を向上させる。

use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueType, Token};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

// ============================================================================
// トークン処理関数（execute_section の共通化）
// ============================================================================

/// トークンから値を作成し、単一要素ベクタでラップする
///
/// 【責務】
/// - Token から Value への変換ロジックを一元化
/// - execute_section_sync と execute_section の重複を削減
///
/// 【引数】
/// - token: 変換元のトークン
///
/// 【戻り値】
/// - Ok(Some(Value)): 値が作成された場合
/// - Ok(None): 値を作成しないトークン（Symbol, GuardSeparator など）
/// - Err: パースエラー
pub fn token_to_wrapped_value(token: &Token) -> Result<Option<Value>> {
    match token {
        Token::Number(n) => {
            let val = Value { val_type: ValueType::Number(Fraction::from_str(n).map_err(AjisaiError::from)?) };
            Ok(Some(wrap_in_square_vector(val)))
        },
        Token::String(s) => {
            let val = Value { val_type: ValueType::String(s.clone()) };
            Ok(Some(wrap_in_square_vector(val)))
        },
        Token::Boolean(b) => {
            let val = Value { val_type: ValueType::Boolean(*b) };
            Ok(Some(wrap_in_square_vector(val)))
        },
        Token::Nil => {
            let val = Value { val_type: ValueType::Nil };
            Ok(Some(wrap_in_square_vector(val)))
        },
        _ => Ok(None), // Symbol, VectorStart, GuardSeparator, LineBreak は呼び出し側で処理
    }
}

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
        ValueType::Tensor(t) if t.data().len() == 1 => {
            let n = &t.data()[0];
            if n.denominator == BigInt::one() {
                n.numerator.to_i64().ok_or_else(|| AjisaiError::from("Integer value is too large for i64"))
            } else {
                Err(AjisaiError::type_error("integer", "fraction"))
            }
        },
        _ => Err(AjisaiError::type_error("single-element vector or tensor with integer", "other type")),
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
        ValueType::Tensor(t) if t.data().len() == 1 => {
            let n = &t.data()[0];
            if n.denominator == BigInt::one() {
                Ok(n.numerator.clone())
            } else {
                Err(AjisaiError::type_error("integer", "fraction"))
            }
        },
        _ => Err(AjisaiError::type_error("single-element vector or tensor with integer", "other type")),
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
        ValueType::Tensor(t) if t.data().len() == 1 => Ok(&t.data()[0]),
        _ => Err(AjisaiError::type_error("number or single-element number vector/tensor", "other type")),
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

/// 値を単一要素の角括弧ベクタ []でラップする
///
/// **非推奨**: 数値には `wrap_single_value` または `Value::from_tensor(Tensor::vector(...))` を使用してください。
/// この関数は非数値型（String、Boolean、Nil等）のラップにのみ使用すべきです。
///
/// 【責務】
/// - 任意の値を [value] の形式にラップ
///
/// 【用途】
/// - 非数値型のリテラル値のスタックへのプッシュ
/// - 非数値型の演算結果の統一形式での返却
///
/// 【引数】
/// - value: ラップする値
///
/// 【戻り値】
/// - [value]形式のベクタ
#[deprecated(note = "数値には wrap_single_value または Value::from_tensor を使用してください")]
pub fn wrap_in_square_vector(value: Value) -> Value {
    Value { val_type: ValueType::Vector(vec![value]) }
}

/// 単一値をラップ（数値ならTensor、それ以外ならVector）
///
/// 【責務】
/// - 数値の場合はTensorとしてラップ
/// - それ以外の場合はVectorとしてラップ
/// - Tensor移行の統一的なラッピング戦略を提供
///
/// 【用途】
/// - 演算結果の返却
/// - MAP/FILTERなどの高階関数の結果処理
///
/// 【引数】
/// - value: ラップする値
///
/// 【戻り値】
/// - 数値: 1要素のTensor
/// - それ以外: [value]形式のVector
pub fn wrap_single_value(value: Value) -> Value {
    use crate::types::tensor::Tensor;
    match &value.val_type {
        ValueType::Number(f) => Value::from_tensor(Tensor::vector(vec![f.clone()])),
        _ => Value { val_type: ValueType::Vector(vec![value]) }
    }
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
        ValueType::Tensor(t) if t.data().len() == 1 => {
            Value { val_type: ValueType::Number(t.data()[0].clone()) }
        },
        _ => value,
    }
}

/// 値をラップして結果形式として返す
///
/// 【責務】
/// - 比較演算・論理演算の結果を単一要素ベクタにラップ
/// - 統一的な結果形式の提供
///
/// 【用途】
/// - 比較演算子の結果返却
/// - 論理演算子の結果返却
///
/// 【引数】
/// - value: ラップする結果値
///
/// 【戻り値】
/// - [value]形式のベクタ
pub fn wrap_result_value(value: Value) -> Value {
    wrap_in_square_vector(value)
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
        let num = Value { val_type: ValueType::Number(Fraction::new(BigInt::from(42), BigInt::one())) };
        let wrapped = wrap_in_square_vector(num.clone());
        let unwrapped = unwrap_single_element(wrapped);
        assert_eq!(unwrapped, num);
    }
}
