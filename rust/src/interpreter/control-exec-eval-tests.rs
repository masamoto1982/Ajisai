#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    // === EXEC tests ===

    #[tokio::test]
    async fn test_exec_stack_top_simple() {
        let mut interp = Interpreter::new();

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
    async fn test_exec_stack_top_with_vectors() {
        let mut interp = Interpreter::new();

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

    // === EVAL tests ===

    #[tokio::test]
    async fn test_eval_stack_top_simple() {
        let mut interp = Interpreter::new();

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

        interp.execute("[ 2 ] * | 'DOUBLE' DEF").await.unwrap();

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

        interp.execute("[ 2 ] * | 'DOUBLE' DEF").await.unwrap();

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

        let result = interp.execute("EXEC").await;

        assert!(result.is_err(), "EXEC on empty stack should fail");
    }

    #[tokio::test]
    async fn test_eval_empty_stack_error() {
        let mut interp = Interpreter::new();

        let result = interp.execute("EVAL").await;

        assert!(result.is_err(), "EVAL on empty stack should fail");
    }
}
