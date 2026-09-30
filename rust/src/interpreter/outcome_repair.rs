//! Where an error category is repaired, read from the outcome registry
//! (`spec/outcomes.json`) rather than restated here.
//!
//! The diagnosis used to answer this with its own seven-value
//! `recoverability` scale (`fixInput`, `fixProgram`, `fixHost`, …), computed
//! from the cause class beside a registry that already declares the answer as
//! `repair: "program"` (absent: the operand is what is wrong). Two
//! classifications of one fact drift, and the one an agent could check
//! against `word_contract` was not the one it was sent. The registry is the
//! answer; this module only reads it.

use std::collections::HashSet;
use std::sync::OnceLock;

const OUTCOMES_JSON: &str = include_str!("../../../spec/outcomes.json");

fn program_repaired() -> &'static HashSet<String> {
    static IDS: OnceLock<HashSet<String>> = OnceLock::new();
    IDS.get_or_init(|| {
        let parsed: serde_json::Value =
            serde_json::from_str(OUTCOMES_JSON).expect("spec/outcomes.json must parse");
        parsed["errorCategories"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|entry| entry["repair"].as_str() == Some("program"))
            .filter_map(|entry| entry["id"].as_str().map(str::to_string))
            .collect()
    })
}

/// `Some("program")` exactly when spec/outcomes.json marks `category`
/// `repair: program`; `None` otherwise, as the registry leaves the field
/// absent — which it defines as "the operand is what is wrong".
pub(crate) fn repair_for_category(category: &str) -> Option<&'static str> {
    program_repaired().contains(category).then_some("program")
}
