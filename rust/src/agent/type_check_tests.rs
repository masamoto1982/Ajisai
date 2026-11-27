//! Typed User Words (LANG.CONTRACT.TYPES): the signature a `#:contract`
//! line writes, the check before anything runs, and the `typeViolation` a
//! value outside a declared range answers at run time.

use crate::agent::api::{check, compute};
use serde_json::Value as Json;

fn contract_decls(source: &str) -> Json {
    check(source).to_json()["contractDecls"].clone()
}

async fn run(source: &str) -> Json {
    compute(source, Default::default()).await.to_json()
}

#[test]
fn a_body_that_fits_its_signature_is_verified() {
    for source in [
        "#:contract SUMSQ [int] -> int\n[ [ 'X' BIND X X MUL ] MAP 0 [ ADD ] FOLD ] 'SUMSQ' DEF",
        "#:contract SUBTOTAL [rat] [int] -> [rat]\n[ MUL ] 'SUBTOTAL' DEF",
        "#:contract WITH-TAX [int] -> [int]\n[ 110 MUL 100 DIV FLOOR ] 'WITH-TAX' DEF",
        "#:contract HALF int -> rat field=closed\n[ 2 DIV ] 'HALF' DEF",
        "#:contract POS [int] -> [int]\n[ [ 0 GT ] FILTER ] 'POS' DEF",
        "#:contract MEAN [int] -> rat\n[ 'XS' BIND XS 0 [ ADD ] FOLD XS LENGTH DIV ] 'MEAN' DEF",
        "#:contract RUNNING [int] -> [rat]\n[ 0 [ ADD 1/2 MUL ] SCAN ] 'RUNNING' DEF",
        "#:contract PAIR -> [int]\n[ [ 1 2 ] ] 'PAIR' DEF",
        "#:contract NAME -> str\n[ 'ajisai' ] 'NAME' DEF",
        "#:contract SQ int -> int\n[ 'X' BIND X X MUL ] 'SQ' DEF\n\
         #:contract SQS [int] -> [int]\n[ [ SQ ] MAP ] 'SQS' DEF",
        "#:contract CLAMP [int] -> [int]\n[ 'X' BIND X 0 X 0 LT SELECT ] 'CLAMP' DEF",
    ] {
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "value", "{source}: {decls}");
        assert_eq!(check(source).exit_code(), 0, "{source}");
    }
}

#[test]
fn a_body_that_does_not_fit_is_violated_before_running() {
    for (source, says) in [
        (
            "#:contract H int -> int\n[ 2 DIV ] 'H' DEF",
            "output 1 is rat",
        ),
        (
            "#:contract R [int] -> int\n[ SQRT ] 'R' DEF",
            "SQRT has no type rule",
        ),
        (
            "#:contract B int -> bool\n[ 1 ADD ] 'B' DEF",
            "declares bool",
        ),
        (
            "#:contract N int -> int\n[ 'A' ADD ] 'N' DEF",
            "ADD has no rule",
        ),
        ("#:contract C [int] -> int\n[ 1 2 ] 'C' DEF", "inputs=1"),
        (
            "#:contract U int -> int\n[ OTHER ] 'U' DEF [ 1 ADD ] 'OTHER' DEF",
            "OTHER has no type rule",
        ),
    ] {
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "error", "{source}: {decls}");
        let message = decls["findings"][0]["message"].as_str().unwrap_or_default();
        assert!(message.contains(says), "{source}: {message}");
        assert_eq!(check(source).exit_code(), 1, "{source}");
    }
}

#[test]
fn a_malformed_signature_is_an_error() {
    for source in [
        "#:contract W float -> int\n[ ] 'W' DEF",
        "#:contract W [int -> int\n[ ] 'W' DEF",
        "#:contract W int -> int inputs=1\n[ ] 'W' DEF",
        "#:contract W int int\n[ ] 'W' DEF",
        "#:contract W int -> int -> int\n[ ] 'W' DEF",
    ] {
        assert_eq!(contract_decls(source)["outcome"], "error", "{source}");
    }
}

#[tokio::test]
async fn erasing_the_signature_leaves_the_same_answer() {
    let typed = "#:contract SUMSQ [int] -> int\n\
                 [ [ 'X' BIND X X MUL ] MAP 0 [ ADD ] FOLD ] 'SUMSQ' DEF\n\
                 [ 1 2 3 NIL ] SUMSQ";
    let untyped = typed.lines().skip(1).collect::<Vec<_>>().join("\n");
    let (a, b) = (run(typed).await, run(&untyped).await);
    assert_eq!(a["status"], "ok", "{a}");
    assert_eq!(a["stack"], b["stack"]);
}

#[tokio::test]
async fn a_caller_outside_the_input_range_is_named() {
    let json = run("#:contract SUMSQ [int] -> int\n\
                    [ [ 'X' BIND X X MUL ] MAP 0 [ ADD ] FOLD ] 'SUMSQ' DEF\n\
                    [ 1 1/2 3 ] SUMSQ")
    .await;
    assert_eq!(json["error"]["category"], "typeViolation", "{json}");
    let message = json["message"].as_str().unwrap_or_default();
    assert!(message.contains("input 1"), "{message}");
    assert!(message.contains("1/2"), "{message}");
}

#[tokio::test]
async fn a_result_outside_the_output_range_is_the_words() {
    let json = run("#:contract SQ int -> int\n[ 'X' BIND X X MUL ] 'SQ' DEF\n\
                    4294967296 SQ")
    .await;
    assert_eq!(json["error"]["category"], "typeViolation", "{json}");
    let message = json["message"].as_str().unwrap_or_default();
    assert!(message.starts_with("SQ: output 1"), "{message}");
}

#[tokio::test]
async fn a_value_beyond_64_bits_is_outside_int_but_fine_untyped() {
    let big = "18446744073709551616";
    let typed = run(&format!(
        "#:contract ID int -> int\n[ 0 ADD ] 'ID' DEF\n{big} ID"
    ))
    .await;
    assert_eq!(typed["error"]["category"], "typeViolation", "{typed}");
    let untyped = run(&format!("[ 0 ADD ] 'ID' DEF\n{big} ID")).await;
    assert_eq!(untyped["status"], "ok", "{untyped}");
}

#[tokio::test]
async fn nil_and_the_points_over_zero_fit_where_they_should() {
    let ok = run("#:contract R rat -> rat\n[ 1 MUL ] 'R' DEF\n1/0 R 0/0 R NIL R").await;
    assert_eq!(ok["status"], "ok", "{ok}");
    let refused = run("#:contract I int -> int\n[ 1 MUL ] 'I' DEF\n1/0 I").await;
    assert_eq!(refused["error"]["category"], "typeViolation", "{refused}");
}

#[tokio::test]
async fn redefining_without_a_signature_drops_the_type() {
    let json =
        run("#:contract ID int -> int\n[ 0 ADD ] 'ID' DEF\n[ 0 ADD ] 'ID' DEF\n1/2 ID").await;
    assert_eq!(json["status"], "ok", "{json}");
}
