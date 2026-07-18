//! Monotone joins for inferred word-contract facets.

use super::word_contract::{
    ContractConfidence, ContractDeterminism, ContractPurity, NilBehavior, OrderSensitivity,
    UnknownBehavior, WaterSensitivity,
};

pub(super) fn widen_purity(a: ContractPurity, b: ContractPurity) -> ContractPurity {
    match (a, b) {
        (ContractPurity::Effectful, _) | (_, ContractPurity::Effectful) => {
            ContractPurity::Effectful
        }
        (ContractPurity::Observable, _) | (_, ContractPurity::Observable) => {
            ContractPurity::Observable
        }
        _ => ContractPurity::Pure,
    }
}

pub(super) fn widen_determinism(
    a: ContractDeterminism,
    b: ContractDeterminism,
) -> ContractDeterminism {
    if matches!(a, ContractDeterminism::NonDeterministic)
        || matches!(b, ContractDeterminism::NonDeterministic)
    {
        ContractDeterminism::NonDeterministic
    } else {
        ContractDeterminism::Deterministic
    }
}

pub(super) fn widen_order(a: OrderSensitivity, b: OrderSensitivity) -> OrderSensitivity {
    if matches!(a, OrderSensitivity::OrderSensitive)
        || matches!(b, OrderSensitivity::OrderSensitive)
    {
        OrderSensitivity::OrderSensitive
    } else {
        OrderSensitivity::OrderIndependent
    }
}

pub(super) fn widen_nil(a: NilBehavior, b: NilBehavior) -> NilBehavior {
    use NilBehavior::*;
    match (a, b) {
        (MayCreate, _) | (_, MayCreate) => MayCreate,
        (RejectsNil, _) | (_, RejectsNil) => RejectsNil,
        (ConsumesNil, _) | (_, ConsumesNil) => ConsumesNil,
        (Propagates, _) | (_, Propagates) => Propagates,
        _ => NeverCreates,
    }
}

pub(super) fn widen_unknown(a: UnknownBehavior, b: UnknownBehavior) -> UnknownBehavior {
    if matches!(a, UnknownBehavior::MayCreate) || matches!(b, UnknownBehavior::MayCreate) {
        UnknownBehavior::MayCreate
    } else {
        UnknownBehavior::NeverCreates
    }
}

pub(super) fn widen_water(a: WaterSensitivity, b: WaterSensitivity) -> WaterSensitivity {
    if matches!(a, WaterSensitivity::WaterSensitive)
        || matches!(b, WaterSensitivity::WaterSensitive)
    {
        WaterSensitivity::WaterSensitive
    } else {
        WaterSensitivity::NotWaterSensitive
    }
}

pub(super) fn widen_confidence(a: ContractConfidence, b: ContractConfidence) -> ContractConfidence {
    if matches!(a, ContractConfidence::Conservative)
        || matches!(b, ContractConfidence::Conservative)
    {
        ContractConfidence::Conservative
    } else {
        ContractConfidence::Complete
    }
}
