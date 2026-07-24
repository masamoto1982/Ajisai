//! Regression tests for inferred static space bounds (Phase 2.2;
//! `word_space.rs`). Each asserts both the class *and* the exactness witness,
//! because it is the witness — not the class alone — that licenses the
//! declaration checker to raise an `error` rather than a note.

use crate::interpreter::word_space::SpaceClass;
use crate::interpreter::Interpreter;

async fn space_of(src: &str, name: &str) -> (SpaceClass, bool) {
    let mut interp = Interpreter::new();
    interp.execute(src).await.unwrap();
    let contract = interp.infer_word_contract(name).expect("contract");
    (contract.space, contract.space_exact)
}

#[tokio::test]
async fn literal_operand_range_is_const_and_exact() {
    // The marquee case: `[ 0 10 ] RANGE` materializes a length set by a
    // compile-time-literal pair, so its footprint is input-independent.
    let (class, exact) = space_of("{ [ 0 10 ] RANGE } 'A' DEF", "A").await;
    assert_eq!(class, SpaceClass::Const);
    assert!(exact, "a literal-driven RANGE has a provably const footprint");
}

#[tokio::test]
async fn input_operand_range_is_unbounded_and_exact() {
    // A bare `RANGE` materializes a length set by the *value* of its input
    // pair, not by input size — provably unbounded over input size.
    let (class, exact) = space_of("{ RANGE } 'B' DEF", "B").await;
    assert_eq!(class, SpaceClass::Unbounded);
    assert!(
        exact,
        "an input-driven RANGE provably exceeds any size-bounded class"
    );
}

#[tokio::test]
async fn literal_operand_fill_is_const_but_input_fill_is_unbounded() {
    let (lit_class, lit_exact) = space_of("{ [ 2 2 0 ] FILL } 'F' DEF", "F").await;
    assert_eq!(lit_class, SpaceClass::Const);
    assert!(lit_exact);

    let (in_class, in_exact) = space_of("{ FILL } 'G' DEF", "G").await;
    assert_eq!(in_class, SpaceClass::Unbounded);
    assert!(in_exact);
}

#[tokio::test]
async fn elementwise_arith_over_input_is_linear_and_exact() {
    // `[ 1 ] ADD` broadcasts against an input vector, materializing an
    // output the size of that input in the worst case.
    let (class, exact) = space_of("{ [ 1 ] ADD } 'INC' DEF", "INC").await;
    assert_eq!(class, SpaceClass::Linear);
    assert!(exact);
}

#[tokio::test]
async fn all_literal_body_is_const() {
    let (class, exact) = space_of("{ [ 42 ] } 'MK' DEF", "MK").await;
    assert_eq!(class, SpaceClass::Const);
    assert!(exact);
}

#[tokio::test]
async fn observation_word_is_const() {
    // GET shares persistent structure: O(1) new materialization.
    let (class, _) = space_of("{ GET } 'PICK' DEF", "PICK").await;
    assert_eq!(class, SpaceClass::Const);
}

#[tokio::test]
async fn higher_order_word_is_unbounded_but_not_exact() {
    // MAP runs a caller-supplied body a data-dependent number of times: the
    // sound upper bound is unbounded, but it is *not* a proven violation of a
    // tighter declaration (the body could be trivial), so it stays inexact —
    // a note, never a false error.
    let (class, exact) = space_of("{ [ 1 ] MAP } 'M' DEF", "M").await;
    assert_eq!(class, SpaceClass::Unbounded);
    assert!(
        !exact,
        "a higher-order upper bound must not be a proven witness"
    );
}

#[tokio::test]
async fn recursion_is_conservative_unbounded_without_a_witness() {
    let (class, exact) = space_of("{ REC } 'REC' DEF", "REC").await;
    assert_eq!(class, SpaceClass::Unbounded);
    assert!(!exact);
}

#[tokio::test]
async fn unresolved_dependency_degrades_without_a_false_witness() {
    // `DUP` is not an Ajisai word: an unresolved symbol poisons provenance, so
    // the literal operand can no longer pin the RANGE — the bound widens to
    // unbounded but *without* an exactness witness (no false error).
    let (class, exact) = space_of("{ [ 0 10 ] DUP RANGE } 'C' DEF", "C").await;
    assert_eq!(class, SpaceClass::Unbounded);
    assert!(!exact);
}

#[tokio::test]
async fn const_chain_composes_through_a_user_word() {
    // A user word wrapping a literal RANGE is const; a word calling it stays
    // const (the dependency's proven const bound composes).
    let (class, exact) =
        space_of("{ [ 0 10 ] RANGE } 'BASE' DEF { BASE } 'WRAP' DEF", "WRAP").await;
    assert_eq!(class, SpaceClass::Const);
    assert!(exact);
}
