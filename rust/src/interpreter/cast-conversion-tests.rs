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
        // Number
        let num = Value::from_fraction(Fraction::new(BigInt::from(42), BigInt::one()));
        assert_eq!(format_value_to_string_repr(&num), "42");

        // Boolean (now just a scalar in the new architecture, so displays as "1")
        let bool_val = Value::from_bool(true);
        assert_eq!(format_value_to_string_repr(&bool_val), "1");

        // Nil
        let nil = Value::nil();
        assert_eq!(format_value_to_string_repr(&nil), "NIL");
    }

    #[test]
    fn test_str_conversion() {
        let mut interp = Interpreter::new();

        // Number → String
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

        // String → Number (正常ケース)
        interp.stack.push(Value::from_string("42"));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
            if let Some(f) = val.as_scalar() {
                assert_eq!(f.numerator(), BigInt::from(42));
            }
        }

        // 分数文字列 → Number
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

        // パース失敗 → NIL (エラーではない)
        interp.stack.clear();
        interp.stack.push(Value::from_string("ABC"));
        let result = op_num(&mut interp);
        assert!(result.is_ok()); // エラーではない
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil()); // NILが返される
        }

        // 既に数値 → エラー (変化なしはエラー原則)
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(123), BigInt::one())));
        let result = op_num(&mut interp);
        assert!(result.is_err());

        // Boolean → エラー (Stringのみ受け付ける)
        interp.stack.clear();
        interp.stack.push(Value::from_bool(true));
        let result = op_num(&mut interp);
        assert!(result.is_err());
    }

    #[test]
    fn test_bool_conversion() {
        let mut interp = Interpreter::new();

        // String 'TRUE' → Boolean TRUE
        interp.stack.push(Value::from_string("TRUE"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // String 'true' (小文字) → Boolean TRUE
        interp.stack.clear();
        interp.stack.push(Value::from_string("true"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // String 'false' → Boolean FALSE
        interp.stack.clear();
        interp.stack.push(Value::from_string("false"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(f.is_zero()); // FALSE
            }
        }

        // String '1' → NIL (新仕様: 'true'/'false'以外はNIL)
        interp.stack.clear();
        interp.stack.push(Value::from_string("1"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil()); // パース失敗 → NIL
        }

        // String 'other' → NIL
        interp.stack.clear();
        interp.stack.push(Value::from_string("other"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil()); // パース失敗 → NIL
        }

        // Number 100 → Boolean TRUE (Truthiness: 0以外はTRUE)
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(100), BigInt::one())));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // Number 0 → Boolean FALSE
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(0), BigInt::one())));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(f.is_zero()); // FALSE
            }
        }

        // 分数 1/2 → Boolean TRUE (0以外はTRUE)
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(1), BigInt::from(2))));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // from_bool creates a scalar (same as number in new architecture),
        // so op_bool treats it as a number and applies truthiness conversion.
        // This is no longer an error since is_boolean_value always returns false.
        interp.stack.clear();
        interp.stack.push(Value::from_bool(true));
        let result = op_bool(&mut interp);
        assert!(result.is_ok());
    }

    // ============================================================================
    // CHR テスト
    // ============================================================================

    #[test]
    fn test_chr_basic() {
        let mut interp = Interpreter::new();

        // 65 → 'A'
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(65), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "A");
        }

        // 97 → 'a'
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

        // 10 → 改行
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

        // 48 → '0'
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

        // Note: マルチバイト文字（日本語など）のテストは、Value::from_stringが
        // bytes()を使用しているため、value_as_stringとの互換性の問題があります。
        // これは既存の設計上の制約です。
    }

    #[test]
    fn test_chr_errors() {
        let mut interp = Interpreter::new();

        // 文字列 → エラー
        interp.stack.push(Value::from_string("A"));
        let result = op_chr(&mut interp);
        assert!(result.is_err());

        // 分数 → エラー (整数のみ)
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(1), BigInt::from(2))));
        let result = op_chr(&mut interp);
        assert!(result.is_err());

        // 負の数 → エラー
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(-1), BigInt::one())));
        let result = op_chr(&mut interp);
        assert!(result.is_err());

        // 範囲外 (0x110000) → エラー
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

        // 65 CHR → 'A'
        interp.execute("65 CHR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "A");
        }
    }

    // ============================================================================
    // NUM/STR/BOOL 統合テスト
    // ============================================================================

    #[tokio::test]
    async fn test_num_str_roundtrip() {
        let mut interp = Interpreter::new();

        // '123' NUM STR → '123' (往復変換)
        interp.execute("'123' NUM STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "123");
        }

        // '1/3' NUM STR → '1/3'
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

        // 'ABC' NUM → NIL (パース失敗はNIL)
        interp.execute("'ABC' NUM").await.unwrap();
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }

    #[tokio::test]
    async fn test_bool_string_parsing() {
        let mut interp = Interpreter::new();

        // 'true' BOOL → TRUE (scalar 1)
        interp.execute("'true' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(val.is_truthy());
        }

        // 'FALSE' BOOL → FALSE (scalar 0)
        interp.stack.clear();
        interp.execute("'FALSE' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(!val.is_truthy());
        }

        // 'other' BOOL → NIL
        interp.stack.clear();
        interp.execute("'other' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }

    #[tokio::test]
    async fn test_bool_number_truthiness() {
        let mut interp = Interpreter::new();

        // 100 BOOL → TRUE (0以外はTRUE)
        interp.execute("100 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(val.is_truthy());
        }

        // 0 BOOL → FALSE
        interp.stack.clear();
        interp.execute("0 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(!val.is_truthy());
        }

        // -1 BOOL → TRUE
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

        // TRUE STR → '1' (in new architecture, booleans are just scalars)
        interp.execute("TRUE STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "1");
        }

        // FALSE STR → '0' (in new architecture, booleans are just scalars)
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

        // NIL STR → NIL (仕様セクション7.2: 不明な値に変換を射しても不明)
        interp.execute("NIL STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil(), "NIL STR should return NIL, not a string");
        }
    }
}
