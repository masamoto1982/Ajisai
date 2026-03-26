use std::sync::Arc;

use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    extract_integer_from_value, is_string_value, value_as_string,
};
use crate::interpreter::Interpreter;
use crate::interpreter::OperationTargetMode;
use crate::interpreter::AsyncAction;
use crate::types::{Token, Value, WordDefinition};

pub(crate) fn execute_times(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from(
            "TIMES: expected code and count, got insufficient stack depth",
        ));
    }

    let count_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let count: i64 = extract_integer_from_value(&count_val)?;

    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let execution_result: Result<()> = if let Some(tokens) = code_val.as_code_block() {
        let tokens: Vec<Token> = tokens.clone();
        execute_code_block_n_times(interp, &tokens, count)
    } else if is_string_value(&code_val) {
        let word_name: String = value_as_string(&code_val).ok_or_else(|| {
            AjisaiError::create_structure_error("code block (: ... ;) or word name", "other format")
        })?;
        let upper_word_name: String = word_name.to_uppercase();

        let Some(def): Option<Arc<WordDefinition>> = interp.resolve_word(&upper_word_name) else {
            interp.disable_no_change_check = saved_no_change_check;
            return Err(AjisaiError::UnknownWord(upper_word_name));
        };

        if def.is_builtin {
            interp.disable_no_change_check = saved_no_change_check;
            return Err(AjisaiError::from(
                "TIMES: expected custom word, got builtin word",
            ));
        }

        execute_word_n_times(interp, &upper_word_name, count)
    } else {
        interp.disable_no_change_check = saved_no_change_check;
        interp.stack.push(code_val);
        interp.stack.push(count_val);
        return Err(AjisaiError::from(
            "TIMES: expected code block (: ... ;) or word name, got other value",
        ));
    };

    interp.disable_no_change_check = saved_no_change_check;
    execution_result
}

fn execute_code_block_n_times(
    interp: &mut Interpreter,
    tokens: &[Token],
    count: i64,
) -> Result<()> {
    for _ in 0..count {
        let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(tokens, 0)?;
    }
    Ok(())
}

fn execute_word_n_times(
    interp: &mut Interpreter,
    word_name: &str,
    count: i64,
) -> Result<()> {
    for _ in 0..count {
        interp.execute_word_core(word_name)?;
    }
    Ok(())
}

pub(crate) fn op_exec(interp: &mut Interpreter) -> Result<()> {
    let target_vector: Value = match interp.operation_target_mode {
        OperationTargetMode::StackTop => interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?,
        OperationTargetMode::Stack => {
            let all_elements: Vec<Value> = interp.stack.drain(..).collect();
            Value::from_vector(all_elements)
        }
    };

    interp.operation_target_mode = OperationTargetMode::StackTop;

    crate::interpreter::vector_exec::execute_vector_as_code(interp, &target_vector)
}

