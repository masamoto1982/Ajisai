// rust/src/interpreter/mod.rs (execute_tokens修正版)

    fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(num, den) => {
                    self.stack.push(Value {
                        val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                    });
                    i += 1;
                },
                Token::String(s) => {
                    self.stack.push(Value {
                        val_type: ValueType::String(s.clone()),
                    });
                    i += 1;
                },
                Token::Boolean(b) => {
                    self.stack.push(Value {
                        val_type: ValueType::Boolean(*b),
                    });
                    i += 1;
                },
                Token::Nil => {
                    self.stack.push(Value {
                        val_type: ValueType::Nil,
                    });
                    i += 1;
                },
                Token::VectorStart => {
                    let (vector_values, consumed) = self.collect_vector(tokens, i)?;
                    self.stack.push(Value {
                        val_type: ValueType::Vector(vector_values),
                    });
                    i += consumed; // 消費されたトークン分をスキップ
                },
                Token::QuotationStart => {
                    let (quotation_tokens, consumed) = self.collect_quotation(tokens, i)?;
                    self.stack.push(Value {
                        val_type: ValueType::Quotation(quotation_tokens),
                    });
                    i += consumed; // 消費されたトークン分をスキップ
                },
                Token::Symbol(name) => {
                    if name == "DEF" {
                        self.handle_def()?;
                    } else {
                        self.execute_word(name)?;
                    }
                    i += 1;
                },
                Token::VectorEnd | Token::QuotationEnd => {
                    return Err(error::AjisaiError::from("Unexpected closing delimiter"));
                },
                _ => {
                    return Err(error::AjisaiError::from("Unexpected token"));
                }
            }
        }
        Ok(())
    }

    fn collect_vector(&mut self, tokens: &[Token], start: usize) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = start + 1;
        let mut depth = 1;

        while i < tokens.len() && depth > 0 {
            match &tokens[i] {
                Token::VectorStart => depth += 1,
                Token::VectorEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((values, i - start + 1));
                    }
                },
                token if depth == 1 => {
                    values.push(self.token_to_value(token)?);
                }
                _ => {} // ネストしたベクターの内部は別途処理
            }
            i += 1;
        }

        Err(error::AjisaiError::from("Unclosed vector"))
    }

    fn collect_quotation(&mut self, tokens: &[Token], start: usize) -> Result<(Vec<Token>, usize)> {
        let mut quotation_tokens = Vec::new();
        let mut i = start + 1;
        let mut depth = 1;

        while i < tokens.len() && depth > 0 {
            match &tokens[i] {
                Token::QuotationStart => {
                    quotation_tokens.push(tokens[i].clone());
                    depth += 1;
                },
                Token::QuotationEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((quotation_tokens, i - start + 1));
                    } else {
                        quotation_tokens.push(tokens[i].clone());
                    }
                },
                token => {
                    quotation_tokens.push(token.clone());
                }
            }
            i += 1;
        }

        Err(error::AjisaiError::from("Unclosed quotation"))
    }
