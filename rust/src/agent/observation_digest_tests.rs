//! Step 1.4 of `docs/dev/competitive-advantage-work-order-2026-08.md`: the
//! ten required tests for the observation digest. Test 1 decides the Phase —
//! if it fails, the byte grammar is wrong and must be fixed, not loosened.

#[cfg(test)]
mod observation_digest_tests {
    use crate::agent::api::{compute, ComputeOptions};
    use crate::agent::observation_digest::{observation_digest, ObservationDigestInput};
    use crate::error::NilReason;
    use crate::semantic::Recoverability;
    use crate::types::exact::ExactReal;
    use crate::types::fraction::Fraction;
    use crate::types::{Interpretation, Value};
    use num_bigint::BigInt;
    use std::collections::HashSet;

    fn digest_of(value: &Value) -> Option<String> {
        let stack = [value.clone()];
        observation_digest(ObservationDigestInput {
            status: "ok",
            stack: &stack,
            output: &[],
            user_words: &[],
            error_category: None,
        })
    }

    fn frac(n: i64, d: i64) -> Fraction {
        Fraction::new(BigInt::from(n), BigInt::from(d))
    }

    fn sqrt_of(n: i64) -> Value {
        Value::from_exact_real(
            ExactReal::from_sqrt_rational(frac(n, 1)).expect("a non-negative radicand"),
        )
    }

    fn as_exact(value: &Value) -> ExactReal {
        match &value.data {
            crate::types::ValueData::ExactScalar(e) => e.clone(),
            crate::types::ValueData::Scalar(f) => ExactReal::from_fraction(f.clone()),
            other => panic!("not an exact scalar: {other:?}"),
        }
    }

    fn exact_mul(left: &Value, right: &Value) -> Value {
        let (a, b) = (as_exact(left), as_exact(right));
        Value::from_exact_real(a.mul(&b))
    }

    fn exact_add(left: &Value, right: &Value) -> Value {
        let (a, b) = (as_exact(left), as_exact(right));
        Value::from_exact_real(a.add(&b))
    }

    async fn agent_json(source: &str) -> serde_json::Value {
        compute(source, ComputeOptions::default()).await.to_json()
    }

    async fn digest_field(source: &str) -> serde_json::Value {
        agent_json(source).await["observationDigest"].clone()
    }

    /// The Phase's central invariant: `a == b` must imply `encode(a) ==
    /// encode(b)`, over a corpus mixing every relation the digest has to
    /// respect (algebraic rebasing, Vector/Tensor cross-equality, Fraction
    /// reduction) with the ones it must not collapse (NIL reasons, domain
    /// disjointness).
    #[test]
    fn equal_values_digest_equally() {
        let corpus: Vec<Value> = vec![
            Value::from_int(0),
            Value::from_int(1),
            Value::from_int(-1),
            Value::from_fraction(frac(1, 2)),
            Value::from_fraction(Fraction::create_unreduced(BigInt::from(2), BigInt::from(4))),
            Value::from_fraction(frac(-3, 7)),
            Value::from_bool(true),
            Value::from_bool(false),
            Value::from_string(""),
            Value::from_string("A"),
            Value::nil_with_reason_and_recoverability(
                NilReason::DivisionByZero,
                Recoverability::Unknown,
            ),
            Value::nil_with_reason_and_recoverability(
                NilReason::MissingField,
                Recoverability::Unknown,
            ),
            sqrt_of(2),
            sqrt_of(3),
            sqrt_of(12),
            exact_mul(&Value::from_int(2), &sqrt_of(3)),
            Value::from_vector(Vec::new()),
            Value::from_vector(vec![Value::from_int(65)]),
            Value::from_vector(vec![
                Value::from_int(1),
                Value::from_int(2),
                Value::from_int(3),
            ]),
            Value::from_int_tensor(vec![1, 2, 3]),
            Value::from_int_tensor(vec![1, 2, 4]),
            Value::from_vector(vec![
                Value::from_vector(vec![Value::from_int(1), Value::from_int(2)]),
                Value::from_vector(vec![Value::from_int(3), Value::from_int(4)]),
            ]),
            Value::from_tensor(
                vec![frac(1, 1), frac(2, 1), frac(3, 1), frac(4, 1)],
                vec![2, 2],
            ),
            Value::from_vector(vec![
                Value::from_int(1),
                Value::from_vector(vec![Value::from_int(2), Value::from_int(3)]),
            ]),
            Value::from_vector(vec![Value::from_int(1), Value::from_string("x")]),
            Value::from_vector(vec![sqrt_of(12)]),
            Value::from_vector(vec![exact_mul(&Value::from_int(2), &sqrt_of(3))]),
        ];

        for (i, left) in corpus.iter().enumerate() {
            for (j, right) in corpus.iter().enumerate() {
                if left == right {
                    assert_eq!(
                        digest_of(left),
                        digest_of(right),
                        "corpus[{i}] == corpus[{j}] but digested differently: \
                         left={left:?} right={right:?}"
                    );
                }
            }
        }
    }

