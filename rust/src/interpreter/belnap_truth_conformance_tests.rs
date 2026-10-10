//! The four-valued truth domain (LANG.VALUES.TRUTH): Belnap's tables for
//! `AND`/`NOT`, `RECONCILE` as the join of the information order, and what
//! `SELECT` and `FILTER` do with BOTH. The three-valued rows are
//! `kleene_truth_conformance_tests`; this file holds the rows BOTH adds and
//! the laws that make `RECONCILE` safe to fold.

use crate::error::NilReason;
use crate::test_support::run_ok;
use crate::types::Value;

async fn one(code: &str) -> Value {
    let stack = run_ok(code).await;
    assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
    stack[0].clone()
}

/// Every row of Belnap's `AND` and `NOT`, UNKNOWN written as `NIL`.
/// `NIL BOTH AND` is FALSE: UNKNOWN and BOTH are incomparable in the truth
/// order, and their meet is FALSE.
#[tokio::test]
async fn belnap_and_not_truth_tables() {
    let values = ["TRUE", "FALSE", "BOTH", "NIL"];
    #[rustfmt::skip]
    let and_table = [
        //  TRUE     FALSE    BOTH     NIL
        ["TRUE",  "FALSE", "BOTH",  "NIL"  ], // TRUE
        ["FALSE", "FALSE", "FALSE", "FALSE"], // FALSE
        ["BOTH",  "FALSE", "BOTH",  "FALSE"], // BOTH
        ["NIL",   "FALSE", "FALSE", "NIL"  ], // NIL
    ];
    for (i, a) in values.iter().enumerate() {
        for (j, b) in values.iter().enumerate() {
            let code = format!("{a} {b} AND");
            assert_eq!(one(&code).await.to_string(), and_table[i][j], "`{code}`");
        }
    }
    for (code, expected) in [
        ("TRUE NOT", "FALSE"),
        ("FALSE NOT", "TRUE"),
        ("BOTH NOT", "BOTH"),
        ("NIL NOT", "NIL"),
    ] {
        assert_eq!(one(code).await.to_string(), expected, "`{code}`");
    }
}

/// A disjunction written from `AND` and `NOT` is Belnap's join of the truth
/// order: `NIL BOTH` OR is TRUE, the dual of their meet.
#[tokio::test]
async fn de_morgan_disjunction_is_the_truth_join() {
    for (a, b, expected) in [
        ("NIL", "BOTH", "TRUE"),
        ("BOTH", "FALSE", "BOTH"),
        ("BOTH", "TRUE", "TRUE"),
        ("NIL", "FALSE", "NIL"),
    ] {
        let code = format!("{a} NOT {b} NOT AND NOT");
        assert_eq!(one(&code).await.to_string(), expected, "`{code}`");
    }
}

/// UNKNOWN keeps its reason through the rows where it is the answer, and
/// BOTH is a Boolean, not an absence.
#[tokio::test]
async fn both_is_present_and_unknown_keeps_its_reason() {
    assert_eq!(one("BOTH NIL?").await.to_string(), "FALSE");
    assert_eq!(one("BOTH BOTH EQ").await.to_string(), "TRUE");
    assert_eq!(one("BOTH TRUE EQ").await.to_string(), "FALSE");
    let unknown = one("BOTH -1 SQRT AND NOT TRUE AND").await;
    assert_eq!(unknown.to_string(), "TRUE", "BOTH AND UNKNOWN is FALSE");
    let kept = one("TRUE -1 SQRT AND").await;
    assert_eq!(kept.nil_reason(), Some(&NilReason::DomainMiss));
}

