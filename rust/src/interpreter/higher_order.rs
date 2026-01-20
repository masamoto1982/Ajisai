// rust/src/interpreter/higher_order.rs
//
// 【責務】
// 高階関数（MAP、FILTER）を実装する。
// これらの関数はカスタムワードを引数として受け取り、
// ベクタまたはスタック上の各要素に適用する。
//
// 統一分数アーキテクチャ版

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_word_name_from_value, get_integer_from_value, unwrap_single_element, wrap_value};
use crate::types::{Value, ValueData, DisplayHint, Block};

// ============================================================================
// ヘルパー関数（統一Value宇宙アーキテクチャ用）
// ============================================================================

/// Block または ワード名を表す列挙型
enum BlockOrWord {
    Block(Block),
    WordName(String),
}

/// Value から Block または ワード名を抽出する
fn get_block_or_word(val: &Value) -> Result<BlockOrWord> {
    match &val.data {
        ValueData::Block(block) => Ok(BlockOrWord::Block(block.clone())),
        ValueData::Vector(_) if val.display_hint == DisplayHint::String => {
            get_word_name_from_value(val).map(BlockOrWord::WordName)
        }
        _ => Err(AjisaiError::from("Expected block or word name"))
    }
}

/// ベクタ値かどうかを判定
fn is_vector_value(val: &Value) -> bool {
    matches!(&val.data, ValueData::Vector(_))
}

/// 真偽値として解釈
fn is_boolean_true(val: &Value) -> bool {
    if val.display_hint == DisplayHint::Boolean {
        if let Some(f) = val.as_scalar() {
            return !f.is_zero();
        }
    }
    false
}

/// ベクタの子要素を取得
fn get_vector_children(val: &Value) -> Option<&Vec<Value>> {
    if let ValueData::Vector(children) = &val.data {
        Some(children)
    } else {
        None
    }
}

/// ベクタの要素を再構築する（新しい再帰的Value用）
fn reconstruct_vector_elements(val: &Value) -> Vec<Value> {
    if let Some(children) = get_vector_children(val) {
        children.clone()
    } else {
        // スカラーの場合は単一要素として返す
        vec![val.clone()]
    }
}

// ============================================================================
// 高階関数の実装
// ============================================================================

