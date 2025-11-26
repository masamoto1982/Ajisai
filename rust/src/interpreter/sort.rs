// rust/src/interpreter/sort.rs
//
// 【責務】
// ソートアルゴリズムの実装を提供する。
// 各ソートアルゴリズムは、StackTopモードとStackモードの両方をサポートする。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::ToPrimitive;

/// 数値ベクタを取得し、ソート可能な値に変換する
fn extract_sortable_numbers(vec: &[Value]) -> Result<Vec<(f64, Value)>> {
    vec.iter().map(|v| {
        match &v.val_type {
            ValueType::Number(frac) => {
                let f = frac.to_f64()
                    .ok_or_else(|| AjisaiError::from("Number too large to sort"))?;
                Ok((f, v.clone()))
            }
            _ => Err(AjisaiError::type_error("number", "other type")),
        }
    }).collect()
}

/// ソート済みかどうかをチェック（昇順）
fn is_sorted(items: &[(f64, Value)]) -> bool {
    items.windows(2).all(|w| w[0].0 <= w[1].0)
}

/// バブルソート
pub fn op_bubblesort(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(v) => {
                    if v.is_empty() {
                        return Err(AjisaiError::from("Cannot sort empty vector"));
                    }

                    let mut items = extract_sortable_numbers(&v)?;

                    // すでにソート済みならエラー（"No change is an error" 原則）
                    if is_sorted(&items) {
                        interp.stack.push(Value { val_type: ValueType::Vector(v) });
                        return Err(AjisaiError::from("BUBBLESORT resulted in no change (already sorted)"));
                    }

                    // バブルソート実装
                    let n = items.len();
                    for i in 0..n {
                        for j in 0..n - i - 1 {
                            if items[j].0 > items[j + 1].0 {
                                items.swap(j, j + 1);
                            }
                        }
                    }

                    let sorted: Vec<Value> = items.into_iter().map(|(_, v)| v).collect();
                    interp.stack.push(Value { val_type: ValueType::Vector(sorted) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::from("Cannot sort empty stack"));
            }

            let items_vec: Vec<Value> = interp.stack.drain(..).collect();
            let mut items = extract_sortable_numbers(&items_vec)?;

            // すでにソート済みならエラー
            if is_sorted(&items) {
                interp.stack = items_vec;
                return Err(AjisaiError::from("BUBBLESORT resulted in no change (already sorted)"));
            }

            // バブルソート実装
            let n = items.len();
            for i in 0..n {
                for j in 0..n - i - 1 {
                    if items[j].0 > items[j + 1].0 {
                        items.swap(j, j + 1);
                    }
                }
            }

            interp.stack = items.into_iter().map(|(_, v)| v).collect();
            Ok(())
        }
    }
}

/// 選択ソート
pub fn op_selectionsort(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(v) => {
                    if v.is_empty() {
                        return Err(AjisaiError::from("Cannot sort empty vector"));
                    }

                    let mut items = extract_sortable_numbers(&v)?;

                    if is_sorted(&items) {
                        interp.stack.push(Value { val_type: ValueType::Vector(v) });
                        return Err(AjisaiError::from("SELECTIONSORT resulted in no change (already sorted)"));
                    }

                    // 選択ソート実装
                    let n = items.len();
                    for i in 0..n {
                        let mut min_idx = i;
                        for j in (i + 1)..n {
                            if items[j].0 < items[min_idx].0 {
                                min_idx = j;
                            }
                        }
                        if min_idx != i {
                            items.swap(i, min_idx);
                        }
                    }

                    let sorted: Vec<Value> = items.into_iter().map(|(_, v)| v).collect();
                    interp.stack.push(Value { val_type: ValueType::Vector(sorted) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::from("Cannot sort empty stack"));
            }

            let items_vec: Vec<Value> = interp.stack.drain(..).collect();
            let mut items = extract_sortable_numbers(&items_vec)?;

            if is_sorted(&items) {
                interp.stack = items_vec;
                return Err(AjisaiError::from("SELECTIONSORT resulted in no change (already sorted)"));
            }

            // 選択ソート実装
            let n = items.len();
            for i in 0..n {
                let mut min_idx = i;
                for j in (i + 1)..n {
                    if items[j].0 < items[min_idx].0 {
                        min_idx = j;
                    }
                }
                if min_idx != i {
                    items.swap(i, min_idx);
                }
            }

            interp.stack = items.into_iter().map(|(_, v)| v).collect();
            Ok(())
        }
    }
}

