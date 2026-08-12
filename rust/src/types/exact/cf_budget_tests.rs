//! Writing a value down must not cost more than computing it.
//!
//! The continued fraction is a *rendering* of an algebraic value, not the value
//! (SPEC §4.2.3 calls the display budget implementation-defined, and
//! `exactTerms` is what carries the number). It used to be budgeted by term
//! count — 32 partial quotients, whatever each cost — and the cost is not flat.
//! Measured on the reference container, a multiquadratic product of 2 / 4 / 8 /
//! 16 / 32 terms took 0.1 ms to build throughout while rendering it took 5 ms /
//! 57 ms / 951 ms / 9.1 s / 147 s.
//!
//! Nothing priced that. `wallTimeMs` was the only ceiling that noticed, and it
//! answers with a timeout — which names nothing, and told an agent its program
//! was slow when what was slow was the act of showing it the answer.

#[cfg(test)]
mod cf_budget_tests {
    use crate::interpreter::Interpreter;
    use crate::types::display::format_with_hint;
    use crate::types::Interpretation;

    /// `(√p₁+√q₁)·(√p₂+√q₂)·…`, whose term count doubles per factor.
    fn algebraic_product(factors: usize) -> String {
        const PAIRS: [(u32, u32); 6] = [(2, 3), (5, 7), (11, 13), (17, 19), (23, 29), (31, 37)];
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

    async fn rendered(source: &str) -> String {
        let mut interp = Interpreter::new();
        interp
            .execute(source)
            .await
            .unwrap_or_else(|e| panic!("`{source}` must compute, got: {e:?}"));
        let stack = interp.get_stack();
        let (value, role) = stack.iter_slots().last().expect("a value on the stack");
        format_with_hint(value, role)
    }

    #[tokio::test]
    async fn an_ordinary_irrational_still_writes_its_whole_display() {
        // The budget has to be invisible where it matters most. `2 SQRT` is the
        // first irrational anyone meets, and the quickstart documents its
        // display; a work budget that shortened it would have been a worse
        // trade than the problem it fixes.
        let display = rendered("2 SQRT").await;
        assert!(
            display.starts_with("( 1 ( 2 ( 2 ( 2 "),
            "√2 must still expand to its canonical CF, got: {display}"
        );
        assert!(
            display.matches("( ").count() >= 32,
            "and to the full display budget of 32 quotients, got: {display}"
        );
    }

    #[tokio::test]
    async fn a_two_term_sum_still_writes_its_whole_display() {
        let display = rendered("2 SQRT 3 SQRT +").await;
        assert!(
            display.matches("( ").count() >= 32,
            "a two-term value is cheap to expand, got: {display}"
        );
    }

    #[tokio::test]
    async fn an_expansion_too_dear_to_afford_says_so_rather_than_paying() {
        // Sixteen terms: 9.1 seconds of rendering before the budget, and the
        // whole of it spent after the value was already computed.
        let display = rendered(&algebraic_product(4)).await;
        assert!(
            display.ends_with("...)"),
            "a cut-short expansion must carry the truncation marker, got: {display}"
        );
        assert!(
            display.matches("( ").count() < 32,
            "and must actually be shorter than the term budget, got: {display}"
        );
    }

    #[tokio::test]
    async fn an_unaffordable_expansion_renders_the_undetermined_marker() {
        // Sixty-four terms: not even `a0` is worth its price. `( ...)` is the
        // marker the display already had for a CF it could not determine.
        let display = rendered(&algebraic_product(6)).await;
        assert_eq!(
            display, "( ...)",
            "an expansion nobody can afford must say so, not guess"
        );
    }

    #[tokio::test]
    async fn a_shortened_display_never_shortens_the_value() {
        // The whole trade only works because the CF is a rendering. What the
        // budget must never touch is `exactTerms`, which *is* the value.
        let mut interp = Interpreter::new();
        interp
            .execute(&algebraic_product(6))
            .await
            .expect("a 64-term product computes");
        let stack = interp.get_stack();
        let (value, _) = stack.iter_slots().last().expect("a value");
        let terms = crate::types::value_protocol::exact_terms(value).expect("algebraic terms");
        assert_eq!(
            terms.len(),
            64,
            "every term survives however little of the CF is written"
        );
    }

    #[tokio::test]
    async fn the_budget_does_not_reach_a_rational() {
        // A rational's CF terminates and is exact — it is not a prefix and not
        // an approximation, so nothing here may touch it, and in particular it
        // must never gain a truncation marker.
        for source in ["1 2 /", "7 2 /", "355 113 /"] {
            let mut interp = Interpreter::new();
            interp.execute(source).await.expect("computes");
            let stack = interp.get_stack();
            let (value, _) = stack.iter_slots().last().expect("a value");
            let display = format_with_hint(value, Interpretation::ContinuedFraction);
            assert!(
                display.ends_with(") )") || display.ends_with(" )"),
                "`{source}` is exact and finite, got: {display}"
            );
            assert!(
                !display.contains("...)"),
                "`{source}` terminates and must not be marked truncated, got: {display}"
            );
        }
    }

    #[tokio::test]
    async fn a_cut_short_expansion_is_marked_truncated_not_complete() {
        // Truncation used to be inferred from the length reaching the term
        // budget. Under a work budget a short prefix is still a prefix, and
        // rendering it as a finished CF would state a false value.
        let display = rendered(&algebraic_product(3)).await;
        assert!(
            display.contains("...)"),
            "an irrational's expansion never terminates, got: {display}"
        );
        assert!(
            display.matches("( ").count() < 32,
            "and this one was cut short by the budget, not by the term count, got: {display}"
        );
    }

    #[tokio::test]
    async fn the_role_hinted_renderer_agrees_with_the_default_one() {
        // `format_as_continued_fraction` is a second call site of the same
        // expansion; both had to learn the same truncation rule.
        let mut interp = Interpreter::new();
        interp
            .execute(&algebraic_product(4))
            .await
            .expect("computes");
        let stack = interp.get_stack();
        let (value, _) = stack.iter_slots().last().expect("a value");
        let hinted = format_with_hint(value, Interpretation::ContinuedFraction);
        assert!(
            hinted.ends_with("...)"),
            "the CF-hinted rendering must be marked truncated too, got: {hinted}"
        );
    }
}
