//! The `cost` axis of a `#:contract` declaration (Phase 5 of `docs/dev/
//! competitive-advantage-work-order-2026-08.md`; design rationale in
//! `docs/dev/cost-contract-design.md`). Split out of `contract_decl.rs` to
//! keep that file within the §14.1 file-size budget, mirroring
//! `contract_linearity.rs`'s split for the `linear`/`affine`/`droppable` axis.

use super::contract_decl::{DeclFinding, Severity};
use crate::interpreter::word_cost::{CostBound, CostClass};

/// The `cost` term's three declarable axes — each `Some` axis is checked
/// against `WordContract::cost`; each `None` axis is left unchecked, the
/// same "omitted term is not checked" rule the other axes already follow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CostDecl {
    pub steps: Option<CostClass>,
    pub numeric: Option<CostClass>,
    pub collection: Option<CostClass>,
}

/// Parse the `axis=class` terms following a `cost` keyword, starting at
/// `rest[i]`. Returns the index just past the last consumed term, or an
/// error message on the first malformed term or an empty term list.
pub(crate) fn parse_cost_terms(
    rest: &[&str],
    mut i: usize,
    name: &str,
    cost: &mut CostDecl,
) -> Result<usize, String> {
    let mut saw_axis = false;
    while i < rest.len() {
        let Some((axis, class_word)) = rest[i].split_once('=') else {
            break;
        };
        let Some(class) = CostClass::from_spec_str(class_word) else {
            return Err(format!(
                "`#:contract {name}`: unknown cost class `{class_word}` (expected `const`/`linear`/`superlinear`/`unbounded`)"
            ));
        };
        match axis {
            "steps" => cost.steps = Some(class),
            "numeric" => cost.numeric = Some(class),
            "collection" => cost.collection = Some(class),
            other_axis => {
                return Err(format!(
                    "`#:contract {name}`: unknown cost axis `{other_axis}` (expected `steps`/`numeric`/`collection`)"
                ));
            }
        }
        saw_axis = true;
        i += 1;
    }
    if !saw_axis {
        return Err(format!(
            "`#:contract {name}`: `cost` with no `axis=class` term"
        ));
    }
    Ok(i)
}

/// Check every declared cost axis on `decl_cost` against `contract_cost`,
/// pushing a finding for each axis the inferred bound exceeds.
pub(crate) fn check_cost_decl(
    name: &str,
    decl_cost: CostDecl,
    contract_cost: CostBound,
    code: Option<&'static str>,
    findings: &mut Vec<DeclFinding>,
) {
    if let Some(declared) = decl_cost.steps {
        check_cost_axis(name, "steps", declared, contract_cost.steps, code, findings);
    }
    if let Some(declared) = decl_cost.numeric {
        check_cost_axis(
            name,
            "numeric",
            declared,
            contract_cost.numeric,
            code,
            findings,
        );
    }
    if let Some(declared) = decl_cost.collection {
        check_cost_axis(
            name,
            "collection",
            declared,
            contract_cost.collection,
            code,
            findings,
        );
    }
}

/// Check one `cost` axis (Step 5.5). Unlike arity/purity/nil-free, severity
/// here is driven by *this axis's own* `exact` bit, not the word's overall
/// `ContractConfidence` — `word_space`'s "never a false error" invariant
/// (`docs/dev/cost-contract-design.md` §3): a mismatch is only ever a proven
/// violation when the inferred class is provably attained.
fn check_cost_axis(
    name: &str,
    axis: &str,
    declared: CostClass,
    inferred: (CostClass, bool),
    code: Option<&'static str>,
    findings: &mut Vec<DeclFinding>,
) {
    let (inferred_class, exact) = inferred;
    if inferred_class > declared {
        findings.push(DeclFinding {
            severity: if exact {
                Severity::Error
            } else {
                Severity::Note
            },
            message: format!(
                "`#:contract {name}`: declared cost {axis}={} but inferred {}{}.",
                declared.as_spec_str(),
                inferred_class.as_spec_str(),
                if exact { "" } else { " (unverified)" }
            ),
            code: if exact { None } else { code },
        });
    }
}