    /// The regression this Phase exists to fix: `8 SQRT` keeps the coarse
    /// basis `{8}`, `2 SQRT 2 SQRT +` keeps `{3}` — equal values with
    /// disagreeing `normal_form_terms()` (pitfall A).
    #[tokio::test]
    async fn sqrt_eight_matches_sqrt_two_plus_sqrt_two() {
        let a = digest_field("8 SQRT").await;
        let b = digest_field("2 SQRT 2 SQRT +").await;
        assert!(a.is_string(), "expected a digest string, got {a}");
        assert_eq!(a, b);
    }

    /// A rectangular numeric `Vector` and the `Tensor` holding the same lanes
    /// are the same value (pitfall B); nested vs rank-2 too.
    #[test]
    fn tensor_and_nested_vector_digest_equally() {
        let boxed = Value::from_vector(vec![
            Value::from_int(1),
            Value::from_int(2),
            Value::from_int(3),
        ]);
        let dense = Value::from_int_tensor(vec![1, 2, 3]);
        assert_eq!(digest_of(&boxed), digest_of(&dense));

        let nested = Value::from_vector(vec![
            Value::from_vector(vec![Value::from_int(1), Value::from_int(2)]),
            Value::from_vector(vec![Value::from_int(3), Value::from_int(4)]),
        ]);
        let rank2 = Value::from_tensor(
            vec![frac(1, 1), frac(2, 1), frac(3, 1), frac(4, 1)],
            vec![2, 2],
        );
        assert_eq!(digest_of(&nested), digest_of(&rank2));
    }

    /// `hint` is presentation, not meaning (pitfall C): two values that only
    /// differ in `hint` must digest the same.
    #[test]
    fn hint_does_not_change_the_digest() {
        let mut raw = Value::from_int(1);
        raw.hint = Interpretation::RawNumber;
        let mut unassigned = raw.clone();
        unassigned.hint = Interpretation::Unassigned;
        assert_ne!(raw.hint, unassigned.hint);
        assert_eq!(raw, unassigned);
        assert_eq!(digest_of(&raw), digest_of(&unassigned));
    }

    /// The reverse of the previous test: the NIL reason *is* meaning
    /// (LANG.VALUES.NIL), so two differently-caused NILs must digest apart.
    #[test]
    fn nil_reasons_separate_digests() {
        let division = Value::nil_with_reason_and_recoverability(
            NilReason::DivisionByZero,
            Recoverability::Unknown,
        );
        let missing = Value::nil_with_reason_and_recoverability(
            NilReason::MissingField,
            Recoverability::Unknown,
        );
        assert_ne!(division, missing);
        assert_ne!(digest_of(&division), digest_of(&missing));
    }

