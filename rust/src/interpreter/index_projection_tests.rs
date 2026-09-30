//! Past-the-end is one condition with one answer (LANG.FAILURE.PROJECT).
//!
//! `GET`, `TAKE`, `DROP` and `PUT` all address a position in a Vector, and all four
//! can be handed a well-formed operand that names a position the Vector does
//! not have. That is the trichotomy's middle case — data that did not work
//! out, not a program that is wrong — so all three project NIL with the
//! reason `indexOutOfBounds`.
//!
//! They did not always. `GET` projected while `TAKE` and `PUT` raised, which
//! made `indexOutOfBounds` the one condition the registry answered two ways
//! and left a caller unable to ask "did this land?" without guarding the call
//! first. The probes live together here, rather than beside each Word,
//! because what they check is the agreement rather than any one Word's
//! behavior: a future change that splits them again fails here.

use crate::interpreter::Interpreter;
use crate::types::Value;

/// Run `code` and answer the value it leaves on top; failing to run is the
/// probe's own bug.
async fn top_of(code: &str) -> Value {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
    interp.stack.last().cloned().expect("an answer was pushed")
}

/// The reason of the NIL `code` projects, or `None` if it carries none.
async fn projected_reason(code: &str) -> Option<String> {
    let answer = top_of(code).await;
    assert!(answer.is_nil(), "`{code}` must project NIL, got {answer:?}");
    answer
        .absence_metadata()
        .and_then(|absence| absence.reason.as_ref())
        .map(|reason| reason.as_protocol_str().to_string())
}

async fn raises(code: &str) -> bool {
    Interpreter::new().execute(code).await.is_err()
}

/// `GET`, `TAKE`, `DROP` and `PUT` all project a position past the end, and all
/// four name it `indexOutOfBounds`.
///
/// `TAKE` and `PUT` used to raise instead, which made this the one condition
/// the registry answered two ways and left a caller unable to ask "did this
/// land?" without guarding the call first. LANG.FAILURE.PROJECT reserves NIL
/// for well-formed data that did not work out, and an index or count that is
/// simply too large is exactly that.
#[tokio::test]
async fn past_the_end_projects_the_same_reason_from_every_addressing_word() {
    for code in [
        "[ 1 2 3 ] 5 GET",
        "[ 1 2 3 ] -5 GET",
        "[ 1 2 3 ] 5 TAKE",
        "[ 1 2 3 ] -5 TAKE",
        "[ ] 1 TAKE",
        "[ 1 2 3 ] 5 DROP",
        "[ 1 2 3 ] -5 DROP",
        "[ ] 1 DROP",
        "[ 1 2 3 ] 9 5 PUT",
        "[ 1 2 3 ] -9 5 PUT",
    ] {
        assert_eq!(
            projected_reason(code).await.as_deref(),
            Some("indexOutOfBounds"),
            "`{code}` must project past-the-end"
        );
    }
}

/// In range, each Word still answers with the value it always did: projecting
/// the miss changed nothing about the hit.
#[tokio::test]
async fn an_address_that_lands_is_untouched() {
    for (code, want) in [
        ("[ 1 2 3 ] 2 GET", "3/1"),
        ("[ 1 2 3 ] 2 TAKE", "[ 1/1 2/1 ]"),
        ("[ 1 2 3 ] -2 TAKE", "[ 2/1 3/1 ]"),
        ("[ 1 2 3 ] 2 DROP", "[ 3/1 ]"),
        ("[ 1 2 3 ] -2 DROP", "[ 1/1 ]"),
        ("[ 1 2 3 ] 3 DROP", "[ ]"),
        ("[ 1 2 3 ] 0 DROP", "[ 1/1 2/1 3/1 ]"),
        ("[ 1 2 3 ] -1 9 PUT", "[ 1/1 2/1 9/1 ]"),
    ] {
        assert_eq!(format!("{}", top_of(code).await), want, "`{code}`");
    }
}

/// Nothing a caller wanted preserved is lost by projecting. `PUT` answers
/// with the whole Vector, which was the standing argument for raising on a
/// miss — but the Vector is one the caller wrote, so `BIND`, `NIL?` and
/// `SELECT` hand it back in one phrase and the absence stays inspectable until
/// they do.
#[tokio::test]
async fn a_projected_address_is_recovered_in_one_phrase() {
    assert_eq!(
        format!(
            "{}",
            top_of("[ 1 2 3 ] 9 5 PUT 'S' BIND [ 1 2 3 ] S S NIL? SELECT").await
        ),
        "[ 1/1 2/1 3/1 ]"
    );
    assert_eq!(
        format!(
            "{}",
            top_of("[ 1 2 3 ] 5 TAKE 'S' BIND [ 1 2 3 ] S S NIL? SELECT").await
        ),
        "[ 1/1 2/1 3/1 ]"
    );
}

/// The other half of the same rule: an operand that is not an address at all
/// is the program being wrong, and stays an ERROR.
#[tokio::test]
async fn an_operand_that_is_not_an_address_still_raises() {
    for code in [
        "[ 1 2 3 ] [ 'x' ] TAKE",
        "[ 1 2 3 ] [ 'x' ] DROP",
        "[ 1 2 3 ] 'x' 5 PUT",
        "1 [ 1 2 ] GET",
    ] {
        assert!(raises(code).await, "`{code}` must raise, not project");
    }
}