/// クイックソートのヘルパー関数
fn quicksort_helper(items: &mut [(f64, Value)]) {
    if items.len() <= 1 {
        return;
    }

    let pivot_idx = items.len() / 2;
    let pivot_value = items[pivot_idx].0;

    let mut i = 0;
    let mut j = items.len() - 1;

    loop {
        while items[i].0 < pivot_value {
            i += 1;
        }
        while items[j].0 > pivot_value {
            j = j.saturating_sub(1);
        }

        if i >= j {
            break;
        }

        items.swap(i, j);
        i += 1;
        j = j.saturating_sub(1);
    }

    let mid = i;
    if mid > 0 {
        quicksort_helper(&mut items[..mid]);
    }
    if mid < items.len() {
        quicksort_helper(&mut items[mid..]);
    }
}

/// クイックソート
pub fn op_quicksort(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(v) => {
                    if v.is_empty() {
                        return Err(AjisaiError::from("Cannot sort empty vector"));
                    }

                    let mut items = extract_sortable_numbers(&v)?;

                    if is_sorted(&items) {
                        interp.stack.push(Value { val_type: ValueType::Vector(v) });
                        return Err(AjisaiError::from("QUICKSORT resulted in no change (already sorted)"));
                    }

                    quicksort_helper(&mut items);

                    let sorted: Vec<Value> = items.into_iter().map(|(_, v)| v).collect();
                    interp.stack.push(Value { val_type: ValueType::Vector(sorted) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::from("Cannot sort empty stack"));
            }

            let items_vec: Vec<Value> = interp.stack.drain(..).collect();
            let mut items = extract_sortable_numbers(&items_vec)?;

            if is_sorted(&items) {
                interp.stack = items_vec;
                return Err(AjisaiError::from("QUICKSORT resulted in no change (already sorted)"));
            }

            quicksort_helper(&mut items);

            interp.stack = items.into_iter().map(|(_, v)| v).collect();
            Ok(())
        }
    }
}

/// マージソートのヘルパー関数
fn mergesort_helper(items: &mut Vec<(f64, Value)>) {
    let len = items.len();
    if len <= 1 {
        return;
    }

    let mid = len / 2;
    let mut left = items[..mid].to_vec();
    let mut right = items[mid..].to_vec();

    mergesort_helper(&mut left);
    mergesort_helper(&mut right);

    let mut i = 0;
    let mut j = 0;
    let mut k = 0;

    while i < left.len() && j < right.len() {
        if left[i].0 <= right[j].0 {
            items[k] = left[i].clone();
            i += 1;
        } else {
            items[k] = right[j].clone();
            j += 1;
        }
        k += 1;
    }

    while i < left.len() {
        items[k] = left[i].clone();
        i += 1;
        k += 1;
    }

    while j < right.len() {
        items[k] = right[j].clone();
        j += 1;
        k += 1;
    }
}

/// マージソート
pub fn op_mergesort(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(v) => {
                    if v.is_empty() {
                        return Err(AjisaiError::from("Cannot sort empty vector"));
                    }

                    let mut items = extract_sortable_numbers(&v)?;

                    if is_sorted(&items) {
                        interp.stack.push(Value { val_type: ValueType::Vector(v) });
                        return Err(AjisaiError::from("MERGESORT resulted in no change (already sorted)"));
                    }

                    mergesort_helper(&mut items);

                    let sorted: Vec<Value> = items.into_iter().map(|(_, v)| v).collect();
                    interp.stack.push(Value { val_type: ValueType::Vector(sorted) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::from("Cannot sort empty stack"));
            }

            let items_vec: Vec<Value> = interp.stack.drain(..).collect();
            let mut items = extract_sortable_numbers(&items_vec)?;

            if is_sorted(&items) {
                interp.stack = items_vec;
                return Err(AjisaiError::from("MERGESORT resulted in no change (already sorted)"));
            }

            mergesort_helper(&mut items);

            interp.stack = items.into_iter().map(|(_, v)| v).collect();
            Ok(())
        }
    }
}

