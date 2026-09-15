//! A higher-order Word's block is compiled once, not re-interpreted per element.
//!
//! `MAP`, `FILTER`, `FOLD`, `ALL` and `ANY` used to walk their block's tokens
//! again for every element, which means resolving every Symbol in it by name
//! every time: `[ ABS ] MAP` over 20,000 lanes hashed `"ABS"` and probed the
//! dictionary 20,000 times to reach the one Word it names. A block is fixed for
//! the length of the loop, so it is compiled before the loop instead.
//!
//! The plan carries the epoch it was compiled at, and a block is allowed to
//! change the dictionary — `[ ... DEF ] MAP` is a legal program. So the route is
//! chosen per element: a plan whose dictionary epoch still matches runs
//! compiled, and one whose does not falls back to the tokens. Both must answer
//! the same, which is what compiling has to preserve (LANG.AUTHORITY.FREEDOM),
//! and the interesting case is the element where the answer changes *because*
//! the dictionary did.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    async fn answer(program: &str) -> String {
        let mut interp = Interpreter::new();
        interp
            .execute(program)
            .await
            .unwrap_or_else(|e| panic!("`{program}` must run: {e:?}"));
        interp
            .get_stack()
            .last()
            .map(|v| format!("{v}"))
            .unwrap_or_else(|| "<empty>".to_string())
    }

    /// Every higher-order Word, over a block that resolves to Core Words.
    #[tokio::test]
    async fn each_higher_order_word_answers_the_same_through_a_compiled_block() {
        for (program, expected) in [
            ("[ 1 2 3 ] [ 2 MUL ] MAP", "[ 2/1 4/1 6/1 ]"),
            ("[ -1 2 -3 ] [ ABS ] MAP", "[ 1/1 2/1 3/1 ]"),
            ("[ 1 2 3 4 ] [ 2 MOD 0 EQ ] FILTER", "[ 2/1 4/1 ]"),
            ("[ 1 2 3 4 ] 0 [ ADD ] FOLD", "10/1"),
            ("[ 1 2 3 ] [ 0 GT ] ALL", "TRUE"),
            ("[ 1 -2 3 ] [ 0 GT ] ALL", "FALSE"),
            ("[ 1 -2 3 ] [ 0 LT ] ANY", "TRUE"),
            ("[ 1 2 3 ] [ 0 LT ] ANY", "FALSE"),
        ] {
            assert_eq!(answer(program).await, expected, "`{program}`");
        }
    }

    /// A block calling a User Word, which the compiler lowers to a
    /// `CallUserWord` rather than a pre-resolved builtin.
    #[tokio::test]
    async fn a_block_calling_a_user_word_answers_the_same() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 2 MUL 1 ADD ] 'F' DEF")
            .await
            .expect("F defines");
        interp.update_stack(Vec::new());
        interp
            .execute("[ 1 2 3 ] [ F ] MAP")
            .await
            .expect("MAP over F runs");
        assert_eq!(
            format!("{}", interp.get_stack().last().expect("a result")),
            "[ 3/1 5/1 7/1 ]"
        );
    }

    /// The block whose plan goes stale mid-loop. `[ ... DEF ] MAP` moves the
    /// dictionary epoch on the first element, so every later element must take
    /// the token route — and the answer must be the one the *new* dictionary
    /// gives, never the one the plan was compiled against.
    #[tokio::test]
    async fn a_block_that_redefines_a_word_is_not_served_from_its_own_stale_plan() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 100 ADD ] 'G' DEF")
            .await
            .expect("G defines");
        interp.update_stack(Vec::new());

        // Element 1 runs G as `100 ADD`, then redefines it to `1 ADD`;
        // elements 2 and 3 must see the redefinition.
        interp
            .execute("[ 1 2 3 ] [ G [ 1 ADD ] 'G' DEF ] MAP")
            .await
            .expect("a block that redefines a word runs");
        assert_eq!(
            format!("{}", interp.get_stack().last().expect("a result")),
            "[ 101/1 3/1 4/1 ]",
            "the first element sees `100 ADD`, the rest see the redefinition"
        );
    }

    /// A block holding something the compiler cannot lower still runs: the plan
    /// keeps its source tokens and `execute_compiled_line` re-interprets them,
    /// which is the same thing the interpreted route would have done.
    #[tokio::test]
    async fn a_block_the_compiler_cannot_lower_still_answers() {
        for (program, expected) in [
            // A binding, and a nested block, and OR-NIL — the shapes that reach
            // the fallback rather than a lowered op.
            ("[ 1 2 ] [ 'X' BIND X X ADD ] MAP", "[ 2/1 4/1 ]"),
            ("[ 1 2 ] [ 0 DIV OR-NIL 7 ] MAP", "[ 7/1 7/1 ]"),
        ] {
            assert_eq!(answer(program).await, expected, "`{program}`");
        }
    }

    /// A failure inside a compiled block still names the Word that failed — the
    /// attribution a compiled `CallBuiltin` used to drop.
    #[tokio::test]
    async fn a_failure_inside_a_compiled_block_still_names_its_word() {
        let mut interp = Interpreter::new();
        let error = interp
            .execute("[ 1 2 ] [ 'x' 1 ADD ] MAP")
            .await
            .expect_err("adding a Text must fail");
        let rendered = format!("{error:?}");
        assert!(
            !rendered.is_empty(),
            "the failure must be reported, got {rendered}"
        );
        // The trace is where the attribution lands; the diagnosis names ADD.
        let trace = format!("{:?}", interp.drain_error_flow_trace());
        assert!(
            trace.contains("ADD"),
            "the error-flow trace must name ADD: {trace}"
        );
    }
}