    /// `create_unreduced` never calls the gcd normalizer; the digest must
    /// still land on the same bytes as the reduced form.
    #[test]
    fn unreduced_fraction_matches_reduced() {
        let reduced = Value::from_fraction(frac(1, 2));
        let unreduced =
            Value::from_fraction(Fraction::create_unreduced(BigInt::from(2), BigInt::from(4)));
        assert_eq!(digest_of(&reduced), digest_of(&unreduced));
    }

    /// Ten unmistakably different programs; all ten digests must be
    /// pairwise distinct.
    #[tokio::test]
    async fn different_results_digest_differently() {
        let programs = [
            "1",
            "2",
            "TRUE",
            "FALSE",
            "'hello'",
            "[ 1 2 3 ]",
            "0 0 DIV",
            "2 SQRT",
            "1 PRINT",
            "[ ]",
        ];
        let mut digests = Vec::new();
        for program in programs {
            digests.push(digest_field(program).await.to_string());
        }
        let distinct: HashSet<&String> = digests.iter().collect();
        assert_eq!(
            distinct.len(),
            digests.len(),
            "expected {} distinct digests, got {:?}",
            digests.len(),
            digests
        );
    }

    /// `PRINT` output is part of the observation, in order (SPEC:
    /// determinism holds for the whole run, not just the final stack).
    #[tokio::test]
    async fn output_order_is_observable() {
        let a = digest_field("1 PRINT 2 PRINT").await;
        let b = digest_field("2 PRINT 1 PRINT").await;
        assert_ne!(a, b);
    }

    /// Two runs with the same (empty) stack and output but different user
    /// dictionaries must not collapse to one digest.
    #[tokio::test]
    async fn dictionary_state_is_observable() {
        let a = digest_field("[ 1 ] 'F' DEF").await;
        let b = digest_field("[ 2 ] 'G' DEF").await;
        assert!(a.is_string(), "expected a digest string, got {a}");
        assert_ne!(a, b);
    }

    /// Purity (SPEC: 64 of 65 Words are deterministic, and `PRINT`'s effect
    /// is itself a deterministic sequence) means the same source digests the
    /// same every time.
    #[tokio::test]
    async fn digest_is_stable_across_runs() {
        let source = "[ 1 2 3 ] 1 { + } FOLD";
        let a = digest_field(source).await;
        let b = digest_field(source).await;
        assert!(a.is_string(), "expected a digest string, got {a}");
        assert_eq!(a, b);
    }

    /// Regression: keying the algebraic digest at a much larger precision
    /// than `HASH_KEY_BITS` made a 256-term value (reachable in practice —
    /// `max_algebraic_terms` allows up to 512) cost tens of milliseconds per
    /// value in a release build and hundreds in a debug one; multiplied
    /// across a stack of them, enough to time out an unrelated CI job. This
    /// pins the *budget*, not the mechanism, so a future precision change
    /// remains free as long as it stays cheap.
    #[test]
    fn many_term_algebraic_digest_stays_fast() {
        let pairs = [
            (2, 3),
            (5, 7),
            (11, 13),
            (17, 19),
            (23, 29),
            (31, 37),
            (41, 43),
            (47, 53),
        ];
        let mut factors = pairs
            .into_iter()
            .map(|(p, q)| exact_add(&sqrt_of(p), &sqrt_of(q)));
        let mut value = factors.next().expect("at least one factor");
        for factor in factors {
            value = exact_mul(&value, &factor);
        }
        let terms = match &value.data {
            crate::types::ValueData::ExactScalar(ExactReal::Algebraic(alg)) => {
                alg.normal_form_terms().len()
            }
            other => panic!("expected an algebraic scalar: {other:?}"),
        };
        assert!(terms > 100, "expected a many-term value, got {terms} terms");

        let start = std::time::Instant::now();
        let digest = digest_of(&value);
        let elapsed = start.elapsed();
        assert!(digest.is_some());
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "digesting a {terms}-term algebraic value took {elapsed:?} (debug build); \
             expected well under 2s"
        );
    }
}