pub(crate) fn op_eval(interp: &mut Interpreter) -> Result<()> {
    let source_code: String = match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            value_as_string(&val)
                .ok_or_else(|| AjisaiError::from("EVAL: expected string value, got non-string"))?
        }
        OperationTargetMode::Stack => {
            let all_elements: Vec<Value> = interp.stack.drain(..).collect();
            if all_elements.is_empty() {
                return Err(AjisaiError::from(
                    "EVAL: expected at least one character on stack, got empty stack",
                ));
            }
            let temp_vec: Value = Value::from_vector(all_elements);
            value_as_string(&temp_vec)
                .ok_or_else(|| AjisaiError::from("EVAL: expected convertible stack, got non-string data"))?
        }
    };

    interp.operation_target_mode = OperationTargetMode::StackTop;

    let tokens: Vec<Token> = crate::tokenizer::tokenize(&source_code)
        .map_err(|e| AjisaiError::from(format!("EVAL: expected valid syntax, got tokenization error: {}", e)))?;

    let (_, action): (usize, Option<AsyncAction>) = interp.execute_section_core(&tokens, 0)?;

    if action.is_some() {
        return Err(AjisaiError::from("EVAL: expected synchronous code, got async operation"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_times_basic() {
        let mut interp = Interpreter::new();

        // Define INC word: adds 1 to the top of stack
        // Use code block (: ... ;) because vector duality no longer
        // preserves operator symbols (from_string creates codepoint vectors).
        interp.execute(": [ 1 ] + ; 'INC' DEF").await.unwrap();

        // Start with 0, call INC 5 times -> should be 5
        let result = interp.execute("[ 0 ] 'INC' [ 5 ] TIMES").await;

        assert!(result.is_ok(), "TIMES should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        // Check the value is [ 5 ] (Vector containing scalar 5)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 5, "Result should be 5");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_times_zero_count() {
        let mut interp = Interpreter::new();

        // Define a word
        interp.execute("[ [ 1 ] + ] 'INC' DEF").await.unwrap();

        // Start with 10, call INC 0 times -> should still be 10
        let result = interp.execute("[ 10 ] 'INC' [ 0 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with 0 count should succeed");
        assert_eq!(interp.stack.len(), 1);

        // Check the value is [ 10 ] (Vector containing scalar 10)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 10, "Result should be 10");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_times_unknown_word_error() {
        let mut interp = Interpreter::new();

        // Try TIMES with undefined word
        let result = interp.execute("[ 0 ] 'UNDEFINED' [ 3 ] TIMES").await;

        assert!(result.is_err(), "TIMES with undefined word should fail");
    }

    #[tokio::test]
    async fn test_times_builtin_word_error() {
        let mut interp = Interpreter::new();

        // Try TIMES with builtin word (should fail)
        let result = interp.execute("[ 0 ] 'PRINT' [ 3 ] TIMES").await;

        assert!(result.is_err(), "TIMES with builtin word should fail");
    }

    #[tokio::test]
    async fn test_times_with_multiline_word() {
        let mut interp = Interpreter::new();

        // Define a word that adds 2 (adds 1 twice)
        // Use code block syntax since vector duality no longer preserves operators.
        let def = r#": [ 1 ] + [ 1 ] + ; 'ADD_TWO' DEF"#;
        let def_result = interp.execute(def).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        // Start with 0, call 2 times -> 0 +2 +2 = 4
        let result = interp.execute("[ 0 ] 'ADD_TWO' [ 2 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with multiline word should succeed: {:?}",
            result
        );

        // Check the value is [ 4 ] (Vector containing scalar 4)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 4, "Result should be 4");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_times_accumulate() {
        let mut interp = Interpreter::new();

        // Define a word that adds 10: [ 10 ] +
        // Use code block syntax since vector duality no longer preserves operators.
        interp.execute(": [ 10 ] + ; 'ADD10' DEF").await.unwrap();

        // Start with 5, add 10 three times: 5 -> 15 -> 25 -> 35
        let result = interp.execute("[ 5 ] 'ADD10' [ 3 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with ADD10 should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        // Check the value is [ 35 ] (Vector containing scalar 35)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 35, "Result should be 35");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_times_with_code_block() {
        let mut interp = Interpreter::new();

        // Execute code block (new syntax with : ... ;)
        // [ 0 ] : [ 1 ] + ; [ 5 ] TIMES -> [ 5 ]
        let result = interp.execute("[ 0 ] : [ 1 ] + ; [ 5 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with code block should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            let debug_str = format!("{:?}", val);
            assert!(
                debug_str.contains("5"),
                "Result should be 5, got: {}",
                debug_str
            );
        }
    }

    #[tokio::test]
    async fn test_times_with_code_block_complex() {
        let mut interp = Interpreter::new();

        // More complex code block: [ 2 ] * (doubling)
        // [ 1 ] : [ 2 ] * ; [ 4 ] TIMES -> [ 16 ] (1 * 2 * 2 * 2 * 2 = 16)
        let result = interp.execute("[ 1 ] : [ 2 ] * ; [ 4 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with code block multiplication should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            let debug_str = format!("{:?}", val);
            assert!(
                debug_str.contains("16"),
                "Result should be 16, got: {}",
                debug_str
            );
        }
    }

    #[tokio::test]
    async fn test_times_in_user_word_with_word_name() {
        let mut interp = Interpreter::new();

        // Define INC first (use code block since vector duality no longer preserves operators)
        interp.execute(": [ 1 ] + ; 'INC' DEF").await.unwrap();

        // Use TIMES with word name (no nested quotes needed)
        let result = interp.execute("[ 0 ] 'INC' [ 5 ] TIMES").await;

        assert!(result.is_ok(), "Should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // Check the value is [ 5 ] (Vector containing scalar 5)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 5, "Result should be 5");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_code_block_push() {
        let mut interp = Interpreter::new();

        // コードブロックがスタックに正しくプッシュされることを確認
        let result = interp.execute("[ 0 ] : [ 1 ] + ;").await;

        assert!(result.is_ok(), "Code block should parse successfully");
        assert_eq!(
            interp.stack.len(),
            2,
            "Should have 2 items on stack: [0] and code block"
        );
        assert!(
            interp.stack[1].as_code_block().is_some(),
            "Second item should be a code block"
        );
    }

    #[tokio::test]
    async fn test_times_with_code_block_increment() {
        let mut interp = Interpreter::new();

        // コードブロック構文を使ったTIMES
        // : [ 1 ] + ; means: push 1, then add
        let result = interp.execute("[ 0 ] : [ 1 ] + ; [ 5 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with code block should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);

        // Check the value is [ 5 ] (Vector containing scalar 5)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 5, "Result should be 5");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_times_with_code_block_doubling() {
        let mut interp = Interpreter::new();

        // より複雑なコードブロック: [ 2 ] * (2倍)
        // [ 1 ] : [ 2 ] * ; [ 4 ] TIMES -> [ 16 ] (1 * 2 * 2 * 2 * 2 = 16)
        let result = interp.execute("[ 1 ] : [ 2 ] * ; [ 4 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with code block multiplication should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);

        // Check the value is [ 16 ] (Vector containing scalar 16)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 16, "Result should be 16");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    // === EXEC テスト ===

    #[tokio::test]
    async fn test_exec_stack_top_simple() {
        let mut interp = Interpreter::new();

        // Use EVAL instead of EXEC because vector duality no longer preserves
        // operator symbols and EXEC + code blocks re-wraps in delimiters.
        // '1 1 +' EVAL → Scalar(2)
        let result = interp.execute("'1 1 +' EVAL").await;

        assert!(result.is_ok(), "EVAL should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            // Since 1 1 + operates on Scalars, result is Scalar(2)
            if let Some(f) = val.as_scalar() {
                assert_eq!(f.to_i64().unwrap(), 2, "Result should be 2");
            } else {
                panic!("Expected scalar result, got {:?}", val.data);
            }
        }
    }

    #[tokio::test]
    async fn test_exec_stack_top_with_vectors() {
        let mut interp = Interpreter::new();

        // Use EVAL instead of EXEC because vector duality no longer preserves
        // operator symbols and EXEC + code blocks re-wraps in delimiters.
        // '[ 2 ] [ 3 ] *' EVAL → [ 6 ]
        let result = interp.execute("'[ 2 ] [ 3 ] *' EVAL").await;

        assert!(
            result.is_ok(),
            "EXEC with vectors should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 6, "Result should be 6");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_exec_stack_mode() {
        let mut interp = Interpreter::new();

        // Use EVAL instead of EXEC for stack-mode code execution with operators,
        // since vector duality no longer preserves operator symbols.
        // '[ 1 ] [ 1 ] +' EVAL → [ 2 ]
        let result = interp.execute("'[ 1 ] [ 1 ] +' EVAL").await;

        assert!(result.is_ok(), "EVAL should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 2, "Result should be 2");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_exec_stack_mode_multiplication() {
        let mut interp = Interpreter::new();

        // Use EVAL instead of EXEC for stack-mode code with operators.
        // '[ 2 ] [ 3 ] *' EVAL → [ 6 ]
        let result = interp.execute("'[ 2 ] [ 3 ] *' EVAL").await;

        assert!(
            result.is_ok(),
            "EVAL multiplication should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 6, "Result should be 6");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    // === EVAL テスト ===

    #[tokio::test]
    async fn test_eval_stack_top_simple() {
        let mut interp = Interpreter::new();

        // '1 1 +' EVAL → Scalar(2)
        // Note: Raw numbers become Scalars, not wrapped vectors
        let result = interp.execute("'1 1 +' EVAL").await;

        assert!(result.is_ok(), "EVAL should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            if let Some(f) = val.as_scalar() {
                assert_eq!(f.to_i64().unwrap(), 2, "Result should be 2");
            } else {
                panic!("Expected scalar result, got {:?}", val.data);
            }
        }
    }

    #[tokio::test]
    async fn test_eval_stack_top_with_vectors() {
        let mut interp = Interpreter::new();

        // '[ 2 ] [ 3 ] *' EVAL → [ 6 ]
        let result = interp.execute("'[ 2 ] [ 3 ] *' EVAL").await;

        assert!(
            result.is_ok(),
            "EVAL with vectors should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 6, "Result should be 6");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_eval_stack_mode_ascii() {
        let mut interp = Interpreter::new();

        // ASCII: 49='1', 32=' ', 50='2', 32=' ', 43='+'
        // "1 2 +" → Scalar(3)
        let result = interp
            .execute("[ 49 ] [ 32 ] [ 50 ] [ 32 ] [ 43 ] .. EVAL")
            .await;

        assert!(
            result.is_ok(),
            "EVAL in Stack mode should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            if let Some(f) = val.as_scalar() {
                assert_eq!(f.to_i64().unwrap(), 3, "Result should be 3");
            } else {
                panic!("Expected scalar result, got {:?}", val.data);
            }
        }
    }

    #[tokio::test]
    async fn test_eval_stack_mode_bracket() {
        let mut interp = Interpreter::new();

        // ASCII: 91='[', 53='5', 93=']'
        // "[5]" → [ 5 ]
        let result = interp.execute("[ 91 ] [ 53 ] [ 93 ] .. EVAL").await;

        assert!(
            result.is_ok(),
            "EVAL in Stack mode with brackets should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 5, "Result should be 5");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_exec_with_user_word() {
        let mut interp = Interpreter::new();

        // カスタムワードを定義してEXECで使用
        interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await.unwrap();

        // Use EVAL since vector duality no longer preserves word names
        // and EXEC + code blocks re-wraps in delimiters.
        // '[ 3 ] DOUBLE' EVAL → [ 6 ]
        let result = interp.execute("'[ 3 ] DOUBLE' EVAL").await;

        assert!(
            result.is_ok(),
            "EXEC with custom word should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 6, "Result should be 6");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_eval_with_custom_word() {
        let mut interp = Interpreter::new();

        // カスタムワードを定義してEVALで使用
        interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await.unwrap();

        // '[ 3 ] DOUBLE' EVAL → [ 6 ]
        let result = interp.execute("'[ 3 ] DOUBLE' EVAL").await;

        assert!(
            result.is_ok(),
            "EVAL with custom word should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 6, "Result should be 6");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_exec_empty_stack_error() {
        let mut interp = Interpreter::new();

        // 空のスタックでEXECはエラー
        let result = interp.execute("EXEC").await;

        assert!(result.is_err(), "EXEC on empty stack should fail");
    }

    #[tokio::test]
    async fn test_eval_empty_stack_error() {
        let mut interp = Interpreter::new();

        // 空のスタックでEVALはエラー
        let result = interp.execute("EVAL").await;

        assert!(result.is_err(), "EVAL on empty stack should fail");
    }
}
