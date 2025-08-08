use crate::types::{Value, ValueType, Token};
use super::{Interpreter, error::{AjisaiError, Result}};
use wasm_bindgen::JsValue;
use web_sys::console;

impl Interpreter {
    pub(super) fn collect_vector_as_data(&self, tokens: &[Token]) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorEnd => return Ok((values, i + 1)),
                Token::VectorStart => {
                    let (nested_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                    values.push(Value { val_type: ValueType::Vector(nested_values) });
                    i += consumed;
                    continue;
                },
                Token::Number(num, den) => values.push(Value { 
                    val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)) 
                }),
                Token::String(s) => values.push(Value { val_type: ValueType::String(s.clone()) }),
                Token::Boolean(b) => values.push(Value { val_type: ValueType::Boolean(*b) }),
                Token::Nil => values.push(Value { val_type: ValueType::Nil }),
                Token::Symbol(s) => values.push(Value { val_type: ValueType::Symbol(s.clone()) }),
            }
            i += 1;
        }

        Err(AjisaiError::from("Unclosed vector"))
    }

    pub(super) fn rearrange_tokens(&self, tokens: &[Token]) -> Vec<Token> {
    console::log_1(&JsValue::from_str("--- rearrange_tokens ---"));
    console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));
    
    // 演算子の位置を特定
    let mut operator_positions = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        if let Token::Symbol(name) = token {
            if self.is_operator(name) {
                // 演算子の前後が適切な要素かチェック
                let valid_prefix = i == 0 || self.is_valid_operand(&tokens[i-1]);
                let valid_suffix = i == tokens.len() - 1 || self.is_valid_operand(&tokens[i+1]);
                
                // 前置記法または中置記法として有効な場合のみ
                if (i == 0 && tokens.len() >= 3 && valid_suffix) ||  // 前置
                   (i > 0 && i < tokens.len() - 1 && valid_prefix && valid_suffix) {  // 中置
                    operator_positions.push(i);
                }
            }
        }
    }
    
    if operator_positions.is_empty() {
        return tokens.to_vec();
    }
    
    // 演算子が1つの場合の処理
    if operator_positions.len() == 1 {
        let op_pos = operator_positions[0];
        let op = &tokens[op_pos];
        
        // 前置記法: + a b
        if op_pos == 0 && tokens.len() >= 3 {
            let mut result = vec![tokens[1].clone(), tokens[2].clone(), op.clone()];
            // 残りのトークンを追加
            for i in 3..tokens.len() {
                result.push(tokens[i].clone());
            }
            console::log_1(&JsValue::from_str(&format!("Prefix notation converted to RPN: {:?}", result)));
            return result;
        }
        // 中置記法: a + b
        else if op_pos > 0 && op_pos < tokens.len() - 1 {
            let mut result = vec![tokens[op_pos - 1].clone(), tokens[op_pos + 1].clone(), op.clone()];
            // 残りのトークンを追加
            for i in op_pos + 2..tokens.len() {
                result.push(tokens[i].clone());
            }
            console::log_1(&JsValue::from_str(&format!("Infix notation converted to RPN: {:?}", result)));
            return result;
        }
    }
    
    tokens.to_vec()
}

// 演算可能な要素かチェック
fn is_valid_operand(&self, token: &Token) -> bool {
    match token {
        Token::Number(_, _) => true,
        Token::VectorStart | Token::VectorEnd => true,
        Token::Symbol(s) => self.dictionary.contains_key(s),
        _ => false,
    }
}

    fn is_operator(&self, name: &str) -> bool {
        matches!(name, "+" | "-" | "*" | "/" | ">" | ">=" | "=" | "<" | "<=")
    }

    fn collect_operand(&self, tokens: &[Token]) -> Vec<Token> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut depth = 0;
    
    while i < tokens.len() {
        match &tokens[i] {
            Token::VectorStart => {
                if depth == 0 && !result.is_empty() {
                    break;
                }
                depth += 1;
                result.push(tokens[i].clone());
            }
            Token::VectorEnd => {
                result.push(tokens[i].clone());
                depth -= 1;
                if depth == 0 {
                    // 構造が完結したので終了
                    break;  // i += 1 を削除
                }
            }
            _ => {
                result.push(tokens[i].clone());
                if depth == 0 {
                    // 単一トークンのオペランドなので終了
                    break;  // i += 1 を削除
                }
            }
        }
        i += 1;
    }
    
    result
}

    fn collect_operands(&self, tokens: &[Token], count: usize) -> Vec<Token> {
        let mut result = Vec::new();
        let mut pos = 0;
        
        for _ in 0..count {
            if pos >= tokens.len() {
                break;
            }
            let operand = self.collect_operand(&tokens[pos..]);
            let operand_len = operand.len();
            result.extend(operand);
            pos += operand_len;
        }
        
        result
    }

    pub(super) fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n, d) => if *d == 1 { n.to_string() } else { format!("{}/{}", n, d) },
            Token::String(s) => format!("\"{}\"", s),
            Token::Boolean(b) => b.to_string(),
            Token::Nil => "nil".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
        }
    }
}
