//! `check` resolves names without running anything, so it has to know which
//! names are bindings rather than Words: a `'NAME' BIND` and a parameter
//! header (`[ A B | … ]`, LANG.SOURCE.FRAME). Both used to be reported as
//! unknown Words, which made `check` refuse programs that run.

#[test]
fn bound_names_and_header_parameters_are_not_unknown_words() {
    for source in [
        "[ 'V' BIND V V LENGTH / ] 'MEAN' DEF",
        "1 'Q' BIND Q",
        "[ X | X [ 1 ] + ] 'INC' DEF",
        "[ A B | A B - ] 'DIFF' DEF 10 3 DIFF",
        "[ | 42 ] 'K' DEF K",
    ] {
        let response = super::api::check(source, false).to_json();
        assert_eq!(response["status"], "ok", "`{source}`: {response}");
    }
}

#[test]
fn a_name_the_header_does_not_declare_is_still_unknown() {
    let response = super::api::check("[ X | Y ] 'M' DEF", false).to_json();
    assert_eq!(response["status"], "error");
    assert!(
        response["message"].as_str().unwrap_or("").contains('Y'),
        "{response}"
    );
}
