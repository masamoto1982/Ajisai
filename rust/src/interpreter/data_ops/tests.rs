//! Tests for the DATA module words (Phase 8C).

use super::*;

#[test]
fn parses_a_simple_table() {
    let rows = parse_csv_rows("a,b\n1,2\n3,4").unwrap();
    assert_eq!(rows, vec![vec!["a", "b"], vec!["1", "2"], vec!["3", "4"]]);
}

#[test]
fn trailing_newline_does_not_add_a_row() {
    let rows = parse_csv_rows("a,b\n1,2\n").unwrap();
    assert_eq!(rows, vec![vec!["a", "b"], vec!["1", "2"]]);
}

#[test]
fn crlf_ends_a_line() {
    let rows = parse_csv_rows("a,b\r\n1,2\r\n").unwrap();
    assert_eq!(rows, vec![vec!["a", "b"], vec!["1", "2"]]);
}

#[test]
fn quoted_fields_carry_commas_quotes_and_newlines() {
    let rows = parse_csv_rows("name,note\n\"Doe, John\",\"a \"\"quote\"\"\"\n").unwrap();
    assert_eq!(
        rows,
        vec![
            vec!["name".to_string(), "note".to_string()],
            vec!["Doe, John".to_string(), "a \"quote\"".to_string()],
        ]
    );
}

#[test]
fn empty_text_is_zero_rows() {
    assert_eq!(parse_csv_rows("").unwrap(), Vec::<Vec<String>>::new());
}

#[test]
fn unterminated_quote_is_rejected() {
    assert!(parse_csv_rows("a,b\n\"oops,1\n").is_none());
}

#[test]
fn ragged_row_makes_the_table_none() {
    let rows = parse_csv_rows("a,b\n1,2,3\n").unwrap();
    assert!(rows_to_record_vector(rows).is_none());
}

#[test]
fn header_only_is_an_empty_table() {
    let rows = parse_csv_rows("a,b\n").unwrap();
    let v = rows_to_record_vector(rows).unwrap();
    assert!(matches!(&v.data, ValueData::Vector(items) if items.is_empty()));
}

#[test]
fn round_trips_through_records() {
    let csv = "a,b\n1,2\n3,4\n";
    let value = rows_to_record_vector(parse_csv_rows(csv).unwrap()).unwrap();
    assert_eq!(record_vector_to_csv(&value).as_deref(), Some(csv));
}

#[test]
fn round_trips_fields_needing_quotes() {
    let csv = "name,note\n\"Doe, John\",\"a \"\"quote\"\"\"\n";
    let value = rows_to_record_vector(parse_csv_rows(csv).unwrap()).unwrap();
    assert_eq!(record_vector_to_csv(&value).as_deref(), Some(csv));
}

#[test]
fn stringify_rejects_a_non_table() {
    assert!(record_vector_to_csv(&Value::from_int(5)).is_none());
}

#[test]
fn empty_vector_stringifies_to_empty_text() {
    assert_eq!(
        record_vector_to_csv(&vector_of(Vec::new())).as_deref(),
        Some("")
    );
}

#[test]
fn encode_field_quotes_only_when_needed() {
    assert_eq!(encode_field("plain"), "plain");
    assert_eq!(encode_field("a,b"), "\"a,b\"");
    assert_eq!(encode_field("a\"b"), "\"a\"\"b\"");
}

// Execution-level tests: the words are reachable through IMPORT and drive
// the production interpreter path.

#[tokio::test]
async fn csv_round_trips_through_the_interpreter() {
    let mut interp = Interpreter::new();
    interp
        .execute("'a,b\n1,2\n3,4' 'DATA' IMPORT CSV-PARSE CSV-STRINGIFY PRINT")
        .await
        .unwrap();
    assert_eq!(interp.collect_output().trim_end(), "a,b\n1,2\n3,4");
}

#[tokio::test]
async fn csv_parse_of_ragged_input_bubbles_to_nil() {
    let mut interp = Interpreter::new();
    interp
        .execute("'a,b\n1,2,3' 'DATA' IMPORT CSV-PARSE")
        .await
        .unwrap();
    let top = interp.get_stack().last().expect("a value on the stack");
    assert!(top.is_absent(), "expected a NIL, got {:?}", top);
}

#[tokio::test]
async fn select_keeps_only_named_columns_in_order() {
    let mut interp = Interpreter::new();
    interp
        .execute(
            "'name,age\nalice,30\nbob,40' 'DATA' IMPORT \
                 CSV-PARSE [ 'age' ] SELECT CSV-STRINGIFY PRINT",
        )
        .await
        .unwrap();
    assert_eq!(interp.collect_output().trim_end(), "age\n30\n40");
}

#[tokio::test]
async fn select_of_a_missing_column_yields_a_nil_cell() {
    // The missing column still appears; its cell is NIL (MissingField), so
    // CSV-STRINGIFY renders it as an empty field.
    let mut interp = Interpreter::new();
    interp
        .execute(
            "'name\nalice' 'DATA' IMPORT \
                 CSV-PARSE [ 'name' 'missing' ] SELECT CSV-STRINGIFY PRINT",
        )
        .await
        .unwrap();
    assert_eq!(interp.collect_output().trim_end(), "name,missing\nalice,");
}

#[tokio::test]
async fn where_keeps_rows_matching_the_column_predicate() {
    let mut interp = Interpreter::new();
    interp
        .execute(
            "'name,active\nalice,yes\nbob,no\ncarol,yes' 'DATA' IMPORT \
                 CSV-PARSE 'active' { 'yes' = } WHERE CSV-STRINGIFY PRINT",
        )
        .await
        .unwrap();
    assert_eq!(
        interp.collect_output().trim_end(),
        "name,active\nalice,yes\ncarol,yes"
    );
}

#[tokio::test]
async fn where_on_a_missing_column_drops_every_row() {
    // A missing column reads as NIL, whose predicate result is not true, so
    // every row drops and the result is an empty table.
    let mut interp = Interpreter::new();
    interp
        .execute("'name\nalice' 'DATA' IMPORT CSV-PARSE 'nope' { 'yes' = } WHERE")
        .await
        .unwrap();
    let top = interp.get_stack().last().expect("a value on the stack");
    assert_eq!(top.len(), 0, "expected an empty table, got {:?}", top);
}

#[tokio::test]
async fn select_of_a_non_table_bubbles_to_nil() {
    let mut interp = Interpreter::new();
    interp
        .execute("[ 5 ] [ 'a' ] 'DATA' IMPORT SELECT")
        .await
        .unwrap();
    let top = interp.get_stack().last().expect("a value on the stack");
    assert!(top.is_absent(), "expected a NIL, got {:?}", top);
}
