//! A Word costs one execution step wherever it is written.
//!
//! The step is what a Word *costs*, not what one route happens to count, and two
//! routes dispatch Words: the interpreted loop, and a compiled plan. The
//! compiled one used to count nothing. A Core Word reached through
//! `CompiledOp::CallBuiltin` cost 0 steps instead of 1, and `TRUE`/`FALSE`/`NIL`
//! — Core Words in the registry — were lowered to a plain literal push and cost
//! 0 too.
//!
//! Two things followed, and the second is the serious one. `executionSteps`
//! under-reported a budget `docs/dev/agent-cli-output-contract.md` says an agent
//! plans against. And the ceiling could not refuse a program: because a User
//! Word's body compiles, **work moved inside a User Word escaped the step
//! ceiling entirely** — 160 builtins written inline were refused at a 50-step
//! ceiling while the same 160 inside twenty calls of a Word ran to completion.
//!
//! Inline is the oracle here: a program of N word dispatches costs N steps, and
//! wrapping those same dispatches in a Word adds one step per call and nothing
//! else. Anything that makes a route cheaper than the oracle is the hole coming
//! back.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    async fn steps(setup: &str, program: &str) -> u64 {
        let mut interp = Interpreter::new();
        interp.set_max_execution_steps(1_000_000_000);
        if !setup.is_empty() {
            interp
                .execute(setup)
                .await
                .unwrap_or_else(|e| panic!("setup `{setup}` must run: {e:?}"));
            interp.update_stack(Vec::new());
        }
        interp
            .execute(program)
            .await
            .unwrap_or_else(|e| panic!("`{program}` must run: {e:?}"));
        interp.resource_usage().execution_steps
    }

    /// The same dispatches, written three ways, cost the same.
    #[tokio::test]
    async fn a_word_costs_a_step_inline_in_a_body_and_in_a_block() {
        // `body` holds `per_call` word dispatches; five copies either way.
        for (body, per_call, seed) in [
            ("1 ADD 1 ADD 1 ADD 1 ADD", 4, "0 "),
            ("2 MUL 3 ADD", 2, "1 "),
            ("1 ADD", 1, "0 "),
        ] {
            let inline = steps("", &format!("{seed}{}", format!("{body} ").repeat(5))).await;
            assert_eq!(
                inline,
                per_call * 5,
                "inline is the oracle: `{body}` x5 is {} dispatches",
                per_call * 5
            );

            let via_body = steps(
                &format!("[ {body} ] 'W' DEF"),
                &format!("{seed}{}", "W ".repeat(5)),
            )
            .await;
            assert_eq!(
                via_body,
                inline + 5,
                "`{body}` inside a Word costs the same plus the five calls"
            );

            // `[ 0 4 ] RANGE` is two dispatches of its own, then `MAP` is a third.
            let via_block = steps("", &format!("[ 0 4 ] RANGE [ {body} ] MAP")).await;
            assert_eq!(
                via_block,
                inline + 2,
                "`{body}` inside a block costs the same plus RANGE and MAP"
            );
        }
    }

    /// `TRUE`, `FALSE` and `NIL` are Core Words, so they cost a step — including
    /// on the compiled route, which lowers them to a literal push and would
    /// otherwise make them free.
    #[tokio::test]
    async fn a_word_that_compiles_to_a_literal_still_costs_a_step() {
        for name in ["TRUE", "FALSE", "NIL"] {
            let inline = steps("", &format!("{name} ").repeat(10)).await;
            assert_eq!(inline, 10, "ten `{name}` inline");

            let via_body =
                steps(&format!("[ {name} {name} ] 'PAIR' DEF"), &"PAIR ".repeat(5)).await;
            assert_eq!(
                via_body, 15,
                "five calls of a Word holding two `{name}` is 10 + 5"
            );
        }
    }

    /// The ceiling has to be able to refuse, wherever the work is written. This
    /// is the hole itself: the middle case ran to completion.
    #[tokio::test]
    async fn the_step_ceiling_refuses_work_wherever_it_is_written() {
        let body = "1 ADD 1 ADD 1 ADD 1 ADD 1 ADD 1 ADD 1 ADD 1 ADD";
        for (label, setup, program) in [
            (
                "inline",
                String::new(),
                format!("0 {}", format!("{body} ").repeat(20)),
            ),
            (
                "inside a Word body",
                format!("[ {body} ] 'BUMP' DEF"),
                format!("0 {}", "BUMP ".repeat(20)),
            ),
            (
                "inside a block",
                String::new(),
                format!("[ 0 19 ] RANGE [ {body} ] MAP"),
            ),
        ] {
            let mut interp = Interpreter::new();
            interp.set_max_execution_steps(50);
            if !setup.is_empty() {
                interp.execute(&setup).await.expect("setup runs");
                interp.update_stack(Vec::new());
            }
            let error = interp.execute(&program).await.expect_err(&format!(
                "160 dispatches {label} must exceed a 50-step ceiling"
            ));
            assert!(
                format!("{error:?}").contains("ExecutionLimitExceeded"),
                "{label}: expected the step ceiling to refuse, got {error:?}"
            );
        }
    }

    /// The ceiling firing on a Word *is* that Word failing, so it is recorded
    /// like any other failure — on either route.
    ///
    /// This is the case the compiled route got wrong twice over. It first did
    /// not charge at all; then it charged with `?`, which let the ceiling's own
    /// refusal escape before the failure record, so the trace named nothing
    /// where the interpreted route names the Word. The interpreted route makes
    /// its charge *inside* the dispatch, so its refusal lands in the same error
    /// arm as any other, and that is the behaviour to match.
    #[tokio::test]
    async fn a_refusal_by_the_ceiling_names_the_word_it_fired_on() {
        // Inline, and inside a Word body whose compiled plan is the route that
        // dropped the name.
        for (setup, program) in [
            (String::new(), format!("0 {}", "1 ADD ".repeat(40))),
            (
                "[ 1 ADD 1 ADD 1 ADD 1 ADD ] 'BUMP' DEF".to_string(),
                format!("0 {}", "BUMP ".repeat(10)),
            ),
        ] {
            let mut interp = Interpreter::new();
            interp.set_max_execution_steps(12);
            if !setup.is_empty() {
                interp.execute(&setup).await.expect("setup runs");
                interp.update_stack(Vec::new());
            }
            let error = interp
                .execute(&program)
                .await
                .expect_err("the ceiling must refuse");
            assert!(
                format!("{error:?}").contains("ExecutionLimitExceeded"),
                "expected the ceiling, got {error:?}"
            );
            let trace = interp.drain_error_flow_trace();
            assert!(
                trace.iter().any(|e| e.word.as_deref() == Some("ADD")),
                "the refusal must name ADD; trace was {trace:?}"
            );
        }
    }
}
