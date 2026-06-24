//! Static mass-conservation validator (SPEC §13.1).
//!
//! SPEC §13.1 makes flow-mass conservation a compile/JIT/load-time property: a
//! Coreword Contract declares arity / consumption / production / bifurcation,
//! and flow-accounting failures (over-consumption, unconsumed leaks, flow
//! breaks, bifurcation-ratio violations) must be reported by the compiler/JIT,
//! loader, or **developer diagnostics** before an optimized path executes. The
//! optimized path itself already exists (`compiled_plan::execute_compiled_plan`,
//! gated by `is_plan_valid`).
//!
//! This module supplies the missing piece: a pure, diagnostic-only abstract
//! interpretation over a [`CompiledPlan`]. It reads each word's static
//! [`MassContract`](crate::coreword_registry::MassContract) (the §7.14 contract
//! field) and tracks abstract stack depth, reporting over-consumption. It does
//! **not** gate execution and the ordinary runtime keeps no per-value
//! `FlowToken` (SPEC §13.1); it is a load-time/diagnostic check.
//!
//! The validator abstains (`all_known = false`) the moment it meets a word
//! whose arity is not statically pinned (`Dynamic`: `STAK` count-folds, vector
//! ops, user words, fallbacks), since the abstract depth is then unreliable.

use super::compiled_plan::{compile_word_definition, CompiledOp, CompiledPlan};
use super::Interpreter;
use crate::coreword_registry::mass_contract;
use crate::types::{Capabilities, ExecutionLine, Stability, Tier, WordDefinition};

/// Result of statically analyzing a flow for mass conservation. Depths are
/// measured from an empty starting stack over the statically-known prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MassReport {
    /// Net stack-depth change contributed by the analyzed (known) prefix.
    pub net_mass: i64,
    /// Lowest abstract depth reached over the known prefix. Negative means the
    /// flow reads more operands than it has produced/been given — i.e. it can
    /// only run against a pre-existing stack of at least `-min_depth` items.
    pub min_depth: i64,
    /// `false` once a `Dynamic`-arity word is reached; the analysis then froze.
    pub all_known: bool,
}

impl MassReport {
    /// A flow is self-contained mass-conserving when, from an empty stack, it
    /// never over-consumes (never dips below zero) over the known prefix.
    pub fn over_consumes_from_empty(&self) -> bool {
        self.min_depth < 0
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Target {
    Top,
    Stack,
}

#[derive(Clone, Copy, PartialEq)]
enum Consume {
    Eat,
    Keep,
}

/// Abstract-interpret a compiled plan against the static mass contracts.
pub fn validate_mass_conservation(plan: &CompiledPlan) -> MassReport {
    let mut cur: i64 = 0;
    let mut min: i64 = 0;
    let mut all_known = true;
    // Modifier state applies to the next word, then resets to the defaults
    // (TOP / EAT, SPEC §6.1/§6.2).
    let mut target = Target::Top;
    let mut consume = Consume::Eat;

    'outer: for line in &plan.lines {
        for op in &line.ops {
            match op {
                CompiledOp::PushLiteral(_)
                | CompiledOp::PushVectorLiteral(_, _)
                | CompiledOp::PushCodeBlock(_) => {
                    cur += 1;
                }
                CompiledOp::SetTargetModeStackTop => target = Target::Top,
                CompiledOp::SetTargetModeStack => target = Target::Stack,
                CompiledOp::SetConsumptionConsume => consume = Consume::Eat,
                CompiledOp::SetConsumptionKeep => consume = Consume::Keep,
                CompiledOp::LineBreak | CompiledOp::BeginGuardedBlock => {}
                CompiledOp::CallBuiltin(name) => {
                    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
                    // STAK folds the top `count` items — a data-dependent arity
                    // (§6.1) we cannot pin statically; abstain.
                    if target == Target::Stack {
                        all_known = false;
                        break 'outer;
                    }
                    match mass_contract(canonical.as_ref()).fixed() {
                        Some((consumes, produces)) => {
                            // The word reads `consumes` operands (a depth dip),
                            // then pushes `produces`; under KEEP the operands
                            // are also retained (bifurcation, §13.2).
                            cur -= consumes as i64;
                            if cur < min {
                                min = cur;
                            }
                            cur += produces as i64;
                            if consume == Consume::Keep {
                                cur += consumes as i64;
                            }
                        }
                        None => {
                            all_known = false;
                            break 'outer;
                        }
                    }
                    target = Target::Top;
                    consume = Consume::Eat;
                }
                // User words, qualified words and raw fallbacks have no static
                // arity here; freeze the analysis.
                CompiledOp::CallUserWord(_)
                | CompiledOp::CallQualifiedWord { .. }
                | CompiledOp::CondDispatch(_)
                | CompiledOp::FallbackToken(_) => {
                    all_known = false;
                    break 'outer;
                }
            }
        }
    }

    MassReport {
        net_mass: cur,
        min_depth: min,
        all_known,
    }
}

/// Developer-diagnostic entry point: tokenize a source flow, compile it to a
/// plan in the context of `interp` (for word resolution), and statically
/// validate its mass conservation. This is the load-time/diagnostic check of
/// SPEC §13.1; it never executes the flow.
pub fn analyze_source(interp: &Interpreter, src: &str) -> Result<MassReport, String> {
    let tokens = crate::tokenizer::tokenize(src)?;
    let def = WordDefinition {
        lines: vec![ExecutionLine {
            body_tokens: tokens.into(),
        }]
        .into(),
        is_builtin: false,
        tier: Tier::Contrib,
        stability: Stability::Stable,
        capabilities: Capabilities::PURE,
        description: None,
        dependencies: Default::default(),
        original_source: None,
        namespace: None,
        registration_order: 0,
        execution_plans: None,
    };
    let plan = compile_word_definition(&def, interp);
    Ok(validate_mass_conservation(&plan))
}
