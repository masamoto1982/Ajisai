//! The phase-one remediation of typed Ajisai (LANG.CONTRACT.TYPES): typed
//! `COLLECT` over a count written right there, the order and text Words'
//! rules, a typed body calling an untyped User Word, `SHAPE` of a Record, and
//! the name rule between bindings and Words (LANG.DICTIONARY.RESOLUTION).

use crate::agent::api::{check, compute};
use serde_json::Value as Json;

fn contract_decls(source: &str) -> Json {
    check(source).to_json()["contractDecls"].clone()
}

async fn run(source: &str) -> Json {
    compute(source, Default::default()).await.to_json()
}

#[test]
fn the_newly_typed_words_verify() {
    for source in [
        "#:contract ROW int str -> {p:int q:str}\n\
         [ 'P' BIND 'Q' BIND Q P 2 COLLECT 'X' BIND [ 'p' 'q' ] X RECORD ] 'ROW' DEF",
        "#:contract ROW2 int str -> {p:int q:str}\n\
         [ 'Q' BIND 'P' BIND [ 'p' 'q' ] P Q 2 COLLECT RECORD ] 'ROW2' DEF",
        "#:contract PAIR int int -> [int]\n[ 2 COLLECT ] 'PAIR' DEF",
        "#:contract RANKED [int] -> [int]\n[ 'V' BIND V V ORDER GET ] 'RANKED' DEF",
        "#:contract IDX [rat] -> [int]\n[ ORDER ] 'IDX' DEF",
        "#:contract SHOUT [str] -> [str]\n[ UPPER ] 'SHOUT' DEF",
        "#:contract LABEL [int] -> str\n[ STR ', ' 'X' BIND JOIN ] 'LABEL' DEF",
        "#:contract CELLS str -> [str]\n[ ',' TOKENIZE ] 'CELLS' DEF",
        "#:contract MONEY [rat] -> [str]\n[ 2 FORMAT ] 'MONEY' DEF",
        "#:contract AT str -> int\n[ 'x' SEARCH ] 'AT' DEF",
        "#:contract FLAT [[int]] -> [int] int\n[ 'V' BIND V FLATTEN V DEPTH ] 'FLAT' DEF",
        "#:contract GONE int -> bool\n[ NIL? ] 'GONE' DEF",
    ] {
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "value", "{source}: {decls}");
    }
}

#[test]
fn collect_needs_its_count_written_right_there() {
    for (source, says) in [
        (
            "#:contract C int int int -> [int]\n[ COLLECT ] 'C' DEF",
            "count written right before",
        ),
        (
            "#:contract C int str -> [int]\n[ 2 COLLECT ] 'C' DEF",
            "do not join",
        ),
        (
            "#:contract C int -> [int]\n[ 2 COLLECT ] 'C' DEF",
            "too few values",
        ),
        (
            "#:contract C int str -> {p:str q:str}\n[ 2 COLLECT 'X' BIND [ 'p' 'q' ] X RECORD ] 'C' DEF",
            "declares {p:str q:str}",
        ),
    ] {
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "error", "{source}: {decls}");
        assert!(decls.to_string().contains(says), "{source}: {decls}");
    }
}

#[test]
fn a_typed_word_walks_an_untyped_word_it_calls() {
    let fits = "[ 2 MUL ] 'TWICE' DEF\n\
                #:contract QUAD int -> int\n[ TWICE TWICE ] 'QUAD' DEF";
    assert_eq!(contract_decls(fits)["outcome"], "value");

    let through = "[ 'price' GET ] 'PRICE-OF' DEF\n\
                   #:contract P {price:[int]} -> [int]\n[ PRICE-OF ] 'P' DEF";
    assert_eq!(contract_decls(through)["outcome"], "value");

    let misfits = "[ 1/2 MUL ] 'HALF' DEF\n\
                   #:contract H int -> int\n[ HALF ] 'H' DEF";
    let decls = contract_decls(misfits);
    assert_eq!(decls["outcome"], "error", "{decls}");

    let untypable = "[ CONTRACT ] 'ASK' DEF\n\
                     #:contract A int -> int\n[ ASK ] 'A' DEF";
    let decls = contract_decls(untypable).to_string();
    assert!(decls.contains("ASK, walked with the types"), "{decls}");
}

#[tokio::test]
async fn a_typed_call_into_an_untyped_word_answers_as_untyped() {
    let body = "[ 2 MUL ] 'TWICE' DEF\n[ TWICE TWICE ] 'QUAD' DEF\n5 QUAD";
    let typed = run(&format!("#:contract QUAD int -> int\n{body}")).await;
    let untyped = run(body).await;
    assert_eq!(typed["status"], "ok", "{typed}");
    assert_eq!(typed["stack"], untyped["stack"]);
}

#[tokio::test]
async fn shape_of_a_record_is_a_record_of_shapes() {
    let out = run("[ 'a' 'b' ] [ [ 1 2 ] [ 3 ] ] RECORD SHAPE").await;
    let expected = run("[ 'a' 'b' ] [ [ 2 ] [ 1 ] ] RECORD").await;
    assert_eq!(out["status"], "ok", "{out}");
    assert_eq!(out["stack"], expected["stack"]);
}

#[tokio::test]
async fn a_word_may_not_take_a_name_another_word_binds() {
    for source in [
        "[ 'K' BIND K 1 ADD ] 'F' DEF\n[ 5 ] 'K' DEF",
        "[ 5 ] 'K' DEF\n[ 'K' BIND K 1 ADD ] 'F' DEF",
        "[ 5 ] 'K' DEF\n[ [ 'J' 'K' ] BIND J ] 'F' DEF",
    ] {
        let out = run(source).await;
        assert_eq!(out["error"]["category"], "nameConflict", "{source}: {out}");
    }
    let shared = run("[ 'K' BIND K ] 'F' DEF\n[ 'K' BIND K K ADD ] 'G' DEF\n3 G").await;
    assert_eq!(shared["status"], "ok", "{shared}");
    let a_value_not_a_name = run("[ 'hello' 'N' BIND N ] 'F' DEF\n[ 1 ] 'HELLO' DEF").await;
    assert_eq!(a_value_not_a_name["status"], "ok", "{a_value_not_a_name}");
}
