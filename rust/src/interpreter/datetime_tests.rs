//! Test suite for the TIME module (`datetime` + `time_ops` + `time_calendar`).
//!
//! TIME follows the BigQuery date/time philosophy: timezone is never stored in
//! a value, only supplied at the instant <-> civil boundary as a UTC offset in
//! hours. All conversions are exact (no floating point) and host-independent.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    async fn civil(program: &str) -> Vec<(i64, i64)> {
        let mut interp = Interpreter::new();
        interp
            .execute(&format!("'time' IMPORT {}", program))
            .await
            .expect("program should succeed");
        let view = interp.stack[0]
            .as_vector_view()
            .expect("expected a civil vector");
        view.iter()
            .map(|e| {
                let f = e.as_scalar().expect("scalar field");
                (
                    f.numerator().try_into().unwrap(),
                    f.denominator().try_into().unwrap(),
                )
            })
            .collect()
    }

    async fn ints(program: &str) -> Vec<i64> {
        civil(program).await.into_iter().map(|(n, _)| n).collect()
    }

    async fn number(program: &str) -> (i64, i64) {
        let mut interp = Interpreter::new();
        interp
            .execute(&format!("'time' IMPORT {}", program))
            .await
            .expect("program should succeed");
        let f = interp.stack[0].as_scalar().expect("scalar result");
        (
            f.numerator().try_into().unwrap(),
            f.denominator().try_into().unwrap(),
        )
    }

    async fn text(program: &str) -> String {
        let mut interp = Interpreter::new();
        interp
            .execute(&format!("'time' IMPORT {}", program))
            .await
            .expect("program should succeed");
        format!("{}", interp.stack[0])
    }

    #[tokio::test]
    async fn now_rejects_stack_mode() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'time' IMPORT .. NOW").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("NOW"));
    }

    #[tokio::test]
    async fn epoch_at_utc_is_1970() {
        assert_eq!(ints("0 0 DATETIME").await, vec![1970, 1, 1, 0, 0, 0]);
    }

    #[tokio::test]
    async fn offset_shifts_wall_clock() {
        // +9 hours (Asia/Tokyo) at the epoch instant
        assert_eq!(ints("0 9 DATETIME").await, vec![1970, 1, 1, 9, 0, 0]);
        // 32400 seconds = 9 hours past the epoch, read at UTC
        assert_eq!(ints("32400 0 DATETIME").await, vec![1970, 1, 1, 9, 0, 0]);
    }

    #[tokio::test]
    async fn fractional_offset_is_exact() {
        // +5:30 (India) == 11/2 hours
        assert_eq!(ints("0 11/2 DATETIME").await, vec![1970, 1, 1, 5, 30, 0]);
    }

    #[tokio::test]
    async fn timestamp_inverts_datetime() {
        assert_eq!(number("[ 1970 1 1 9 0 0 ] 9 TIMESTAMP").await, (0, 1));
    }

    #[tokio::test]
    async fn datetime_timestamp_roundtrip() {
        assert_eq!(
            ints("[ 2024 11 25 14 30 0 ] 9 TIMESTAMP 9 DATETIME").await,
            vec![2024, 11, 25, 14, 30, 0]
        );
    }

    #[tokio::test]
    async fn subsecond_is_preserved_exactly() {
        // half a second past the epoch, at UTC
        assert_eq!(civil("1/2 0 DATETIME").await[5], (1, 2));
        // round-trips back to the exact instant
        assert_eq!(number("[ 1970 1 1 0 0 1/2 ] 0 TIMESTAMP").await, (1, 2));
    }

    #[tokio::test]
    async fn date_and_time_extraction() {
        assert_eq!(
            ints("[ 2024 11 25 14 30 5 ] DATE").await,
            vec![2024, 11, 25]
        );
        assert_eq!(ints("[ 2024 11 25 14 30 5 ] TIME").await, vec![14, 30, 5]);
    }

    #[tokio::test]
    async fn field_accessors() {
        assert_eq!(number("[ 2024 11 25 14 30 5 ] YEAR").await, (2024, 1));
        assert_eq!(number("[ 2024 11 25 14 30 5 ] MONTH").await, (11, 1));
        assert_eq!(number("[ 2024 11 25 14 30 5 ] DAY").await, (25, 1));
        assert_eq!(number("[ 2024 11 25 14 30 5 ] HOUR").await, (14, 1));
        assert_eq!(number("[ 2024 11 25 14 30 5 ] MINUTE").await, (30, 1));
        assert_eq!(number("[ 2024 11 25 14 30 5 ] SECOND").await, (5, 1));
        // HOUR on a bare [h m s] TIME value
        assert_eq!(number("[ 14 30 5 ] HOUR").await, (14, 1));
    }

    #[tokio::test]
    async fn weekday_is_iso() {
        assert_eq!(number("[ 2024 11 25 ] WEEKDAY").await, (1, 1)); // Monday
        assert_eq!(number("[ 2000 1 1 ] WEEKDAY").await, (6, 1)); // Saturday
    }

    #[tokio::test]
    async fn add_days_crosses_month_boundary() {
        assert_eq!(ints("[ 2024 1 31 ] 1 ADD-DAYS").await, vec![2024, 2, 1]);
        // preserves time-of-day on a datetime
        assert_eq!(
            ints("[ 2024 12 31 23 59 59 ] 1 ADD-DAYS").await,
            vec![2025, 1, 1, 23, 59, 59]
        );
    }

    #[tokio::test]
    async fn diff_days_counts_calendar_days() {
        assert_eq!(number("[ 2024 1 2 ] [ 2024 1 1 ] DIFF-DAYS").await, (1, 1));
        assert_eq!(number("[ 2024 3 1 ] [ 2024 2 1 ] DIFF-DAYS").await, (29, 1));
        // leap year
    }

    #[tokio::test]
    async fn format_iso() {
        assert_eq!(text("[ 2024 11 25 ] FORMAT").await, "'2024-11-25'");
        assert_eq!(
            text("[ 2024 11 25 14 30 5 ] FORMAT").await,
            "'2024-11-25T14:30:05'"
        );
    }

    #[tokio::test]
    async fn add_months_clamps_and_rolls_over() {
        assert_eq!(ints("[ 2024 1 31 ] 1 ADD-MONTHS").await, vec![2024, 2, 29]);
        assert_eq!(ints("[ 2023 1 31 ] 1 ADD-MONTHS").await, vec![2023, 2, 28]);
        assert_eq!(ints("[ 2024 12 15 ] 1 ADD-MONTHS").await, vec![2025, 1, 15]);
        // time-of-day preserved on a datetime
        assert_eq!(
            ints("[ 2024 1 31 9 30 0 ] 1 ADD-MONTHS").await,
            vec![2024, 2, 29, 9, 30, 0]
        );
    }

    #[tokio::test]
    async fn add_years_clamps_leap_day() {
        assert_eq!(ints("[ 2024 2 29 ] 1 ADD-YEARS").await, vec![2025, 2, 28]);
        assert_eq!(ints("[ 2020 2 29 ] 4 ADD-YEARS").await, vec![2024, 2, 29]);
    }

    #[tokio::test]
    async fn parse_iso_date_and_datetime() {
        assert_eq!(
            ints("'2024-11-25' PARSE-ISO").await,
            vec![2024, 11, 25, 0, 0, 0]
        );
        assert_eq!(
            ints("'2024-11-25T14:30:05' PARSE-ISO").await,
            vec![2024, 11, 25, 14, 30, 5]
        );
        // space separator is also accepted
        assert_eq!(
            ints("'2024-11-25 14:30:05' PARSE-ISO").await,
            vec![2024, 11, 25, 14, 30, 5]
        );
    }

    #[tokio::test]
    async fn parse_fractional_second_is_exact() {
        assert_eq!(civil("'1970-01-01T00:00:00.5' PARSE-ISO").await[5], (1, 2));
    }

    #[tokio::test]
    async fn parse_roundtrips_with_format() {
        assert_eq!(
            text("'2024-11-25T14:30:05' PARSE-ISO FORMAT").await,
            "'2024-11-25T14:30:05'"
        );
    }

    #[tokio::test]
    async fn parse_invalid_text_is_bubble() {
        let mut interp = Interpreter::new();
        interp
            .execute("'time' IMPORT 'not-a-date' PARSE-ISO")
            .await
            .expect("unparseable text is a Bubble, not an error");
        assert!(interp.stack[0].is_nil());
        // a fallback datetime can be supplied with VENT
        let mut interp2 = Interpreter::new();
        interp2
            .execute("'time' IMPORT [ 1970 1 1 0 0 0 ] '13-99' PARSE-ISO ^")
            .await
            .expect("should succeed");
        assert_eq!(
            interp2.stack[0]
                .as_vector_view()
                .unwrap()
                .iter()
                .map(|e| e.as_scalar().unwrap().to_i64().unwrap())
                .collect::<Vec<_>>(),
            vec![1970, 1, 1, 0, 0, 0]
        );
    }

    #[tokio::test]
    async fn parse_non_text_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'time' IMPORT 42 PARSE-ISO").await;
        assert!(result.is_err(), "PARSE of a number is malformed use");
    }

    #[tokio::test]
    async fn datetime_rejects_stack_mode() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'time' IMPORT 0 0 .. DATETIME").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("DATETIME"));
    }

    #[tokio::test]
    async fn wrong_shape_errors() {
        let mut interp = Interpreter::new();
        // a 3-element vector is not a valid datetime for TIMESTAMP (needs 6)
        let result = interp
            .execute("'time' IMPORT [ 2024 1 1 ] 0 TIMESTAMP")
            .await;
        assert!(result.is_err());
    }
}