/// ヒープソートのヘルパー関数
fn heapify(items: &mut [(f64, Value)], n: usize, i: usize) {
    let mut largest = i;
    let left = 2 * i + 1;
    let right = 2 * i + 2;

    if left < n && items[left].0 > items[largest].0 {
        largest = left;
    }

    if right < n && items[right].0 > items[largest].0 {
        largest = right;
    }

    if largest != i {
        items.swap(i, largest);
        heapify(items, n, largest);
    }
}

/// ヒープソート
pub fn op_heapsort(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(v) => {
                    if v.is_empty() {
                        return Err(AjisaiError::from("Cannot sort empty vector"));
                    }

                    let mut items = extract_sortable_numbers(&v)?;

                    if is_sorted(&items) {
                        interp.stack.push(Value { val_type: ValueType::Vector(v) });
                        return Err(AjisaiError::from("HEAPSORT resulted in no change (already sorted)"));
                    }

                    // ヒープソート実装
                    let n = items.len();

                    // Build max heap
                    for i in (0..n / 2).rev() {
                        heapify(&mut items, n, i);
                    }

                    // Extract elements from heap one by one
                    for i in (1..n).rev() {
                        items.swap(0, i);
                        heapify(&mut items, i, 0);
                    }

                    let sorted: Vec<Value> = items.into_iter().map(|(_, v)| v).collect();
                    interp.stack.push(Value { val_type: ValueType::Vector(sorted) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::from("Cannot sort empty stack"));
            }

            let items_vec: Vec<Value> = interp.stack.drain(..).collect();
            let mut items = extract_sortable_numbers(&items_vec)?;

            if is_sorted(&items) {
                interp.stack = items_vec;
                return Err(AjisaiError::from("HEAPSORT resulted in no change (already sorted)"));
            }

            // ヒープソート実装
            let n = items.len();

            // Build max heap
            for i in (0..n / 2).rev() {
                heapify(&mut items, n, i);
            }

            // Extract elements from heap one by one
            for i in (1..n).rev() {
                items.swap(0, i);
                heapify(&mut items, i, 0);
            }

            interp.stack = items.into_iter().map(|(_, v)| v).collect();
            Ok(())
        }
    }
}

/// スターリンソート（冗談のソートアルゴリズム）
/// ソートされていない要素を削除する
pub fn op_stalinsort(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(v) => {
                    if v.is_empty() {
                        return Err(AjisaiError::from("Cannot sort empty vector"));
                    }

                    let items = extract_sortable_numbers(&v)?;

                    // すでにソート済みならエラー（変化なし）
                    if is_sorted(&items) {
                        interp.stack.push(Value { val_type: ValueType::Vector(v) });
                        return Err(AjisaiError::from("STALINSORT resulted in no change (already sorted)"));
                    }

                    // スターリンソート: 昇順でない要素を削除
                    let mut result = Vec::new();
                    if !items.is_empty() {
                        result.push(items[0].1.clone());
                        let mut last_value = items[0].0;

                        for (val, item) in items.iter().skip(1) {
                            if *val >= last_value {
                                result.push(item.clone());
                                last_value = *val;
                            }
                        }
                    }

                    interp.stack.push(Value { val_type: ValueType::Vector(result) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::from("Cannot sort empty stack"));
            }

            let items_vec: Vec<Value> = interp.stack.drain(..).collect();
            let items = extract_sortable_numbers(&items_vec)?;

            // すでにソート済みならエラー
            if is_sorted(&items) {
                interp.stack = items_vec;
                return Err(AjisaiError::from("STALINSORT resulted in no change (already sorted)"));
            }

            // スターリンソート: 昇順でない要素を削除
            let mut result = Vec::new();
            if !items.is_empty() {
                result.push(items[0].1.clone());
                let mut last_value = items[0].0;

                for (val, item) in items.iter().skip(1) {
                    if *val >= last_value {
                        result.push(item.clone());
                        last_value = *val;
                    }
                }
            }

            interp.stack = result;
            Ok(())
        }
    }
}
