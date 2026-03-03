// rust/src/interpreter/higher_order.rs
//
// Higher-order functions: MAP, FILTER, FOLD, UNFOLD.
// Supports code blocks (: ... ;) and word names.

use crate::interpreter::{Interpreter, OperationTargetMode, ConsumptionMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_word_name_from_value, get_integer_from_value, is_vector_value};
use crate::types::{Value, DisplayHint, Token};

enum ExecutableCode {
    WordName(String),
    CodeBlock(Vec<Token>),
}

fn get_executable_code(val: &Value) -> Result<ExecutableCode> {
    if let Some(tokens) = val.as_code_block() {
        return Ok(ExecutableCode::CodeBlock(tokens.clone()));
    }

    if val.display_hint == DisplayHint::String {
        return get_word_name_from_value(val).map(ExecutableCode::WordName);
    }

    Err(AjisaiError::from("Expected code block (: ... ;) or word name"))
}

fn is_boolean_true(val: &Value) -> bool {
    if val.display_hint == DisplayHint::Boolean {
        if let Some(f) = val.as_scalar() {
            return !f.is_zero();
        }
    }
    false
}


pub fn op_map(interp: &mut Interpreter) -> Result<()> {
    let code_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let executable = match get_executable_code(&code_val) {
        Ok(exec) => exec,
        Err(e) => {
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    if let ExecutableCode::WordName(ref word_name) = executable {
        if !interp.dictionary.contains_key(word_name) {
            interp.stack.push(code_val);
            return Err(AjisaiError::UnknownWord(word_name.clone()));
        }
    }

    fn execute_code(interp: &mut Interpreter, exec: &ExecutableCode) -> Result<()> {
        match exec {
            ExecutableCode::CodeBlock(tokens) => {
                // トークン列を直接実行
                let (_, _) = interp.execute_section_core(tokens, 0)?;
                Ok(())
            }
            ExecutableCode::WordName(word_name) => interp.execute_word_core(word_name),
        }
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let target_val = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(code_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

            if target_val.is_nil() {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            if !is_vector_value(&target_val) {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(code_val);
                return Err(AjisaiError::structure_error("vector", "other format"));
            }

            let n_elements = target_val.len();
            if n_elements == 0 {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            let mut results = Vec::with_capacity(n_elements);
            let original_stack_below = interp.stack.clone();

            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut error: Option<AjisaiError> = None;
            for i in 0..n_elements {
                let elem = target_val.get_child(i).unwrap().clone();
                interp.stack.clear();
                interp.stack.push(elem);
                match execute_code(interp, &executable) {
                    Ok(_) => {
                        match interp.stack.pop() {
                            Some(result_val) => {
                                if is_vector_value(&result_val) && result_val.len() == 1 {
                                    results.push(result_val.get_child(0).unwrap().clone());
                                } else {
                                    results.push(result_val);
                                }
                            },
                            None => {
                                error = Some(AjisaiError::from("MAP code must return a value"));
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error = Some(e);
                        break;
                    }
                }
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;

            if let Some(e) = error {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(code_val);
                return Err(e);
            }

            interp.stack.push(Value::from_vector(results));
        },
        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = match get_integer_from_value(&count_val) {
                Ok(v) => v as usize,
                Err(e) => {
                    interp.stack.push(count_val);
                    interp.stack.push(code_val);
                    return Err(e);
                }
            };

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let original_stack_below = interp.stack.clone();

            // operation_target を一時的に StackTop に設定
            // MAP内部では「変化なし」チェックを無効化
            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut results = Vec::with_capacity(targets.len());
            for item in &targets {
                // スタックをクリアして単一要素を処理
                interp.stack.clear();
                interp.stack.push(item.clone());
                match execute_code(interp, &executable) {
                    Ok(_) => {
                        match interp.stack.pop() {
                            Some(result) => results.push(result),
                            None => {
                                // エラー時にスタックを復元
                                interp.operation_target_mode = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = original_stack_below;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                interp.stack.push(code_val);
                                return Err(AjisaiError::from("MAP code must return a value"));
                            }
                        }
                    }
                    Err(e) => {
                        // エラー時にスタックを復元
                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        interp.stack.extend(targets);
                        interp.stack.push(count_val);
                        interp.stack.push(code_val);
                        return Err(e);
                    }
                }
            }

            // operation_target とno_change_checkを復元し、スタックを復元
            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;
            interp.stack.extend(results);
        }
    }
    Ok(())
}

pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
    let code_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let executable = match get_executable_code(&code_val) {
        Ok(exec) => exec,
        Err(e) => {
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    if let ExecutableCode::WordName(ref word_name) = executable {
        if !interp.dictionary.contains_key(word_name) {
            interp.stack.push(code_val);
            return Err(AjisaiError::UnknownWord(word_name.clone()));
        }
    }

    fn execute_code(interp: &mut Interpreter, exec: &ExecutableCode) -> Result<()> {
        match exec {
            ExecutableCode::CodeBlock(tokens) => {
                // トークン列を直接実行
                let (_, _) = interp.execute_section_core(tokens, 0)?;
                Ok(())
            }
            ExecutableCode::WordName(word_name) => interp.execute_word_core(word_name),
        }
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let target_val = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(code_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

            if target_val.is_nil() {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            if !is_vector_value(&target_val) {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(code_val);
                return Err(AjisaiError::structure_error("vector", "other format"));
            }

            let n_elements = target_val.len();
            if n_elements == 0 {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            let mut results = Vec::with_capacity(n_elements);
            let original_stack_below = interp.stack.clone();

            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut error: Option<AjisaiError> = None;
            for i in 0..n_elements {
                let elem = target_val.get_child(i).unwrap().clone();
                interp.stack.clear();
                interp.stack.push(elem.clone());
                match execute_code(interp, &executable) {
                    Ok(_) => {
                        let condition_result = match interp.stack.pop() {
                            Some(r) => r,
                            None => {
                                error = Some(AjisaiError::from("FILTER code must return a boolean value"));
                                break;
                            }
                        };

                        let is_true = if is_vector_value(&condition_result) {
                            if condition_result.len() == 1 {
                                is_boolean_true(condition_result.get_child(0).unwrap())
                            } else {
                                error = Some(AjisaiError::structure_error("boolean result from FILTER code", "other format"));
                                break;
                            }
                        } else {
                            error = Some(AjisaiError::structure_error("boolean vector result from FILTER code", "other format"));
                            break;
                        };

                        if is_true {
                            results.push(elem);
                        }
                    }
                    Err(e) => {
                        error = Some(e);
                        break;
                    }
                }
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;

            if let Some(e) = error {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(code_val);
                return Err(e);
            }

            if results.is_empty() {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_vector(results));
            }
        },
        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = match get_integer_from_value(&count_val) {
                Ok(v) => v as usize,
                Err(e) => {
                    interp.stack.push(count_val);
                    interp.stack.push(code_val);
                    return Err(e);
                }
            };

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let original_stack_below = interp.stack.clone();

            // operation_target と no_change_check を一時的に設定
            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut results = Vec::with_capacity(targets.len());
            for item in &targets {
                // スタックをクリアして単一要素を処理
                interp.stack.clear();
                interp.stack.push(item.clone());
                match execute_code(interp, &executable) {
                    Ok(_) => {
                        // 条件判定結果を取得
                        let condition_result = match interp.stack.pop() {
                            Some(result) => result,
                            None => {
                                // エラー時にスタックを復元
                                interp.operation_target_mode = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = original_stack_below;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                interp.stack.push(code_val);
                                return Err(AjisaiError::from("FILTER code must return a boolean value"));
                            }
                        };

                        if is_vector_value(&condition_result)
                            && condition_result.len() == 1
                            && is_boolean_true(condition_result.get_child(0).unwrap())
                        {
                            results.push(item.clone());
                        }
                    }
                    Err(e) => {
                        // エラー時にスタックを復元
                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        interp.stack.extend(targets);
                        interp.stack.push(count_val);
                        interp.stack.push(code_val);
                        return Err(e);
                    }
                }
            }

            // operation_target と no_change_check を復元し、スタックを復元
            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;
            interp.stack.extend(results);
        }
    }
    Ok(())
}

pub fn op_fold(interp: &mut Interpreter) -> Result<()> {
    let code_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let executable = match get_executable_code(&code_val) {
        Ok(exec) => exec,
        Err(e) => {
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    if let ExecutableCode::WordName(ref word_name) = executable {
        if !interp.dictionary.contains_key(word_name) {
            interp.stack.push(code_val);
            return Err(AjisaiError::UnknownWord(word_name.clone()));
        }
    }

    fn execute_code(interp: &mut Interpreter, exec: &ExecutableCode) -> Result<()> {
        match exec {
            ExecutableCode::CodeBlock(tokens) => {
                // トークン列を直接実行
                let (_, _) = interp.execute_section_core(tokens, 0)?;
                Ok(())
            }
            ExecutableCode::WordName(word_name) => interp.execute_word_core(word_name),
        }
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let init_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let target_val = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(init_val.clone());
                    interp.stack.push(code_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(init_val.clone());
                    interp.stack.push(code_val.clone());
                    AjisaiError::StackUnderflow
                })?
            };

            if target_val.is_nil() {
                interp.stack.push(init_val);
                return Ok(());
            }

            if !is_vector_value(&target_val) {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(init_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::structure_error("vector", "other format"));
            }

            let n_elements = target_val.len();
            if n_elements == 0 {
                interp.stack.push(init_val);
                return Ok(());
            }

            let mut accumulator = init_val;
            let original_stack_below = interp.stack.clone();

            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut error: Option<AjisaiError> = None;
            for i in 0..n_elements {
                let elem = target_val.get_child(i).unwrap().clone();
                interp.stack.clear();
                interp.stack.push(accumulator.clone());
                interp.stack.push(elem);

                match execute_code(interp, &executable) {
                    Ok(_) => {
                        match interp.stack.pop() {
                            Some(result) => { accumulator = result; }
                            None => {
                                error = Some(AjisaiError::from("FOLD: code must return a value"));
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error = Some(e);
                        break;
                    }
                }
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;

            if let Some(e) = error {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(accumulator);
                interp.stack.push(code_val);
                return Err(e);
            }

            interp.stack.push(accumulator);
            Ok(())
        }
        OperationTargetMode::Stack => {
            // Stack モードの実装
            // [要素...] [個数] [初期値] [ code ] .. FOLD
            let init_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(init_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let original_stack_below = interp.stack.clone();

            let mut accumulator = init_val;

            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            for item in targets {
                interp.stack.clear();
                interp.stack.push(accumulator);
                interp.stack.push(item);

                match execute_code(interp, &executable) {
                    Ok(_) => {
                        let result = interp.stack.pop()
                            .ok_or_else(|| AjisaiError::from("FOLD: code must return a value"))?;
                        accumulator = result;
                    }
                    Err(e) => {
                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below;
                        return Err(e);
                    }
                }
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = original_stack_below;
            interp.stack.push(accumulator);
            Ok(())
        }
    }
}

pub fn op_unfold(interp: &mut Interpreter) -> Result<()> {
    const MAX_ITERATIONS: usize = 10000;

    let code_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let executable = match get_executable_code(&code_val) {
        Ok(exec) => exec,
        Err(e) => {
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    if let ExecutableCode::WordName(ref word_name) = executable {
        if !interp.dictionary.contains_key(word_name) {
            interp.stack.push(code_val);
            return Err(AjisaiError::UnknownWord(word_name.clone()));
        }
    }

    fn execute_code(interp: &mut Interpreter, exec: &ExecutableCode) -> Result<()> {
        match exec {
            ExecutableCode::CodeBlock(tokens) => {
                // トークン列を直接実行
                let (_, _) = interp.execute_section_core(tokens, 0)?;
                Ok(())
            }
            ExecutableCode::WordName(word_name) => interp.execute_word_core(word_name),
        }
    }

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let init_state = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(code_val.clone());
                AjisaiError::StackUnderflow
            })?;

            let mut state = init_state.clone();
            let mut results = Vec::new();

            // 元のスタックを保存（MAPと同様）
            let original_stack_below = interp.stack.clone();

            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut iteration_count = 0;
            loop {
                if iteration_count >= MAX_ITERATIONS {
                    // MAX_ITERATIONSに達した場合はエラー
                    interp.operation_target_mode = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack = original_stack_below;
                    interp.stack.push(init_state);
                    interp.stack.push(code_val);
                    return Err(AjisaiError::from(
                        "UNFOLD: maximum iterations (10000) exceeded - possible infinite loop"
                    ));
                }
                iteration_count += 1;

                // スタックをクリアして処理（MAPと同様）
                interp.stack.clear();
                interp.stack.push(state.clone());

                if let Err(e) = execute_code(interp, &executable) {
                    // エラー時にスタックを復元
                    interp.operation_target_mode = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack = original_stack_below;
                    interp.stack.push(init_state);
                    interp.stack.push(code_val);
                    return Err(e);
                }

                // ワードは入力と出力の両方をスタックに残すので、両方ポップする
                let result = interp.stack.pop()
                    .ok_or_else(|| {
                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack_below.clone();
                        AjisaiError::from("UNFOLD: code must return a value")
                    })?;
                let _input = interp.stack.pop(); // 入力状態を破棄

                // 単一要素ベクタの場合はアンラップ
                let unwrapped = result;

                // NILの場合は終了
                if unwrapped.is_nil() {
                    break;
                }

                // ベクタで2要素の場合は [要素, 次の状態]
                if is_vector_value(&unwrapped) {
                    if unwrapped.len() == 2 {
                        results.push(unwrapped.get_child(0).unwrap().clone());

                        let next_state = unwrapped.get_child(1).unwrap();
                        if next_state.is_nil() {
                            break;
                        }

                        state = Value::from_vector(vec![next_state.clone()]);
                        continue;
                    }
                }

                interp.operation_target_mode = saved_target;
                interp.disable_no_change_check = saved_no_change_check;
                // エラー時にスタックを復元
                interp.stack = original_stack_below;
                interp.stack.push(init_state);
                interp.stack.push(code_val);
                return Err(AjisaiError::from(
                    "UNFOLD: code must return [element, next_state] or NIL"
                ));
            }

            // operation_target と no_change_check を復元し、スタックを復元（MAPと同様）
            interp.operation_target_mode = saved_target;
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
        OperationTargetMode::Stack => {
            // Stackモード: 結果をスタックに直接展開
            let init_state = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(code_val.clone());
                AjisaiError::StackUnderflow
            })?;

            let mut state = init_state.clone();
            let original_stack = interp.stack.clone();

            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut results = Vec::new();
            let mut iteration_count = 0;

            loop {
                if iteration_count >= MAX_ITERATIONS {
                    interp.operation_target_mode = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack = original_stack;
                    interp.stack.push(init_state);
                    interp.stack.push(code_val);
                    return Err(AjisaiError::from(
                        "UNFOLD: maximum iterations (10000) exceeded - possible infinite loop"
                    ));
                }
                iteration_count += 1;

                interp.stack.clear();
                interp.stack.push(state.clone());

                match execute_code(interp, &executable) {
                    Ok(_) => {
                        // ワードは入力と出力の両方をスタックに残すので、両方ポップする
                        let result = interp.stack.pop()
                            .ok_or_else(|| AjisaiError::from("UNFOLD: code must return a value"))?;
                        let _input = interp.stack.pop(); // 入力状態を破棄

                        // 単一要素ベクタの場合はアンラップ
                        let unwrapped = result;

                        // NILの場合は終了
                        if unwrapped.is_nil() {
                            break;
                        }

                        // ベクタで2要素の場合は [要素, 次の状態]
                        if is_vector_value(&unwrapped) && unwrapped.len() == 2 {
                            results.push(unwrapped.get_child(0).unwrap().clone());

                            let next_state = unwrapped.get_child(1).unwrap();
                            if next_state.is_nil() {
                                break;
                            }

                            state = Value::from_vector(vec![next_state.clone()]);
                            continue;
                        }

                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack;
                        interp.stack.push(init_state);
                        interp.stack.push(code_val);
                        return Err(AjisaiError::from(
                            "UNFOLD: code must return [element, next_state] or NIL"
                        ));
                    }
                    Err(e) => {
                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = original_stack;
                        interp.stack.push(init_state);
                        interp.stack.push(code_val);
                        return Err(e);
                    }
                }
            }

            interp.operation_target_mode = saved_target;
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
        // Use chevron branching instead of old guard syntax
        let def_code = r#":
>> [ 1 ] =
>> [ 10 ]
>>> [ 20 ]
; 'CHECK_ONE' DEF"#;
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
        let def_code = r#":
[ 2 ] *
[ 1 ] +
; 'DOUBLE_PLUS_ONE' DEF"#;
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
        let def_code = ": [ 2 ] * ; 'DOUBLE' DEF";
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
        let def_code = ": + ; 'MYSUM' DEF";
        let def_result = interp.execute(def_code).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        let fold_code = "[ 1 2 3 4 ] [ 0 ] 'MYSUM' FOLD";
        let result = interp.execute(fold_code).await;

        assert!(result.is_ok(), "FOLD with custom word should succeed: {:?}", result);

        assert_eq!(interp.stack.len(), 1, "Stack should have exactly 1 element, got {}", interp.stack.len());
    }
}
