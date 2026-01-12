// rust/src/interpreter/helpers.rs
//
// 【責務】
// インタプリタ内で頻繁に使用される共通ヘルパー関数を提供する。
// 型変換、値の抽出、エラーハンドリングなどの定型処理を一元化し、
// コードの重複を排除して保守性を向上させる。
//
// 統一分数アーキテクチャ対応版

use crate::error::{AjisaiError, Result};
use crate::types::Value;
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

// ============================================================================
// 整数・インデックス抽出関数
// ============================================================================

/// 単一要素の値から整数値（i64）を抽出する
///
/// 【責務】
/// - 値が単一要素であることを検証
/// - 内部の数値が整数（分母が1）であることを検証
/// - i64範囲内に収まることを検証
///
/// 【用途】
/// - カウント引数の取得（MAP/FILTER/算術演算のSTACKモード）
/// - 整数パラメータの取得
///
/// 【エラー】
/// - 単一要素でない場合
/// - 分数の場合
/// - i64範囲を超える場合
pub fn get_integer_from_value(value: &Value) -> Result<i64> {
    if value.data.len() != 1 {
        return Err(AjisaiError::structure_error("single-element value with integer", "multi-element or empty value"));
    }

    let f = &value.data[0];
    if f.denominator != BigInt::one() {
        return Err(AjisaiError::structure_error("integer", "fraction"));
    }

    f.numerator.to_i64().ok_or_else(|| AjisaiError::from("Integer value is too large for i64"))
}

/// 単一要素の値からBigInt整数値を抽出する
///
/// 【責務】
/// - 値が単一要素であることを検証
/// - 内部の数値が整数（分母が1）であることを検証
/// - BigIntとして返す（サイズ制限なし）
///
/// 【用途】
/// - インデックス指定（GET/INSERT/REPLACE/REMOVE）
/// - 大きな整数値の取得
///
/// 【エラー】
/// - 単一要素でない場合
/// - 分数の場合
pub fn get_bigint_from_value(value: &Value) -> Result<BigInt> {
    if value.data.len() != 1 {
        return Err(AjisaiError::structure_error("single-element value with integer", "multi-element or empty value"));
    }

    let f = &value.data[0];
    if f.denominator != BigInt::one() {
        return Err(AjisaiError::structure_error("integer", "fraction"));
    }

    Ok(f.numerator.clone())
}

// ============================================================================
// 値抽出・アンラップ関数
// ============================================================================

/// 値から数値（Fraction）への参照を抽出する
///
/// 【責務】
/// - 単一要素の値の場合はその要素を返す
///
/// 【用途】
/// - 算術演算での数値取得
/// - 数値が必要な演算全般
///
/// 【エラー】
/// - 空の値の場合
#[allow(dead_code)]
pub fn extract_number(val: &Value) -> Result<&Fraction> {
    if val.data.len() == 1 {
        Ok(&val.data[0])
    } else if val.data.is_empty() {
        Err(AjisaiError::from("Cannot extract number from NIL"))
    } else {
        Err(AjisaiError::structure_error("single-element value", "multi-element value"))
    }
}

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
/// - 空の値の場合
pub fn get_word_name_from_value(value: &Value) -> Result<String> {
    if value.data.is_empty() {
        return Err(AjisaiError::from("Cannot get word name from NIL"));
    }

    // 分数の配列を文字列として解釈
    let chars: String = value.data.iter()
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
/// - 単一要素の数値Value
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
/// - DateTime ヒント付きの単一要素Value
pub fn wrap_datetime(fraction: Fraction) -> Value {
    Value::from_datetime(fraction)
}

/// 値を作成するための互換性関数
///
/// 【注意】
/// この関数は後方互換性のために残されています。
/// 新しいコードでは直接 Value のコンストラクタを使用してください。
pub fn wrap_value(value: Value) -> Value {
    value
}

/// 後方互換性: 単一要素の値を取り出す
///
/// 統一分数アーキテクチャでは、この関数は値をそのまま返します。
/// 旧アーキテクチャとの互換性のために残されています。
pub fn unwrap_single_element(value: Value) -> Value {
    value
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
        assert_eq!(wrapped.data.len(), 1);
        assert_eq!(wrapped.data[0], frac);
    }

    #[test]
    fn test_extract_number() {
        let frac = Fraction::new(BigInt::from(42), BigInt::one());
        let wrapped = wrap_number(frac.clone());
        let extracted = extract_number(&wrapped).unwrap();
        assert_eq!(extracted, &frac);
    }

    #[test]
    fn test_get_integer_from_value() {
        let wrapped = wrap_number(Fraction::new(BigInt::from(42), BigInt::one()));
        let result = get_integer_from_value(&wrapped).unwrap();
        assert_eq!(result, 42);
    }
}
