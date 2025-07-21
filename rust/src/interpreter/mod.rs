pub fn execute_tokens_with_context(&mut self, tokens: &[Token]) -> Result<()> {
    let mut i = 0;
    let mut pending_description: Option<String> = None;

    while i < tokens.len() {
        let token = &tokens[i];
        match token {
            Token::Description(text) => {
                pending_description = Some(text.clone());
            },
            Token::Number(num, den) => {
                self.stack.push(Value {
                    val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                });
            },
            Token::String(s) => {
                self.stack.push(Value {
                    val_type: ValueType::String(s.clone()),
                });
            },
            Token::Boolean(b) => {
                self.stack.push(Value {
                    val_type: ValueType::Boolean(*b),
                });
            },
            Token::Nil => {
                self.stack.push(Value {
                    val_type: ValueType::Nil,
                });
            },
            Token::VectorStart => {
                let (vector_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                self.stack.push(Value {
                    val_type: ValueType::Vector(vector_values),
                });
                i += consumed - 1;
            },
            Token::BlockStart => {
                let (block_tokens, next_index) = self.collect_block_tokens(tokens, i)?;
                self.stack.push(Value {
                    val_type: ValueType::Quotation(block_tokens),
                });
                i = next_index -1;
            },
            Token::Symbol(name) => {
                if name == "DEF" {
                    // DEFの後のDescriptionトークンを探す
                    let mut description = None;
                    if i + 1 < tokens.len() {
                        if let Token::Description(text) = &tokens[i + 1] {
                            description = Some(text.clone());
                            i += 1; // Descriptionトークンをスキップ
                        }
                    }
                    control::op_def(self, description)?;
                    pending_description = None; // DEF処理後は保留中の説明をクリア
                } else if let Some(def) = self.dictionary.get(name).cloned() {
                    if def.is_builtin {
                        self.execute_builtin(name)?;
                    } else {
                        self.execute_custom_word(name, &def.tokens)?;
                    }
                } else {
                    return Err(AjisaiError::UnknownWord(name.clone()));
                }
            },
            Token::VectorEnd | Token::BlockEnd => return Err(AjisaiError::from("Unexpected closing delimiter found.")),
        }
        i += 1;
    }
    Ok(())
}
