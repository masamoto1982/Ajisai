//! Behavioral probes for the Phase 7 numeric Words: `POW` `GCD` `RATIO`
//! `EXP` `LN` `SIN` `COS` `ATAN` — the tier each answer lands in, the
//! projections the contracts declare, and the lifting every arithmetic Word
//! shares (LANG.VALUES.EXACT, LANG.COLLECTIONS.LIFT).

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
        assert_eq!(top("8 1/3 POW").await, "2/1");
        assert_eq!(top("27/8 -2/3 POW").await, "4/9");
        assert_eq!(top("2 1/2 POW 2 SQRT EQ").await, "TRUE");
        assert_eq!(top("2 3/2 POW 2 SQRT 2 MUL EQ").await, "TRUE");
        assert_eq!(top("2 SQRT 2 POW").await, "2/1");
        assert_eq!(top("2 SQRT -2 POW").await, "1/2");
        assert_eq!(top("[ 1 2 3 ] 2 POW").await, "[ 1/1 4/1 9/1 ]");
        assert_eq!(top("2 [ 1 2 3 ] POW").await, "[ 2/1 4/1 8/1 ]");
    }

    #[tokio::test]
    async fn pow_answers_computable_reals_elsewhere() {
        assert_eq!(top("2 1/3 POW 6 FORMAT").await, "'1.259921'");
        assert_eq!(top("PI 2 POW 6 FORMAT").await, "'9.869604'");
        assert_eq!(top("PI -1 POW 6 FORMAT").await, "'0.318310'");
        assert_eq!(top("2 PI POW 6 FORMAT").await, "'8.824978'");
        assert_eq!(top("2 SQRT 2 SQRT POW 6 FORMAT").await, "'1.632527'");
        assert_eq!(top("2 1/3 POW 3 POW 2 EQ").await, "NIL");
        assert_eq!(top("2 1/3 POW 3 POW 2 LT").await, "NIL");
    }

    #[tokio::test]
    async fn pow_projects_what_has_no_value() {
        assert_eq!(top("0 -1 POW NIL-REASON").await, "NIL 'divisionByZero'");
        assert_eq!(top("0 -1/2 POW NIL-REASON").await, "NIL 'divisionByZero'");
        assert_eq!(top("-8 1/3 POW NIL-REASON").await, "NIL 'domainMiss'");
        assert_eq!(top("-2 PI POW NIL-REASON").await, "NIL 'domainMiss'");
        assert_eq!(
            top("PI PI SUB 1/3 POW NIL-REASON").await,
            "NIL 'undecidable'"
        );
        assert_eq!(
            top("2 1000000000 POW NIL-REASON").await,
            "NIL 'spaceExhausted'"
        );
        assert_eq!(top("NIL 2 POW").await, "NIL");
        assert_eq!(error_of("'x' 2 POW").await, "nonNumeric");
        assert_eq!(error_of("[ 1 2 ] [ 1 2 3 ] POW").await, "shapeMismatch");
        assert_eq!(top("2 3 KEEP POW").await, "2/1 3/1 8/1");
    }

    #[tokio::test]
    async fn gcd_and_ratio_read_the_rationals() {
        assert_eq!(top("12 18 GCD").await, "6/1");
        assert_eq!(top("-12 18 GCD").await, "6/1");
        assert_eq!(top("0 0 GCD").await, "0/1");
        assert_eq!(top("7 0 GCD").await, "7/1");
        assert_eq!(top("[ 12 9 ] 6 GCD").await, "[ 6/1 3/1 ]");
        assert_eq!(top("1/2 4 GCD NIL-REASON").await, "NIL 'domainMiss'");
        assert_eq!(top("2 SQRT 4 GCD NIL-REASON").await, "NIL 'domainMiss'");
        assert_eq!(top("PI 4 GCD NIL-REASON").await, "NIL 'undecidable'");
        assert_eq!(error_of("'a' 4 GCD").await, "nonNumeric");
        assert_eq!(top("6/4 RATIO").await, "[ 3/1 2/1 ]");
        assert_eq!(top("-3 RATIO").await, "[ -3/1 1/1 ]");
        assert_eq!(top("0 RATIO").await, "[ 0/1 1/1 ]");
        assert_eq!(
            top("[ 1/2 3/4 ] RATIO").await,
            "[ [ 1/1 2/1 ] [ 3/1 4/1 ] ]"
        );
        assert_eq!(top("2 SQRT RATIO NIL-REASON").await, "NIL 'domainMiss'");
        assert_eq!(top("PI RATIO NIL-REASON").await, "NIL 'undecidable'");
        assert_eq!(error_of("'a' RATIO").await, "nonNumeric");
        // RATIO then DIV is the identity on a rational.
        assert_eq!(
            top("6/4 RATIO 0 GET 6/4 RATIO 1 GET DIV 3/2 EQ").await,
            "TRUE"
        );
    }

    #[tokio::test]
    async fn transcendentals_answer_computable_reals_with_exact_corners() {
        assert_eq!(top("0 EXP").await, "1/1");
        assert_eq!(top("1 LN").await, "0/1");
        assert_eq!(top("0 SIN").await, "0/1");
        assert_eq!(top("0 COS").await, "1/1");
        assert_eq!(top("0 ATAN").await, "0/1");
        assert_eq!(top("1 EXP 10 FORMAT").await, "'2.7182818285'");
        assert_eq!(top("10 LN 6 FORMAT").await, "'2.302585'");
        assert_eq!(top("1 SIN 6 FORMAT").await, "'0.841471'");
        assert_eq!(top("1 COS 6 FORMAT").await, "'0.540302'");
        assert_eq!(top("1 ATAN 4 MUL 8 FORMAT").await, "'3.14159265'");
        assert_eq!(top("PI 3 DIV SIN 6 FORMAT").await, "'0.866025'");
        assert_eq!(top("PI 4 DIV COS 6 FORMAT").await, "'0.707107'");
        // sin π encloses 0 and cos π encloses −1 without ever proving
        // either, so a decimal rendering cannot settle its last digit.
        assert_eq!(
            top("PI SIN 10 FORMAT NIL-REASON").await,
            "NIL 'undecidable'"
        );
        assert_eq!(top("PI COS -1 LT").await, "NIL");
        assert_eq!(top("PI COS -1 GT").await, "NIL");
        assert_eq!(top("2 SQRT SIN 6 FORMAT").await, "'0.987766'");
        assert_eq!(top("2 EXP LN 3 LT").await, "TRUE");
        // ln(exp 1) encloses 1 without proving it: even its rendering starves.
        assert_eq!(
            top("1 EXP LN 6 FORMAT NIL-REASON").await,
            "NIL 'undecidable'"
        );
        assert_eq!(top("[ 0 1 ] EXP 0 GET").await, "1/1");
        assert_eq!(top("1 EXP 2 GT").await, "TRUE");
        assert_eq!(top("1 EXP 3 LT").await, "TRUE");
    }

    #[tokio::test]
    async fn transcendentals_say_what_they_cannot_answer() {
        assert_eq!(top("1 EXP 1 EXP EQ").await, "NIL");
        assert_eq!(top("PI SIN 0 EQ").await, "NIL");
        assert_eq!(top("0 LN NIL-REASON").await, "NIL 'domainMiss'");
        assert_eq!(top("-1 LN NIL-REASON").await, "NIL 'domainMiss'");
        assert_eq!(top("PI PI SUB LN NIL-REASON").await, "NIL 'undecidable'");
        assert_eq!(
            top("1000000000 EXP NIL-REASON").await,
            "NIL 'spaceExhausted'"
        );
        assert_eq!(
            top("100000000000000000000 SIN NIL-REASON").await,
            "NIL 'spaceExhausted'"
        );
        assert_eq!(top("NIL EXP").await, "NIL");
        assert_eq!(top("1 0 DIV LN NIL-REASON").await, "NIL 'divisionByZero'");
        for word in ["EXP", "LN", "SIN", "COS", "ATAN"] {
            assert_eq!(
                error_of(&format!("'x' {word}")).await,
                "nonNumeric",
                "{word}"
            );
            let mut interp = Interpreter::new();
            let _ = interp.execute(&format!("'x' {word}")).await;
            assert_eq!(interp.stack.len(), 1, "{word} must restore its operand");
        }
        assert_eq!(top("1 KEEP EXP 2 GT").await, "1/1 TRUE");
    }

    #[tokio::test]
    async fn the_numeric_words_lift_over_records() {
        assert_eq!(
            top("[ 'a' 'b' ] [ 2 3 ] RECORD 2 POW").await,
            "[ 'a' 'b' ] [ 4/1 9/1 ] RECORD"
        );
        assert_eq!(
            top("[ 'a' 'b' ] [ 12 9 ] RECORD 6 GCD").await,
            "[ 'a' 'b' ] [ 6/1 3/1 ] RECORD"
        );
        assert_eq!(
            top("[ 'a' ] [ 1/2 ] RECORD RATIO").await,
            "[ 'a' ] [ [ 1/1 2/1 ] ] RECORD"
        );
        assert_eq!(
            top("[ 'a' ] [ 0 ] RECORD EXP").await,
            "[ 'a' ] [ 1/1 ] RECORD"
        );
    }
}
