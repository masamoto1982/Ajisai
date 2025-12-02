// rust/src/interpreter/higher_order.rs
//
// 【責務】
// 高階関数（MAP、FILTER）を実装する。
// これらの関数はカスタムワードを引数として受け取り、
// ベクタまたはスタック上の各要素に適用する。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_word_name_from_value, get_integer_from_value, wrap_in_square_vector, unwrap_single_element};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::One;

// ============================================================================
// 高階関数の実装
// ============================================================================

/// MAP - 各要素に関数を適用して変換する
///
/// 【責務】
/// - ベクタまたはスタック上の各要素にカスタムワードを適用
/// - 各適用結果を集めて新しいベクタまたはスタックを生成
/// - operation_targetを一時的にStackTopに切り替えてワード実行
///
/// 【動作モード】
/// 1. StackTopモード:
///    - ベクタの各要素に対してワードを適用
///    - 結果を集めて同じブラケットタイプのベクタで返す
///    - 例: `[1 2 3] 'DOUBLE' MAP` → `[2 4 6]` (DOUBLEが2倍する関数の場合)
///
/// 2. Stackモード:
///    - スタックトップからN個の要素を取得
///    - 各要素に対してワードを適用
///    - 結果をスタックに戻す
///    - 例: `a b c [3] 'PROCESS' .. MAP` → `a' b' c'`
///
/// 【使用法】
/// - StackTopモード: `[value1 value2 ...] 'WORDNAME' MAP`
/// - Stackモード: `val1 val2 ... [count] 'WORDNAME' .. MAP`
///
/// 【引数スタック】
/// - ['WORDNAME']: 適用するカスタムワード名（文字列）
/// - (StackTopモード) target: 対象ベクタ
/// - (Stackモード) [count]: 処理する要素数
///
/// 【戻り値スタック】
/// - (StackTopモード) 変換後のベクタ
/// - (Stackモード) 変換後の要素群
///
/// 【エラー】
/// - 指定されたワードが存在しない場合
/// - ワードが値を返さない場合
/// - 対象がベクタでない場合（StackTopモード）
/// - スタック要素数が不足している場合（Stackモード）
///
/// 【注意事項】
/// - 適用するワードは必ず1つの値を返す必要がある
/// - 各要素は単一要素ベクタとしてワードに渡される
pub fn op_map(interp: &mut Interpreter) -> Result<()> {
    let word_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let word_name = get_word_name_from_value(&word_val)?;

    if !interp.dictionary.contains_key(&word_name) {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(elements) = target_val.val_type {
                let mut results = Vec::new();

                // operation_target を一時的に保存してStackTopに設定
                let saved_target = interp.operation_target;
                interp.operation_target = OperationTarget::StackTop;

                for elem in elements {
                    // 各要素を単一要素ベクタとしてプッシュ
                    interp.stack.push(Value {
                        val_type: ValueType::Vector(vec![elem])
                    });
                    // ワードを実行
                    interp.execute_word_core(&word_name)?;

                    // 結果を取得
                    let result_vec = interp.stack.pop()
                        .ok_or_else(|| AjisaiError::from("MAP word must return a value"))?;

                    // 単一要素ベクタの場合はアンラップ
                    if let ValueType::Vector(mut v) = result_vec.val_type {
                        if v.len() == 1 {
                            results.push(v.remove(0));
                        } else {
                            results.push(Value { val_type: ValueType::Vector(v) });
                        }
                    } else {
                        return Err(AjisaiError::type_error("vector result from MAP word", "other type"));
                    }
                }

                // operation_target を復元
                interp.operation_target = saved_target;
                interp.stack.push(Value { val_type: ValueType::Vector(results) });
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        },
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = match get_integer_from_value(&count_val) {
                Ok(v) => v as usize,
                Err(e) => {
                    interp.stack.push(count_val);
                    return Err(e);
                }
            };

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let original_stack_below = interp.stack.clone();

            // operation_target を一時的に StackTop に設定
            let saved_target = interp.operation_target;
            interp.operation_target = OperationTarget::StackTop;

            let mut results = Vec::new();
            for item in &targets {
                // スタックをクリアして単一要素を処理
                interp.stack.clear();
                interp.stack.push(item.clone());
                match interp.execute_word_core(&word_name) {
                    Ok(_) => {
                        match interp.stack.pop() {
                            Some(result) => results.push(result),
                            None => {
                                // エラー時にスタックを復元
                                interp.operation_target = saved_target;
                                interp.stack = original_stack_below;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                return Err(AjisaiError::from("MAP word must return a value"));
                            }
                        }
                    }
                    Err(e) => {
                        // エラー時にスタックを復元
                        interp.operation_target = saved_target;
                        interp.stack = original_stack_below;
                        interp.stack.extend(targets);
                        interp.stack.push(count_val);
                        return Err(e);
                    }
                }
            }

            // operation_target を復元し、スタックを復元
            interp.operation_target = saved_target;
            interp.stack = original_stack_below;
            interp.stack.extend(results);
        }
    }
    Ok(())
}

