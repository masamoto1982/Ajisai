// rust/src/interpreter/token_processor.rs

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
                _ => {}
            }
            i += 1;
        }

        Err(AjisaiError::from("Unclosed vector"))
    }

    pub(super) fn collect_block_tokens(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Token>, usize)> {
        let mut block_tokens = Vec::new();
        let mut depth = 1;
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::BlockStart => depth += 1,
                Token::BlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((block_tokens, i + 1));
                    }
                },
                _ => {}
            }
            block_tokens.push(tokens[i].clone());
            i += 1;
        }

        Err(AjisaiError::from("Unclosed block"))
    }

    // 新しい統一的なRPN変換メソッド
    pub(super) fn convert_to_rpn(&self, tokens: &[Token]) -> Vec<Token> {
        console::log_1(&JsValue::from_str("--- convert_to_rpn ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));

        // 演算子の位置を特定
        let mut operator_positions = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            if let Token::Symbol(name) = token {
                if self.is_operator(name) {
                    operator_positions.push(i);
                }
            }
        }
        
        if operator_positions.is_empty() {
            // 演算子がない場合はそのまま返す
            return tokens.to_vec();
        }
        
        // 演算子が1つの場合の処理
        if operator_positions.len() == 1 {
            let op_pos = operator_positions[0];
            let op = &tokens[op_pos];
            
            // 前置記法: + a b
            if op_pos == 0 && tokens.len() >= 3 {
                // Vector/Blockを含む任意のオペランドに対応
                let operands = self.collect_operands(&tokens[1..], 2);
                if operands.len() >= 2 {
                    let mut result = operands;
                    result.push(op.clone());
                    console::log_1(&JsValue::from_str(&format!("Prefix notation converted to RPN: {:?}", result)));
                    return result;
                }
            }
            // 中置記法: a + b
            else if op_pos > 0 && op_pos < tokens.len() - 1 {
                let left_operand = self.collect_operand(&tokens[..op_pos]);
                let right_operand = self.collect_operand(&tokens[op_pos + 1..]);
                
                let mut result = left_operand;
                result.extend(right_operand);
                result.push(op.clone());
                console::log_1(&JsValue::from_str(&format!("Infix notation converted to RPN: {:?}", result)));
                return result;
            }
            // 後置記法: a b +
            else if op_pos == tokens.len() - 1 && tokens.len() >= 3 {
                // すでにRPN形式
                return tokens.to_vec();
            }
        }
        
        // 複数の演算子がある場合や他のパターンはそのまま返す
        tokens.to_vec()
    }

    // オペランドを1つ収集（Vectorやブロックを含む）
    fn collect_operand(&self, tokens: &[Token]) -> Vec<Token> {
        let mut result = Vec::new();
        let mut i = 0;
        let mut depth = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart | Token::BlockStart => {
                    if depth == 0 && !result.is_empty() {
                        // 新しい構造の開始なので、前のオペランドで終了
                        break;
                    }
                    depth += 1;
                    result.push(tokens[i].clone());
                }
                Token::VectorEnd | Token::BlockEnd => {
                    result.push(tokens[i].clone());
                    depth -= 1;
                    if depth == 0 {
                        // 構造が完結したので終了
                        i += 1;
                        break;
                    }
                }
                _ => {
                    result.push(tokens[i].clone());
                    if depth == 0 {
                        // 単一トークンのオペランドなので終了
                        i += 1;
                        break;
                    }
                }
            }
            i += 1;
        }
        
        result
    }

    // 指定された数のオペランドを収集
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

    // rearrange_tokensを統一的なconvert_to_rpnを使うように修正
    pub(super) fn rearrange_tokens(&self, tokens: &[Token]) -> Vec<Token> {
        console::log_1(&JsValue::from_str("--- rearrange_tokens ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));
        
        // 統一的なRPN変換を使用
        let result = self.convert_to_rpn(tokens);
        
        console::log_1(&JsValue::from_str(&format!("Output tokens (RPN): {:?}", result)));
        console::log_1(&JsValue::from_str("--- end rearrange_tokens ---"));
        result
    }

    fn is_operator(&self, name: &str) -> bool {
        matches!(name, "+" | "-" | "*" | "/" | ">" | ">=" | "=" | "<" | "<=")
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
            Token::BlockStart => "{".to_string(),
            Token::BlockEnd => "}".to_string(),
        }
    }

    // 以下の2つのメソッドは削除（convert_to_rpn_structureとis_commutative_operator）
    // これらの機能はconvert_to_rpnに統合されました
}
