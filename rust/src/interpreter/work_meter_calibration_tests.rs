//! The work meter's price has to track what an operation actually costs.
//!
//! `max_numeric_work` is a stand-in for "too long", so a unit only means
//! anything if it buys roughly the same amount of time on every path. When it
//! does not, the ceiling silently becomes a different control depending on
//! which path a program takes — which is how a 4096-digit *addition* came to
//! be charged 213 units while the same-width multiplication was charged 45,369
//! and ran in half the time.
//!
//! These are ratio assertions over charged units, never wall clocks: a timing
//! test in CI is flaky, and the absolute rates belong to one machine and one
//! `num-bigint`. `examples/work_meter_calibration` is where the rates are
//! measured; this is where the shape of the pricing is pinned so a re-measure
//! is a deliberate act rather than a surprise.

#[cfg(test)]
mod work_meter_calibration_tests {
    use crate::interpreter::Interpreter;

    async fn charged_by(source: &str) -> u64 {
        let mut interp = Interpreter::new();
        interp.execute(source).await.unwrap_or_else(|e| {
            panic!(
                "`{}...` must compute, got: {e:?}",
                &source[..20.min(source.len())]
            )
        });
        interp.numeric_work_used()
    }

    /// 4096 digits — the widest single literal `numericLiteralDigits` admits,
    /// so the widest operand a source can state outright.
    fn wide() -> String {
        "9".repeat(4096)
    }

    #[tokio::test]
    async fn a_wide_addition_costs_what_a_wide_multiplication_costs() {
        // `a/b + c/d` is `(ad + cb)/(bd)` followed by a gcd: three
        // multiplications and a Euclid, none of them linear in the wider
        // operand. Priced as linear it charged 213 against multiplication's
        // 45,369 — a 200x gap on an operation measurably *dearer* than the one
        // it was being undercut by (745 µs against 366 µs on the reference
        // container).
        let wide = wide();
        let addition = charged_by(&format!("{wide} {wide} +")).await;
        let multiplication = charged_by(&format!("{wide} {wide} *")).await;
        assert_eq!(
            addition, multiplication,
            "the same operand widths must cost the same whichever schema runs"
        );
    }

    #[tokio::test]
    async fn subtraction_and_division_are_priced_with_their_partners() {
        let wide = wide();
        let addition = charged_by(&format!("{wide} {wide} +")).await;
        for source in [format!("{wide} {wide} -"), format!("{wide} {wide} /")] {
            assert_eq!(
                charged_by(&source).await,
                addition,
                "every schema at one width is one price"
            );
        }
    }

    #[tokio::test]
    async fn width_dominates_the_price_of_an_addition() {
        // The property the linear price destroyed: a wide addition has to be
        // recognisably dearer than a narrow one, or a chain of them is bounded
        // only by the step budget — which counts words, not the size of the
        // numbers in them.
        let narrow = charged_by("2 3 +").await;
        let wide = charged_by(&format!("{} {} +", wide(), wide())).await;
        assert!(
            wide > narrow.saturating_mul(10_000),
            "a 4096-digit addition must dwarf a one-digit one, got {wide} against {narrow}"
        );
    }

    #[tokio::test]
    async fn shape_does_not_change_the_price() {
        // The same guarantee `arithmetic_meter` exists for, restated at the
        // widths this module cares about: a broadcast is `lanes` of the scalar
        // price and nothing else.
        let wide = wide();
        let scalar = charged_by(&format!("{wide} {wide} +")).await;
        let one_lane = charged_by(&format!("[ {wide} ] {wide} +")).await;
        let three_lanes = charged_by(&format!("[ {wide} {wide} {wide} ] {wide} +")).await;
        assert_eq!(
            scalar, one_lane,
            "a one-lane vector is one scalar operation"
        );
        assert_eq!(
            three_lanes,
            scalar * 3,
            "three lanes are three of them, exactly"
        );
    }

    #[tokio::test]
    async fn ordinary_arithmetic_is_still_far_below_the_ceiling() {
        // Repricing addition must not turn everyday work into a refusal. The
        // agent profile's ceiling is 10,000,000.
        for source in [
            "1 3 /",
            "0.1 0.2 +",
            "[ 0 999 ] RANGE SUM",
            "[ 1 2 3 ] [ 4 5 6 ] *",
            "2 SQRT 3 SQRT + 5 SQRT 7 SQRT + *",
        ] {
            let charged = charged_by(source).await;
            assert!(
                charged < 100_000,
                "`{source}` charged {charged} units, which is not ordinary"
            );
        }
    }
}
