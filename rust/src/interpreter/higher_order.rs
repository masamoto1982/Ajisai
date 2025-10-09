// rust/src/interpreter/higher_order.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, BracketType};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

// スタックトップから数値の引数を取得するヘルパー
fn get_optional_count(interp: &mut Interpreter) -> Result<Option<usize>> {
    if let Some(top) = interp.stack.last() {
        if let ValueType::Vector(v, _) = &top.val_type {
            if v.len() == 1 {
                if let ValueType::Number(n) = &v[0].val_type {
                    if n.denominator == BigInt::one() {
                        let count = n.numerator.to_usize().ok_or_else(|| AjisaiError::from("Count too large"))?;
                        interp.stack.pop(); // countを消費
                        return Ok(Some(count));
                    }
                }
            }
        }
    }
    Ok(None)
}

impl Interpreter {
    pub(crate) fn execute_map(&mut self) -> Result<()> {
        let word_name_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let word_name = get_word_name(&word_name_val)?;
        let upper_name = word_name.to_uppercase();
        if !self.dictionary.contains_key(&upper_name) {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        if let Some(n) = get_optional_count(self)? {
            // スタック上のN個のVectorに適用
            if self.stack.len() < n { return Err(AjisaiError::StackUnderflow); }
            let mut results = Vec::with_capacity(n);
            let mut items_to_map = self.stack.drain(self.stack.len() - n ..).collect::<Vec<_>>();
            
            for elem in items_to_map.drain(..) {
                self.stack.push(elem);
                self.execute_word_sync(&upper_name)?;
                results.push(self.stack.pop().unwrap());
            }
            self.stack.extend(results);

        } else {
            // 単一Vectorの要素に適用
            let vector_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let elements = match vector_val.val_type {
                ValueType::Vector(v, _) => v,
                _ => return Err(AjisaiError::type_error("vector", "other type")),
            };

            let mut results = Vec::new();
            for elem in elements {
                self.stack.push(Value { val_type: ValueType::Vector(vec![elem], BracketType::Square) });
                self.execute_word_sync(&upper_name)?;
                let result = self.stack.pop().ok_or_else(|| AjisaiError::from("MAP word must return a value"))?;
                
                match result.val_type {
                    ValueType::Vector(v, _) if v.len() == 1 => results.push(v[0].clone()),
                    _ => results.push(result),
                }
            }
            self.stack.push(Value { val_type: ValueType::Vector(results, BracketType::Square) });
        }
        Ok(())
    }

    pub(crate) fn execute_filter(&mut self) -> Result<()> {
        let word_name_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let word_name = get_word_name(&word_name_val)?;
        let upper_name = word_name.to_uppercase();
        if !self.dictionary.contains_key(&upper_name) {
            return Err(AjisaiError::UnknownWord(word_name));
        }
        
        if let Some(n) = get_optional_count(self)? {
            // スタック上のN個のVectorをフィルタリング
            if self.stack.len() < n { return Err(AjisaiError::StackUnderflow); }
            let mut items_to_check = self.stack.drain(self.stack.len() - n ..).collect::<Vec<_>>();
            let mut items_to_keep = Vec::new();

            for item in items_to_check.drain(..) {
                self.stack.push(item.clone());
                self.execute_word_sync(&upper_name)?;
                let result = self.stack.pop().ok_or_else(|| AjisaiError::from("FILTER word must return a boolean"))?;
                if is_true(&result)? {
                    items_to_keep.push(item);
                }
            }
            self.stack.extend(items_to_keep);

        } else {
            // 単一Vectorの要素をフィルタリング
            let vector_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let elements = match vector_val.val_type {
                ValueType::Vector(v, _) => v,
                _ => return Err(AjisaiError::type_error("vector", "other type")),
            };
            
            let mut results = Vec::new();
            for elem in elements {
                self.stack.push(Value { val_type: ValueType::Vector(vec![elem.clone()], BracketType::Square) });
                self.execute_word_sync(&upper_name)?;
                let result = self.stack.pop().ok_or_else(|| AjisaiError::from("FILTER word must return a value"))?;
                if is_true(&result)? {
                    results.push(elem);
                }
            }
            self.stack.push(Value { val_type: ValueType::Vector(results, BracketType::Square) });
        }
        Ok(())
    }

    pub(crate) fn execute_reduce(&mut self) -> Result<()> {
        let word_name_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let init_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let word_name = get_word_name(&word_name_val)?;
        let upper_name = word_name.to_uppercase();
        if !self.dictionary.contains_key(&upper_name) {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        let mut accumulator = init_val;

        if let Some(n) = get_optional_count(self)? {
            // スタック上のN個のVectorを畳み込み
            if self.stack.len() < n { return Err(AjisaiError::StackUnderflow); }
            let elements = self.stack.drain(self.stack.len() - n ..).collect::<Vec<_>>();
            for elem in elements {
                self.stack.push(accumulator);
                self.stack.push(elem);
                self.execute_word_sync(&upper_name)?;
                accumulator = self.stack.pop().ok_or_else(|| AjisaiError::from("REDUCE word must return a value"))?;
            }
        } else {
            // 単一Vectorの要素を畳み込み
            let vector_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let elements = match vector_val.val_type {
                ValueType::Vector(v, _) => v,
                _ => return Err(AjisaiError::type_error("vector", "other type")),
            };

            for elem in elements {
                self.stack.push(accumulator);
                self.stack.push(Value { val_type: ValueType::Vector(vec![elem], BracketType::Square) });
                self.execute_word_sync(&upper_name)?;
                accumulator = self.stack.pop().ok_or_else(|| AjisaiError::from("REDUCE word must return a value"))?;
            }
        }
        self.stack.push(accumulator);
        Ok(())
    }

    pub(crate) fn execute_each(&mut self) -> Result<()> {
        let word_name_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let word_name = get_word_name(&word_name_val)?;
        let upper_name = word_name.to_uppercase();
        if !self.dictionary.contains_key(&upper_name) {
            return Err(AjisaiError::UnknownWord(word_name));
        }
        
        if let Some(n) = get_optional_count(self)? {
            // スタック上のN個のVectorに副作用実行
            if self.stack.len() < n { return Err(AjisaiError::StackUnderflow); }
            let elements = self.stack.drain(self.stack.len() - n ..).collect::<Vec<_>>();
            for elem in elements {
                self.stack.push(elem);
                self.execute_word_sync(&upper_name)?;
                if !self.stack.is_empty() { self.stack.pop(); } // 副作用なので結果は捨てる
            }
        } else {
            // 単一Vectorの要素に副作用実行
            let vector_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let elements = match vector_val.val_type {
                ValueType::Vector(v, _) => v,
                _ => return Err(AjisaiError::type_error("vector", "other type")),
            };
            for elem in elements {
                self.stack.push(Value { val_type: ValueType::Vector(vec![elem], BracketType::Square) });
                self.execute_word_sync(&upper_name)?;
                if !self.stack.is_empty() { self.stack.pop(); } // 副作用なので結果は捨てる
            }
        }
        Ok(())
    }
}

// ヘルパー関数
fn get_word_name(name_val: &Value) -> Result<String> {
    match &name_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
            ValueType::String(s) => Ok(s.clone()),
            _ => Err(AjisaiError::type_error("string", "other type")),
        },
        _ => Err(AjisaiError::type_error("single-element vector with string", "other type")),
    }
}

fn is_true(value: &Value) -> Result<bool> {
    match &value.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
            ValueType::Boolean(b) => Ok(*b),
            _ => Ok(false), // boolean以外はfalse扱い
        },
        _ => Ok(false),
    }
}
