use super::api::predict_outcomes;

#[test]
fn malformed_source_predicts_exactly_that() {
    let response = predict_outcomes("[ 1 2").to_json();
    assert_eq!(response["status"], "ok");
    assert_eq!(response["exact"], true);
    assert_eq!(
        response["outcomes"],
        serde_json::json!(["error:malformedSource"])
    );
}

#[test]
fn an_unknown_word_predicts_exactly_that() {
    let response = predict_outcomes("FROBNICATE").to_json();
    assert_eq!(response["exact"], true);
    assert_eq!(
        response["outcomes"],
        serde_json::json!(["error:unknownWord"])
    );
}

#[test]
fn a_program_that_calls_nothing_still_carries_structural_ceilings() {
    // Even pure literals with no Word call at all can in principle hit a
    // structural ceiling (a numeric literal too long for the profile, for
    // instance — see `word_outcome_vocabulary::structural_ceiling_ids`'s
    // doc), so this is never exact; only the truly empty program is.
    let response = predict_outcomes("1 2 3").to_json();
    assert_eq!(response["exact"], false);
    let outcomes = response["outcomes"].as_array().unwrap();
    assert!(outcomes.iter().any(|v| v == "value"));
    assert!(outcomes.iter().any(|v| v == "error:resourceLimitExceeded"));
}

#[test]
fn the_empty_program_predicts_exactly_value() {
    let response = predict_outcomes("").to_json();
    assert_eq!(response["exact"], true);
    assert_eq!(response["outcomes"], serde_json::json!(["value"]));
}

#[test]
fn every_response_names_its_limit_profile() {
    let response = predict_outcomes("1 2 ADD").to_json();
    assert!(response["limitProfile"]["executionSteps"].is_u64());
    assert!(response["limitProfile"]["materializedElements"].is_u64());
}

#[test]
fn a_nontrivial_program_over_approximates_and_says_so() {
    let response = predict_outcomes("1 2 ADD").to_json();
    let outcomes: Vec<String> = response["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(outcomes.contains(&"value".to_string()));
    assert!(outcomes.contains(&"error:nonNumeric".to_string()));
    assert_eq!(response["exact"], false);
}
