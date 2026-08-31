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
        .execute("[ 3 1 2 ] [ KEEP PRINT 10 ADD ] MAP")
        .await
        .unwrap();

    assert_eq!(effect_payloads(&interpreter), ["3/1", "1/1", "2/1"]);
    assert_eq!(rendered_stack(&interpreter), ["[ 13/1 11/1 12/1 ]"]);
}

#[tokio::test]
async fn filter_visits_in_index_order_and_observes_predicate_truth() {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute("[ 3 1 2 ] [ KEEP PRINT 1 GT ] FILTER")
        .await
        .unwrap();

    assert_eq!(effect_payloads(&interpreter), ["3/1", "1/1", "2/1"]);
    assert_eq!(rendered_stack(&interpreter), ["[ 3/1 2/1 ]"]);
}

#[tokio::test]
async fn any_and_all_short_circuit_before_unvisited_effects() {
    let mut any = Interpreter::new();
    any.execute("[ 1 2 3 ] [ KEEP PRINT 2 EQ ] ANY")
        .await
        .unwrap();
    assert_eq!(effect_payloads(&any), ["1/1", "2/1"]);
    assert_eq!(rendered_stack(&any), ["TRUE"]);

    let mut all = Interpreter::new();
    all.execute("[ 1 2 3 ] [ KEEP PRINT 2 LT ] ALL")
        .await
        .unwrap();
    assert_eq!(effect_payloads(&all), ["1/1", "2/1"]);
    assert_eq!(rendered_stack(&all), ["FALSE"]);
}

