//! Behavioral probes for the number-closing Words `POW` `GCD` `RATIO` — the
//! tier each answer lands in, the projections the contracts declare, and the
//! lifting every arithmetic Word shares (LANG.VALUES.EXACT,
//! LANG.COLLECTIONS.LIFT).

#[cfg(test)]
mod power_words_tests {
    use crate::interpreter::Interpreter;

    async fn top(code: &str) -> String {
        let mut interp = Interpreter::new();
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
        interp
            .get_stack()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    }

    async fn error_of(code: &str) -> String {
        let mut interp = Interpreter::new();
        let err = interp.execute(code).await.expect_err("must raise an ERROR");
        crate::error::ErrorCategory::from_error(&err)
            .as_protocol_str()
            .to_string()
    }

    #[tokio::test]
    async fn pow_stays_exact_where_the_field_holds_the_answer() {
        assert_eq!(top("2 10 POW").await, "1024/1");
        assert_eq!(top("2 -2 POW").await, "1/4");
        assert_eq!(top("-3 3 POW").await, "-27/1");
        assert_eq!(top("0 0 POW").await, "1/1");
        assert_eq!(top("4 1/2 POW").await, "2/1");
        assert_eq!(top("9/4 -3/2 POW").await, "8/27");
        assert_eq!(top("0 1/2 POW").await, "0/1");
        assert_eq!(top("2 1/2 POW 2 SQRT EQ").await, "TRUE");
        assert_eq!(top("2 3/2 POW 2 SQRT 2 MUL EQ").await, "TRUE");
        assert_eq!(top("2 SQRT 2 POW").await, "2/1");
        assert_eq!(top("2 SQRT -2 POW").await, "1/2");
        assert_eq!(top("2 SQRT 3 POW 2 SQRT 2 MUL EQ").await, "TRUE");
        assert_eq!(
            top("2 SQRT 3 SQRT ADD -1 POW 3 SQRT 2 SQRT SUB EQ").await,
            "TRUE"
        );
        assert_eq!(top("[ 1 2 3 ] 2 POW").await, "[ 1/1 4/1 9/1 ]");
        assert_eq!(top("2 [ 1 2 3 ] POW").await, "[ 2/1 4/1 8/1 ]");
    }

    /// Every other exponent leaves the field, and POW says so rather than
    /// answer with something no comparison could decide: a root other than a
    /// square root, a square root of an irrational base, an irrational
    /// exponent — even where the answer happens to be rational (`8 1/3 POW`).
    #[tokio::test]
    async fn pow_projects_an_exponent_outside_the_field() {
        for code in [
            "8 1/3 POW",
            "2 1/3 POW",
            "27/8 -2/3 POW",
            "2 SQRT 1/2 POW",
            "2 2 SQRT POW",
            "1 2 SQRT POW",
            "0 2 SQRT POW",
        ] {
            assert_eq!(
                top(&format!("{code} NIL-REASON")).await,
                "'domainMiss'",
                "`{code}`"
            );
        }
        assert_eq!(
            top("[ 4 8 ] 1/3 POW 1 GET NIL-REASON").await,
            "'domainMiss'"
        );
    }

    #[tokio::test]
    async fn pow_projects_what_has_no_value() {
        assert_eq!(top("0 -1 POW NIL-REASON").await, "'divisionByZero'");
        assert_eq!(top("0 -1/2 POW NIL-REASON").await, "'divisionByZero'");
        assert_eq!(top("-8 1/3 POW NIL-REASON").await, "'domainMiss'");
        assert_eq!(top("-2 1/2 POW NIL-REASON").await, "'domainMiss'");
        assert_eq!(top("-2 SQRT 3/2 POW NIL-REASON").await, "'domainMiss'");
        assert_eq!(top("2 1000000000 POW NIL-REASON").await, "'spaceExhausted'");
        assert_eq!(top("NIL 2 POW").await, "NIL");
        assert_eq!(error_of("'x' 2 POW").await, "nonNumeric");
        assert_eq!(error_of("[ 1 2 ] [ 1 2 3 ] POW").await, "shapeMismatch");
        assert_eq!(
            top("2 'A' BIND 3 'B' BIND A B A B POW").await,
            "2/1 3/1 8/1"
        );
    }

    #[tokio::test]
    async fn gcd_and_ratio_read_the_rationals() {
        assert_eq!(top("12 18 GCD").await, "6/1");
        assert_eq!(top("-12 18 GCD").await, "6/1");
        assert_eq!(top("0 0 GCD").await, "0/1");
        assert_eq!(top("7 0 GCD").await, "7/1");
        assert_eq!(top("[ 12 9 ] 6 GCD").await, "[ 6/1 3/1 ]");
        assert_eq!(top("1/2 4 GCD NIL-REASON").await, "'domainMiss'");
        assert_eq!(top("2 SQRT 4 GCD NIL-REASON").await, "'domainMiss'");
        assert_eq!(error_of("'a' 4 GCD").await, "nonNumeric");
        assert_eq!(top("6/4 RATIO").await, "[ 3/1 2/1 ]");
        assert_eq!(top("-3 RATIO").await, "[ -3/1 1/1 ]");
        assert_eq!(top("0 RATIO").await, "[ 0/1 1/1 ]");
        assert_eq!(
            top("[ 1/2 3/4 ] RATIO").await,
            "[ [ 1/1 2/1 ] [ 3/1 4/1 ] ]"
        );
        assert_eq!(top("2 SQRT RATIO NIL-REASON").await, "'domainMiss'");
        assert_eq!(error_of("'a' RATIO").await, "nonNumeric");
        // RATIO then DIV is the identity on a rational.
        assert_eq!(
            top("6/4 RATIO 0 GET 6/4 RATIO 1 GET DIV 3/2 EQ").await,
            "TRUE"
        );
    }

    #[tokio::test]
    async fn the_numeric_words_lift_over_records() {
        assert_eq!(
            top("[ 'a' 'b' ] [ 2 3 ] RECORD 2 POW").await,
            "{ 'a' 4/1 'b' 9/1 }"
        );
        assert_eq!(
            top("[ 'a' 'b' ] [ 12 9 ] RECORD 6 GCD").await,
            "{ 'a' 6/1 'b' 3/1 }"
        );
        assert_eq!(
            top("[ 'a' ] [ 1/2 ] RECORD RATIO").await,
            "{ 'a' [ 1/1 2/1 ] }"
        );
        assert_eq!(top("[ 'a' ] [ 4 ] RECORD 1/2 POW").await, "{ 'a' 2/1 }");
    }
}
