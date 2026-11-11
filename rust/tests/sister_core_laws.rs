//! The shared core with the sister language Arare.
//!
//! Arare is Ajisai's counterpart (Arare's ADR-0015): it fixes the cost of a
//! value where Ajisai fixes the value, and on the Words they share the two are
//! meant to differ only in rounding (spec/CHANGELOG.md, 1.0.0-beta.4). Each
//! checks the other: Arare compares its answers with Ajisai's exact ones, and
//! this suite holds Ajisai to the programs Arare compares.
//!
//! `tests/sister/arare-shared-core.json` holds those programs — Arare's
//! corpus and the first of its seeded, generated programs — with the prelude
//! that gives Arare's QUANTIZE, RESCALE, NARROW and NEG their exact meanings,
//! and Ajisai's answer to each. A change to what the two languages share
//! fails here when it is made, not first when Arare next pins Ajisai. A deliberate change is recorded in spec/CHANGELOG.md and the
//! answers are recorded again with `node scripts/sync-sister-core.mjs`.

use ajisai_core::agent::api::{compute, ComputeOptions};

const SHARED_CORE: &str = include_str!("../../tests/sister/arare-shared-core.json");

#[tokio::test]
async fn every_shared_program_answers_what_ajisai_recorded() {
    let doc: serde_json::Value = serde_json::from_str(SHARED_CORE).expect("arare-shared-core.json");
    let prelude: Vec<&str> = doc["prelude"]
        .as_array()
        .expect("prelude")
        .iter()
        .map(|line| line.as_str().expect("a prelude line"))
        .collect();
    let prelude = prelude.join("\n");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(
        cases.len() > 1000,
        "the shared core holds only {} programs",
        cases.len()
    );
    let mut differences = Vec::new();
    for case in cases {
        let source = case["source"].as_str().expect("source");
        let report = compute(
            &format!("{prelude}\n{source}\n"),
            ComputeOptions::agent(None),
        )
        .await
        .to_json();
        let answer = if report["status"] == "ok" {
            serde_json::json!({ "stack": report["stackDisplay"] })
        } else {
            serde_json::json!({ "error": report["error"]["category"] })
        };
        let recorded = match (case.get("stack"), case.get("error")) {
            (Some(stack), _) => serde_json::json!({ "stack": stack }),
            (_, Some(error)) => serde_json::json!({ "error": error }),
            _ => panic!("`{source}` has no recorded answer"),
        };
        if answer != recorded {
            differences.push(format!("`{source}`: recorded {recorded}, now {answer}"));
        }
    }
    assert!(
        differences.is_empty(),
        "{} shared program(s) changed their answer; if the change is deliberate, record it in \
         spec/CHANGELOG.md and run `node scripts/sync-sister-core.mjs`:\n{}",
        differences.len(),
        differences.join("\n")
    );
}
