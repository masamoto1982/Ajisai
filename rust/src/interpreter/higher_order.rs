// rust/src/interpreter/higher_order.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, BracketType};

impl Interpreter {
    // === 高階関数の実装 ===

    pub(crate) fn execute_map(&mut self) -> Result<()> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::from("MAP requires vector and word name. Usage: [ data ] 'WORD' MAP"));
        }

        let name_val = self.stack.pop().unwrap();
        let vector_val = self.stack.pop().unwrap();

        // ワード名を取得
        let word_name = match &name_val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::String(s) => s.clone(),
                    _ => return Err(AjisaiError::type_error("string", "other type")),
                }
            },
            _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
        };

        // ベクトルを取得
        let elements = match &vector_val.val_type {
            ValueType::Vector(v, _) => v.clone(),
            _ => return Err(AjisaiError::type_error("vector", "other type")),
        };

        let upper_name = word_name.to_uppercase();
        
        // ワードが存在するか確認
        if !self.dictionary.contains_key(&upper_name) {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        // 各要素にワードを適用
        let mut results = Vec::new();
        for elem in elements {
            // 要素をベクトルとしてスタックにプッシュ
            self.stack.push(Value {
                val_type: ValueType::Vector(vec![elem], BracketType::Square)
            });
            
            // ワードを実行 (同期的に)
            self.execute_word_sync(&upper_name)?;
            
            // 結果を取得
            let result = self.stack.pop().ok_or_else(|| AjisaiError::from("MAP word must return a value"))?;
            
            // 結果がベクトルの場合、その最初の要素を取得
            match result.val_type {
                ValueType::Vector(v, _) if v.len() == 1 => {
                    results.push(v[0].clone());
                },
                _ => {
                    // ベクトルでない場合やサイズが1でない場合はそのまま追加
                    results.push(result);
                }
            }
        }

        // 結果をベクトルとしてスタックにプッシュ
        self.stack.push(Value {
            val_type: ValueType::Vector(results, BracketType::Square)
        });

        Ok(())
    }

    pub(crate) fn execute_filter(&mut self) -> Result<()> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::from("FILTER requires vector and word name. Usage: [ data ] 'WORD' FILTER"));
        }

        let name_val = self.stack.pop().unwrap();
        let vector_val = self.stack.pop().unwrap();

        let word_name = match &name_val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::String(s) => s.clone(),
                    _ => return Err(AjisaiError::type_error("string", "other type")),
                }
            },
            _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
        };

        let elements = match &vector_val.val_type {
            ValueType::Vector(v, _) => v.clone(),
            _ => return Err(AjisaiError::type_error("vector", "other type")),
        };

        let upper_name = word_name.to_uppercase();
        if !self.dictionary.contains_key(&upper_name) {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        let mut results = Vec::new();
        for elem in elements {
            self.stack.push(Value {
                val_type: ValueType::Vector(vec![elem.clone()], BracketType::Square)
            });
            
            self.execute_word_sync(&upper_name)?;
            
            let result = self.stack.pop().ok_or_else(|| AjisaiError::from("FILTER word must return a value"))?;
            
            let is_true = if let ValueType::Vector(v, _) = &result.val_type {
                if v.len() == 1 {
                    if let ValueType::Boolean(b) = &v[0].val_type { *b } else { false }
                } else { false }
            } else { false };
            
            if is_true {
                results.push(elem);
            }
        }

        self.stack.push(Value {
            val_type: ValueType::Vector(results, BracketType::Square)
        });

        Ok(())
    }

    pub(crate) fn execute_reduce(&mut self) -> Result<()> {
        if self.stack.len() < 3 {
            return Err(AjisaiError::from("REDUCE requires vector, initial value, and word name. Usage: [ data ] [ init ] 'WORD' REDUCE"));
        }

        let name_val = self.stack.pop().unwrap();
        let init_val = self.stack.pop().unwrap();
        let vector_val = self.stack.pop().unwrap();

        let word_name = match &name_val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::String(s) => s.clone(),
                    _ => return Err(AjisaiError::type_error("string", "other type")),
                }
            },
            _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
        };

        let elements = match &vector_val.val_type {
            ValueType::Vector(v, _) => v.clone(),
            _ => return Err(AjisaiError::type_error("vector", "other type")),
        };

        let initial = match &init_val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => v[0].clone(),
            _ => return Err(AjisaiError::type_error("single-element vector", "other type")),
        };

        let upper_name = word_name.to_uppercase();
        if !self.dictionary.contains_key(&upper_name) {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        let mut accumulator = initial;

        for elem in elements {
            self.stack.push(Value {
                val_type: ValueType::Vector(vec![accumulator], BracketType::Square)
            });
            self.stack.push(Value {
                val_type: ValueType::Vector(vec![elem], BracketType::Square)
            });
            
            self.execute_word_sync(&upper_name)?;
            
            let result = self.stack.pop().ok_or_else(|| AjisaiError::from("REDUCE word must return a value"))?;
            
            accumulator = match result.val_type {
                ValueType::Vector(v, _) if v.len() == 1 => v[0].clone(),
                _ => result,
            };
        }

        self.stack.push(Value {
            val_type: ValueType::Vector(vec![accumulator], BracketType::Square)
        });

        Ok(())
    }

    pub(crate) fn execute_each(&mut self) -> Result<()> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::from("EACH requires vector and word name. Usage: [ data ] 'WORD' EACH"));
        }

        let name_val = self.stack.pop().unwrap();
        let vector_val = self.stack.pop().unwrap();

        let word_name = match &name_val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::String(s) => s.clone(),
                    _ => return Err(AjisaiError::type_error("string", "other type")),
                }
            },
            _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
        };

        let elements = match &vector_val.val_type {
            ValueType::Vector(v, _) => v.clone(),
            _ => return Err(AjisaiError::type_error("vector", "other type")),
        };

        let upper_name = word_name.to_uppercase();
        if !self.dictionary.contains_key(&upper_name) {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        for elem in elements {
            self.stack.push(Value {
                val_type: ValueType::Vector(vec![elem], BracketType::Square)
            });
            
            self.execute_word_sync(&upper_name)?;
            
            if !self.stack.is_empty() {
                self.stack.pop();
            }
        }
        Ok(())
    }
}
