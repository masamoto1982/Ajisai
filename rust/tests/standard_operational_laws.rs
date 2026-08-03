//! Observable contracts that justify retaining native operational Standard Words.

use ajisai_core::interpreter::Interpreter;
use ajisai_core::NilReason;

fn rendered_stack(interpreter: &Interpreter) -> Vec<String> {
    interpreter
        .get_stack()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn effect_payloads(interpreter: &Interpreter) -> Vec<&str> {
    interpreter
        .host_effects()
        .iter()
        .map(|effect| effect.payload())
        .collect()
}

#[tokio::test]
async fn map_visits_in_index_order_with_isolated_stacks_and_ordered_effects() {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute("[ 3 1 2 ] { ,, PRINT 10 ADD } MAP")
        .await
        .unwrap();

    assert_eq!(effect_payloads(&interpreter), ["3/1", "1/1", "2/1"]);
    assert_eq!(rendered_stack(&interpreter), ["[ 13/1 11/1 12/1 ]"]);
}

#[tokio::test]
async fn filter_visits_in_index_order_and_observes_predicate_truth() {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute("[ 3 1 2 ] { ,, PRINT 1 GT } FILTER")
        .await
        .unwrap();

    assert_eq!(effect_payloads(&interpreter), ["3/1", "1/1", "2/1"]);
    assert_eq!(rendered_stack(&interpreter), ["[ 3/1 2/1 ]"]);
}

#[tokio::test]
async fn any_and_all_short_circuit_before_unvisited_effects() {
    let mut any = Interpreter::new();
    any.execute("[ 1 2 3 ] { ,, PRINT 2 EQ } ANY")
        .await
        .unwrap();
    assert_eq!(effect_payloads(&any), ["1/1", "2/1"]);
    assert_eq!(rendered_stack(&any), ["TRUE"]);

    let mut all = Interpreter::new();
    all.execute("[ 1 2 3 ] { ,, PRINT 2 LT } ALL")
        .await
        .unwrap();
    assert_eq!(effect_payloads(&all), ["1/1", "2/1"]);
    assert_eq!(rendered_stack(&all), ["FALSE"]);
}

#[tokio::test]
async fn higher_order_errors_restore_the_original_operand_atomically() {
    for word in ["MAP", "FILTER", "ANY", "ALL"] {
        let mut interpreter = Interpreter::new();
        let source = format!("[ 3 1 2 ] {{ UNKNOWN-CALLBACK }} {word}");
        assert!(interpreter.execute(&source).await.is_err(), "{word}");
        assert_eq!(
            rendered_stack(&interpreter),
            ["[ 3/1 1/1 2/1 ]", "UNKNOWN-CALLBACK"],
            "{word}"
        );
    }
}

#[tokio::test]
async fn fill_checks_overflow_and_ceiling_before_materializing() {
    for source in [
        "[ 1000000 1000000 7 ] FILL",
        "[ 99999999 99999999 99999999 1 ] FILL",
    ] {
        let mut interpreter = Interpreter::new();
        interpreter.execute(source).await.unwrap();
        let value = interpreter.get_stack().last().expect("FILL result");
        assert!(value.is_nil());
        assert_eq!(value.nil_reason(), Some(&NilReason::SpaceExhausted));
    }
}

#[tokio::test]
async fn sort_is_deterministic_and_restores_malformed_operands() {
    for _ in 0..3 {
        let mut interpreter = Interpreter::new();
        interpreter.execute("[ 3 1 2 1 ] SORT").await.unwrap();
        assert_eq!(rendered_stack(&interpreter), ["[ 1/1 1/1 2/1 3/1 ]"]);
    }

    let mut malformed = Interpreter::new();
    assert!(malformed.execute("[ 3 'x' 2 ] SORT").await.is_err());
    assert_eq!(rendered_stack(&malformed), ["[ 3/1 'x' 2/1 ]"]);
}
