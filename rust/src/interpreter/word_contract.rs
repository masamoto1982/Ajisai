//! Inferred contracts for user-defined words.
//!
//! Phase 1 deliberately does not add surface syntax. A user word's contract
//! is inferred from its body and resolved dependency contracts without
//! executing Ajisai code. Built-in contracts are projected from the existing
//! §7.14 registry; user-word contracts widen monotonically as dependencies
//! join. When recursion or a dynamic structure prevents a complete proof,
//! the result is conservative rather than Ajisai's logical `UNKNOWN` value.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::agent::contract_gap::GapCode;
use crate::coreword_registry::{
    get_coreword_metadata, Determinism, MassContract, NilPolicy, Purity,
};
use crate::types::{Capabilities, Token, WordDefinition};

use super::word_contract_lattice::{
    widen_confidence, widen_determinism, widen_nil, widen_order, widen_purity,
};
use super::word_cost::{CostBound, CostSim, DepCost};
use super::word_space::{DepSpace, SpaceBound, SpaceClass, SpaceSim};
use super::Interpreter;

pub const WORD_CONTRACT_SCHEMA_VERSION: u32 = 2;
pub const WORD_CONTRACT_CORE_SCHEMA_VERSION: u32 = 2;

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
    /// Sound upper bound on the word's space growth (Phase 2.2; `word_space`).
    pub space: SpaceClass,
    /// True when the bound is provably attained, licensing a declaration error.
    pub space_exact: bool,
    /// Sound upper bound on the word's charged time cost (`ResourceUsage`'s
    /// three axes; Phase 5, `word_cost`). `pub(crate)` because `CostBound` is.
    pub(crate) cost: CostBound,
    pub confidence: ContractConfidence,
    /// Why inference could not fully verify this word when `confidence` is
    /// `Conservative` (empty when `Complete`, Phase 3); `pub(crate)` because `GapCode` is.
    pub(crate) gaps: Vec<GapCode>,
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

/// A cache key for a builtin/leaf contract: no dependencies, current schema.
fn leaf_cache_key(word_identity: String) -> WordContractCacheKey {
    WordContractCacheKey {
        word_identity,
        dependency_identities: Vec::new(),
        core_schema_version: WORD_CONTRACT_CORE_SCHEMA_VERSION,
        inference_schema_version: WORD_CONTRACT_SCHEMA_VERSION,
    }
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
            space: SpaceClass::Unbounded,
            space_exact: false,
            cost: CostBound::CONSERVATIVE,
            confidence: ContractConfidence::Conservative,
            gaps: vec![GapCode::ConservativeSeed],
            cache_key: key,
        }
    }

    fn identity(name: &str) -> Self {
        let key = leaf_cache_key(format!("builtin:{name}"));
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
            space: SpaceClass::Const,
            space_exact: true,
            cost: CostBound::IDENTITY,
            confidence: ContractConfidence::Complete,
            gaps: Vec::new(),
            cache_key: key,
        }
    }
}

impl From<Purity> for ContractPurity {
    fn from(value: Purity) -> Self {
        match value {
            Purity::Pure => ContractPurity::Pure,
            // `conditional` means the Word is as pure as the block it is
            // given. The inference walk already visits the block's symbols
            // and widens with them, so charging the Word itself would
            // double-count — a `MAP` over a pure block is pure, and over an
            // effectful one the block's own effects make the caller effectful.
            Purity::Conditional => ContractPurity::Pure,
            Purity::Observational => ContractPurity::Observable,
            Purity::Effectful => ContractPurity::Effectful,
        }
    }
}

