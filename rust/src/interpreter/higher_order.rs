// rust/src/interpreter/higher_order.rs
//
// 【責務】
// 高階関数（MAP、FILTER）を実装する。
// これらの関数はカスタムワードを引数として受け取り、
// ベクタまたはスタック上の各要素に適用する。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_word_name_from_value, get_integer_from_value, unwrap_single_element, wrap_single_value};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;

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
        interp.stack.push(word_val);
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // TensorまたはVectorを処理
            let elements = match target_val.val_type {
                ValueType::Vector(v) => v,
                ValueType::Tensor(ref t) => {
                    // Tensorを要素のVectorに変換
                    t.data().iter().map(|f| Value {
                        val_type: ValueType::Number(f.clone())
                    }).collect()
                },
                _ => {
                    interp.stack.push(target_val);
                    return Err(AjisaiError::type_error("vector or tensor", "other type"));
                }
            };

            let mut results = Vec::new();

            // operation_target を一時的に保存してStackTopに設定
            // MAP内部では「変化なし」チェックを無効化
            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            for elem in elements {
                // 各要素をラップしてプッシュ（数値はTensor、その他はVector）
                let wrapped = match &elem.val_type {
                    ValueType::Number(f) => {
                        use crate::types::tensor::Tensor;
                        Value::from_tensor(Tensor::vector(vec![f.clone()]))
                    },
                    _ => Value { val_type: ValueType::Vector(vec![elem]) }
                };
                interp.stack.push(wrapped);
                // ワードを実行
                interp.execute_word_core(&word_name)?;

                // 結果を取得
                let result_vec = interp.stack.pop()
                    .ok_or_else(|| AjisaiError::from("MAP word must return a value"))?;

                // 単一要素ベクタまたはテンソルの場合はアンラップ
                match result_vec.val_type {
                    ValueType::Vector(mut v) if v.len() == 1 => {
                        results.push(v.remove(0));
                    },
                    ValueType::Vector(v) => {
                        results.push(Value { val_type: ValueType::Vector(v) });
                    },
                    ValueType::Tensor(t) if t.data().len() == 1 => {
                        // 単一要素Tensorは数値としてアンラップ
                        results.push(Value { val_type: ValueType::Number(t.data()[0].clone()) });
                    },
                    ValueType::Tensor(t) => {
                        // 複数要素Tensorはそのまま保持
                        results.push(Value { val_type: ValueType::Tensor(t) });
                    },
                    _ => {
                        return Err(AjisaiError::type_error("vector or tensor result from MAP word", "other type"));
                    }
                }
            }

            // operation_target とno_change_checkを復元
            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;

            // Phase 1.2: 結果の返却形式を統一
            // 全て数値の場合はTensorに変換、混合型はVectorのまま
            let result = if results.iter().all(|v| matches!(v.val_type, ValueType::Number(_))) {
                // すべて数値ならTensorに変換
                let fracs: Vec<Fraction> = results.iter()
                    .filter_map(|v| if let ValueType::Number(f) = &v.val_type { Some(f.clone()) } else { None })
                    .collect();
                Value::from_tensor(crate::types::tensor::Tensor::vector(fracs))
            } else {
                // 混合型はVectorのまま
                Value { val_type: ValueType::Vector(results) }
            };
            interp.stack.push(result);
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
            // MAP内部では「変化なし」チェックを無効化
            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

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
                                interp.disable_no_change_check = saved_no_change_check;
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
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        interp.stack.extend(targets);
                        interp.stack.push(count_val);
                        return Err(e);
                    }
                }
            }

            // operation_target とno_change_checkを復元し、スタックを復元
            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
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
        interp.stack.push(word_val);
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // TensorまたはVectorを処理
            let elements = match target_val.val_type {
                ValueType::Vector(v) => v,
                ValueType::Tensor(ref t) => {
                    // Tensorを要素のVectorに変換
                    t.data().iter().map(|f| Value {
                        val_type: ValueType::Number(f.clone())
                    }).collect()
                },
                _ => {
                    interp.stack.push(target_val);
                    return Err(AjisaiError::type_error("vector or tensor", "other type"));
                }
            };

            let mut results = Vec::new();

            // operation_target と no_change_check を保存
            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            for elem in elements {
                    // 各要素をラップしてプッシュ（数値はTensor、その他はVector）
                    let wrapped = match &elem.val_type {
                        ValueType::Number(f) => {
                            use crate::types::tensor::Tensor;
                            Value::from_tensor(Tensor::vector(vec![f.clone()]))
                        },
                        _ => Value { val_type: ValueType::Vector(vec![elem.clone()]) }
                    };
                    interp.stack.push(wrapped);
                    // ワードを実行
                    interp.execute_word_core(&word_name)?;

                    // 条件判定結果を取得
                    let condition_result = interp.stack.pop()
                        .ok_or_else(|| AjisaiError::from("FILTER word must return a boolean value"))?;

                    // VectorまたはTensorからBoolean値を抽出
                    let is_true = match condition_result.val_type {
                        ValueType::Vector(v) if v.len() == 1 => {
                            if let ValueType::Boolean(b) = v[0].val_type {
                                b
                            } else {
                                return Err(AjisaiError::type_error("boolean result from FILTER word", "other type"));
                            }
                        },
                        ValueType::Tensor(_) => {
                            // Tensorから直接Booleanは取得できないのでエラー
                            return Err(AjisaiError::type_error("boolean result from FILTER word", "tensor type"));
                        },
                        _ => {
                            return Err(AjisaiError::type_error("boolean vector result from FILTER word", "other type"));
                        }
                    };

                    if is_true {
                        results.push(elem);
                    }
                }

            // operation_target と no_change_check を復元
            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;

            // Phase 1.2: 結果の返却形式を統一
            // 全て数値の場合はTensorに変換、混合型はVectorのまま
            let result = if results.iter().all(|v| matches!(v.val_type, ValueType::Number(_))) {
                // すべて数値ならTensorに変換
                let fracs: Vec<Fraction> = results.iter()
                    .filter_map(|v| if let ValueType::Number(f) = &v.val_type { Some(f.clone()) } else { None })
                    .collect();
                Value::from_tensor(crate::types::tensor::Tensor::vector(fracs))
            } else {
                // 混合型はVectorのまま
                Value { val_type: ValueType::Vector(results) }
            };
            interp.stack.push(result);
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

            // operation_target と no_change_check を一時的に設定
            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

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
                                interp.disable_no_change_check = saved_no_change_check;
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
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        interp.stack.extend(targets);
                        interp.stack.push(count_val);
                        return Err(e);
                    }
                }
            }

            // operation_target と no_change_check を復元し、スタックを復元
            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;
            interp.stack.extend(results);
        }
    }
    Ok(())
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

            // TensorまたはVectorを処理
            let elements = match target_val.val_type {
                ValueType::Vector(v) => v,
                ValueType::Tensor(ref t) => {
                    // Tensorを要素のVectorに変換
                    t.data().iter().map(|f| Value {
                        val_type: ValueType::Number(f.clone())
                    }).collect()
                },
                _ => {
                    interp.stack.push(target_val);
                    interp.stack.push(init_val);
                    interp.stack.push(word_val);
                    return Err(AjisaiError::type_error("vector or tensor", "other type"));
                }
            };

            // 初期値をアンラップ
            let mut accumulator = unwrap_single_element(init_val);

            if elements.is_empty() {
                // 空ベクタ: 初期値をそのまま返す
                interp.stack.push(wrap_single_value(accumulator));
                return Ok(());
            }

                let saved_target = interp.operation_target;
                let saved_no_change_check = interp.disable_no_change_check;
                interp.operation_target = OperationTarget::StackTop;
                interp.disable_no_change_check = true;

                for elem in elements {
                    interp.stack.push(wrap_single_value(accumulator));
                    interp.stack.push(wrap_single_value(elem));

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
            interp.stack.push(wrap_single_value(accumulator));
            Ok(())
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
                interp.stack.push(wrap_single_value(accumulator));
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
            interp.stack.push(wrap_single_value(accumulator));
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
            let init_state = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(word_val.clone());
                AjisaiError::StackUnderflow
            })?;

            let mut state = init_state.clone();
            let mut results = Vec::new();

            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            let mut iteration_count = 0;
            loop {
                if iteration_count >= MAX_ITERATIONS {
                    // MAX_ITERATIONSに達した場合はエラー
                    interp.operation_target = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack.push(init_state);
                    interp.stack.push(word_val);
                    return Err(AjisaiError::from(
                        "UNFOLD: maximum iterations (10000) exceeded - possible infinite loop"
                    ));
                }
                iteration_count += 1;

                interp.stack.push(state.clone());

                if let Err(e) = interp.execute_word_core(&word_name) {
                    // 【修正】エラー時にスタックを復元
                    interp.operation_target = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack.push(init_state);
                    interp.stack.push(word_val);
                    return Err(e);
                }

                // ワードは入力と出力の両方をスタックに残すので、両方ポップする
                let result = interp.stack.pop()
                    .ok_or_else(|| {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        AjisaiError::from("UNFOLD: word must return a value")
                    })?;
                let _input = interp.stack.pop(); // 入力状態を破棄

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

                        // 次の状態がNILの場合は終了
                        if matches!(&v[1].val_type, ValueType::Nil) {
                            break;
                        }

                        state = Value { val_type: ValueType::Vector(vec![v[1].clone()]) };
                    }
                    _ => {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        // 【修正】エラー時にスタックを復元
                        interp.stack.push(init_state);
                        interp.stack.push(word_val);
                        return Err(AjisaiError::from(
                            "UNFOLD: word must return [element, next_state] or NIL"
                        ));
                    }
                }
            }

            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack.push(Value { val_type: ValueType::Vector(results) });
            Ok(())
        }
        OperationTarget::Stack => {
            // Stackモード: 結果をスタックに直接展開
            let init_state = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(word_val.clone());
                AjisaiError::StackUnderflow
            })?;

            let mut state = init_state.clone();
            let original_stack = interp.stack.clone();

            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            let mut results = Vec::new();
            let mut iteration_count = 0;

            loop {
                if iteration_count >= MAX_ITERATIONS {
                    interp.operation_target = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack = original_stack;
                    interp.stack.push(init_state);
                    interp.stack.push(word_val);
                    return Err(AjisaiError::from(
                        "UNFOLD: maximum iterations (10000) exceeded - possible infinite loop"
                    ));
                }
                iteration_count += 1;

                interp.stack.clear();
                interp.stack.push(state.clone());

                match interp.execute_word_core(&word_name) {
                    Ok(_) => {
                        // ワードは入力と出力の両方をスタックに残すので、両方ポップする
                        let result = interp.stack.pop()
                            .ok_or_else(|| AjisaiError::from("UNFOLD: word must return a value"))?;
                        let _input = interp.stack.pop(); // 入力状態を破棄

                        // 単一要素ベクタの場合はアンラップ
                        let unwrapped = unwrap_single_element(result);

                        match &unwrapped.val_type {
                            ValueType::Nil => break,
                            ValueType::Vector(v) if v.len() == 2 => {
                                // 修正: 数値ならTensor、非数値ならVectorでラップ
                                results.push(wrap_single_value(v[0].clone()));

                                // 次の状態がNILの場合は終了
                                if matches!(&v[1].val_type, ValueType::Nil) {
                                    break;
                                }

                                state = Value { val_type: ValueType::Vector(vec![v[1].clone()]) };
                            }
                            _ => {
                                interp.operation_target = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = original_stack;
                                interp.stack.push(init_state);
                                interp.stack.push(word_val);
                                return Err(AjisaiError::from(
                                    "UNFOLD: word must return [element, next_state] or NIL"
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack;
                        interp.stack.push(init_state);
                        interp.stack.push(word_val);
                        return Err(e);
                    }
                }
            }

            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
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
    async fn test_unfold_fixed_return() {
        let mut interp = Interpreter::new();
        // 常に [1 2] を返すワードで、UNFOLDが1回だけ実行されることをテスト
        let code = r#"
[ ': [1 NIL]' ] 'GEN_ONE' DEF
[ 0 ] 'GEN_ONE' UNFOLD
"#;
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "UNFOLD should succeed: {:?}", result);

        // スタックの内容を確認
        println!("Stack after UNFOLD:");
        for (i, val) in interp.stack.iter().enumerate() {
            println!("  [{}]: {}", i, val);
        }

        // 結果が [1] であることを確認（1回だけ生成）
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                println!("Result vector length: {}", v.len());
                assert_eq!(v.len(), 1, "Should generate 1 element");
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "1");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_unfold_immediate_nil() {
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
