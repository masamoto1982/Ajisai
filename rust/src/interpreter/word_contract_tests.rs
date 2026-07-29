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
    let contract = contract_for("{ [ 1 ] ADD } 'INC' DEF", "INC").await;
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
    let contract = contract_for("{ PRINT } 'SAY' DEF", "SAY").await;
    assert_eq!(contract.purity, ContractPurity::Effectful);
    assert!(contract.capabilities.contains(Capabilities::IO));
}

#[tokio::test]
async fn now_dependency_makes_user_word_observable_and_nondeterministic() {
    let contract = contract_for("{ NOW } 'STAMP' DEF", "STAMP").await;
    assert_eq!(contract.purity, ContractPurity::Observable);
    assert_eq!(contract.determinism, ContractDeterminism::NonDeterministic);
    assert!(contract.capabilities.contains(Capabilities::TIME));
}
#[tokio::test]
async fn dependency_chains_widen_monotonically() {
    let pure = contract_for(
        "{ [ 1 ] ADD } 'INC' DEF { INC } 'INC2' DEF { INC2 } 'INC3' DEF",
        "INC3",
    )
    .await;
    assert_eq!(pure.purity, ContractPurity::Pure);

    let impure = contract_for("{ PRINT } 'A' DEF { A } 'B' DEF { B } 'C' DEF", "C").await;
    assert_eq!(impure.purity, ContractPurity::Effectful);
}

#[tokio::test]
async fn recursive_words_are_conservative_without_looping() {
    let direct = contract_for("{ REC } 'REC' DEF", "REC").await;
    assert_eq!(direct.confidence, ContractConfidence::Conservative);
    assert_eq!(direct.flow, ContractFlow::Dynamic);

    let mutual = contract_for("{ B } 'A' DEF { A } 'B' DEF", "A").await;
    assert_eq!(mutual.confidence, ContractConfidence::Conservative);
    assert_eq!(mutual.flow, ContractFlow::Dynamic);
}

#[tokio::test]
async fn redefinition_invalidates_old_contract_cache_key() {
    let mut interp = Interpreter::new();
    interp.execute("{ [ 1 ] ADD } 'W' DEF").await.unwrap();
    let first = interp.infer_word_contract("W").unwrap();
    interp.execute("{ DIV } 'W' DEF").await.unwrap();
    let second = interp.infer_word_contract("W").unwrap();
    assert_ne!(first.cache_key, second.cache_key);
    assert_eq!(second.nil_behavior, NilBehavior::MayCreate);
}

#[tokio::test]
async fn identical_content_reuses_contract_cache_entry() {
    let mut interp = Interpreter::new();
    interp
        .execute("{ [ 1 ] ADD } 'A' DEF { [ 1 ] ADD } 'B' DEF")
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
        .execute("{ [ 1 ] ADD } 'A' DEF { [ 2 ] ADD } 'B' DEF")
        .await
        .unwrap();
    let a = interp.infer_word_contract("A").unwrap();
    let b = interp.infer_word_contract("B").unwrap();
    assert_ne!(a.cache_key, b.cache_key);
}

#[tokio::test]
async fn del_invalidates_dependency_contract_to_conservative() {
    let mut interp = Interpreter::new();
    interp
        .execute("{ DIV } 'DEP' DEF { DEP } 'USE' DEF")
        .await
        .unwrap();
    let before = interp.infer_word_contract("USE").unwrap();
    assert_eq!(before.nil_behavior, NilBehavior::MayCreate);

    interp.execute("! 'DEP' DEL").await.unwrap();
    let after = interp.infer_word_contract("USE").unwrap();
    assert_eq!(after.confidence, ContractConfidence::Conservative);
    assert_eq!(after.flow, ContractFlow::Dynamic);
}
