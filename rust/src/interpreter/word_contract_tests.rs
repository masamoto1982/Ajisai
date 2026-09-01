//! Regression tests for inferred user-word contracts.

use std::sync::Arc;

use crate::interpreter::word_contract::*;
use crate::interpreter::Interpreter;
use crate::types::Capabilities;

async fn contract_for(src: &str, name: &str) -> Arc<WordContract> {
    let mut interp = Interpreter::new();
    interp.execute(src).await.unwrap();
    interp.infer_word_contract(name).expect("contract")
}

#[tokio::test]
async fn pure_arithmetic_user_word_is_complete_and_pure() {
    let contract = contract_for("[ [ 1 ] ADD ] 'INC' DEF", "INC").await;
    assert_eq!(contract.purity, ContractPurity::Pure);
    assert_eq!(contract.determinism, ContractDeterminism::Deterministic);
    assert_eq!(contract.nil_behavior, NilBehavior::Propagates);
    assert_eq!(
        contract.flow,
        ContractFlow::Fixed {
            consumes: 1,
            produces: 1
        }
    );
    assert_eq!(contract.confidence, ContractConfidence::Complete);
}

#[tokio::test]
async fn print_dependency_makes_user_word_effectful() {
    let contract = contract_for("[ PRINT ] 'SAY' DEF", "SAY").await;
    assert_eq!(contract.purity, ContractPurity::Effectful);
    assert!(contract.capabilities.contains(Capabilities::IO));
}
#[tokio::test]
async fn dependency_chains_widen_monotonically() {
    let pure = contract_for(
        "[ [ 1 ] ADD ] 'INC' DEF [ INC ] 'INC2' DEF [ INC2 ] 'INC3' DEF",
        "INC3",
    )
    .await;
    assert_eq!(pure.purity, ContractPurity::Pure);

    let impure = contract_for("[ PRINT ] 'A' DEF [ A ] 'B' DEF [ B ] 'C' DEF", "C").await;
    assert_eq!(impure.purity, ContractPurity::Effectful);
}

// `recursive_words_are_conservative_without_looping` tested that contract
// inference went conservative on re-entrant (direct or mutual) recursion.
// SPEC §8.7's DEF-time acyclicity check now refuses such a definition
// outright — `[ REC ] 'REC' DEF` and `[ B ] 'A' DEF [ A ] 'B' DEF` both fail
// before contract inference ever runs — so there is no longer a program that
// reaches this path.

#[tokio::test]
async fn redefinition_invalidates_old_contract_cache_key() {
    let mut interp = Interpreter::new();
    interp.execute("[ [ 1 ] ADD ] 'W' DEF").await.unwrap();
    let first = interp.infer_word_contract("W").unwrap();
    interp.execute("[ DIV ] 'W' DEF").await.unwrap();
    let second = interp.infer_word_contract("W").unwrap();
    assert_ne!(first.cache_key, second.cache_key);
    assert_eq!(second.nil_behavior, NilBehavior::MayCreate);
}

#[tokio::test]
async fn identical_content_reuses_contract_cache_entry() {
    let mut interp = Interpreter::new();
    interp
        .execute("[ [ 1 ] ADD ] 'A' DEF [ [ 1 ] ADD ] 'B' DEF")
        .await
        .unwrap();
    let a = interp.infer_word_contract("A").unwrap();
    let before = interp.word_contract_cache_len();
    let b = interp.infer_word_contract("B").unwrap();
    assert_eq!(a.cache_key, b.cache_key);
    assert_eq!(before, interp.word_contract_cache_len());
}

#[tokio::test]
async fn different_content_does_not_share_contract_cache_entry() {
    let mut interp = Interpreter::new();
    interp
        .execute("[ [ 1 ] ADD ] 'A' DEF [ [ 2 ] ADD ] 'B' DEF")
        .await
        .unwrap();
    let a = interp.infer_word_contract("A").unwrap();
    let b = interp.infer_word_contract("B").unwrap();
    assert_ne!(a.cache_key, b.cache_key);
}

#[tokio::test]
async fn del_refuses_while_a_dependent_would_be_left_dangling() {
    // A word's inferred contract cannot go stale by losing a dependency: DEL
    // fails with ERROR while any definition still depends on the target
    // (LANG.DICTIONARY.MUTATION), so there is no force path to a dangling
    // reference.
    let mut interp = Interpreter::new();
    interp
        .execute("[ DIV ] 'DEP' DEF [ DEP ] 'USE' DEF")
        .await
        .unwrap();
    let before = interp.infer_word_contract("USE").unwrap();
    assert_eq!(before.nil_behavior, NilBehavior::MayCreate);

    interp
        .execute("'DEP' DEL")
        .await
        .expect_err("DEL must refuse while USE still depends on DEP");

    let after = interp.infer_word_contract("USE").unwrap();
    assert_eq!(after.nil_behavior, NilBehavior::MayCreate);
}