impl From<Determinism> for ContractDeterminism {
    fn from(value: Determinism) -> Self {
        match value {
            Determinism::Deterministic => ContractDeterminism::Deterministic,
            // Both non-deterministic classes collapse here: the lattice asks
            // only whether a result is reproducible from its operands.
            Determinism::StateRelative | Determinism::HostRelative => {
                ContractDeterminism::NonDeterministic
            }
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
    confidence: ContractConfidence,
    gaps: Vec<GapCode>,
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
            confidence: contract.confidence,
            gaps: contract.gaps.clone(),
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
        self.confidence = widen_confidence(self.confidence, other.confidence);
        // Incompleteness propagates like a NIL reason; canonicalized once at
        // the end of accumulation, not per widen.
        self.gaps.extend(other.gaps.iter().copied());
    }
}

fn static_word_contract(name: &str, def: &WordDefinition) -> WordContract {
    let key = leaf_cache_key(format!("static:{}:{}", name, def.registration_order));
    let Some(meta) = get_coreword_metadata(name) else {
        return WordContract::conservative(key);
    };
    let nil_behavior = match meta.nil_policy {
        NilPolicy::Passthrough | NilPolicy::PreserveReason => NilBehavior::Propagates,
        // `passthroughThenProject` does both: a NIL operand flows through,
        // and a well-formed operand may still project onto one — `MayCreate`
        // is the wider of the two, the one a caller has to plan for.
        NilPolicy::PassthroughThenProject | NilPolicy::CreatesNil => NilBehavior::MayCreate,
        NilPolicy::RejectNil => NilBehavior::RejectsNil,
        // `inspectNil` reads NIL-ness rather than propagating it — `ConsumesNil`.
        NilPolicy::ConsumeNil | NilPolicy::InspectNil => NilBehavior::ConsumesNil,
    };
    let (space, space_exact) = super::word_space::builtin_space_for(name);
    let cost = super::word_cost::builtin_cost_for(name);
    WordContract {
        flow: meta.mass.into(),
        purity: meta.purity.into(),
        effects: meta.effects,
        capabilities: def.capabilities,
        determinism: meta.determinism.into(),
        order_sensitivity: OrderSensitivity::OrderIndependent,
        nil_behavior,
        space,
        space_exact,
        cost,
        confidence: ContractConfidence::Complete,
        gaps: Vec::new(),
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
        self.runtime_scratch.remove(WORD_CONTRACT_CACHE_STATE_KEY);
    }

    #[cfg(test)]
    pub(crate) fn word_contract_cache_len(&self) -> usize {
        self.word_contract_cache_ref()
            .map_or(0, WordContractCache::len)
    }

    fn word_contract_cache_ref(&self) -> Option<&WordContractCache> {
        self.runtime_scratch
            .get(WORD_CONTRACT_CACHE_STATE_KEY)
            .and_then(|cache| cache.downcast_ref::<WordContractCache>())
    }

    fn word_contract_cache_mut(&mut self) -> &mut WordContractCache {
        self.runtime_scratch
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
        if let Some(cached) = self.word_contract_cache_ref().and_then(|c| c.get(&key)) {
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
        let mut sim = SpaceSim::new();
        let mut cost_sim = CostSim::new();
        let mut complete = true;

        'lines: for line in def.lines.iter() {
            for token in line.body_tokens.iter() {
                match token {
                    Token::Number(_) | Token::String(_) => {
                        flow.push_literal();
                        sim.feed_literal();
                        cost_sim.feed_literal();
                    }
                    Token::Symbol(symbol) => {
                        let canonical =
                            crate::core_word_aliases::canonicalize_core_word_name(symbol);
                        let Some((dep_name, dep_def)) = self.resolve_word_entry(&canonical) else {
                            complete = false;
                            flow.dynamic = true;
                            sim.feed_unresolved();
                            cost_sim.feed_unresolved();
                            acc.gaps.push(GapCode::UnresolvedWord);
                            continue;
                        };
                        let dep_contract = if dep_def.is_builtin {
                            Arc::new(static_word_contract(&dep_name, &dep_def))
                        } else if visiting.contains(&dep_name) {
                            complete = false;
                            acc.gaps.push(GapCode::RecursiveDependency);
                            // Cleared, not merged: incompleteness here is
                            // attributed above, not the placeholder's own seed.
                            let mut placeholder = WordContract::conservative(
                                self.contract_cache_key(&dep_name, &dep_def),
                            );
                            placeholder.gaps.clear();
                            Arc::new(placeholder)
                        } else {
                            match self.infer_word_contract_inner(&dep_name, &dep_def, visiting) {
                                Some(contract) => contract,
                                None => {
                                    complete = false;
                                    flow.dynamic = true;
                                    sim.abandon_line();
                                    cost_sim.abandon_line();
                                    acc.gaps.push(GapCode::DependencyUnknown);
                                    continue 'lines;
                                }
                            }
                        };
                        flow.apply(&dep_contract.flow);
                        sim.feed_word(&if dep_def.is_builtin {
                            DepSpace::of_builtin(&dep_name, &dep_contract)
                        } else {
                            DepSpace::of_user_word(&dep_contract)
                        });
                        cost_sim.feed_word(&if dep_def.is_builtin {
                            DepCost::of_builtin(&dep_name)
                        } else {
                            DepCost::of_user_word(&dep_contract)
                        });
                        acc.widen_with(&dep_contract);
                    }
                    Token::VectorStart
                    | Token::VectorEnd
                    | Token::BlockStart
                    | Token::BlockEnd
                    | Token::NilCoalesce
                    | Token::CondClauseSep
                    | Token::LineBreak => {
                        sim.feed_structural(token);
                        cost_sim.feed_structural(token);
                    }
                }
            }
        }

        visiting.remove(resolved_name);
        acc.flow = flow.finish();
        let SpaceBound {
            class: space,
            exact: space_exact,
        } = sim.finish();
        let cost = cost_sim.finish();
        if !complete {
            acc.confidence = ContractConfidence::Conservative;
        }
        // The gap set is a set, not a log: canonical order, not visit order.
        acc.gaps.sort();
        acc.gaps.dedup();
        let contract = Arc::new(WordContract {
            flow: acc.flow,
            purity: acc.purity,
            effects: acc.effects,
            capabilities: acc.capabilities,
            determinism: acc.determinism,
            order_sensitivity: acc.order_sensitivity,
            nil_behavior: acc.nil_behavior,
            space,
            space_exact,
            cost,
            confidence: acc.confidence,
            gaps: acc.gaps,
            cache_key: key,
        });
        self.word_contract_cache_mut()
            .insert(contract.cache_key.clone(), contract.clone());
        Some(contract)
    }

    fn contract_cache_key(&self, name: &str, def: &WordDefinition) -> WordContractCacheKey {
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
                .word_identity(name)
                .cloned()
                .unwrap_or_else(|| format!("unidentified:{name}:{}", def.registration_order)),
            dependency_identities,
            core_schema_version: WORD_CONTRACT_CORE_SCHEMA_VERSION,
            inference_schema_version: WORD_CONTRACT_SCHEMA_VERSION,
        }
    }
}
