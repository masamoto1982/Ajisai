//! The sheet view of the User dictionary: a cell's formula, effect and
//! value, read without disturbing the session.

use crate::agent::block_on;
use crate::interpreter::sheet_cell::{suggest_type, CellValue, SheetTable};
use crate::interpreter::Interpreter;
use crate::types::Value;

fn session(source: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    block_on(interp.execute(source)).unwrap();
    interp
}

#[test]
fn a_cell_is_a_word_with_its_body_as_the_formula() {
    let interp = session(
        "[ [ 120 80 300 ] ] 'PRICE' DEF [ PRICE 3 MUL ] 'TOTAL' DEF [ 2 MUL ] 'DOUBLE' DEF",
    );
    let total = interp.sheet_cell("total").unwrap();
    assert_eq!(total.formula, "PRICE 3 MUL");
    assert_eq!(total.effect, Some((0, 1)));
    assert!(total.identity.is_some());
    assert_eq!(interp.sheet_cell("DOUBLE").unwrap().effect, Some((1, 1)));
    assert!(interp.sheet_cell("NOPE").is_none());
}

#[test]
fn evaluating_a_cell_leaves_the_session_as_it_was() {
    let mut interp = session("[ [ 120 80 300 ] ] 'PRICE' DEF [ PRICE 3 MUL ] 'TOTAL' DEF 7");
    let usage = interp.resource_usage();
    let value = block_on(interp.evaluate_sheet_cell("TOTAL"));
    assert_eq!(
        value,
        CellValue::Value {
            display: "[ 360/1 240/1 900/1 ]".to_string(),
            suggested_type: Some("[int]".to_string()),
            table: None,
        }
    );
    assert_eq!(interp.resource_usage(), usage);
    assert_eq!(interp.get_stack().len(), 1);
    assert!(!interp.collect_output().contains("360"));
}

#[test]
fn a_cell_that_fails_shows_its_error_and_a_typed_one_suggests_nothing() {
    let interp = session("[ 1 ADD ] 'BROKEN' DEF");
    assert!(matches!(
        block_on(interp.evaluate_sheet_cell("BROKEN")),
        CellValue::Error(_)
    ));
    let interp = session("#:contract HALF -> rat\n[ 1 2 DIV ] 'HALF' DEF");
    assert_eq!(
        interp.sheet_cell("HALF").unwrap().signature.as_deref(),
        Some("-> rat")
    );
    assert_eq!(
        block_on(interp.evaluate_sheet_cell("HALF")),
        CellValue::Value {
            display: "1/2".to_string(),
            suggested_type: None,
            table: None,
        }
    );
}

#[test]
fn the_suggested_type_is_the_narrowest_that_fits() {
    let of = |source: &str| {
        let mut interp = Interpreter::new();
        block_on(interp.execute(source)).unwrap();
        let value: Value = interp.get_stack().last().unwrap().clone();
        suggest_type(&value).map(|ty| ty.to_string())
    };
    assert_eq!(of("3").as_deref(), Some("int"));
    assert_eq!(of("1/3").as_deref(), Some("rat"));
    assert_eq!(of("18446744073709551616").as_deref(), None);
    assert_eq!(of("TRUE").as_deref(), Some("bool"));
    assert_eq!(of("'a'").as_deref(), Some("str"));
    assert_eq!(of("[ 1 1/2 ]").as_deref(), Some("[rat]"));
    assert_eq!(of("[ [ 1 2 ] [ 3 4 ] ]").as_deref(), Some("[[int]]"));
    assert_eq!(of("NIL").as_deref(), None);
}

#[test]
fn a_typed_body_may_call_a_typed_word_the_session_already_holds() {
    let interp = session("#:contract PRICE -> [int]\n[ [ 120 80 ] ] 'PRICE' DEF");
    let source = "#:contract TOTAL -> [int]\n[ PRICE 3 MUL ] 'TOTAL' DEF";
    let alone = crate::agent::contract::declared_contract_check(source).unwrap();
    assert!(
        alone.violated,
        "the source alone does not know PRICE's type"
    );
    let in_session = crate::agent::contract::declared_contract_check_in(source, &interp).unwrap();
    assert!(!in_session.violated, "{}", in_session.to_json());
}

#[test]
fn a_record_of_columns_is_a_table_and_suggests_a_record_type() {
    let interp = session(
        "[ [ 'item' 'price' ] [ [ 'tea' 'cake' ] [ 120 1/2 ] ] RECORD ] 'MENU' DEF \
         [ [ 'a' 'b' ] [ [ 1 2 ] [ 3 ] ] RECORD ] 'RAGGED' DEF",
    );
    assert_eq!(
        block_on(interp.evaluate_sheet_cell("MENU")),
        CellValue::Value {
            display: "[ 'item' 'price' ] [ [ 'tea' 'cake' ] [ 120/1 1/2 ] ] RECORD".to_string(),
            suggested_type: Some("{item:[str] price:[rat]}".to_string()),
            table: Some(SheetTable {
                columns: vec!["item".into(), "price".into()],
                cells: vec![
                    vec!["'tea'".into(), "120/1".into()],
                    vec!["'cake'".into(), "1/2".into()],
                ],
                rows: 2,
            }),
        }
    );
    let CellValue::Value { table, .. } = block_on(interp.evaluate_sheet_cell("RAGGED")) else {
        panic!("RAGGED answers a value");
    };
    assert_eq!(table, None);
}
