#[cfg(test)]
mod tests {
    use crate::interpreter::cast::cast_conversions::{op_str, op_num, op_bool, op_chr};
    use crate::interpreter::cast::cast_value_helpers::{
        format_value_to_string_repr, is_number_value, is_string_value,
    };
    use crate::interpreter::value_extraction_helpers::{create_number_value, value_as_string};
    use crate::interpreter::Interpreter;
    use crate::types::fraction::Fraction;
    use crate::types::Value;
    use num_bigint::BigInt;
    use num_traits::One;

    #[test]
    fn test_format_value_to_string_repr() {

        let num = Value::from_fraction(Fraction::new(BigInt::from(42), BigInt::one()));
        assert_eq!(format_value_to_string_repr(&num), "42");


        let bool_val = Value::from_bool(true);
        assert_eq!(format_value_to_string_repr(&bool_val), "1");


        let nil = Value::nil();
        assert_eq!(format_value_to_string_repr(&nil), "NIL");
    }

    #[test]
    fn test_str_conversion() {
        let mut interp = Interpreter::new();


        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(42), BigInt::one())));
        op_str(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "42");
        }
    }

    #[test]
    fn test_num_conversion() {
        let mut interp = Interpreter::new();


        interp.stack.push(Value::from_string("42"));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
            if let Some(f) = val.as_scalar() {
                assert_eq!(f.numerator(), BigInt::from(42));
            }
        }


        interp.stack.clear();
        interp.stack.push(Value::from_string("1/3"));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
            if let Some(f) = val.as_scalar() {
                assert_eq!(f.numerator(), BigInt::from(1));
                assert_eq!(f.denominator(), BigInt::from(3));
            }
        }


        interp.stack.clear();
        interp.stack.push(Value::from_string("ABC"));
        let result = op_num(&mut interp);
        assert!(result.is_ok());
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }


        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(123), BigInt::one())));
        let result = op_num(&mut interp);
        assert!(result.is_ok());
        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
        }


        interp.stack.clear();
        interp.stack.push(Value::from_bool(true));
        let result = op_num(&mut interp);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bool_conversion() {
        let mut interp = Interpreter::new();


        interp.stack.push(Value::from_string("TRUE"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero());
            }
        }


        interp.stack.clear();
        interp.stack.push(Value::from_string("true"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero());
            }
        }


        interp.stack.clear();
        interp.stack.push(Value::from_string("false"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(f.is_zero());
            }
        }


        interp.stack.clear();
        interp.stack.push(Value::from_string("1"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }


        interp.stack.clear();
        interp.stack.push(Value::from_string("other"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }


        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(100), BigInt::one())));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero());
            }
        }


        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(0), BigInt::one())));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(f.is_zero());
            }
        }


        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(1), BigInt::from(2))));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero());
            }
        }




        interp.stack.clear();
        interp.stack.push(Value::from_bool(true));
        let result = op_bool(&mut interp);
        assert!(result.is_ok());
    }





    #[test]
    fn test_chr_basic() {
        let mut interp = Interpreter::new();


        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(65), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "A");
        }


        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(97), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "a");
        }


        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(10), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "\n");
        }


        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(48), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "0");
        }




    }

    #[test]
    fn test_chr_errors() {
        let mut interp = Interpreter::new();


        interp.stack.push(Value::from_string("A"));
        let result = op_chr(&mut interp);
        assert!(result.is_err());


        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(1), BigInt::from(2))));
        let result = op_chr(&mut interp);
        assert!(result.is_err());


        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(-1), BigInt::one())));
        let result = op_chr(&mut interp);
        assert!(result.is_err());


        interp.stack.clear();
        interp.stack.push(create_number_value(Fraction::new(
            BigInt::from(0x110000),
            BigInt::one(),
        )));
        let result = op_chr(&mut interp);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chr_integration() {
        let mut interp = Interpreter::new();


        interp.execute("65 CHR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "A");
        }
    }





    #[tokio::test]
    async fn test_num_str_roundtrip() {
        let mut interp = Interpreter::new();


        interp.execute("'123' NUM STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "123");
        }


        interp.stack.clear();
        interp.execute("'1/3' NUM STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "1/3");
        }
    }

    #[tokio::test]
    async fn test_str_num_parse_fail() {
        let mut interp = Interpreter::new();


        interp.execute("'ABC' NUM").await.unwrap();
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }

    #[tokio::test]
    async fn test_bool_string_parsing() {
        let mut interp = Interpreter::new();


        interp.execute("'true' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(val.is_truthy());
        }


        interp.stack.clear();
        interp.execute("'FALSE' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(!val.is_truthy());
        }


        interp.stack.clear();
        interp.execute("'other' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }

    #[tokio::test]
    async fn test_bool_number_truthiness() {
        let mut interp = Interpreter::new();


        interp.execute("100 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(val.is_truthy());
        }


        interp.stack.clear();
        interp.execute("0 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(!val.is_truthy());
        }


        interp.stack.clear();
        interp.execute("-1 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(val.is_truthy());
        }
    }

    #[tokio::test]
    async fn test_str_boolean() {
        let mut interp = Interpreter::new();


        interp.execute("TRUE STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "1");
        }


        interp.stack.clear();
        interp.execute("FALSE STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "0");
        }
    }

    #[tokio::test]
    async fn test_str_nil() {
        let mut interp = Interpreter::new();


        interp.execute("NIL STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil(), "NIL STR should return NIL, not a string");
        }
    }
}
