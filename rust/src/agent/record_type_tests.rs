//! Record types (LANG.CONTRACT.TYPES over LANG.RECORDS.STRUCTURE): a
//! `{name:T …}` type, the Record Words' type rules, and the check where a
//! typed Word is entered and returns. A table is a Record of columns.

use crate::agent::api::{check, compute};
use crate::interpreter::word_types::split_signature;
use serde_json::Value as Json;

fn contract_decls(source: &str) -> Json {
    check(source).to_json()["contractDecls"].clone()
}

async fn run(source: &str) -> Json {
    compute(source, Default::default()).await.to_json()
}

const SUBTOTALS: &str = "#:contract SUBTOTALS {price:[int] qty:[int]} -> [int]\n\
                         [ 'T' BIND T 'price' GET T 'qty' GET MUL ] 'SUBTOTALS' DEF\n";

#[test]
fn a_record_type_is_written_with_its_fields_in_order() {
    let (sig, rest) =
        split_signature("{price:[int] qty:[int]} -> [ { name:str } ] field=closed").unwrap();
    assert_eq!(
        sig.unwrap().to_string(),
        "{price:[int] qty:[int]} -> [{name:str}]"
    );
    assert_eq!(rest, "field=closed");
    for malformed in [
        "{price [int]} ->",
        "{a:int a:int} ->",
        "{a:int ->",
        "{:int} ->",
        "{a:float} ->",
    ] {
        assert!(split_signature(malformed).is_err(), "{malformed}");
    }
}

#[test]
fn a_body_over_records_that_fits_is_verified() {
    for source in [
        SUBTOTALS.to_string(),
        "#:contract TAXED {price:[int] qty:[int]} -> {price:[rat] qty:[rat]}\n\
         [ 11/10 MUL ] 'TAXED' DEF"
            .into(),
        "#:contract ORDERS -> {price:[int] qty:[int]}\n\
         [ [ 'price' 'qty' ] [ [ 120 80 ] [ 2 1 ] ] RECORD ] 'ORDERS' DEF"
            .into(),
        "#:contract WITH-TOTAL {price:[int] qty:[int]} -> {price:[int] qty:[int] total:[int]}\n\
         [ 'T' BIND T 'total' T 'price' GET T 'qty' GET MUL PUT ] 'WITH-TOTAL' DEF"
            .into(),
        "#:contract ONE -> {a:int}\n[ [ ] [ ] RECORD 'a' 1 PUT ] 'ONE' DEF".into(),
        "#:contract BOTH {a:int} {b:str} -> {a:int b:str}\n[ MERGE ] 'BOTH' DEF".into(),
        "#:contract DROP-B {a:int b:str} -> {a:int}\n[ 'b' WITHOUT ] 'DROP-B' DEF".into(),
        "#:contract NAMES {a:int b:int} -> [str] [int] bool\n\
         [ 'R' BIND R KEYS R VALUES R 'a' HAS? ] 'NAMES' DEF"
            .into(),
        "#:contract WHO [{name:str age:int}] -> [str]\n[ [ 'name' GET ] MAP ] 'WHO' DEF".into(),
        "#:contract SAME {a:int b:int} {a:int b:int} -> {a:int b:int}\n[ ADD ] 'SAME' DEF".into(),
    ] {
        let decls = contract_decls(&source);
        assert_eq!(decls["outcome"], "value", "{source}: {decls}");
    }
}

#[test]
fn a_body_over_records_that_does_not_fit_is_violated_before_running() {
    for (source, says) in [
        (
            "#:contract C {price:[int]} -> [int]\n[ 'cost' GET ] 'C' DEF",
            "no field 'cost'",
        ),
        (
            "#:contract K {price:[int]} str -> [int]\n[ GET ] 'K' DEF",
            "key written right there",
        ),
        (
            "#:contract H {price:[int]} -> {price:[int]}\n[ 2 DIV ] 'H' DEF",
            "declares {price:[int]}",
        ),
        (
            "#:contract M {a:int} {b:int} -> {a:int}\n[ ADD ] 'M' DEF",
            "ADD has no rule",
        ),
        (
            "#:contract V {a:int b:str} -> [int]\n[ VALUES ] 'V' DEF",
            "does not join",
        ),
    ] {
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "error", "{source}: {decls}");
        let message = decls["findings"][0]["message"].as_str().unwrap_or_default();
        assert!(message.contains(says), "{source}: {message}");
    }
}

#[tokio::test]
async fn a_table_runs_typed_as_it_runs_untyped() {
    let call = "[ 'price' 'qty' ] [ [ 120 80 300 ] [ 2 1 4 ] ] RECORD SUBTOTALS";
    let typed = run(&format!("{SUBTOTALS}{call}")).await;
    let untyped = run(&format!("{}\n{call}", SUBTOTALS.lines().nth(1).unwrap())).await;
    assert_eq!(typed["status"], "ok", "{typed}");
    assert_eq!(typed["stack"], untyped["stack"]);
}

#[tokio::test]
async fn a_record_outside_its_type_is_named_by_key_and_field() {
    let reordered = run(&format!(
        "{SUBTOTALS}[ 'qty' 'price' ] [ [ 2 ] [ 120 ] ] RECORD SUBTOTALS"
    ))
    .await;
    assert_eq!(
        reordered["error"]["category"], "typeViolation",
        "{reordered}"
    );
    let message = reordered["message"].as_str().unwrap_or_default();
    assert!(message.contains("keys"), "{message}");

    let fraction = run(&format!(
        "{SUBTOTALS}[ 'price' 'qty' ] [ [ 120 ] [ 1/2 ] ] RECORD SUBTOTALS"
    ))
    .await;
    let message = fraction["message"].as_str().unwrap_or_default();
    assert!(message.contains("field qty"), "{message}");

    let nil = run(&format!(
        "{SUBTOTALS}[ 'price' 'qty' ] [ [ 120 NIL ] [ 2 1 ] ] RECORD SUBTOTALS"
    ))
    .await;
    assert_eq!(nil["status"], "ok", "{nil}");
}
