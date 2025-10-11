// rust/src/interpreter/eval.rs

use crate::interpreter::{Interpreter, OperationTarget, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType};
use std::collections::HashSet;
use num_bigint::BigInt;
use num_traits::One;

pub fn op_eval(interp: &mut Interpreter) -> Result<()> {
    let code_string = match interp.operation_target {
        OperationTarget::StackTop => {
            let code_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            vector_to_code_string(&code_vec)?
        },
        OperationTarget::Stack => {
            let all_vecs = std::mem::take(&mut interp.stack);
            let code_parts: Result<Vec<String>> = all_vecs.iter()
                .map(vector_to_code_string)
                .collect();
            code_parts?.join(" ")
        },
    };

    execute_code(interp, &code_string)
}

fn vector_to_code_string(vec: &Value) -> Result<String> {
    match &vec.val_type {
        ValueType::Vector(elements, _) => {
            let parts: Result<Vec<String>> = elements.iter()
                .map(|elem| match &elem.val_type {
                    ValueType::String(s) => Ok(s.clone()),
                    ValueType::Number(n) => {
                        if n.denominator == BigInt::one() {
                            Ok(n.numerator.to_string())
                        } else {
                            Ok(format!("{}/{}", n.numerator, n.denominator))
                        }
                    },
                    ValueType::Symbol(s) => Ok(s.clone()),
                    ValueType::Boolean(b) => Ok(if *b { "TRUE".to_string() } else { "FALSE".to_string() }),
                    ValueType::Nil => Ok("NIL".to_string()),
                    ValueType::Vector(_, bracket_type) => {
                        Ok(format!("{}", elem))
                    },
                    _ => Err(AjisaiError::from("EVAL cannot convert this element type to code")),
                })
                .collect();
            
            Ok(parts?.join(" "))
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

fn execute_code(interp: &mut Interpreter, code: &str) -> Result<()> {
    let custom_word_names: HashSet<String> = interp.dictionary.iter()
        .filter(|(_, def)| !def.is_builtin)
        .map(|(name, _)| name.clone())
        .collect();
    
    let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)
        .map_err(|e| AjisaiError::from(format!("EVAL tokenization error: {}", e)))?;
    
    interp.execute_tokens_sync(&tokens)
}
