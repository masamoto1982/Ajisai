// rust/src/interpreter/helpers.rs
//
// 【責務】
// インタプリタ内で頻繁に使用される共通ヘルパー関数を提供する。
// 型変換、値の抽出、エラーハンドリングなどの定型処理を一元化し、
// コードの重複を排除して保守性を向上させる。
//
// 統一Value宇宙アーキテクチャ対応版

use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

// ============================================================================
// 整数・インデックス抽出関数
// ============================================================================

/// 単一要素の値から整数値（i64）を抽出する
///
/// 【責務】
/// - 値がスカラーまたは単一要素ベクタであることを検証
/// - 内部の数値が整数（分母が1）であることを検証
/// - i64範囲内に収まることを検証
///
/// 【用途】
/// - カウント引数の取得（MAP/FILTER/算術演算のSTACKモード）
/// - 整数パラメータの取得
///
/// 【エラー】
/// - 複数要素ベクタの場合
/// - 分数の場合
/// - i64範囲を超える場合
pub fn get_integer_from_value(value: &Value) -> Result<i64> {
    match &value.data {
        ValueData::Scalar(f) => {
            if f.denominator != BigInt::one() {
                return Err(AjisaiError::structure_error("integer", "fraction"));
            }
            f.numerator.to_i64().ok_or_else(|| AjisaiError::from("Integer value is too large for i64"))
        }
        ValueData::Nil => {
            Err(AjisaiError::structure_error("single-element value with integer", "NIL"))
        }
        ValueData::Vector(children) if children.len() == 1 => {
            // 単一要素ベクタの場合、再帰的に処理
            get_integer_from_value(&children[0])
        }
        ValueData::Vector(_) => {
            Err(AjisaiError::structure_error("single-element value with integer", "multi-element vector"))
        }
    }
}

/// 単一要素の値からBigInt整数値を抽出する
///
/// 【責務】
/// - 値がスカラーまたは単一要素ベクタであることを検証
/// - 内部の数値が整数（分母が1）であることを検証
/// - BigIntとして返す（サイズ制限なし）
///
/// 【用途】
/// - インデックス指定（GET/INSERT/REPLACE/REMOVE）
/// - 大きな整数値の取得
///
/// 【エラー】
/// - 複数要素ベクタの場合
/// - 分数の場合
pub fn get_bigint_from_value(value: &Value) -> Result<BigInt> {
    match &value.data {
        ValueData::Scalar(f) => {
            if f.denominator != BigInt::one() {
                return Err(AjisaiError::structure_error("integer", "fraction"));
            }
            Ok(f.numerator.clone())
        }
        ValueData::Nil => {
            Err(AjisaiError::structure_error("single-element value with integer", "NIL"))
        }
        ValueData::Vector(children) if children.len() == 1 => {
            // 単一要素ベクタの場合、再帰的に処理
            get_bigint_from_value(&children[0])
        }
        ValueData::Vector(_) => {
            Err(AjisaiError::structure_error("single-element value with integer", "multi-element vector"))
        }
    }
}

// ============================================================================
// 値抽出・アンラップ関数
// ============================================================================

/// 単一要素の値から文字列を抽出する（文字列ヒント付きの値から）
///
/// 【責務】
/// - 文字列ヒント付きの値から文字列を復元
/// - ワード名として使用するため大文字に変換
///
/// 【用途】
/// - ワード名の取得（MAP/FILTER）
/// - 文字列パラメータの取得
///
/// 【エラー】
/// - NILの場合
pub fn get_word_name_from_value(value: &Value) -> Result<String> {
    if value.is_nil() {
        return Err(AjisaiError::from("Cannot get word name from NIL"));
    }

    // 分数の配列を文字列として解釈
    let fractions = value.flatten_fractions();
    let chars: String = fractions.iter()
        .filter_map(|f| {
            f.to_i64().and_then(|n| {
                if n >= 0 && n <= 0x10FFFF {
                    char::from_u32(n as u32)
                } else {
                    None
                }
            })
        })
        .collect();

    Ok(chars.to_uppercase())
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
// 値作成・ラッピング関数
// ============================================================================

/// 数値を単一要素の値として作成する
///
/// 【責務】
/// - 数値（Fraction）を1要素のValueとして作成
///
/// 【用途】
/// - 数値リテラルのスタックへのプッシュ
/// - 数値演算結果の統一形式での返却
///
/// 【引数】
/// - fraction: ラップする数値
///
/// 【戻り値】
/// - スカラーの数値Value
pub fn wrap_number(fraction: Fraction) -> Value {
    Value::from_fraction(fraction)
}

/// DateTimeを単一要素の値として作成する
///
/// 【責務】
/// - DateTime型のFractionを単一要素の値として作成
/// - 日付時刻ワードの結果を返すために使用
///
/// 【引数】
/// - fraction: ラップするタイムスタンプ（Unixタイムスタンプ）
///
/// 【戻り値】
/// - DateTime ヒント付きのスカラーValue
pub fn wrap_datetime(fraction: Fraction) -> Value {
    Value::from_datetime(fraction)
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
    fn test_wrap_number() {
        let frac = Fraction::new(BigInt::from(42), BigInt::one());
        let wrapped = wrap_number(frac.clone());
        assert!(wrapped.is_scalar());
        assert_eq!(wrapped.as_scalar(), Some(&frac));
    }

    #[test]
    fn test_get_integer_from_value() {
        let wrapped = wrap_number(Fraction::new(BigInt::from(42), BigInt::one()));
        let result = get_integer_from_value(&wrapped).unwrap();
        assert_eq!(result, 42);
    }
}
