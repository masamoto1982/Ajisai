//! `check` resolves names without running anything, so it has to know which
//! names are bindings rather than Words. A `'NAME' BIND` used to be reported
//! as an unknown Word, which made `check` refuse programs that run.

#[test]
fn bound_names_are_not_unknown_words() {
    for source in [
        "[ 'V' BIND V V LENGTH / ] 'MEAN' DEF",
        "1 'Q' BIND Q",
        "[ 2 7 ] [ 'W' 'B' ] BIND W B ADD",
    ] {
        let response = super::api::check(source, false).to_json();
        assert_eq!(response["status"], "ok", "`{source}`: {response}");
    }
}

#[test]
fn a_name_nothing_binds_is_still_unknown() {
    let response = super::api::check("[ 'V' BIND W ] 'M' DEF", false).to_json();
    assert_eq!(response["status"], "error");
    assert!(
        response["message"].as_str().unwrap_or("").contains('W'),
        "{response}"
    );
}