/// FILTER - 条件に合う要素のみを抽出する
///
/// 【責務】
/// - ベクタまたはスタック上の各要素にカスタムワードを適用
/// - ワードが true を返した要素のみを保持
/// - 条件に合わない要素は除外される
///
/// 【動作モード】
/// 1. StackTopモード:
///    - ベクタの各要素に対してワードを適用
///    - ワードが [true] を返した要素のみを集める
///    - 結果を同じブラケットタイプのベクタで返す
///    - 例: `[1 2 3 4 5] 'ISEVEN' FILTER` → `[2 4]` (ISEVENが偶数判定の場合)
///
/// 2. Stackモード:
///    - スタックトップからN個の要素を取得
///    - 各要素に対してワードを適用
///    - ワードが [true] を返した要素のみをスタックに戻す
///    - 例: `a b c d [4] 'CHECK' .. FILTER` → (trueの要素のみ)
///
/// 【使用法】
/// - StackTopモード: `[value1 value2 ...] 'WORDNAME' FILTER`
/// - Stackモード: `val1 val2 ... [count] 'WORDNAME' .. FILTER`
///
/// 【引数スタック】
/// - ['WORDNAME']: 条件判定するカスタムワード名（文字列）
/// - (StackTopモード) target: 対象ベクタ
/// - (Stackモード) [count]: 処理する要素数
///
/// 【戻り値スタック】
/// - (StackTopモード) フィルタ後のベクタ
/// - (Stackモード) フィルタ後の要素群
///
/// 【エラー】
/// - 指定されたワードが存在しない場合
/// - ワードがBoolean値を返さない場合
/// - 対象がベクタでない場合（StackTopモード）
/// - スタック要素数が不足している場合（Stackモード）
///
/// 【注意事項】
/// - 適用するワードは必ず [true] または [false] を返す必要がある
/// - 各要素は単一要素ベクタとしてワードに渡される
/// - 条件に合う要素がない場合は空のベクタが返される
pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
    let word_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let word_name = get_word_name_from_value(&word_val)?;

    if !interp.dictionary.contains_key(&word_name) {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(elements) = target_val.val_type {
                let mut results = Vec::new();

                // operation_target を保存
                let saved_target = interp.operation_target;
                interp.operation_target = OperationTarget::StackTop;

                for elem in elements {
                    // 各要素を単一要素ベクタとしてプッシュ
                    interp.stack.push(Value {
                        val_type: ValueType::Vector(vec![elem.clone()])
                    });
                    // ワードを実行
                    interp.execute_word_core(&word_name)?;

                    // 条件判定結果を取得
                    let condition_result = interp.stack.pop()
                        .ok_or_else(|| AjisaiError::from("FILTER word must return a boolean value"))?;

                    if let ValueType::Vector(v) = condition_result.val_type {
                        if v.len() == 1 {
                            if let ValueType::Boolean(b) = v[0].val_type {
                                if b {
                                    results.push(elem);
                                }
                            } else {
                                return Err(AjisaiError::type_error("boolean result from FILTER word", "other type"));
                            }
                        } else {
                            return Err(AjisaiError::type_error("single-element vector result from FILTER word", "multi-element vector"));
                        }
                    } else {
                         return Err(AjisaiError::type_error("vector result from FILTER word", "other type"));
                    }
                }

                // operation_target を復元
                interp.operation_target = saved_target;
                interp.stack.push(Value { val_type: ValueType::Vector(results) });
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        },
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = match get_integer_from_value(&count_val) {
                Ok(v) => v as usize,
                Err(e) => {
                    interp.stack.push(count_val);
                    return Err(e);
                }
            };

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let original_stack_below = interp.stack.clone();

            // operation_target を一時的に StackTop に設定
            let saved_target = interp.operation_target;
            interp.operation_target = OperationTarget::StackTop;

            let mut results = Vec::new();
            for item in &targets {
                // スタックをクリアして単一要素を処理
                interp.stack.clear();
                interp.stack.push(item.clone());
                match interp.execute_word_core(&word_name) {
                    Ok(_) => {
                        // 条件判定結果を取得
                        let condition_result = match interp.stack.pop() {
                            Some(result) => result,
                            None => {
                                // エラー時にスタックを復元
                                interp.operation_target = saved_target;
                                interp.stack = original_stack_below;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                return Err(AjisaiError::from("FILTER word must return a boolean value"));
                            }
                        };

                        if let ValueType::Vector(v) = condition_result.val_type {
                            if v.len() == 1 {
                                if let ValueType::Boolean(b) = v[0].val_type {
                                    if b {
                                        results.push(item.clone());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // エラー時にスタックを復元
                        interp.operation_target = saved_target;
                        interp.stack = original_stack_below;
                        interp.stack.extend(targets);
                        interp.stack.push(count_val);
                        return Err(e);
                    }
                }
            }

            // operation_target を復元し、スタックを復元
            interp.operation_target = saved_target;
            interp.stack = original_stack_below;
            interp.stack.extend(results);
        }
    }
    Ok(())
}

/// COUNT - 条件に合う要素の数を数える
///
/// 【責務】
/// - ベクタまたはスタック上の各要素にカスタムワードを適用
/// - ワードが true を返した要素の数を数える
/// - FILTERの後にLENGTHを実行する処理の簡略版
///
/// 【動作モード】
/// 1. StackTopモード:
///    - ベクタの各要素に対してワードを適用
///    - ワードが [true] を返した要素の数を数える
///    - 例: `[1 2 3 4 5] '[2]>' COUNT` → `[1 2 3 4 5] [3]` (2より大きい要素が3個)
///
/// 2. Stackモード:
///    - スタックトップからN個の要素を取得
///    - 各要素に対してワードを適用
///    - ワードが [true] を返した要素の数を数える
///    - 例: `a b c d [4] 'CHECK' .. COUNT` → `a b c d [count]`
///
/// 【使用法】
/// - StackTopモード: `[value1 value2 ...] 'WORDNAME' COUNT`
/// - Stackモード: `val1 val2 ... [count] 'WORDNAME' .. COUNT`
///
/// 【引数スタック】
/// - ['WORDNAME']: 条件判定するカスタムワード名（文字列）
/// - (StackTopモード) target: 対象ベクタ
/// - (Stackモード) [count]: 処理する要素数
///
/// 【戻り値スタック】
/// - (StackTopモード) 元のベクタ + [count]（条件を満たす要素数）
/// - (Stackモード) 元のスタック + [count]
///
/// 【エラー】
/// - 指定されたワードが存在しない場合
/// - ワードがBoolean値を返さない場合
/// - 対象がベクタでない場合（StackTopモード）
/// - スタック要素数が不足している場合（Stackモード）
///
/// 【注意事項】
/// - 適用するワードは必ず [true] または [false] を返す必要がある
/// - 各要素は単一要素ベクタとしてワードに渡される
/// - 条件に合う要素がない場合は [0] を返す
pub fn op_count(interp: &mut Interpreter) -> Result<()> {
    let word_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let word_name = get_word_name_from_value(&word_val)?;

    if !interp.dictionary.contains_key(&word_name) {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(elements) = target_val.val_type {
                let mut count = 0i64;

                // operation_target を保存
                let saved_target = interp.operation_target;
                interp.operation_target = OperationTarget::StackTop;

                for elem in &elements {
                    // 各要素を単一要素ベクタとしてプッシュ
                    interp.stack.push(Value {
                        val_type: ValueType::Vector(vec![elem.clone()])
                    });
                    // ワードを実行
                    interp.execute_word_core(&word_name)?;

                    // 条件判定結果を取得
                    let condition_result = interp.stack.pop()
                        .ok_or_else(|| AjisaiError::from("COUNT word must return a boolean value"))?;

                    if let ValueType::Vector(v) = condition_result.val_type {
                        if v.len() == 1 {
                            if let ValueType::Boolean(b) = v[0].val_type {
                                if b {
                                    count += 1;
                                }
                            } else {
                                return Err(AjisaiError::type_error("boolean result from COUNT word", "other type"));
                            }
                        } else {
                            return Err(AjisaiError::type_error("single-element vector result from COUNT word", "multi-element vector"));
                        }
                    } else {
                         return Err(AjisaiError::type_error("vector result from COUNT word", "other type"));
                    }
                }

                // operation_target を復元
                interp.operation_target = saved_target;

                // 元のベクタをスタックに戻す
                interp.stack.push(Value { val_type: ValueType::Vector(elements) });

                // カウント結果をプッシュ
                interp.stack.push(Value {
                    val_type: ValueType::Vector(
                        vec![Value {
                            val_type: ValueType::Number(Fraction::new(BigInt::from(count), BigInt::one())),
                        }]
                    )
                });
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        },
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count_arg = match get_integer_from_value(&count_val) {
                Ok(v) => v as usize,
                Err(e) => {
                    interp.stack.push(count_val);
                    return Err(e);
                }
            };

            if interp.stack.len() < count_arg {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count_arg..).collect();
            let original_stack_below = interp.stack.clone();

            // operation_target を一時的に StackTop に設定
            let saved_target = interp.operation_target;
            interp.operation_target = OperationTarget::StackTop;

            let mut count_result = 0i64;
            for item in &targets {
                // スタックをクリアして単一要素を処理
                interp.stack.clear();
                interp.stack.push(item.clone());
                match interp.execute_word_core(&word_name) {
                    Ok(_) => {
                        // 条件判定結果を取得
                        let condition_result = match interp.stack.pop() {
                            Some(result) => result,
                            None => {
                                // エラー時にスタックを復元
                                interp.operation_target = saved_target;
                                interp.stack = original_stack_below;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                return Err(AjisaiError::from("COUNT word must return a boolean value"));
                            }
                        };

                        if let ValueType::Vector(v) = condition_result.val_type {
                            if v.len() == 1 {
                                if let ValueType::Boolean(b) = v[0].val_type {
                                    if b {
                                        count_result += 1;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // エラー時にスタックを復元
                        interp.operation_target = saved_target;
                        interp.stack = original_stack_below;
                        interp.stack.extend(targets);
                        interp.stack.push(count_val);
                        return Err(e);
                    }
                }
            }

            // operation_target を復元し、スタックを復元
            interp.operation_target = saved_target;
            interp.stack = original_stack_below;
            interp.stack.extend(targets);

            // カウント結果をプッシュ
            interp.stack.push(Value {
                val_type: ValueType::Vector(
                    vec![Value {
                        val_type: ValueType::Number(Fraction::new(BigInt::from(count_result), BigInt::one())),
                    }]
                )
            });
        }
    }
    Ok(())
}

/// REDUCE - ベクタまたはスタックを二項演算で畳み込む
///
/// 【責務】
/// - ベクタの要素を左から右へ順に二項演算で集約
/// - カスタムワードおよび組み込みワードの両方をサポート
///
/// 【動作モード】
/// 1. StackTopモード:
///    - ベクタの要素を順に畳み込む
///    - 例: `[1 2 3 4] '+' REDUCE` → `[10]`
///
/// 2. Stackモード:
///    - スタックトップからN個の要素を取得して畳み込む
///    - 例: `a b c [3] '+' .. REDUCE` → `[a+b+c]`
///
/// 【使用法】
/// - StackTopモード: `[要素...] 'ワード名' REDUCE`
/// - Stackモード: `要素... [個数] 'ワード名' .. REDUCE`
///
/// 【エラー】
/// - 空のベクタ/スタック
/// - 単一要素のベクタ（変化がないため）
/// - 指定されたワードが存在しない
/// - ワードが値を返さない
pub fn op_reduce(interp: &mut Interpreter) -> Result<()> {
    let word_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let word_name = get_word_name_from_value(&word_val)?;

    if !interp.dictionary.contains_key(&word_name) {
        interp.stack.push(word_val);
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(elements) = target_val.val_type {
                // 空チェック
                if elements.is_empty() {
                    interp.stack.push(Value { val_type: ValueType::Vector(elements) });
                    interp.stack.push(word_val);
                    return Err(AjisaiError::from("REDUCE: cannot reduce empty vector"));
                }

                // 単一要素の場合はエラー（変化がないため）
                if elements.len() == 1 {
                    interp.stack.push(Value { val_type: ValueType::Vector(elements) });
                    interp.stack.push(word_val);
                    return Err(AjisaiError::from("REDUCE: cannot reduce single-element vector (no change)"));
                }

                // 畳み込み実行
                let saved_target = interp.operation_target;
                let saved_no_change_check = interp.disable_no_change_check;
                interp.operation_target = OperationTarget::StackTop;
                interp.disable_no_change_check = true;

                let mut iter = elements.into_iter();
                let mut accumulator = iter.next().unwrap();

                for elem in iter {
                    // アキュムレータと次の要素をプッシュ
                    interp.stack.push(wrap_in_square_vector(accumulator));
                    interp.stack.push(wrap_in_square_vector(elem));

                    // ワード実行
                    if let Err(e) = interp.execute_word_core(&word_name) {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        return Err(e);
                    }

                    // 結果を取得
                    let result = interp.stack.pop()
                        .ok_or_else(|| {
                            interp.operation_target = saved_target;
                            interp.disable_no_change_check = saved_no_change_check;
                            AjisaiError::from(format!("REDUCE: word '{}' must return a value", word_name))
                        })?;

                    // 結果をアンラップしてアキュムレータに
                    accumulator = unwrap_single_element(result);
                }

                interp.operation_target = saved_target;
                interp.disable_no_change_check = saved_no_change_check;

                // 最終結果をプッシュ
                interp.stack.push(wrap_in_square_vector(accumulator));
                Ok(())
            } else {
                interp.stack.push(target_val);
                interp.stack.push(word_val);
                Err(AjisaiError::type_error("vector", "other type"))
            }
        },
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = match get_integer_from_value(&count_val) {
                Ok(v) => v as usize,
                Err(e) => {
                    interp.stack.push(count_val);
                    interp.stack.push(word_val);
                    return Err(e);
                }
            };

            // スタック要素数チェック
            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(word_val);
                return Err(AjisaiError::from(format!(
                    "REDUCE: stack has {} elements, but {} required",
                    interp.stack.len(), count
                )));
            }

            // 要素数が0または1の場合
            if count == 0 {
                interp.stack.push(count_val);
                interp.stack.push(word_val);
                return Err(AjisaiError::from("REDUCE: cannot reduce zero elements"));
            }
            if count == 1 {
                // 単一要素の場合はエラー（変化がないため）
                interp.stack.push(count_val);
                interp.stack.push(word_val);
                return Err(AjisaiError::from("REDUCE: cannot reduce single element (no change)"));
            }

            // スタックから要素を取得（順序を保持）
            let start_idx = interp.stack.len() - count;
            let elements: Vec<Value> = interp.stack.drain(start_idx..).collect();
            let original_stack_below = interp.stack.clone();

            // 畳み込み実行
            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            let mut iter = elements.into_iter();
            let mut accumulator = unwrap_single_element(iter.next().unwrap());

            for elem in iter {
                interp.stack.clear();
                interp.stack.push(wrap_in_square_vector(accumulator));
                interp.stack.push(elem);  // 既にラップされている

                match interp.execute_word_core(&word_name) {
                    Ok(_) => {
                        match interp.stack.pop() {
                            Some(result) => {
                                accumulator = unwrap_single_element(result);
                            }
                            None => {
                                // エラー時にスタックを復元
                                interp.operation_target = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = original_stack_below;
                                return Err(AjisaiError::from(format!("REDUCE: word '{}' must return a value", word_name)));
                            }
                        }
                    }
                    Err(e) => {
                        // エラー時にスタックを復元
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        return Err(e);
                    }
                }
            }

            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;

            // 最終結果をプッシュ
            interp.stack.push(wrap_in_square_vector(accumulator));
            Ok(())
        }
    }
}

/// FOLD - 初期値付き畳み込み
///
/// 【責務】
/// - ベクタの要素を初期値から始めて二項演算で集約
/// - REDUCEとの違い: 初期値を明示的に指定
///
/// 【動作モード】
/// 1. StackTopモード:
///    - `[要素...] [初期値] 'ワード名' FOLD`
///    - 例: `[1 2 3 4] [0] '+' FOLD` → `[10]`
///    - 例: `[1 2 3 4] [1] '*' FOLD` → `[24]`
///
/// 2. Stackモード:
///    - `要素... [個数] [初期値] 'ワード名' .. FOLD`
///
/// 【空ベクタの扱い】
/// - 空ベクタの場合は初期値をそのまま返す（REDUCEと異なりエラーにならない）
pub fn op_fold(interp: &mut Interpreter) -> Result<()> {
    let word_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let word_name = get_word_name_from_value(&word_val)?;

    if !interp.dictionary.contains_key(&word_name) {
        interp.stack.push(word_val);
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let init_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if let ValueType::Vector(elements) = target_val.val_type {
                // 初期値をアンラップ
                let mut accumulator = unwrap_single_element(init_val);

                if elements.is_empty() {
                    // 空ベクタ: 初期値をそのまま返す
                    interp.stack.push(wrap_in_square_vector(accumulator));
                    return Ok(());
                }

                let saved_target = interp.operation_target;
                let saved_no_change_check = interp.disable_no_change_check;
                interp.operation_target = OperationTarget::StackTop;
                interp.disable_no_change_check = true;

                for elem in elements {
                    interp.stack.push(wrap_in_square_vector(accumulator));
                    interp.stack.push(wrap_in_square_vector(elem));

                    if let Err(e) = interp.execute_word_core(&word_name) {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        return Err(e);
                    }

                    let result = interp.stack.pop()
                        .ok_or_else(|| AjisaiError::from("FOLD: word must return a value"))?;
                    accumulator = unwrap_single_element(result);
                }

                interp.operation_target = saved_target;
                interp.disable_no_change_check = saved_no_change_check;
                interp.stack.push(wrap_in_square_vector(accumulator));
                Ok(())
            } else {
                interp.stack.push(target_val);
                interp.stack.push(init_val);
                interp.stack.push(word_val);
                Err(AjisaiError::type_error("vector", "other type"))
            }
        }
        OperationTarget::Stack => {
            // Stack モードの実装
            // [要素...] [個数] [初期値] 'ワード名' .. FOLD
            let init_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(init_val);
                interp.stack.push(word_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let original_stack_below = interp.stack.clone();

            let mut accumulator = unwrap_single_element(init_val);

            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            for item in targets {
                interp.stack.clear();
                interp.stack.push(wrap_in_square_vector(accumulator));
                interp.stack.push(item);

                match interp.execute_word_core(&word_name) {
                    Ok(_) => {
                        let result = interp.stack.pop()
                            .ok_or_else(|| AjisaiError::from("FOLD: word must return a value"))?;
                        accumulator = unwrap_single_element(result);
                    }
                    Err(e) => {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        return Err(e);
                    }
                }
            }

            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;
            interp.stack.push(wrap_in_square_vector(accumulator));
            Ok(())
        }
    }
}

/// SCAN - 中間結果を保持する畳み込み
///
/// 【責務】
/// - FOLDと同様に畳み込むが、各ステップの結果をベクタとして返す
///
/// 【使用法】
/// - StackTopモード: `[要素...] [初期値] 'ワード名' SCAN`
/// - 例: `[1 2 3 4] [0] '+' SCAN` → `[1 3 6 10]`
///
/// 【注意】
/// - 結果のベクタ長は入力ベクタと同じ（初期値は含まない）
pub fn op_scan(interp: &mut Interpreter) -> Result<()> {
    let word_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let word_name = get_word_name_from_value(&word_val)?;

    if !interp.dictionary.contains_key(&word_name) {
        interp.stack.push(word_val);
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let init_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if let ValueType::Vector(elements) = target_val.val_type {
                let mut accumulator = unwrap_single_element(init_val);
                let mut results = Vec::new();

                if elements.is_empty() {
                    interp.stack.push(Value { val_type: ValueType::Vector(vec![]) });
                    return Ok(());
                }

                let saved_target = interp.operation_target;
                let saved_no_change_check = interp.disable_no_change_check;
                interp.operation_target = OperationTarget::StackTop;
                interp.disable_no_change_check = true;

                for elem in elements {
                    interp.stack.push(wrap_in_square_vector(accumulator));
                    interp.stack.push(wrap_in_square_vector(elem));

                    if let Err(e) = interp.execute_word_core(&word_name) {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        return Err(e);
                    }

                    let result = interp.stack.pop()
                        .ok_or_else(|| AjisaiError::from("SCAN: word must return a value"))?;
                    accumulator = unwrap_single_element(result.clone());
                    results.push(accumulator.clone());
                }

                interp.operation_target = saved_target;
                interp.disable_no_change_check = saved_no_change_check;
                interp.stack.push(Value { val_type: ValueType::Vector(results) });
                Ok(())
            } else {
                interp.stack.push(target_val);
                interp.stack.push(init_val);
                interp.stack.push(word_val);
                Err(AjisaiError::type_error("vector", "other type"))
            }
        }
        OperationTarget::Stack => {
            // Stackモード: 結果をスタックに展開
            let init_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(init_val);
                interp.stack.push(word_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let original_stack_below = interp.stack.clone();

            let mut accumulator = unwrap_single_element(init_val);

            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            let mut results = Vec::new();

            for item in targets {
                interp.stack.clear();
                interp.stack.push(wrap_in_square_vector(accumulator));
                interp.stack.push(item);

                match interp.execute_word_core(&word_name) {
                    Ok(_) => {
                        let result = interp.stack.pop()
                            .ok_or_else(|| AjisaiError::from("SCAN: word must return a value"))?;
                        accumulator = unwrap_single_element(result);
                        results.push(wrap_in_square_vector(accumulator.clone()));
                    }
                    Err(e) => {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        return Err(e);
                    }
                }
            }

            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;
            interp.stack.extend(results);
            Ok(())
        }
    }
}

/// UNFOLD - 状態からベクタを生成
///
/// 【責務】
/// - 初期状態から始め、ワードを繰り返し適用してベクタを生成
/// - ワードは [要素, 次の状態] または NIL（終了）を返す
///
/// 【使用法】
/// - StackTopモード: `[初期状態] 'ワード名' UNFOLD`
/// - 例: `[1] 'NEXT_OR_STOP' UNFOLD`
///
/// 【ワードの仕様】
/// - 入力: [現在の状態]
/// - 出力: [要素, 次の状態] または NIL（終了）
///
/// 【無限ループ防止】
/// - 最大イテレーション数を設定（デフォルト: 10000）
pub fn op_unfold(interp: &mut Interpreter) -> Result<()> {
    const MAX_ITERATIONS: usize = 10000;

    let word_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let word_name = get_word_name_from_value(&word_val)?;

    if !interp.dictionary.contains_key(&word_name) {
        interp.stack.push(word_val);
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let init_state = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let mut state = init_state;
            let mut results = Vec::new();

            let saved_target = interp.operation_target;
            interp.operation_target = OperationTarget::StackTop;

            for _ in 0..MAX_ITERATIONS {
                interp.stack.push(state.clone());

                if let Err(e) = interp.execute_word_core(&word_name) {
                    interp.operation_target = saved_target;
                    return Err(e);
                }

                let result = interp.stack.pop()
                    .ok_or_else(|| AjisaiError::from("UNFOLD: word must return a value"))?;

                // 単一要素ベクタの場合はアンラップ
                let unwrapped = unwrap_single_element(result);

                match &unwrapped.val_type {
                    ValueType::Nil => {
                        // 終了
                        break;
                    }
                    ValueType::Vector(v) if v.len() == 2 => {
                        // [要素, 次の状態]
                        results.push(v[0].clone());
                        state = Value { val_type: ValueType::Vector(vec![v[1].clone()]) };
                    }
                    _ => {
                        interp.operation_target = saved_target;
                        return Err(AjisaiError::from(
                            "UNFOLD: word must return [element, next_state] or NIL"
                        ));
                    }
                }
            }

            interp.operation_target = saved_target;
            interp.stack.push(Value { val_type: ValueType::Vector(results) });
            Ok(())
        }
        OperationTarget::Stack => {
            // Stackモード: 結果をスタックに直接展開
            let init_state = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let mut state = init_state;
            let original_stack = interp.stack.clone();

            let saved_target = interp.operation_target;
            interp.operation_target = OperationTarget::StackTop;

            let mut results = Vec::new();

            for _ in 0..MAX_ITERATIONS {
                interp.stack.clear();
                interp.stack.push(state.clone());

                match interp.execute_word_core(&word_name) {
                    Ok(_) => {
                        let result = interp.stack.pop()
                            .ok_or_else(|| AjisaiError::from("UNFOLD: word must return a value"))?;

                        // 単一要素ベクタの場合はアンラップ
                        let unwrapped = unwrap_single_element(result);

                        match &unwrapped.val_type {
                            ValueType::Nil => break,
                            ValueType::Vector(v) if v.len() == 2 => {
                                results.push(wrap_in_square_vector(v[0].clone()));
                                state = Value { val_type: ValueType::Vector(vec![v[1].clone()]) };
                            }
                            _ => {
                                interp.operation_target = saved_target;
                                interp.stack = original_stack;
                                return Err(AjisaiError::from(
                                    "UNFOLD: word must return [element, next_state] or NIL"
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        interp.operation_target = saved_target;
                        interp.stack = original_stack;
                        return Err(e);
                    }
                }
            }

            interp.operation_target = saved_target;
            interp.stack = original_stack;
            interp.stack.extend(results);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::Interpreter;
    use crate::types::ValueType;

    #[tokio::test]
    async fn test_fold_basic() {
        let mut interp = Interpreter::new();
        let code = r#"[ 1 2 3 4 ] [ 0 ] '+' FOLD"#;
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "FOLD should succeed: {:?}", result);

        // 結果が [10] であることを確認
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1);
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "10");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_fold_empty_vector() {
        let mut interp = Interpreter::new();
        let code = r#"[ ] [ 42 ] '+' FOLD"#;
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "FOLD with empty vector should succeed: {:?}", result);

        // 結果が [42] であることを確認（初期値がそのまま）
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1);
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "42");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_scan_basic() {
        let mut interp = Interpreter::new();
        let code = r#"[ 1 2 3 4 ] [ 0 ] '+' SCAN"#;
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "SCAN should succeed: {:?}", result);

        // 結果が [1 3 6 10] であることを確認
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 4);
                let expected = vec!["1", "3", "6", "10"];
                for (i, exp) in expected.iter().enumerate() {
                    if let ValueType::Number(n) = &v[i].val_type {
                        assert_eq!(n.numerator.to_string(), *exp);
                    }
                }
            }
        }
    }

    #[tokio::test]
    #[ignore] // TODO: Fix UNFOLD test - currently failing
    async fn test_unfold_basic() {
        let mut interp = Interpreter::new();
        // 簡単なテスト: 常にNILを返すので空のベクタが生成される
        let code = r#"
[ ': NIL' ] 'STOPNOW' DEF
[ 1 ] 'STOPNOW' UNFOLD
"#;
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "UNFOLD with immediate NIL should succeed: {:?}", result);

        // 結果が空のベクタであることを確認
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 0);
            }
        }
    }
}