#[tokio::test]
async fn reconcile_agrees_yields_and_conflicts() {
    for (code, expected) in [
        ("1 1 RECONCILE", "1/1"),
        ("NIL 5 RECONCILE", "5/1"),
        ("5 NIL RECONCILE", "5/1"),
        ("TRUE FALSE RECONCILE", "BOTH"),
        ("FALSE TRUE RECONCILE", "BOTH"),
        ("BOTH TRUE RECONCILE", "BOTH"),
        ("TRUE TRUE RECONCILE", "TRUE"),
        ("NIL FALSE RECONCILE", "FALSE"),
        ("[ 1 2 ] [ 1 2 ] RECONCILE", "[ 1/1 2/1 ]"),
    ] {
        assert_eq!(one(code).await.to_string(), expected, "`{code}`");
    }
    for code in [
        "'Tokyo' 'Osaka' RECONCILE",
        "1 2 RECONCILE",
        "TRUE 1 RECONCILE",
        "BOTH 1 RECONCILE",
        "[ 1 2 ] [ 1 3 ] RECONCILE",
        "1 2 RECONCILE 1 RECONCILE",
        "1 2 RECONCILE NIL RECONCILE",
        "NIL 1 2 RECONCILE RECONCILE",
    ] {
        assert_eq!(
            one(code).await.nil_reason(),
            Some(&NilReason::Conflict),
            "`{code}` is a conflict"
        );
    }
    assert_eq!(
        one("-1 SQRT NIL RECONCILE").await.nil_reason(),
        Some(&NilReason::DomainMiss),
        "two absences: the left one, reason intact"
    );
    assert_eq!(
        one("'Tokyo' 'Osaka' RECONCILE NIL-REASON")
            .await
            .to_string(),
        "'conflict'"
    );
}

/// `RECONCILE` is a semilattice up to which absence is kept: commutative,
/// associative and idempotent over a sample of every kind of operand, so a
/// FOLD of it over many sources does not depend on their order.
#[tokio::test]
async fn reconcile_is_a_semilattice() {
    let sample = [
        "NIL",
        "-1 SQRT",
        "TRUE",
        "FALSE",
        "BOTH",
        "1",
        "2",
        "'a'",
        "1 2 RECONCILE",
    ];
    let present = |v: &Value| (v.is_nil(), v.nil_reason().copied(), v.to_string());
    let absent_reason = |v: &Value| v.is_nil() && v.nil_reason() != Some(&NilReason::Conflict);
    for a in sample {
        let aa = one(&format!("{a} {a} RECONCILE")).await;
        assert_eq!(present(&aa), present(&one(a).await), "`{a}` is idempotent");
        for b in sample {
            let ab = one(&format!("{a} {b} RECONCILE")).await;
            let ba = one(&format!("{b} {a} RECONCILE")).await;
            if !(absent_reason(&ab) && absent_reason(&ba)) {
                assert_eq!(present(&ab), present(&ba), "`{a}` and `{b}` commute");
            }
            for c in sample {
                let left = one(&format!("{a} {b} RECONCILE {c} RECONCILE")).await;
                let right = one(&format!("{a} {b} {c} RECONCILE RECONCILE")).await;
                assert_eq!(
                    present(&left),
                    present(&right),
                    "`{a}`, `{b}`, `{c}` associate"
                );
            }
        }
    }
}

/// `SELECT` given BOTH answers its two candidates reconciled; `FILTER`
/// keeps an element its predicate tells true, which BOTH does.
#[tokio::test]
async fn select_and_filter_read_both() {
    assert_eq!(one("1 1 BOTH SELECT").await.to_string(), "1/1");
    assert_eq!(one("TRUE FALSE BOTH SELECT").await.to_string(), "BOTH");
    assert_eq!(
        one("'yes' 'no' BOTH SELECT").await.nil_reason(),
        Some(&NilReason::Conflict)
    );
    assert_eq!(
        one("[ 1 2 3 ] [ 1 5 3 ] [ TRUE BOTH FALSE ] SELECT")
            .await
            .to_string(),
        "[ 1/1 NIL 3/1 ]"
    );
    assert_eq!(
        one("[ 1 2 3 ] [ 2 EQ NOT BOTH AND ] FILTER")
            .await
            .to_string(),
        "[ 1/1 3/1 ]"
    );
    assert_eq!(
        one("[ TRUE BOTH FALSE NIL ] [ ] FILTER").await.to_string(),
        "[ TRUE BOTH ]"
    );
}
