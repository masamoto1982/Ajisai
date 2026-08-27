//! The work meter is charged on the arithmetic, not on the shape it arrived in.
//!
//! The meter used to live in two routes out of the arithmetic dispatch, both
//! reachable only when both operands were scalar-shaped. `2 3 *` was priced;
//! `[ 2 ] 3 *` was free. Because that is a *shape* axis and not a code-path
//! axis, the tests here run the same arithmetic through scalars, one-element
//! vectors, N-lane vectors and irrational lanes, and require the same answer
//! from the meter each time.
//!
//! The path-invariance case at the end is a regression guard rather than a
//! fix: block, user word and `EXEC` already agreed to the unit before this
//! module existed. It is kept so a future optimization that skips the dispatch
//! entry is caught, and labelled so its green is not read as evidence for the
//! shape fix above it.

#[cfg(test)]
mod arithmetic_meter_tests {
    use crate::interpreter::runtime_limits::RuntimeLimits;
    use crate::interpreter::Interpreter;

    fn with_limits(limits: RuntimeLimits) -> Interpreter {
        let mut interp = Interpreter::new();
        interp.set_runtime_limits(limits);
        interp
    }
    // ── the meter prices the arithmetic, not the shape it arrived in ───────

    /// Work charged by a source that computes successfully under default limits.
    async fn charged_by(source: &str) -> u64 {
        let mut interp = Interpreter::new();
        interp
            .execute(source)
            .await
            .unwrap_or_else(|e| panic!("`{source}` must compute, got: {e:?}"));
        interp.numeric_work_used
    }

    /// Assert that `source` is refused by `resource` under an injected ceiling.
    macro_rules! assert_refused_by {
        ($source:expr, $limits:expr, $resource:ident, $limit:expr) => {{
            let mut interp = with_limits($limits);
            let err = interp
                .execute($source)
                .await
                .err()
                .unwrap_or_else(|| panic!("`{}` must be refused, it succeeded", $source));
            assert!(
                matches!(
                    err,
                    crate::error::AjisaiError::ResourceLimitExceeded {
                        resource: crate::error::ResourceLimit::$resource,
                        limit: $limit,
                        ..
                    }
                ),
                "`{}` must be refused by {} at {}, got: {err:?}",
                $source,
                stringify!($resource),
                $limit
            );
        }};
    }

    #[tokio::test]
    async fn multiplication_is_charged_in_every_operand_shape() {
        // The meter used to live in two places, both reachable only when both
        // operands were scalar-shaped. Every other route out of the arithmetic
        // dispatch — SIMD, sparse, rational broadcast, exact-real broadcast —
        // charged nothing, so `[ 2 ] 3 *` was free while `2 3 *` was not. A
        // control a pair of brackets switches off is not a control.
        for source in [
            "2 3 *",                   // scalar × scalar
            "[ 2 ] [ 3 ] *",           // length-1 vectors, matching wraps
            "[ 2 ] 3 *",               // vector × scalar, broadcast
            "3 [ 2 ] *",               // scalar × vector, broadcast
            "[ 2 2 2 2 2 2 2 2 ] 3 *", // N lanes, past the SIMD threshold
            "[ 2 4 6 ] [ 3 5 7 ] *",   // N lanes, pairwise
            "2 SQRT [ 3 ] *",          // an irrational lane, exact-real broadcast
        ] {
            assert!(
                charged_by(source).await > 0,
                "`{source}` reached the work meter at zero"
            );
        }
    }

    #[tokio::test]
    async fn every_schema_is_charged_in_vector_shape() {
        // Division dispatches through its own broadcast arm, and addition and
        // subtraction through a third; all four have to be on the meter or the
        // ceiling is a claim about `*` alone.
        for source in ["[ 6 ] 3 +", "[ 6 ] 3 -", "[ 6 ] 3 *", "[ 6 ] 3 /"] {
            assert!(
                charged_by(source).await > 0,
                "`{source}` reached the work meter at zero"
            );
        }
    }

