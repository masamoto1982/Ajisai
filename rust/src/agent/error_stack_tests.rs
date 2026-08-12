//! An error report must deliver its diagnosis, whatever the failing stack was
//! holding.
//!
//! The case these exist for: the work meter refuses
//! `[ 1 21000 ] RANGE 1 { * } FOLD` and names `numericWork`, but the stack at
//! that moment holds a 21,000-element vector and an 81,649-digit partial
//! product. Serialized in full that is 5.7 MB, which a host response ceiling
//! turns into "your answer was too big" — the opposite of what the program
//! actually did wrong.

#[cfg(test)]
mod error_stack_tests {
    use crate::agent::api::{compute, ComputeOptions, LOCAL_AGENT_RUNTIME_LIMITS};
    use serde_json::Value as Json;

    async fn agent_json(source: &str) -> Json {
        compute(
            source,
            ComputeOptions {
                step_limit: None,
                runtime_limits: Some(LOCAL_AGENT_RUNTIME_LIMITS),
            },
        )
        .await
        .to_json()
    }

    fn compact_len(report: &Json) -> usize {
        serde_json::to_string(report)
            .expect("a report serializes")
            .len()
    }

    /// The source whose refusal used to be unreportable.
    const RUNAWAY_FOLD: &str = "[ 1 21000 ] RANGE 1 { * } FOLD";

    #[tokio::test]
    async fn a_refusal_reports_its_resource_rather_than_its_residue() {
        let report = agent_json(RUNAWAY_FOLD).await;
        assert_eq!(report["status"], "error");
        assert_eq!(
            report["diagnosis"]["resourceLimit"]["resource"], "numericWork",
            "the ceiling that fired must survive to the wire"
        );
        assert!(
            compact_len(&report) < 64 * 1024,
            "a refusal must be deliverable; got {} bytes",
            compact_len(&report)
        );
    }

    #[tokio::test]
    async fn an_elided_slot_says_what_it_dropped() {
        let report = agent_json(RUNAWAY_FOLD).await;
        let elided = &report["stackElided"];
        assert_eq!(elided["reason"], "errorStackBudget");
        assert_eq!(
            elided["slots"][0]["index"], 0,
            "the record names which slots were dropped"
        );
        assert_eq!(
            elided["slots"][0]["elements"], 21000,
            "and how much was in them"
        );
        assert!(
            elided["slots"][0]["approxBytes"].as_u64().unwrap() > 1_000_000,
            "and roughly what they would have cost"
        );
    }

    #[tokio::test]
    async fn an_elided_slot_keeps_its_position_and_its_kind() {
        let report = agent_json(RUNAWAY_FOLD).await;
        let stack = report["stack"].as_array().expect("an array");
        assert_eq!(
            stack.len(),
            report["stackDisplay"].as_array().unwrap().len(),
            "the two views of the stack stay aligned"
        );
        // Dropping the slot outright would renumber everything above it and
        // silently move whatever a diagnosis points at.
        assert_eq!(stack[0]["type"], "vector", "the domain is still named");
        assert_eq!(stack[0]["semantics"]["shape"], "vector");
        assert!(stack[0]["value"].is_null(), "the value is what goes");
        assert_eq!(stack[0]["elided"]["reason"], "errorStackBudget");
        // The block that was applied is small, so it is still there in full:
        // the budget drops what it must, not everything.
        assert_eq!(
            report["stackDisplay"][2], "{ * }",
            "an affordable slot is untouched"
        );
    }

    #[tokio::test]
    async fn a_text_only_client_still_learns_what_was_there() {
        let report = agent_json(RUNAWAY_FOLD).await;
        let marker = report["stackDisplay"][0].as_str().expect("a string");
        assert!(
            marker.contains("elided") && marker.contains("21000"),
            "`stackDisplay` is the whole result for a text-only client, got: {marker}"
        );
    }

    #[tokio::test]
    async fn an_ordinary_error_is_not_elided_at_all() {
        // The budget must be invisible to the errors an agent actually meets.
        for source in [
            "[ 1 2 3 ] LENGHT",
            "1 0 / 2 UNKNOWNWORD",
            "[ 1 2 ] [ 1 2 3 ] +",
        ] {
            let report = agent_json(source).await;
            assert_eq!(report["status"], "error", "`{source}` must fail");
            assert!(
                report["stackElided"].is_null(),
                "`{source}` must report no elision"
            );
            for node in report["stack"].as_array().expect("an array") {
                assert!(
                    node.get("elided").is_none(),
                    "`{source}` must carry its stack in full"
                );
            }
        }
    }

    #[tokio::test]
    async fn a_successful_result_is_never_elided() {
        // A success *is* its stack. Truncating it would change the answer, so
        // an oversized success stays oversized and the host says so.
        let report = agent_json("[ 0 20000 ] RANGE").await;
        assert_eq!(report["status"], "ok");
        assert!(report["stackElided"].is_null());
        assert!(
            compact_len(&report) > 1_000_000,
            "the success path is deliberately untouched"
        );
    }
}
