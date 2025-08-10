// rust/src/interpreter/word_def.rs の define_from_tokens メソッドを修正

pub(super) fn define_from_tokens(&mut self, tokens: &[Token]) -> Result<()> {
    console::log_1(&JsValue::from_str("--- define_from_tokens (auto-naming) ---"));
    console::log_1(&JsValue::from_str(&format!("Original tokens: {:?}", tokens)));

    // 定数同士の演算を検出して事前評価
    let processed_tokens = self.preprocess_constant_expressions(tokens)?;
    
    let name = self.generate_word_name(&processed_tokens);
    console::log_1(&JsValue::from_str(&format!("Generated name: {}", name)));
    
    if self.dictionary.contains_key(&name) {
        // 既存のワードがある場合、それが一時的なワードなら実行して削除
        if let Some(def) = self.dictionary.get(&name).cloned() {
            if def.is_temporary {
                console::log_1(&JsValue::from_str(&format!("Executing temporary word: {}", name)));
                self.execute_custom_word_with_iteration(&name, &def.tokens)?;
                // 実行後に連鎖削除
                self.delete_temporary_word_cascade(&name);
            } else {
                // 永続的なワードの場合は単に実行
                console::log_1(&JsValue::from_str(&format!("Executing permanent word: {}", name)));
                self.execute_custom_word_with_iteration(&name, &def.tokens)?;
            }
        }
        return Ok(());
    }

    // 新規の自動命名ワードを定義（実行はしない）
    self.auto_named = true;
    self.last_auto_named_word = Some(name.clone());

    let storage_tokens = self.rearrange_tokens(&processed_tokens);
    console::log_1(&JsValue::from_str(&format!("Storage tokens (RPN): {:?}", storage_tokens)));

    // 以下同じ...
}

// 定数式を事前評価するメソッド
fn preprocess_constant_expressions(&mut self, tokens: &[Token]) -> Result<Vec<Token>> {
    // パターン: Number Op Number を検出
    if tokens.len() == 3 {
        if let (Token::Number(n1, d1), Token::Symbol(op), Token::Number(n2, d2)) = 
            (&tokens[0], &tokens[1], &tokens[2]) {
            if self.is_operator(op) {
                // 定数同士の演算を事前評価
                let mut temp_stack = Vec::new();
                temp_stack.push(Value {
                    val_type: ValueType::Number(crate::types::Fraction::new(*n1, *d1))
                });
                temp_stack.push(Value {
                    val_type: ValueType::Number(crate::types::Fraction::new(*n2, *d2))
                });
                
                // 演算を実行
                let result = match op.as_str() {
                    "+" => {
                        let b = temp_stack.pop().unwrap();
                        let a = temp_stack.pop().unwrap();
                        if let (ValueType::Number(n1), ValueType::Number(n2)) = 
                            (&a.val_type, &b.val_type) {
                            Value { val_type: ValueType::Number(n1.add(n2)) }
                        } else {
                            return Ok(tokens.to_vec());
                        }
                    },
                    "-" => {
                        let b = temp_stack.pop().unwrap();
                        let a = temp_stack.pop().unwrap();
                        if let (ValueType::Number(n1), ValueType::Number(n2)) = 
                            (&a.val_type, &b.val_type) {
                            Value { val_type: ValueType::Number(n1.sub(n2)) }
                        } else {
                            return Ok(tokens.to_vec());
                        }
                    },
                    "*" => {
                        let b = temp_stack.pop().unwrap();
                        let a = temp_stack.pop().unwrap();
                        if let (ValueType::Number(n1), ValueType::Number(n2)) = 
                            (&a.val_type, &b.val_type) {
                            Value { val_type: ValueType::Number(n1.mul(n2)) }
                        } else {
                            return Ok(tokens.to_vec());
                        }
                    },
                    "/" => {
                        let b = temp_stack.pop().unwrap();
                        let a = temp_stack.pop().unwrap();
                        if let (ValueType::Number(n1), ValueType::Number(n2)) = 
                            (&a.val_type, &b.val_type) {
                            if n2.numerator == 0 {
                                return Ok(tokens.to_vec());
                            }
                            Value { val_type: ValueType::Number(n1.div(n2)) }
                        } else {
                            return Ok(tokens.to_vec());
                        }
                    },
                    _ => return Ok(tokens.to_vec()),
                };
                
                // 結果を単一のトークンとして返す
                if let ValueType::Number(frac) = result.val_type {
                    console::log_1(&JsValue::from_str(&format!(
                        "Pre-evaluated constant expression: {} {} {} = {}/{}", 
                        n1, op, n2, frac.numerator, frac.denominator
                    )));
                    return Ok(vec![
                        Token::Number(frac.numerator, frac.denominator),
                        Token::Symbol("+".to_string())  // スタックに加算する演算子
                    ]);
                }
            }
        }
    }
    
    Ok(tokens.to_vec())
}