    #[tokio::test]
    async fn every_operand_shape_reaches_the_same_named_ceiling() {
        // Shape decides representation. It must not decide which safety
        // control applies, or whether one applies at all.
        for source in [
            "2 3 *",
            "[ 2 ] 3 *",
            "[ 2 2 2 2 2 2 2 2 ] 3 *",
            "[ 2 4 6 ] [ 3 5 7 ] *",
            "2 SQRT [ 3 ] *",
        ] {
            assert_refused_by!(
                source,
                RuntimeLimits {
                    max_numeric_work: 0,
                    ..RuntimeLimits::default()
                },
                NumericWork,
                0
            );
        }
    }

    #[tokio::test]
    async fn a_refusal_by_shape_leaves_the_operands_on_the_stack() {
        // Charged before the operation runs and before its operands are
        // consumed, so the interpreter is still usable afterwards — the same
        // guarantee the scalar-only meter gave. The budget admits a one-lane
        // operation and refuses a three-lane one, which is the whole point:
        // lanes are what is being priced.
        let mut interp = with_limits(RuntimeLimits {
            max_numeric_work: 2,
            ..RuntimeLimits::default()
        });
        interp
            .execute("[ 2 4 6 ] [ 3 5 7 ] *")
            .await
            .expect_err("three lanes must not fit a two-unit budget");
        assert!(
            interp.execute("1 1 +").await.is_ok(),
            "a limit failure must not poison the next program"
        );
    }

    #[tokio::test]
    async fn vector_accumulator_growth_meets_the_same_ceiling_a_scalar_one_does() {
        // `[ 1 21000 ] RANGE [ 1 ] [ * ] FOLD` returned a 271,233-bit integer
        // under an agent profile declaring 262,144 — charged nothing, no
        // ceiling firing — while the identical fold with a scalar accumulator
        // was refused. The difference was one pair of brackets. Reproduced at a
        // low injected ceiling, so the test builds nothing huge.
        for source in [
            "[ 1 40 ] RANGE 1 [ * ] FOLD",
            "[ 1 40 ] RANGE [ 1 ] [ * ] FOLD",
        ] {
            assert_refused_by!(
                source,
                RuntimeLimits {
                    max_bigint_bits: 64,
                    ..RuntimeLimits::default()
                },
                BigintBits,
                64
            );
        }
    }

    #[tokio::test]
    async fn higher_order_application_charges_what_direct_source_charges() {
        // A regression guard, not a fix: these agreed *before* the shape hole
        // was closed, to the unit. The metering gap was never about `MAP` and
        // `FOLD` — the same fold with a scalar accumulator was charged all
        // along, and it was the vector-shaped one that escaped. Kept so a
        // future optimization that bypasses the dispatch entry is caught here.
        let direct = charged_by("[ 1 20 ] RANGE 1 [ * ] FOLD").await;
        let user_word = charged_by("[ * ] 'MULW' DEF [ 1 20 ] RANGE 1 'MULW' FOLD").await;
        let exec = charged_by("[ 1 20 ] RANGE 1 [ [ * ] EXEC ] FOLD").await;
        assert!(direct > 0, "a 20-step fold must reach the meter");
        assert_eq!(
            direct, user_word,
            "a fold through a user word must cost what the block costs"
        );
        assert_eq!(
            direct, exec,
            "a fold through EXEC must cost what the block costs"
        );
    }

    #[tokio::test]
    async fn summation_is_charged() {
        // `SUM` folds through `add_values`, which had no interpreter to charge
        // against — so the one Word whose whole job is to accumulate was the
        // one route that could not be metered.
        assert!(
            charged_by("[ 1 8 ] RANGE SUM").await > 0,
            "SUM reached the work meter at zero"
        );
    }

    #[tokio::test]
    async fn ordinary_vector_arithmetic_stays_far_below_the_meter() {
        // Charging lanes must not turn ordinary array work into a refusal.
        for source in [
            "[ 1 2 3 ] 10 +",
            "[ 0 999 ] RANGE 2 *",
            "[ 0 999 ] RANGE SUM",
            "[ 1 2 3 ] [ 4 5 6 ] *",
        ] {
            let mut interp = Interpreter::new();
            interp
                .execute(source)
                .await
                .expect("ordinary source computes");
            assert!(
                interp.numeric_work_used < 100_000,
                "`{source}` charged {} units, which is not ordinary",
                interp.numeric_work_used
            );
        }
    }
}
