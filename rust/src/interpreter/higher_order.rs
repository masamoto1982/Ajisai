// rust/src/interpreter/higher_order.rs
//
// 【責務】
// 高階関数（MAP、FILTER）を実装する。
// これらの関数はカスタムワードを引数として受け取り、
// ベクタまたはスタック上の各要素に適用する。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_word_name_from_value, get_integer_from_value};
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
///    - 例: `a b c [3] 'PROCESS' STACK MAP` → `a' b' c'`
///
/// 【使用法】
/// - StackTopモード: `[value1 value2 ...] 'WORDNAME' MAP`
/// - Stackモード: `val1 val2 ... [count] 'WORDNAME' STACK MAP`
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
                    interp.execute_word_sync(&word_name)?;

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
            let count = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let original_stack_below = interp.stack.clone();

            // operation_target を一時的に StackTop に設定
            let saved_target = interp.operation_target;
            interp.operation_target = OperationTarget::StackTop;

            let mut results = Vec::new();
            for item in targets {
                // スタックをクリアして単一要素を処理
                interp.stack.clear();
                interp.stack.push(item);
                interp.execute_word_sync(&word_name)?;

                let result = interp.stack.pop()
                    .ok_or_else(|| AjisaiError::from("MAP word must return a value"))?;
                results.push(result);
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
///    - 例: `a b c d [4] 'CHECK' STACK FILTER` → (trueの要素のみ)
///
/// 【使用法】
/// - StackTopモード: `[value1 value2 ...] 'WORDNAME' FILTER`
/// - Stackモード: `val1 val2 ... [count] 'WORDNAME' STACK FILTER`
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
                    interp.execute_word_sync(&word_name)?;

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
            let count = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let original_stack_below = interp.stack.clone();

            // operation_target を一時的に StackTop に設定
            let saved_target = interp.operation_target;
            interp.operation_target = OperationTarget::StackTop;

            let mut results = Vec::new();
            for item in targets {
                // スタックをクリアして単一要素を処理
                interp.stack.clear();
                interp.stack.push(item.clone());
                interp.execute_word_sync(&word_name)?;

                // 条件判定結果を取得
                let condition_result = interp.stack.pop()
                    .ok_or_else(|| AjisaiError::from("FILTER word must return a boolean value"))?;

                if let ValueType::Vector(v) = condition_result.val_type {
                    if v.len() == 1 {
                        if let ValueType::Boolean(b) = v[0].val_type {
                            if b {
                                results.push(item);
                            }
                        }
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
///    - 例: `a b c d [4] 'CHECK' STACK COUNT` → `a b c d [count]`
///
/// 【使用法】
/// - StackTopモード: `[value1 value2 ...] 'WORDNAME' COUNT`
/// - Stackモード: `val1 val2 ... [count] 'WORDNAME' STACK COUNT`
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
                    interp.execute_word_sync(&word_name)?;

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
            let count_arg = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count_arg {
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
                interp.execute_word_sync(&word_name)?;

                // 条件判定結果を取得
                let condition_result = interp.stack.pop()
                    .ok_or_else(|| AjisaiError::from("COUNT word must return a boolean value"))?;

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
