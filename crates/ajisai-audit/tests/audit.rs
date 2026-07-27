//! `ajisai-audit` on its own, and the boundary it sits behind.

use ajisai_core::Interpreter;

/// Nothing in this package touches the language. An interpreter that has never
/// heard of it behaves identically, and there is no audit word to call.
#[test]
fn core_is_complete_without_audit() {
    let mut interpreter = Interpreter::new();
    for name in ["DIGEST", "RECEIPT", "ATTEST", "LOCKFILE", "VERIFY", "AUDIT"] {
        assert!(interpreter.word(name).is_none(), "{name} leaked into Core");
        assert!(interpreter.execute(name).is_err(), "{name} evaluates");
    }
    // Running a program through the audit package and running it directly give
    // the same result, because there is only one interpreter and one path.
    let receipt = ajisai_audit::run_with_receipt("1 2 ADD").unwrap();
    let mut direct = Interpreter::new();
    direct.execute("1 2 ADD").unwrap();
    assert_eq!(receipt.flow.unwrap(), ajisai_core::render_stack(&direct));
}

/// A digest addresses the canonical form, so spelling, spacing, and comments
/// do not change it — and the symbol notation is not a different program.
#[test]
fn the_digest_is_over_canonical_form() {
    let plain = ajisai_audit::digest("1 2 ADD").unwrap();
    for equivalent in [
        "1 2 +",
        "  1   2   ADD  ",
        "1 2 ADD # a trailing comment",
        "1\n2\nadd",
    ] {
        assert_eq!(
            ajisai_audit::digest(equivalent).unwrap().hex,
            plain.hex,
            "`{equivalent}` should address the same content"
        );
    }
    assert_eq!(plain.canonical, "1 2 ADD");
}

/// Different programs address different content.
#[test]
fn different_programs_differ() {
    let a = ajisai_audit::digest("1 2 ADD").unwrap();
    let b = ajisai_audit::digest("2 1 ADD").unwrap();
    let c = ajisai_audit::digest("1 2 SUB").unwrap();
    assert_ne!(a.hex, b.hex);
    assert_ne!(a.hex, c.hex);
    assert_eq!(a.hex.len(), 64);
}

/// A digest is not a word's identity. Two differently addressed programs can
/// define words that are the same word, and the language does not consult a
/// digest to decide anything.
#[test]
fn a_digest_is_not_a_words_identity() {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute("{ 2 MUL } \"DOUBLE\" DEF 21 DOUBLE")
        .unwrap();
    let mut symbolic = Interpreter::new();
    symbolic
        .execute("{ 2 * } \"DOUBLE\" DEF 21 DOUBLE")
        .unwrap();
    assert_eq!(
        ajisai_core::render_stack(&interpreter),
        ajisai_core::render_stack(&symbolic)
    );
}

/// A receipt records an outcome; it is not part of one.
#[test]
fn a_receipt_records_both_outcomes() {
    let ok = ajisai_audit::run_with_receipt("1 2 ADD").unwrap();
    assert!(ok.succeeded());
    assert_eq!(ok.flow.unwrap(), vec!["3"]);

    let failed = ajisai_audit::run_with_receipt("1 0 DIV").unwrap();
    assert!(!failed.succeeded());
    assert!(failed.error.unwrap().contains("division by zero"));
    // Even a failed run addresses content: the program is what it is.
    assert_eq!(failed.digest.canonical, "1 0 DIV");
}

#[test]
fn verification_compares_content() {
    let recorded = ajisai_audit::digest("1 2 ADD").unwrap().hex;
    assert!(ajisai_audit::verify("1 2 +", &recorded).unwrap());
    assert!(!ajisai_audit::verify("1 3 +", &recorded).unwrap());
}

/// A malformed program has no canonical form, so it has no content address —
/// reported as an error rather than as a digest of the raw bytes.
#[test]
fn malformed_source_has_no_content_address() {
    assert!(ajisai_audit::digest("[ 1 2").is_err());
}

/// The construction is pinned, so a change to how digests are computed cannot
/// silently look like a match.
#[test]
fn the_construction_is_pinned() {
    assert_eq!(ajisai_audit::DIGEST_DOMAIN, "ajisai-audit/program-digest/1");
    // A fixed vector: if the framing changes without the domain changing, this
    // fails.
    assert_eq!(
        ajisai_audit::digest("1 2 ADD").unwrap().hex,
        ajisai_audit::sha256::hex(b"ajisai-audit/program-digest/1\n7\n1 2 ADD")
    );
}
