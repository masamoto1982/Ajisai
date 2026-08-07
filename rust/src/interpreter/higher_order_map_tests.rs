//! Test suite for `MAP`'s result contract: the block's one result *is* the
//! mapped element, whatever shape it has.
//!
//! `MAP` used to unwrap a one-element Vector result into its element, a
//! leftover from the time a scalar and a one-element Vector were the same
//! thing. With the domains disjoint (LANG.VALUES.DISJOINT) that unwrapping
//! silently changed the answer and left no way to map onto singletons at all,
//! so these cases pin the shape rather than only the numbers.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::display::render_stack;

    async fn run(source: &str) -> String {
        let mut interp = Interpreter::new();
        interp
            .execute(source)
            .await
            .unwrap_or_else(|e| panic!("{} should run: {:?}", source, e));
        render_stack(interp.get_stack()).join(" ")
    }

    #[tokio::test]
    async fn map_keeps_a_one_element_vector_result_whole() {
        assert_eq!(
            run("[ 1 2 ] { 1 COLLECT } MAP").await,
            "[ [ 1/1 ] [ 2/1 ] ]"
        );
    }

    #[tokio::test]
    async fn map_keeps_a_nested_one_element_vector_whole() {
        assert_eq!(
            run("[ [ 1 ] [ 2 3 ] ] { REVERSE } MAP").await,
            "[ [ 1/1 ] [ 3/1 2/1 ] ]"
        );
    }

    #[tokio::test]
    async fn a_mapped_singleton_stays_a_singleton_downstream() {
        // The failure this rules out was silent rather than loud: the element
        // read back as a scalar, so `5 ADD` answered `6/1` where `[ 6/1 ]` is
        // the answer and `[ 6 ] 6 EQ` is FALSE.
        assert_eq!(
            run("[ [ 1 ] [ 2 3 ] ] { REVERSE } MAP [ 0 ] GET 5 ADD").await,
            "[ 6/1 ]"
        );
    }

    #[tokio::test]
    async fn map_still_collects_scalar_results_as_scalars() {
        assert_eq!(run("[ 1 2 3 ] { 2 MUL } MAP").await, "[ 2/1 4/1 6/1 ]");
    }

    #[tokio::test]
    async fn map_by_word_name_follows_the_same_rule() {
        assert_eq!(
            run("{ 1 COLLECT } 'WRAP' DEF [ 1 2 ] 'WRAP' MAP").await,
            "[ [ 1/1 ] [ 2/1 ] ]"
        );
    }
}