// ---------------------------------------------------------------------------
// Literal-depth regression tests (`word_contract_flow.rs`).
//
// `[ ... ]` pushes exactly one value and consumes nothing; its interior is
// content, not code. Inferring otherwise reported a *correct* `#:contract`
// declaration as a proven violation.
// ---------------------------------------------------------------------------

fn fixed(consumes: u16, produces: u16) -> ContractFlow {
    ContractFlow::Fixed { consumes, produces }
}

#[tokio::test]
async fn a_vector_literal_pushes_one_value_not_its_elements() {
    let contract = contract_for("[ [ 1 2 ] ] 'PAIR' DEF", "PAIR").await;
    assert_eq!(contract.flow, fixed(0, 1));
    assert_eq!(contract.confidence, ContractConfidence::Complete);
}

#[tokio::test]
async fn a_vector_literal_operand_is_consumed_like_any_other_value() {
    let contract = contract_for("[ [ 10 20 ] ADD ] 'ADD-PAIR' DEF", "ADD-PAIR").await;
    assert_eq!(contract.flow, fixed(1, 1));
    assert_eq!(contract.confidence, ContractConfidence::Complete);
}

#[tokio::test]
async fn a_code_block_literal_is_not_inlined_into_the_arity() {
    // The block's `2 MUL` is evaluated only when MAP runs it, so it must not
    // consume an operand at the point the block is written. Inlining it made
    // this word's arity ( 2 -- 1 ).
    let contract = contract_for("[ [ 2 MUL ] MAP ] 'DOUBLE-ALL' DEF", "DOUBLE-ALL").await;
    assert_eq!(contract.flow, fixed(1, 1));
    assert_eq!(contract.confidence, ContractConfidence::Complete);
}

#[tokio::test]
async fn nested_literals_still_push_exactly_one_value() {
    for source in [
        "[ [ [ 1 ] [ 2 ] ] ] 'W' DEF",
        "[ [ [ 1 ] ] ] 'W' DEF",
        "[ [ [ [ 1 ] ] ] ] 'W' DEF",
    ] {
        let contract = contract_for(source, "W").await;
        assert_eq!(contract.flow, fixed(0, 1), "source: {source}");
    }
}

#[tokio::test]
async fn a_symbol_inside_a_vector_literal_is_content_not_a_call() {
    // At run time `[ 'a' PRINT 'b' ]` is the three-element vector
    // [ 'a' 'PRINT' 'b' ], so PRINT's arity must not be applied here.
    let contract = contract_for("[ [ 'a' PRINT 'b' ] ] 'LABELS' DEF", "LABELS").await;
    assert_eq!(contract.flow, fixed(0, 1));
}

#[tokio::test]
async fn or_nil_leaves_the_flow_unmodelled_rather_than_wrong() {
    // `OR-NIL` selects between paths of differing height, so no fixed arity
    // describes it. Reported as a gap, which can only ever produce a note.
    let contract = contract_for("[ 1 0 DIV OR-NIL 9 ] 'FALLBACK' DEF", "FALLBACK").await;
    assert_eq!(contract.flow, ContractFlow::Dynamic);
    assert_eq!(contract.confidence, ContractConfidence::Conservative);
    assert!(contract
        .gaps
        .contains(&crate::agent::contract_gap::GapCode::UnmodelledControlFlow));
}

#[tokio::test]
async fn keep_is_applied_as_a_modifier_not_as_an_arity() {
    // `KEEP`'s registry arity is ( 0 -- 0 ); it makes the *next* Word read its
    // operands without consuming them. Each expectation below is the stack the
    // body actually leaves at run time.
    for (body, consumes, produces) in [
        // `2 3 KEEP ADD` leaves `2 3 5`.
        ("KEEP ADD", 2, 3),
        ("2 3 KEEP ADD", 0, 3),
        // A second KEEP is idempotent, and the flag survives an intervening
        // literal, so `2 3 KEEP 4 ADD` leaves `2 3 4 7`.
        ("2 3 KEEP KEEP ADD", 0, 3),
        ("2 3 KEEP 4 ADD", 0, 4),
        ("[ 1 2 ] KEEP SUM", 0, 2),
        // The modifier reaches exactly one Word: `2 3 KEEP ADD ADD` leaves `2 8`.
        ("2 3 KEEP ADD ADD", 0, 2),
        // Pending at the end of a body is a no-op, as it is at run time.
        ("1 KEEP", 0, 1),
    ] {
        let source = format!("[ {body} ] 'W' DEF");
        let contract = contract_for(&source, "W").await;
        assert_eq!(contract.flow, fixed(consumes, produces), "body: {body}");
        assert_eq!(
            contract.confidence,
            ContractConfidence::Complete,
            "body: {body}"
        );
    }
}