#[tokio::test]
async fn higher_order_errors_restore_the_original_operand_atomically() {
    for word in ["MAP", "FILTER", "ANY", "ALL"] {
        let mut interpreter = Interpreter::new();
        let source = format!("[ 3 1 2 ] [ UNKNOWN-CALLBACK ] {word}");
        assert!(interpreter.execute(&source).await.is_err(), "{word}");
        assert_eq!(
            rendered_stack(&interpreter),
            ["[ 3/1 1/1 2/1 ]", "[ UNKNOWN-CALLBACK ]"],
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

/// `ORDER` answers the permutation `SORT` applies, and answers it stably: two
/// equal keys keep their original positions.
///
/// Stability is the property the reference's hand-written ranking idiom existed
/// to recover — `SORT` then `INDEX-OF` finds the same position twice when two
/// keys tie, so "the k nearest" could hand back k-1 or k+1 neighbours. Native
/// retention rests on this and on the cost: the idiom is O(n²) interpreted
/// steps, which is what put a ceiling of about ninety training points on a
/// k-nearest-neighbour classifier while `SORT` handled five thousand elements.
#[tokio::test]
async fn order_is_the_stable_permutation_sort_applies() {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute("[ 18 13 1 1 13 2 ] ORDER")
        .await
        .unwrap();
    assert_eq!(
        rendered_stack(&interpreter),
        ["[ 2/1 3/1 5/1 1/1 4/1 0/1 ]"]
    );

    // Applying the permutation reproduces SORT exactly.
    let mut applied = Interpreter::new();
    applied
        .execute("[ 18 13 1 1 13 2 ] KEEP ORDER GET")
        .await
        .unwrap();
    let mut sorted = Interpreter::new();
    sorted.execute("[ 18 13 1 1 13 2 ] SORT").await.unwrap();
    assert_eq!(rendered_stack(&applied), rendered_stack(&sorted));

    let mut malformed = Interpreter::new();
    assert!(malformed.execute("[ 3 'x' 2 ] ORDER").await.is_err());
    assert_eq!(rendered_stack(&malformed), ["[ 3/1 'x' 2/1 ]"]);
}

/// `UNIQUE` and `TALLY` are one pass read two ways: same order, aligned
/// lengths, whatever the element domain. Written out, each is an O(n²) scan
/// with `INDEX-OF`, and they are the counting step of a majority vote, a class
/// prior, a histogram, a Gini and a naive-Bayes tally.
#[tokio::test]
async fn unique_and_tally_agree_on_order_and_length() {
    let mut unique = Interpreter::new();
    unique
        .execute("[ 'b' 'a' 'b' 'c' 'b' ] UNIQUE")
        .await
        .unwrap();
    let mut tally = Interpreter::new();
    tally
        .execute("[ 'b' 'a' 'b' 'c' 'b' ] TALLY")
        .await
        .unwrap();
    assert_eq!(rendered_stack(&unique), ["[ 'b' 'a' 'c' ]"]);
    assert_eq!(rendered_stack(&tally), ["[ 3/1 1/1 1/1 ]"]);
}

/// `ZIP` transposes, and transposing twice is the identity on a rectangular
/// matrix. Unequal rows are a length error rather than a ragged result.
#[tokio::test]
async fn zip_is_transposition_and_is_involutive() {
    let mut once = Interpreter::new();
    once.execute("[ [ 1 2 3 ] [ 4 5 6 ] ] ZIP").await.unwrap();
    assert_eq!(
        rendered_stack(&once),
        ["[ [ 1/1 4/1 ] [ 2/1 5/1 ] [ 3/1 6/1 ] ]"]
    );

    let mut twice = Interpreter::new();
    twice
        .execute("[ [ 1 2 3 ] [ 4 5 6 ] ] ZIP ZIP")
        .await
        .unwrap();
    assert_eq!(
        rendered_stack(&twice),
        ["[ [ 1/1 2/1 3/1 ] [ 4/1 5/1 6/1 ] ]"]
    );

    let mut ragged = Interpreter::new();
    assert!(ragged.execute("[ [ 1 2 ] [ 3 ] ] ZIP").await.is_err());
}

/// `PUT` replaces one position and leaves every other alone, counting from the
/// end for a negative index as everything else does.
#[tokio::test]
async fn put_replaces_exactly_one_position() {
    let mut interpreter = Interpreter::new();
    interpreter.execute("[ 1 2 3 ] 1 9 PUT").await.unwrap();
    assert_eq!(rendered_stack(&interpreter), ["[ 1/1 9/1 3/1 ]"]);

    let mut from_end = Interpreter::new();
    from_end.execute("[ 1 2 3 ] -1 9 PUT").await.unwrap();
    assert_eq!(rendered_stack(&from_end), ["[ 1/1 2/1 9/1 ]"]);

    let mut past_end = Interpreter::new();
    assert!(past_end.execute("[ 1 2 3 ] 5 9 PUT").await.is_err());
    assert_eq!(rendered_stack(&past_end), ["[ 1/1 2/1 3/1 ]", "5/1", "9/1"]);
}

/// `GROUP` bundles by key in `UNIQUE` key order and keeps every value exactly
/// once, so a grouping never loses or duplicates data.
#[tokio::test]
async fn group_partitions_without_loss() {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute("[ 1 2 3 4 ] [ 'b' 'a' 'b' 'a' ] GROUP")
        .await
        .unwrap();
    assert_eq!(
        rendered_stack(&interpreter),
        ["[ [ 1/1 3/1 ] [ 2/1 4/1 ] ]"]
    );

    let mut mismatched = Interpreter::new();
    assert!(mismatched
        .execute("[ 1 2 3 ] [ 'a' 'b' ] GROUP")
        .await
        .is_err());
}

/// `RANDOM` is a function of its operands: the same seed draws the same
/// rationals, every time, in any interpreter. That is what lets it into a
/// language with no hidden state — and the draws are exact rationals in
/// [0, 1), so nothing about them is approximate either.
#[tokio::test]
async fn random_is_a_pure_function_of_its_seed() {
    let mut first = Interpreter::new();
    first.execute("7 4 RANDOM").await.unwrap();
    let mut again = Interpreter::new();
    again.execute("7 4 RANDOM").await.unwrap();
    assert_eq!(rendered_stack(&first), rendered_stack(&again));

    let mut other_seed = Interpreter::new();
    other_seed.execute("8 4 RANDOM").await.unwrap();
    assert_ne!(rendered_stack(&first), rendered_stack(&other_seed));

    let mut in_unit_interval = Interpreter::new();
    in_unit_interval
        .execute("7 64 RANDOM KEEP [ 0 LT ] ANY 'BELOW' BIND [ 1 GTE ] ANY")
        .await
        .unwrap();
    assert_eq!(rendered_stack(&in_unit_interval), ["FALSE"]);

    // Beyond the space water level a well-formed request projects onto NIL
    // rather than exhausting the host, the same answer RANGE and FILL give.
    let mut too_many = Interpreter::new();
    too_many.execute("7 99999999 RANDOM").await.unwrap();
    let value = too_many.get_stack().last().expect("RANDOM result");
    assert!(value.is_nil());
    assert_eq!(value.nil_reason(), Some(&NilReason::SpaceExhausted));
}
