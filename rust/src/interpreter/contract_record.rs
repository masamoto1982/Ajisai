//! A Word's contract as a Record — the one shape `CONTRACT` and `PROBE` answer
//! (LANG.CONTRACT.REGISTRY, LANG.CONTRACT.CHECK).
//!
//! Two sources feed it. A Core Word's contract is *registered*: the record in
//! `spec/words.json`, reached through the generated registry, so every field
//! below is the specification's own spelling and nothing is restated by hand.
//! A User Word's or a block's contract is *inferred* by `word_contract`
//! without running anything. The two carry different facts — only a registry
//! declares consumption, projection reasons and ERROR conditions; only an
//! inference has a confidence and gaps — so the Records differ in the keys
//! only one side can supply, and agree on every key both can: `inputs`,
//! `outputs`, `nil`, `purity`, `determinism`, `cost`, `effects`. A program
//! that asks `'purity' AT` of either gets an answer.

use crate::interpreter::word_contract::{
    ContractConfidence, ContractDeterminism, ContractFlow, ContractPurity, NilBehavior,
    WordContract,
};
use crate::interpreter::word_cost::CostBound;
use crate::kernel::generated::{Arity, CostClass, GeneratedWord};
use crate::types::{RecordData, Value};

fn text(s: &str) -> Value {
    Value::from_string(s)
}

fn texts(items: impl IntoIterator<Item = impl AsRef<str>>) -> Value {
    Value::from_vector(items.into_iter().map(|s| text(s.as_ref())).collect())
}

fn record(pairs: Vec<(&str, Value)>) -> Value {
    let (keys, values): (Vec<Value>, Vec<Value>) =
        pairs.into_iter().map(|(k, v)| (text(k), v)).unzip();
    Value::from_record(
        RecordData::new(keys, values).expect("contract keys are distinct by construction"),
    )
}

fn arity(arity: Arity) -> Value {
    match arity {
        Arity::Fixed(n) => Value::from_int(i64::from(n)),
        Arity::Variable => text("variable"),
        Arity::Control => text("control"),
    }
}

fn cost_record(steps: CostClass, numeric: CostClass, collection: CostClass) -> Value {
    record(vec![
        ("steps", text(steps.as_spec_str())),
        ("numeric", text(numeric.as_spec_str())),
        ("collection", text(collection.as_spec_str())),
    ])
}

/// The registered contract of a Core Word, field for field from the registry.
pub(crate) fn registered_contract_record(word: &GeneratedWord) -> Value {
    record(vec![
        ("name", text(word.name)),
        ("tier", text(word.vocabulary_tier.as_spec_str())),
        ("inputs", arity(word.stack_inputs)),
        ("outputs", arity(word.stack_outputs)),
        ("consumption", text(word.consumption.as_spec_str())),
        ("nil", text(word.nil_policy.as_spec_str())),
        ("projection", texts(word.projection_reasons)),
        ("errors", texts(word.error_when)),
        ("partiality", text(word.partiality.as_spec_str())),
        ("purity", text(word.purity.as_spec_str())),
        ("determinism", text(word.determinism.as_spec_str())),
        (
            "cost",
            cost_record(
                word.cost.steps.class,
                word.cost.numeric.class,
                word.cost.collection.class,
            ),
        ),
        ("effects", texts(word.effects)),
    ])
}

fn purity_str(purity: ContractPurity) -> &'static str {
    match purity {
        ContractPurity::Pure => "pure",
        ContractPurity::Observable => "observable",
        ContractPurity::Effectful => "effectful",
    }
}

fn determinism_str(determinism: ContractDeterminism) -> &'static str {
    match determinism {
        ContractDeterminism::Deterministic => "deterministic",
        ContractDeterminism::NonDeterministic => "nonDeterministic",
    }
}

fn nil_str(nil: NilBehavior) -> &'static str {
    match nil {
        NilBehavior::NeverCreates => "neverCreates",
        NilBehavior::Propagates => "propagates",
        NilBehavior::MayCreate => "mayCreate",
        NilBehavior::RejectsNil => "rejectsNil",
        NilBehavior::ConsumesNil => "consumesNil",
    }
}

fn confidence_str(confidence: ContractConfidence) -> &'static str {
    match confidence {
        ContractConfidence::Complete => "complete",
        ContractConfidence::Conservative => "conservative",
    }
}

fn flow(flow: &ContractFlow) -> (Value, Value) {
    match flow {
        ContractFlow::Fixed { consumes, produces } => (
            Value::from_int(i64::from(*consumes)),
            Value::from_int(i64::from(*produces)),
        ),
        ContractFlow::Dynamic => (text("variable"), text("variable")),
    }
}

/// The inferred contract of a User Word or a block. The checkable subset a
/// `#:contract` declaration verifies against (purity, nil behavior, arity)
/// plus what makes *cannot verify* readable as data rather than as an opaque
/// failure: `confidence` and `gaps` (LANG.CONTRACT.CHECK's trichotomy).
pub(crate) fn inferred_contract_record(contract: &WordContract) -> Value {
    let (inputs, outputs) = flow(&contract.flow);
    let CostBound {
        steps,
        numeric,
        collection,
    } = contract.cost;
    record(vec![
        ("inputs", inputs),
        ("outputs", outputs),
        ("nil", text(nil_str(contract.nil_behavior))),
        ("purity", text(purity_str(contract.purity))),
        ("determinism", text(determinism_str(contract.determinism))),
        ("cost", cost_record(steps.0, numeric.0, collection.0)),
        ("effects", texts(&contract.effects)),
        ("confidence", text(confidence_str(contract.confidence))),
        ("gaps", texts(contract.gaps.iter().map(|gap| gap.as_str()))),
    ])
}
