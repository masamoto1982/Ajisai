//! Inferred contracts for user-defined words.
//!
//! Phase 1 deliberately does not add surface syntax. A user word's contract is
//! inferred from its body and resolved dependency contracts without executing
//! Ajisai code. Built-in and module contracts are projected from the existing
//! §7.14 registry; user-word contracts are widened monotonically as dependencies
//! are joined. When recursion or a dynamic structure prevents a complete proof,
//! the result is conservative rather than Ajisai's logical `UNKNOWN` value.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::coreword_registry::{get_coreword_metadata, MassContract, NilPolicy, WordPurity};
use crate::types::{Capabilities, Token, WordDefinition};

use super::word_contract_lattice::{
    widen_confidence, widen_determinism, widen_nil, widen_order, widen_purity, widen_unknown,
    widen_water,
};
use super::Interpreter;

pub const WORD_CONTRACT_SCHEMA_VERSION: u32 = 1;
pub const WORD_CONTRACT_CORE_SCHEMA_VERSION: u32 = 1;

type WordContractCache = HashMap<WordContractCacheKey, Arc<WordContract>>;
const WORD_CONTRACT_CACHE_STATE_KEY: &str = "__ajisai_word_contract_cache";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordContract {
    pub flow: ContractFlow,
    pub purity: ContractPurity,
    pub effects: Vec<String>,
    pub capabilities: Capabilities,
    pub determinism: ContractDeterminism,
    pub order_sensitivity: OrderSensitivity,
    pub nil_behavior: NilBehavior,
    pub unknown_behavior: UnknownBehavior,
    pub water_sensitivity: WaterSensitivity,
    pub confidence: ContractConfidence,
    pub cache_key: WordContractCacheKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContractFlow {
    Fixed { consumes: u16, produces: u16 },
    Dynamic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractPurity {
    Pure,
    Observable,
    Effectful,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractDeterminism {
    Deterministic,
    NonDeterministic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderSensitivity {
    OrderIndependent,
    OrderSensitive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NilBehavior {
    NeverCreates,
    Propagates,
    MayCreate,
    RejectsNil,
    ConsumesNil,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownBehavior {
    NeverCreates,
    MayCreate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterSensitivity {
    NotWaterSensitive,
    WaterSensitive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractConfidence {
    Complete,
    Conservative,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WordContractCacheKey {
    pub word_identity: String,
    pub dependency_identities: Vec<String>,
    pub core_schema_version: u32,
    pub inference_schema_version: u32,
}

impl WordContract {
    fn conservative(key: WordContractCacheKey) -> Self {
        Self {
            flow: ContractFlow::Dynamic,
            purity: ContractPurity::Effectful,
            effects: vec!["conservative".to_string()],
            capabilities: Capabilities::empty(),
            determinism: ContractDeterminism::NonDeterministic,
            order_sensitivity: OrderSensitivity::OrderSensitive,
            nil_behavior: NilBehavior::MayCreate,
            unknown_behavior: UnknownBehavior::MayCreate,
            water_sensitivity: WaterSensitivity::WaterSensitive,
            confidence: ContractConfidence::Conservative,
            cache_key: key,
        }
    }

    fn identity(name: &str) -> Self {
        let key = WordContractCacheKey {
            word_identity: format!("builtin:{name}"),
            dependency_identities: Vec::new(),
            core_schema_version: WORD_CONTRACT_CORE_SCHEMA_VERSION,
            inference_schema_version: WORD_CONTRACT_SCHEMA_VERSION,
        };
        Self {
            flow: ContractFlow::Fixed {
                consumes: 0,
                produces: 0,
            },
            purity: ContractPurity::Pure,
            effects: Vec::new(),
            capabilities: Capabilities::PURE,
            determinism: ContractDeterminism::Deterministic,
            order_sensitivity: OrderSensitivity::OrderIndependent,
            nil_behavior: NilBehavior::NeverCreates,
            unknown_behavior: UnknownBehavior::NeverCreates,
            water_sensitivity: WaterSensitivity::NotWaterSensitive,
            confidence: ContractConfidence::Complete,
            cache_key: key,
        }
    }
}

impl From<WordPurity> for ContractPurity {
    fn from(value: WordPurity) -> Self {
        match value {
            WordPurity::Pure => ContractPurity::Pure,
            WordPurity::Observable => ContractPurity::Observable,
            WordPurity::Effectful => ContractPurity::Effectful,
        }
    }
}

impl From<MassContract> for ContractFlow {
    fn from(value: MassContract) -> Self {
        match value {
            MassContract::Fixed { consumes, produces } => ContractFlow::Fixed {
                consumes: consumes.into(),
                produces: produces.into(),
            },
            MassContract::Dynamic => ContractFlow::Dynamic,
        }
    }
}

#[derive(Default)]
struct FlowAccumulator {
    dynamic: bool,
    required: u16,
    height: u16,
}

impl FlowAccumulator {
    fn push_literal(&mut self) {
        self.height = self.height.saturating_add(1);
    }

    fn apply(&mut self, flow: &ContractFlow) {
        let ContractFlow::Fixed { consumes, produces } = flow else {
            self.dynamic = true;
            return;
        };
        if self.height < *consumes {
            self.required = self.required.saturating_add(consumes - self.height);
            self.height = 0;
        } else {
            self.height -= consumes;
        }
        self.height = self.height.saturating_add(*produces);
    }

    fn finish(self) -> ContractFlow {
        if self.dynamic {
            ContractFlow::Dynamic
        } else {
            ContractFlow::Fixed {
                consumes: self.required,
                produces: self.height,
            }
        }
    }
}

#[derive(Clone)]
struct AccumulatedContract {
    flow: ContractFlow,
    purity: ContractPurity,
    effects: Vec<String>,
    capabilities: Capabilities,
    determinism: ContractDeterminism,
    order_sensitivity: OrderSensitivity,
    nil_behavior: NilBehavior,
    unknown_behavior: UnknownBehavior,
    water_sensitivity: WaterSensitivity,
    confidence: ContractConfidence,
}

impl AccumulatedContract {
    fn from_contract(contract: &WordContract) -> Self {
        Self {
            flow: contract.flow.clone(),
            purity: contract.purity,
            effects: contract.effects.clone(),
            capabilities: contract.capabilities,
            determinism: contract.determinism,
            order_sensitivity: contract.order_sensitivity,
            nil_behavior: contract.nil_behavior,
            unknown_behavior: contract.unknown_behavior,
            water_sensitivity: contract.water_sensitivity,
            confidence: contract.confidence,
        }
    }

    fn widen_with(&mut self, other: &WordContract) {
        self.purity = widen_purity(self.purity, other.purity);
        for effect in &other.effects {
            if !self.effects.contains(effect) {
                self.effects.push(effect.clone());
            }
        }
        self.capabilities = self.capabilities.union(other.capabilities);
        self.determinism = widen_determinism(self.determinism, other.determinism);
        self.order_sensitivity = widen_order(self.order_sensitivity, other.order_sensitivity);
        self.nil_behavior = widen_nil(self.nil_behavior, other.nil_behavior);
        self.unknown_behavior = widen_unknown(self.unknown_behavior, other.unknown_behavior);
        self.water_sensitivity = widen_water(self.water_sensitivity, other.water_sensitivity);
        self.confidence = widen_confidence(self.confidence, other.confidence);
    }
}

fn static_word_contract(name: &str, def: &WordDefinition) -> WordContract {
    let key = WordContractCacheKey {
        word_identity: format!("static:{}:{}", name, def.registration_order),
        dependency_identities: Vec::new(),
        core_schema_version: WORD_CONTRACT_CORE_SCHEMA_VERSION,
        inference_schema_version: WORD_CONTRACT_SCHEMA_VERSION,
    };
    let Some(meta) = get_coreword_metadata(name) else {
        return WordContract::conservative(key);
    };
    let nil_behavior = match meta.nil_policy {
        NilPolicy::Passthrough | NilPolicy::PreservesReason => NilBehavior::Propagates,
        NilPolicy::CreatesNil => NilBehavior::MayCreate,
        NilPolicy::RejectsNil => NilBehavior::RejectsNil,
        NilPolicy::ConsumesNil => NilBehavior::ConsumesNil,
    };
    let unknown_behavior = if name.eq_ignore_ascii_case("COMPARE-WITHIN") {
        UnknownBehavior::MayCreate
    } else {
        UnknownBehavior::NeverCreates
    };
    let water_sensitivity = if name.eq_ignore_ascii_case("COMPARE-WITHIN") {
        WaterSensitivity::WaterSensitive
    } else {
        WaterSensitivity::NotWaterSensitive
    };
    WordContract {
        flow: meta.mass.into(),
        purity: meta.purity.into(),
        effects: meta.effects,
        capabilities: def.capabilities,
        determinism: if meta.deterministic {
            ContractDeterminism::Deterministic
        } else {
            ContractDeterminism::NonDeterministic
        },
        order_sensitivity: OrderSensitivity::OrderIndependent,
        nil_behavior,
        unknown_behavior,
        water_sensitivity,
        confidence: ContractConfidence::Complete,
        cache_key: key,
    }
}

impl Interpreter {
    pub fn infer_word_contract(&mut self, name: &str) -> Option<Arc<WordContract>> {
        let (resolved_name, def) = self.resolve_word_entry(name)?;
        let mut visiting = HashSet::new();
        self.infer_word_contract_inner(&resolved_name, &def, &mut visiting)
    }

    pub(crate) fn clear_word_contract_cache(&mut self) {
        self.module_state.remove(WORD_CONTRACT_CACHE_STATE_KEY);
    }

    #[cfg(test)]
    pub(crate) fn word_contract_cache_len(&self) -> usize {
        self.word_contract_cache_ref()
            .map_or(0, WordContractCache::len)
    }

    fn word_contract_cache_ref(&self) -> Option<&WordContractCache> {
        self.module_state
            .get(WORD_CONTRACT_CACHE_STATE_KEY)
            .and_then(|cache| cache.downcast_ref::<WordContractCache>())
    }

    fn word_contract_cache_mut(&mut self) -> &mut WordContractCache {
        self.module_state
            .entry(WORD_CONTRACT_CACHE_STATE_KEY.to_string())
            .or_insert_with(|| Box::<WordContractCache>::default())
            .downcast_mut::<WordContractCache>()
            .expect("word contract cache state must keep its concrete type")
    }

    fn infer_word_contract_inner(
        &mut self,
        resolved_name: &str,
        def: &Arc<WordDefinition>,
        visiting: &mut HashSet<String>,
    ) -> Option<Arc<WordContract>> {
        if def.is_builtin {
            return Some(Arc::new(static_word_contract(resolved_name, def)));
        }

        let key = self.contract_cache_key(resolved_name, def);
        if let Some(cached) = self
            .word_contract_cache_ref()
            .and_then(|cache| cache.get(&key))
        {
            return Some(cached.clone());
        }

        if !visiting.insert(resolved_name.to_string()) {
            let contract = Arc::new(WordContract::conservative(key));
            self.word_contract_cache_mut()
                .insert(contract.cache_key.clone(), contract.clone());
            return Some(contract);
        }

        let mut flow = FlowAccumulator::default();
        let seed = WordContract::identity(resolved_name);
        let mut acc = AccumulatedContract::from_contract(&seed);
        let mut complete = true;

        'lines: for line in def.lines.iter() {
            for token in line.body_tokens.iter() {
                match token {
                    Token::Number(_) | Token::String(_) => flow.push_literal(),
                    Token::Symbol(symbol) => {
                        let canonical =
                            crate::core_word_aliases::canonicalize_core_word_name(symbol);
                        let Some((dep_name, dep_def)) = self.resolve_word_entry(&canonical) else {
                            complete = false;
                            flow.dynamic = true;
                            continue;
                        };
                        let dep_contract = if dep_def.is_builtin {
                            Arc::new(static_word_contract(&dep_name, &dep_def))
                        } else if visiting.contains(&dep_name) {
                            complete = false;
                            Arc::new(WordContract::conservative(
                                self.contract_cache_key(&dep_name, &dep_def),
                            ))
                        } else {
                            match self.infer_word_contract_inner(&dep_name, &dep_def, visiting) {
                                Some(contract) => contract,
                                None => {
                                    complete = false;
                                    flow.dynamic = true;
                                    continue 'lines;
                                }
                            }
                        };
                        flow.apply(&dep_contract.flow);
                        acc.widen_with(&dep_contract);
                    }
                    Token::VectorStart
                    | Token::VectorEnd
                    | Token::BlockStart
                    | Token::BlockEnd
                    | Token::Pipeline
                    | Token::NilCoalesce
                    | Token::CondClauseSep
                    | Token::LineBreak => {}
                }
            }
        }

        visiting.remove(resolved_name);
        acc.flow = flow.finish();
        if !complete {
            acc.confidence = ContractConfidence::Conservative;
        }
        let contract = Arc::new(WordContract {
            flow: acc.flow,
            purity: acc.purity,
            effects: acc.effects,
            capabilities: acc.capabilities,
            determinism: acc.determinism,
            order_sensitivity: acc.order_sensitivity,
            nil_behavior: acc.nil_behavior,
            unknown_behavior: acc.unknown_behavior,
            water_sensitivity: acc.water_sensitivity,
            confidence: acc.confidence,
            cache_key: key,
        });
        self.word_contract_cache_mut()
            .insert(contract.cache_key.clone(), contract.clone());
        Some(contract)
    }

    fn contract_cache_key(
        &self,
        resolved_name: &str,
        def: &WordDefinition,
    ) -> WordContractCacheKey {
        let mut dependency_identities: Vec<String> = def
            .dependencies
            .iter()
            .map(|dep| {
                self.word_identity(dep)
                    .cloned()
                    .unwrap_or_else(|| format!("static:{dep}"))
            })
            .collect();
        dependency_identities.sort();
        WordContractCacheKey {
            word_identity: self
                .word_identity(resolved_name)
                .cloned()
                .unwrap_or_else(|| {
                    format!("unidentified:{resolved_name}:{}", def.registration_order)
                }),
            dependency_identities,
            core_schema_version: WORD_CONTRACT_CORE_SCHEMA_VERSION,
            inference_schema_version: WORD_CONTRACT_SCHEMA_VERSION,
        }
    }
}