// ---------------------------------------------------------------------------
// Code-operand classification regression tests (`word_contract_widen.rs`).
//
// `{ }` no longer exists, so a `[ ... ]` alone cannot say whether its
// interior is inert data or a fixed-position code operand a higher-order
// Word (`MAP`/`FILTER`/`FOLD`/`ANY`/`ALL`/`EXEC`/`PROBE`/`COND`) will
// actually run. `classify_vector_positions` answers this positionally: a
// `[ ... ]` immediately followed by one of those Words is code, everything
// else is data, and a `Data` ancestor forces `Data` all the way down. Every
// expectation below is the `output` the body actually produces at run time.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_symbol_inside_a_vector_literal_never_widens_purity() {
    // `[ 'a' PRINT 'b' ]` *is* `[ 'a' 'PRINT' 'b' ]` — PRINT never runs.
    let contract = contract_for("[ [ 'a' PRINT 'b' ] ] 'LABELS' DEF", "LABELS").await;
    assert_eq!(contract.purity, ContractPurity::Pure);
    assert_eq!(contract.confidence, ContractConfidence::Complete);
}

#[tokio::test]
async fn a_block_written_inside_a_vector_literal_is_quoted_but_never_run() {
    // A `[ ... ]` nested inside inert data is inert data too, whatever it
    // looks like and however deep it nests: building the outer vector never
    // runs anything nested inside it, and nothing here is followed by a
    // higher-order Word that would make it a code operand.
    for body in [
        "[ PRINT ]",
        "[ [ PRINT ] ]",
        "[ [ PRINT ] [ 1 ] ]",
        "[ [ [ [ PRINT ] 0 GET EXEC ] ] 0 GET EXEC ]",
    ] {
        let source = format!("[ {body} ] 'W' DEF");
        let contract = contract_for(&source, "W").await;
        assert_eq!(contract.purity, ContractPurity::Pure, "body: {body}");
        assert_eq!(
            contract.confidence,
            ContractConfidence::Complete,
            "body: {body}"
        );
    }
}

#[tokio::test]
async fn a_value_only_reaching_exec_through_collect_is_a_known_gap() {
    // Before `{ }` was retired, `{ PRINT }` written outside any vector
    // widened unconditionally, so `COLLECT`ing it and later `GET`+`EXEC`ing
    // it (measured: `'hi' { PRINT } 1 COLLECT [ 0 ] GET EXEC` printed "hi")
    // still counted as effectful. `classify_vector_positions` instead asks
    // whether a code-consuming Word immediately follows a literal's own
    // close — sound for the ordinary `MAP`/`FILTER`/`EXEC`/`COND` shapes,
    // but `COLLECT` gathers already-evaluated stack values built arbitrarily
    // far away, which no positional rule can see. This is an accepted,
    // narrower guarantee, not a regression target: the declaration check
    // stays conservative-by-omission (a missed `note`, never a false
    // `error`), and a value actually reaching `EXEC` this way is unusual
    // enough that recovering it is not worth another special case.
    let contract = contract_for("[ [ PRINT ] 1 COLLECT [ 0 ] GET EXEC ] 'W' DEF", "W").await;
    assert_eq!(contract.purity, ContractPurity::Pure);
}

#[tokio::test]
async fn a_map_over_a_literal_block_still_widens_regardless_of_purity() {
    // Sanity check that the code-operand classification leaves the ordinary,
    // no-vector-involved case exactly as before.
    let pure = contract_for("[ [ 2 MUL ] MAP ] 'DOUBLE-ALL' DEF", "DOUBLE-ALL").await;
    assert_eq!(pure.purity, ContractPurity::Pure);
    let effectful = contract_for("[ [ PRINT ] MAP ] 'PRINTALL' DEF", "PRINTALL").await;
    assert_eq!(effectful.purity, ContractPurity::Effectful);
}