/// MAP - 各要素に関数を適用して変換する
pub fn op_map(interp: &mut Interpreter) -> Result<()> {
    let block_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // Block または ワード名を取得
    let block_or_word = match get_block_or_word(&block_val) {
        Ok(bow) => bow,
        Err(e) => {
            interp.stack.push(block_val);
            return Err(e);
        }
    };

    // ワード名の場合は辞書の存在確認
    if let BlockOrWord::WordName(ref word_name) = block_or_word {
        if !interp.dictionary.contains_key(word_name) {
            interp.stack.push(block_val);
            return Err(AjisaiError::UnknownWord(word_name.clone()));
        }
    }

    /// Block または ワードを実行するヘルパー
    fn execute_block_or_word(interp: &mut Interpreter, bow: &BlockOrWord) -> Result<()> {
        match bow {
            BlockOrWord::Block(block) => interp.execute_block(block),
            BlockOrWord::WordName(word_name) => interp.execute_word_core(word_name),
        }
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // Vectorを処理（NIL = 空ベクタとして扱う）
            let elements = if target_val.is_nil() {
                vec![]
            } else if is_vector_value(&target_val) {
                reconstruct_vector_elements(&target_val)
            } else {
                interp.stack.push(target_val);
                interp.stack.push(block_val);
                return Err(AjisaiError::structure_error("vector", "other format"));
            };

            // 空ベクタ/NILの場合はNILを返す
            if elements.is_empty() {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            let mut results = Vec::new();

            // 元のスタックを保存
            let original_stack_below = interp.stack.clone();

            // operation_target を一時的に保存してStackTopに設定
            // MAP内部では「変化なし」チェックを無効化
            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            for elem in &elements {
                // スタックをクリアして単一要素を処理（Stackモードと同様）
                interp.stack.clear();
                // 各要素を単一要素Vectorでラップしてプッシュ
                interp.stack.push(wrap_value(elem.clone()));
                // Block または ワードを実行
                match execute_block_or_word(interp, &block_or_word) {
                    Ok(_) => {
                        // 結果を取得
                        match interp.stack.pop() {
                            Some(result_val) => {
                                // 結果の処理：スカラーもベクタも受け入れる
                                if is_vector_value(&result_val) {
                                    // ベクタの場合
                                    let v = reconstruct_vector_elements(&result_val);
                                    if v.len() == 1 {
                                        results.push(v[0].clone());
                                    } else {
                                        results.push(Value::from_vector(v));
                                    }
                                } else {
                                    // スカラーやNILの場合はそのまま追加
                                    results.push(result_val);
                                }
                            },
                            None => {
                                // エラー時にスタックを復元
                                interp.operation_target = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = original_stack_below;
                                interp.stack.push(Value::from_vector(elements));
                                interp.stack.push(block_val);
                                return Err(AjisaiError::from("MAP block must return a value"));
                            }
                        }
                    }
                    Err(e) => {
                        // エラー時にスタックを復元
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        interp.stack.push(Value::from_vector(elements));
                        interp.stack.push(block_val);
                        return Err(e);
                    }
                }
            }

            // operation_target とno_change_checkを復元、スタックを復元
            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;

            // 結果をVectorとして返す
            interp.stack.push(Value::from_vector(results));
        },
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = match get_integer_from_value(&count_val) {
                Ok(v) => v as usize,
                Err(e) => {
                    interp.stack.push(count_val);
                    interp.stack.push(block_val);
                    return Err(e);
                }
            };

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(block_val);
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
                match execute_block_or_word(interp, &block_or_word) {
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
                                interp.stack.push(block_val);
                                return Err(AjisaiError::from("MAP block must return a value"));
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
                        interp.stack.push(block_val);
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
pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
    let block_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // Block または ワード名を取得
    let block_or_word = match get_block_or_word(&block_val) {
        Ok(bow) => bow,
        Err(e) => {
            interp.stack.push(block_val);
            return Err(e);
        }
    };

    // ワード名の場合は辞書の存在確認
    if let BlockOrWord::WordName(ref word_name) = block_or_word {
        if !interp.dictionary.contains_key(word_name) {
            interp.stack.push(block_val);
            return Err(AjisaiError::UnknownWord(word_name.clone()));
        }
    }

    /// Block または ワードを実行するヘルパー
    fn execute_block_or_word(interp: &mut Interpreter, bow: &BlockOrWord) -> Result<()> {
        match bow {
            BlockOrWord::Block(block) => interp.execute_block(block),
            BlockOrWord::WordName(word_name) => interp.execute_word_core(word_name),
        }
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // Vectorを処理（NIL = 空ベクタとして扱う）
            let elements = if target_val.is_nil() {
                vec![]
            } else if is_vector_value(&target_val) {
                reconstruct_vector_elements(&target_val)
            } else {
                interp.stack.push(target_val);
                interp.stack.push(block_val);
                return Err(AjisaiError::structure_error("vector", "other format"));
            };

            // 空ベクタ/NILの場合はNILを返す
            if elements.is_empty() {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            let mut results = Vec::new();

            // 元のスタックを保存（MAPと同様）
            let original_stack_below = interp.stack.clone();

            // operation_target と no_change_check を保存
            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            for elem in &elements {
                // スタックをクリアして単一要素を処理（MAPと同様）
                interp.stack.clear();
                // 各要素を単一要素Vectorでラップしてプッシュ
                interp.stack.push(wrap_value(elem.clone()));
                // Block または ワードを実行
                match execute_block_or_word(interp, &block_or_word) {
                    Ok(_) => {
                        // 条件判定結果を取得
                        let condition_result = interp.stack.pop()
                            .ok_or_else(|| AjisaiError::from("FILTER block must return a boolean value"))?;

                        // VectorからBoolean値を抽出
                        let is_true = if is_vector_value(&condition_result) {
                            let v = reconstruct_vector_elements(&condition_result);
                            if v.len() == 1 {
                                is_boolean_true(&v[0])
                            } else {
                                // エラー時にスタックを復元
                                interp.operation_target = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = original_stack_below;
                                interp.stack.push(Value::from_vector(elements));
                                interp.stack.push(block_val);
                                return Err(AjisaiError::structure_error("boolean result from FILTER block", "other format"));
                            }
                        } else {
                            // エラー時にスタックを復元
                            interp.operation_target = saved_target;
                            interp.disable_no_change_check = saved_no_change_check;
                            interp.stack = original_stack_below;
                            interp.stack.push(Value::from_vector(elements));
                            interp.stack.push(block_val);
                            return Err(AjisaiError::structure_error("boolean vector result from FILTER block", "other format"));
                        };

                        if is_true {
                            results.push(elem.clone());
                        }
                    }
                    Err(e) => {
                        // エラー時にスタックを復元
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        interp.stack.push(Value::from_vector(elements));
                        interp.stack.push(block_val);
                        return Err(e);
                    }
                }
            }

            // operation_target と no_change_check を復元し、スタックを復元
            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;

            // 結果が空の場合はNILを返す（空ベクタ禁止ルール）
            if results.is_empty() {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_vector(results));
            }
        },
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = match get_integer_from_value(&count_val) {
                Ok(v) => v as usize,
                Err(e) => {
                    interp.stack.push(count_val);
                    interp.stack.push(block_val);
                    return Err(e);
                }
            };

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(block_val);
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
                match execute_block_or_word(interp, &block_or_word) {
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
                                interp.stack.push(block_val);
                                return Err(AjisaiError::from("FILTER block must return a boolean value"));
                            }
                        };

                        if is_vector_value(&condition_result) {
                            let v = reconstruct_vector_elements(&condition_result);
                            if v.len() == 1 && is_boolean_true(&v[0]) {
                                results.push(item.clone());
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
                        interp.stack.push(block_val);
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
pub fn op_fold(interp: &mut Interpreter) -> Result<()> {
    let block_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // Block または ワード名を取得
    let block_or_word = match get_block_or_word(&block_val) {
        Ok(bow) => bow,
        Err(e) => {
            interp.stack.push(block_val);
            return Err(e);
        }
    };

    // ワード名の場合は辞書の存在確認
    if let BlockOrWord::WordName(ref word_name) = block_or_word {
        if !interp.dictionary.contains_key(word_name) {
            interp.stack.push(block_val);
            return Err(AjisaiError::UnknownWord(word_name.clone()));
        }
    }

    /// Block または ワードを実行するヘルパー
    fn execute_block_or_word(interp: &mut Interpreter, bow: &BlockOrWord) -> Result<()> {
        match bow {
            BlockOrWord::Block(block) => interp.execute_block(block),
            BlockOrWord::WordName(word_name) => interp.execute_word_core(word_name),
        }
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let init_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // Vectorを処理（NIL = 空ベクタとして扱う）
            let elements = if target_val.is_nil() {
                vec![]
            } else if is_vector_value(&target_val) {
                reconstruct_vector_elements(&target_val)
            } else {
                interp.stack.push(target_val);
                interp.stack.push(init_val);
                interp.stack.push(block_val);
                return Err(AjisaiError::structure_error("vector", "other format"));
            };

            // 初期値をアンラップ
            let mut accumulator = unwrap_single_element(init_val);

            if elements.is_empty() {
                // 空ベクタ/NIL: 初期値をそのまま返す
                interp.stack.push(wrap_value(accumulator));
                return Ok(());
            }

            // 元のスタックを保存（MAPと同様）
            let original_stack_below = interp.stack.clone();

            // operation_target と no_change_check を保存
            let saved_target = interp.operation_target;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target = OperationTarget::StackTop;
            interp.disable_no_change_check = true;

            for elem in &elements {
                // スタックをクリアして処理（MAPと同様）
                interp.stack.clear();
                interp.stack.push(wrap_value(accumulator.clone()));
                interp.stack.push(wrap_value(elem.clone()));

                match execute_block_or_word(interp, &block_or_word) {
                    Ok(_) => {
                        let result = interp.stack.pop()
                            .ok_or_else(|| {
                                // エラー時にスタックを復元
                                interp.operation_target = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = original_stack_below.clone();
                                interp.stack.push(Value::from_vector(elements.clone()));
                                interp.stack.push(wrap_value(accumulator.clone()));
                                interp.stack.push(block_val.clone());
                                AjisaiError::from("FOLD: block must return a value")
                            })?;
                        accumulator = unwrap_single_element(result);
                    }
                    Err(e) => {
                        // エラー時にスタックを復元
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        interp.stack.push(Value::from_vector(elements));
                        interp.stack.push(wrap_value(accumulator));
                        interp.stack.push(block_val);
                        return Err(e);
                    }
                }
            }

            // operation_target と no_change_check を復元し、スタックを復元
            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;
            interp.stack.push(wrap_value(accumulator));
            Ok(())
        }
        OperationTarget::Stack => {
            // Stack モードの実装
            // [要素...] [個数] [初期値] "block" .. FOLD
            let init_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(init_val);
                interp.stack.push(block_val);
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
                interp.stack.push(wrap_value(accumulator));
                interp.stack.push(item);

                match execute_block_or_word(interp, &block_or_word) {
                    Ok(_) => {
                        let result = interp.stack.pop()
                            .ok_or_else(|| AjisaiError::from("FOLD: block must return a value"))?;
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
            interp.stack.push(wrap_value(accumulator));
            Ok(())
        }
    }
}

/// UNFOLD - 状態からベクタを生成
pub fn op_unfold(interp: &mut Interpreter) -> Result<()> {
    const MAX_ITERATIONS: usize = 10000;

    let block_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // Block または ワード名を取得
    let block_or_word = match get_block_or_word(&block_val) {
        Ok(bow) => bow,
        Err(e) => {
            interp.stack.push(block_val);
            return Err(e);
        }
    };

    // ワード名の場合は辞書の存在確認
    if let BlockOrWord::WordName(ref word_name) = block_or_word {
        if !interp.dictionary.contains_key(word_name) {
            interp.stack.push(block_val);
            return Err(AjisaiError::UnknownWord(word_name.clone()));
        }
    }

    /// Block または ワードを実行するヘルパー
    fn execute_block_or_word(interp: &mut Interpreter, bow: &BlockOrWord) -> Result<()> {
        match bow {
            BlockOrWord::Block(block) => interp.execute_block(block),
            BlockOrWord::WordName(word_name) => interp.execute_word_core(word_name),
        }
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let init_state = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(block_val.clone());
                AjisaiError::StackUnderflow
            })?;

            let mut state = init_state.clone();
            let mut results = Vec::new();

            // 元のスタックを保存（MAPと同様）
            let original_stack_below = interp.stack.clone();

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
                    interp.stack = original_stack_below;
                    interp.stack.push(init_state);
                    interp.stack.push(block_val);
                    return Err(AjisaiError::from(
                        "UNFOLD: maximum iterations (10000) exceeded - possible infinite loop"
                    ));
                }
                iteration_count += 1;

                // スタックをクリアして処理（MAPと同様）
                interp.stack.clear();
                interp.stack.push(state.clone());

                if let Err(e) = execute_block_or_word(interp, &block_or_word) {
                    // エラー時にスタックを復元
                    interp.operation_target = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack = original_stack_below;
                    interp.stack.push(init_state);
                    interp.stack.push(block_val);
                    return Err(e);
                }

                // ワードは入力と出力の両方をスタックに残すので、両方ポップする
                let result = interp.stack.pop()
                    .ok_or_else(|| {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below.clone();
                        AjisaiError::from("UNFOLD: block must return a value")
                    })?;
                let _input = interp.stack.pop(); // 入力状態を破棄

                // 単一要素ベクタの場合はアンラップ
                let unwrapped = unwrap_single_element(result);

                // NILの場合は終了
                if unwrapped.is_nil() {
                    break;
                }

                // ベクタで2要素の場合は [要素, 次の状態]
                if is_vector_value(&unwrapped) {
                    let v = reconstruct_vector_elements(&unwrapped);
                    if v.len() == 2 {
                        results.push(v[0].clone());

                        // 次の状態がNILの場合は終了
                        if v[1].is_nil() {
                            break;
                        }

                        state = Value::from_vector(vec![v[1].clone()]);
                        continue;
                    }
                }

                interp.operation_target = saved_target;
                interp.disable_no_change_check = saved_no_change_check;
                // エラー時にスタックを復元
                interp.stack = original_stack_below;
                interp.stack.push(init_state);
                interp.stack.push(block_val);
                return Err(AjisaiError::from(
                    "UNFOLD: block must return [element, next_state] or NIL"
                ));
            }

            // operation_target と no_change_check を復元し、スタックを復元（MAPと同様）
            interp.operation_target = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;
            // 結果が空の場合はNILをプッシュ
            if results.is_empty() {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_vector(results));
            }
            Ok(())
        }
        OperationTarget::Stack => {
            // Stackモード: 結果をスタックに直接展開
            let init_state = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(block_val.clone());
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
                    interp.stack.push(block_val);
                    return Err(AjisaiError::from(
                        "UNFOLD: maximum iterations (10000) exceeded - possible infinite loop"
                    ));
                }
                iteration_count += 1;

                interp.stack.clear();
                interp.stack.push(state.clone());

                match execute_block_or_word(interp, &block_or_word) {
                    Ok(_) => {
                        // ワードは入力と出力の両方をスタックに残すので、両方ポップする
                        let result = interp.stack.pop()
                            .ok_or_else(|| AjisaiError::from("UNFOLD: block must return a value"))?;
                        let _input = interp.stack.pop(); // 入力状態を破棄

                        // 単一要素ベクタの場合はアンラップ
                        let unwrapped = unwrap_single_element(result);

                        // NILの場合は終了
                        if unwrapped.is_nil() {
                            break;
                        }

                        // ベクタで2要素の場合は [要素, 次の状態]
                        if is_vector_value(&unwrapped) {
                            let v = reconstruct_vector_elements(&unwrapped);
                            if v.len() == 2 {
                                // 結果をVectorでラップ
                                results.push(wrap_value(v[0].clone()));

                                // 次の状態がNILの場合は終了
                                if v[1].is_nil() {
                                    break;
                                }

                                state = Value::from_vector(vec![v[1].clone()]);
                                continue;
                            }
                        }

                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack;
                        interp.stack.push(init_state);
                        interp.stack.push(block_val);
                        return Err(AjisaiError::from(
                            "UNFOLD: block must return [element, next_state] or NIL"
                        ));
                    }
                    Err(e) => {
                        interp.operation_target = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack;
                        interp.stack.push(init_state);
                        interp.stack.push(block_val);
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
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_fold_basic() {
        let mut interp = Interpreter::new();
        let code = r#"[ 1 2 3 4 ] [ 0 ] '+' FOLD"#;
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "FOLD should succeed: {:?}", result);

        // 結果が [10] であることを確認
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_fold_nil_returns_initial() {
        // NILに対するFOLDは初期値をそのまま返す
        let mut interp = Interpreter::new();
        let code = r#"NIL [ 42 ] '+' FOLD"#;
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "FOLD on NIL should return initial value: {:?}", result);

        // 結果は初期値 [42]
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_map_with_guarded_word() {
        let mut interp = Interpreter::new();
        let def_code = r#"[ ': [ 1 ] =
: [ 10 ]
: [ 20 ]' ] 'CHECK_ONE' DEF"#;
        let def_result = interp.execute(def_code).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        let map_code = "[ 1 2 1 3 1 ] 'CHECK_ONE' MAP";
        let result = interp.execute(map_code).await;

        assert!(result.is_ok(), "MAP with guarded word should succeed: {:?}", result);

        assert_eq!(interp.stack.len(), 1, "Stack should have exactly 1 element, got {}", interp.stack.len());
    }

    #[tokio::test]
    async fn test_map_with_multiline_word() {
        let mut interp = Interpreter::new();
        let def_code = r#"[ ':
[ 2 ] *
[ 1 ] +' ] 'DOUBLE_PLUS_ONE' DEF"#;
        let def_result = interp.execute(def_code).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        let map_code = "[ 1 2 3 ] 'DOUBLE_PLUS_ONE' MAP";
        let result = interp.execute(map_code).await;

        assert!(result.is_ok(), "MAP with multiline word should succeed: {:?}", result);

        assert_eq!(interp.stack.len(), 1, "Stack should have exactly 1 element, got {}", interp.stack.len());
    }

    #[tokio::test]
    async fn test_map_preserves_stack_below() {
        let mut interp = Interpreter::new();
        let def_code = "[ ': 2 *' ] 'DOUBLE' DEF";
        let def_result = interp.execute(def_code).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        let code = "[ 100 ] [ 1 2 3 ] 'DOUBLE' MAP";
        let result = interp.execute(code).await;

        assert!(result.is_ok(), "MAP should preserve stack below: {:?}", result);

        assert_eq!(interp.stack.len(), 2, "Stack should have 2 elements");
    }

    #[tokio::test]
    async fn test_fold_preserves_stack_below() {
        let mut interp = Interpreter::new();
        let code = "[ 100 ] [ 1 2 3 4 ] [ 0 ] '+' FOLD";
        let result = interp.execute(code).await;

        assert!(result.is_ok(), "FOLD should preserve stack below: {:?}", result);

        assert_eq!(interp.stack.len(), 2, "Stack should have 2 elements, got {}", interp.stack.len());
    }

    #[tokio::test]
    async fn test_fold_with_custom_word() {
        let mut interp = Interpreter::new();
        let def_code = "[ ': +' ] 'MYSUM' DEF";
        let def_result = interp.execute(def_code).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        let fold_code = "[ 1 2 3 4 ] [ 0 ] 'MYSUM' FOLD";
        let result = interp.execute(fold_code).await;

        assert!(result.is_ok(), "FOLD with custom word should succeed: {:?}", result);

        assert_eq!(interp.stack.len(), 1, "Stack should have exactly 1 element, got {}", interp.stack.len());
    }
}
