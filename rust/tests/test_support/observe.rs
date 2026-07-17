//! Firewall-clean observation (Phase 1).
//!
//! The formalization §8 defines the observation function
//! `observe(p) = (render(π_Stack ⟦p⟧ σ₀), π_Eff ⟦p⟧ σ₀)`. This module gives the
//! data-plane half of that: it reads a value **only** through the SPEC §2.3
//! semantic axes (`semanticKind`, `shape`, `capabilities`, `truthValue`,
//! `origin`, `absence`) and through the pure renderer `render : (data, role) →
//! display`. It never branches on a Rust enum name, `Debug` string, or display
//! text — the semantic-firewall discipline the roadmap §1.2-3 mandates.
//!
//! Later laws also use `observe_program` from this module to compare the
//! effect trace and error category alongside the data-plane axes, keeping
//! surface/canonical equivalence checks structured rather than string-fragment
//! based.

use ajisai_core::interpreter::Interpreter;
use ajisai_core::types::display::format_with_hint;
use ajisai_core::types::{Interpretation, Value};
use ajisai_core::ErrorCategory;

/// Every interpretation role of SPEC §12.2, in table order.
pub const ALL_ROLES: [Interpretation; 8] = [
    Interpretation::Unassigned,
    Interpretation::RawNumber,
    Interpretation::ContinuedFraction,
    Interpretation::Interval,
    Interpretation::Text,
    Interpretation::TruthValue,
    Interpretation::Timestamp,
    Interpretation::Nil,
];

/// The pure renderer `render : (data, role) → display` (SPEC §12.1). Exposed as
/// a named function so laws read as equations over `render`, not over the
/// `Display` impl.
pub fn render(v: &Value, role: Interpretation) -> String {
    format_with_hint(v, role)
}

/// The protocol-level observation of one value: the SPEC §2.3 semantic axes as
/// canonical lower-camel-case protocol strings. Capabilities are sorted so the
/// observation is order-insensitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosisObservation {
    pub when: &'static str,
    pub where_kind: &'static str,
    pub word: Option<String>,
    pub module: Option<String>,
    pub why: &'static str,
    pub agreed_prefix: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsenceObservation {
    pub reason: Option<&'static str>,
    pub origin: &'static str,
    pub recoverability: &'static str,
    pub diagnosis: Option<DiagnosisObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisObservation {
    pub semantic_kind: &'static str,
    pub shape: &'static str,
    pub capabilities: Vec<&'static str>,
    pub truth_value: Option<&'static str>,
    pub origin: &'static str,
    pub absence: Option<AbsenceObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueObservation {
    pub render: String,
    pub axes: AxisObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramObservation {
    pub stack: Vec<ValueObservation>,
    pub effects: Vec<(String, String)>,
    pub error_category: Option<&'static str>,
}

/// Observe a value through the semantic axes only (firewall-clean).
pub fn observe_axes(v: &Value) -> AxisObservation {
    let mut capabilities: Vec<&'static str> = v
        .capabilities()
        .iter()
        .map(|c| c.as_protocol_str())
        .collect();
    capabilities.sort_unstable();
    let absence = v.absence_metadata().map(|absence| AbsenceObservation {
        reason: absence
            .reason
            .as_ref()
            .map(|reason| reason.as_protocol_str()),
        origin: absence.origin.as_protocol_str(),
        recoverability: absence.recoverability.as_protocol_str(),
        diagnosis: absence
            .diagnosis
            .as_ref()
            .map(|diagnosis| DiagnosisObservation {
                when: diagnosis.when.as_protocol_str(),
                where_kind: diagnosis.where_.kind.as_protocol_str(),
                word: diagnosis.where_.word.clone(),
                module: diagnosis.where_.module.clone(),
                why: diagnosis.why.as_protocol_str(),
                agreed_prefix: diagnosis.agreed_prefix,
            }),
    });
    AxisObservation {
        semantic_kind: v.semantic_kind().as_protocol_str(),
        shape: v.shape_kind().as_protocol_str(),
        capabilities,
        truth_value: v.truth_value(),
        origin: v.origin().as_protocol_str(),
        absence,
    }
}

/// Observe a value as the structured stack payload used by law tests: stable
/// render text plus protocol-level semantic axes, including absence/diagnosis
/// metadata when present.
pub fn observe_value(v: &Value) -> ValueObservation {
    ValueObservation {
        render: render(v, v.hint),
        axes: observe_axes(v),
    }
}

/// Run a program and capture the structured observation fields needed by
/// surface/canonical equivalence laws: stack values, effect trace, and error
/// category. Human-readable error messages are deliberately not observed.
pub fn observe_program(src: &str) -> ProgramObservation {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        let error_category = match interp.execute(src).await {
            Ok(()) => None,
            Err(err) => Some(ErrorCategory::from_error(&err).as_protocol_str()),
        };
        ProgramObservation {
            stack: interp.get_stack().iter().map(observe_value).collect(),
            effects: interp
                .host_effects()
                .iter()
                .map(|effect| (effect.kind().to_string(), effect.payload().to_string()))
                .collect(),
            error_category,
        }
    })
}

/// Source marker for a pre-loaded Tier 2 starvation witness. Comparison is
/// total over Tier ≤ 1 — everything the vocabulary can construct — so laws
/// that need the logical Unknown (U) start from a type-level Tier 2 value:
/// a program beginning with this marker runs with a `Computable` enclosure
/// process (never separable from zero) already on the stack.
pub const TIER2_WITNESS: &str = "<<tier2-witness>>";

/// Strip a leading [`TIER2_WITNESS`] marker, pre-loading the witness onto
/// the interpreter's stack when present; returns the program to execute.
/// Every harness that runs generator sources routes through this.
pub fn prepare<'a>(interp: &mut Interpreter, src: &'a str) -> &'a str {
    match src.strip_prefix(TIER2_WITNESS) {
        Some(rest) => {
            use ajisai_core::types::exact::{Computable, ExactReal};
            interp.update_stack(vec![Value::from_exact_real(ExactReal::Computable(
                Computable::vanishing(),
            ))]);
            rest
        }
        None => src,
    }
}

/// Run an Ajisai program and return the final stack. Panics on execution error
/// so a malformed law program is loud rather than silently skipped (mirrors the
/// existing `algebraic_laws.rs` harness). A leading [`TIER2_WITNESS`] marker
/// pre-loads the Tier 2 starvation witness (see the marker's docs).
pub fn run(src: &str) -> Vec<Value> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        let program = prepare(&mut interp, src);
        interp
            .execute(program)
            .await
            .unwrap_or_else(|e| panic!("program failed: {src:?}: {e}"));
        interp.get_stack().clone()
    })
}

/// Run a program expected to leave exactly one value, returning that value.
pub fn run_one(src: &str) -> Value {
    let stack = run(src);
    assert_eq!(
        stack.len(),
        1,
        "generator program {src:?} must leave exactly one value, got {}",
        stack.len()
    );
    stack.into_iter().next().unwrap()
}
