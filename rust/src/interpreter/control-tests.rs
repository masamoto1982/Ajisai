#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_times_basic() {
        let mut interp = Interpreter::new();

        interp.execute(": [ 1 ] + ; 'INC' DEF").await.unwrap();

        let result = interp.execute("[ 0 ] 'INC' [ 5 ] TIMES").await;

        assert!(result.is_ok(), "TIMES should succeed: {:?}", result);
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
    async fn test_times_zero_count() {
        let mut interp = Interpreter::new();

        interp.execute("[ [ 1 ] + ] 'INC' DEF").await.unwrap();

        let result = interp.execute("[ 10 ] 'INC' [ 0 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with 0 count should succeed");
        assert_eq!(interp.stack.len(), 1);

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

        let result = interp.execute("[ 0 ] 'UNDEFINED' [ 3 ] TIMES").await;

        assert!(result.is_err(), "TIMES with undefined word should fail");
    }

    #[tokio::test]
    async fn test_times_builtin_word_error() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ 0 ] 'PRINT' [ 3 ] TIMES").await;

        assert!(result.is_err(), "TIMES with builtin word should fail");
    }

    #[tokio::test]
    async fn test_times_with_multiline_word() {
        let mut interp = Interpreter::new();

        let def = r#": [ 1 ] + [ 1 ] + ; 'ADD_TWO' DEF"#;
        let def_result = interp.execute(def).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        let result = interp.execute("[ 0 ] 'ADD_TWO' [ 2 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with multiline word should succeed: {:?}",
            result
        );

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

        interp.execute(": [ 10 ] + ; 'ADD10' DEF").await.unwrap();

        let result = interp.execute("[ 5 ] 'ADD10' [ 3 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with ADD10 should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

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

        interp.execute(": [ 1 ] + ; 'INC' DEF").await.unwrap();

        let result = interp.execute("[ 0 ] 'INC' [ 5 ] TIMES").await;

        assert!(result.is_ok(), "Should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

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

        let result = interp.execute("[ 0 ] : [ 1 ] + ; [ 5 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with code block should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);

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

        let result = interp.execute("[ 1 ] : [ 2 ] * ; [ 4 ] TIMES").await;

        assert!(
            result.is_ok(),
            "TIMES with code block multiplication should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);

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
}
