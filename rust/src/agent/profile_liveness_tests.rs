//! A declared ceiling that another ceiling always pre-empts is a claim, not a
//! control.
//!
//! The three internal-cost ceilings are not independent dials. Building a large
//! *exact* value requires work — not as an implementation artifact but as what
//! exactness means, since every digit and every term was actually computed — so
//! each size ceiling has a minimum work cost to reach it:
//!
//! ```text
//! work_to_reach(size ceiling) < max_numeric_work
//! ```
//!
//! A profile that violates this declares a ceiling nothing can ever hit. Its
//! `mcp.limits` entry still refuses the program, but under the wrong name, and
//! the name is what an agent repairs from: "you exceeded the work budget" and
//! "stop multiplying surds like that" are different instructions.
//!
//! `max_algebraic_terms` was 4,096 and violated it. The doubling that would
//! first exceed 4,096 terms charges 16,799,744 units against a 10,000,000
//! budget, so `numericWork` answered every time and the term ceiling had never
//! fired in its life. That went unnoticed through three rounds of external
//! review because nothing checked it. This is what checks it.

#[cfg(test)]
mod profile_liveness_tests {
    use crate::agent::api::{compute, ComputeOptions, LOCAL_AGENT_RUNTIME_LIMITS};

    /// `(√p₁+√q₁)·(√p₂+√q₂)·…` — the term count doubles with every factor, and
    /// it is the only way to grow one.
    fn algebraic_cascade(factors: usize) -> String {
        const PAIRS: [(u32, u32); 12] = [
            (2, 3),
            (5, 7),
            (11, 13),
            (17, 19),
            (23, 29),
            (31, 37),
            (41, 43),
            (47, 53),
            (59, 61),
            (67, 71),
            (73, 79),
            (83, 89),
        ];
        let mut source = String::new();
        for (index, (left, right)) in PAIRS.iter().take(factors).enumerate() {
            source.push_str(&format!("{left} SQRT {right} SQRT +"));
            if index > 0 {
                source.push_str(" *");
            }
            source.push(' ');
        }
        source
    }

    /// Widening by repeated multiplication — the only way to grow an integer,
    /// and quadratic in the width it reaches by construction.
    fn widening_chain(multiplications: usize) -> String {
        format!(
            "{{ {} * }} 'M' DEF 1{}",
            "9".repeat(4096),
            " M".repeat(multiplications)
        )
    }

    /// Run under the declared agent profile and report which ceiling answered,
    /// and what the work meter read when it did.
    async fn refused_by(source: &str) -> (String, u64, u64) {
        let report = compute(
            source,
            ComputeOptions {
                step_limit: None,
                runtime_limits: Some(LOCAL_AGENT_RUNTIME_LIMITS),
            },
        )
        .await
        .to_json();
        let resource = report["diagnosis"]["resourceLimit"]["resource"]
            .as_str()
            .unwrap_or("<none: the program succeeded>")
            .to_string();
        let spent = report["resourceUsage"]["numericWork"].as_u64().unwrap_or(0);
        (resource, spent, LOCAL_AGENT_RUNTIME_LIMITS.max_numeric_work)
    }

    #[tokio::test]
    async fn the_algebraic_term_ceiling_can_be_the_one_that_fires() {
        // One doubling past the ceiling, on the only path that grows a term
        // count.
        let (resource, spent, budget) = refused_by(&algebraic_cascade(10)).await;
        assert_eq!(
            resource, "algebraicTerms",
            "growing past `max_algebraic_terms` must be refused by name, not by \
             whichever ceiling happens to be cheaper to reach. The work meter \
             read {spent} of {budget} at that point; if `numericWork` answered \
             instead, the term ceiling is declared past what the work budget \
             can pay for and one of the two has to move."
        );
    }

    #[tokio::test]
    async fn the_bigint_width_ceiling_can_be_the_one_that_fires() {
        let (resource, spent, budget) = refused_by(&widening_chain(20)).await;
        assert_eq!(
            resource, "bigintBits",
            "growing past `max_bigint_bits` must be refused by name. The work \
             meter read {spent} of {budget} at that point; building an N-limb \
             integer costs about N²/4 limb-operations by construction, so this \
             pair is the closest of the three and the first to break if either \
             price is re-measured."
        );
    }

    #[tokio::test]
    async fn a_value_at_each_ceiling_is_still_allowed() {
        // The other half of a live ceiling: it has to admit what it declares,
        // or it is not the number it says it is.
        for (name, source) in [
            ("algebraicTerms", algebraic_cascade(9)),
            ("bigintBits", widening_chain(19)),
        ] {
            let (resource, spent, budget) = refused_by(&source).await;
            assert_eq!(
                resource, "<none: the program succeeded>",
                "a value just inside `{name}` must be allowed, and this one was \
                 refused by `{resource}` with the work meter at {spent} of \
                 {budget}"
            );
        }
    }

    #[tokio::test]
    async fn the_work_ceiling_is_still_reachable_once_the_size_ceilings_bind() {
        // Tightening a size ceiling must not make `numericWork` unreachable in
        // turn — the three have to be orderable, not merely ordered. Repeating
        // a cascade that stays inside `algebraicTerms` accumulates work without
        // growing any single value, which is exactly what a cumulative ceiling
        // is for.
        let repeated = format!("{{ {} }} 'C' DEF{}", algebraic_cascade(8), " C".repeat(19));
        let (resource, spent, budget) = refused_by(&repeated).await;
        assert_eq!(
            resource, "numericWork",
            "cumulative work with no single value growing must reach the work \
             ceiling; got `{resource}` at {spent} of {budget}"
        );
    }

    #[tokio::test]
    async fn the_collection_ceiling_is_reachable_inside_the_materialization_one() {
        // The same liveness property, on the axis the collection meter added.
        // `max_collection_work` has to be reachable by a vector the
        // `materializedElements` ceiling admits — otherwise the collection
        // ceiling is declared past what the host will ever let a program build,
        // and the quadratic scan it exists to stop stays unnamed. The ordering
        // here runs the other way from the size ceilings above: the *work*
        // ceiling has to bind first, because a vector this size is legal and it
        // is what is done to it that is not.
        let (resource, _, _) = refused_by("[ 0 99999 ] RANGE UNIQUE").await;
        assert_eq!(
            resource, "collectionWork",
            "a quadratic scan over a vector `materializedElements` permits must \
             be refused by name; got `{resource}`"
        );
    }

    #[tokio::test]
    async fn the_materialization_ceiling_is_still_what_bounds_a_bare_vector() {
        // And the other direction: building the largest permitted vector, and
        // walking it once, must stay inside the collection budget. If a linear
        // Word over a legal vector could not run, the collection price would be
        // bounding the wrong thing.
        let (resource, _, _) = refused_by("[ 0 99999 ] RANGE REVERSE LENGTH").await;
        assert_eq!(
            resource, "<none: the program succeeded>",
            "one linear pass over the largest permitted vector must be \
             affordable; got `{resource}`"
        );
    }
}
