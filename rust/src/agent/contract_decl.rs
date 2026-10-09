//! Opt-in per-word contract declarations, checked against the *inferred*
//! contract (`crate::interpreter::word_contract`) before execution — the P2
//! "connect an opt-in declaration to a pre-execution check" step of
//! `docs/dev/external-evaluation-response-strategy.md`.
//!
//! Like the `#@` test directives, a `#:contract` directive is **tooling
//! only**: it adds no language semantics (canonical source:
//! `SPECIFICATION.html`) and is an ordinary comment to the interpreter. It
//! states what a user word's contract is expected to be; `check --contract`
//! infers the word's actual contract *without executing any word body* and
//! reports a declaration the inference contradicts.
//!
//! ```text
//! #:contract INC inputs=1 outputs=1 purity=pure partiality=total
//! #:contract NORMALIZE inputs=1 outputs=1 partiality=projecting
//! #:contract SUM-ALL cost steps=unbounded numeric=linear
//! ```
//!
//! Grammar: `#:contract NAME [inputs=N] [outputs=N] [purity=P]
//! [partiality=Q] [field=F] [determinism=D] [cost AXIS=CLASS...]`. Every key is the
//! field of the same name in a contract Record (`CONTRACT`,
//! `spec/words.json`) and every value one that field admits: `purity` is
//! `pure`/`effectful`, `partiality` `total`/`partial`/`projecting`,
//! `field` `closed`/`leaving` (LANG.CONTRACT.FIELD),
//! `determinism` `deterministic`/`stateRelative`/`hostRelative`, and each
//! `cost` axis (`steps`/`numeric`/`collection`) a class
//! `const`/`linear`/`superlinear`/`unbounded`
//! (`docs/dev/cost-contract-design.md`). `inputs` and `outputs` must equal
//! the inferred counts; every other value is an upper bound the inferred one
//! must not exceed. Each part is optional; fields left out are not checked.
//! Inference is deliberately conservative (SPEC LANG.CONTRACT.REGISTRY), so an
//! unprovable declaration is a `note`, never a false `error`.

use super::contract_gap::{
    check_cost_decl, declaration_json, fold_outcomes, gap_summary_json, CheckOutcome, CostDecl,
    GapCode,
};
use crate::coreword_registry::FieldClosure;
use crate::interpreter::word_contract::{
    ContractConfidence, ContractDeterminism, ContractFlow, ContractPartiality, ContractPurity,
};
use crate::interpreter::Interpreter;
use crate::types::Token;

pub(crate) use super::contract_directive::parse_contract_directives;
use std::collections::HashSet;

/// A parsed `#:contract` declaration. Fields left unstated are `None` and are
/// not checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContractDecl {
    pub name: String,
    pub inputs: Option<u16>,
    pub outputs: Option<u16>,
    pub purity: Option<ContractPurity>,
    pub partiality: Option<ContractPartiality>,
    /// A declared `closed` promises the Word never answers a point over zero
    /// from operands that hold none (LANG.CONTRACT.FIELD).
    pub field: Option<FieldClosure>,
    pub determinism: Option<ContractDeterminism>,
    /// Declared cost-class bounds, one per axis; each `None` axis is not
    /// checked (Phase 5).
    pub cost: CostDecl,
    /// The original directive text, for diagnostics.
    pub raw: String,
}

/// Outcome of checking one declaration axis. The specification fixes exactly
/// three results for `check --contract`: verified (no finding), violated
/// (`Error`), or cannot verify (`Note`) when inference is too conservative to
/// decide. A `Note` never fails the check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Severity {
    Error,
    Note,
}

impl Severity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Note => "note",
        }
    }
}

pub(crate) struct DeclFinding {
    pub severity: Severity,
    pub message: String,
    /// The gap id behind a `Note` finding; always `None` for an `Error`
    /// finding — a proven violation has no gap (pitfall B).
    pub code: Option<&'static str>,
}

/// Result of the declaration check over a whole file.
pub(crate) struct ContractDeclCheck {
    pub findings: Vec<DeclFinding>,
    /// True if any finding is an `error`. Drives the `check` exit code and
    /// the JSON `outcome`, which is `error` exactly when this is true.
    pub violated: bool,
    /// One `(word, outcome)` per successfully-parsed declaration, in source
    /// order — not derived by counting `findings` by severity, since one
    /// declaration can contribute more than one finding. A malformed
    /// directive counts toward `violated`/`findings` but not this list.
    pub decl_outcomes: Vec<(String, CheckOutcome)>,
}

impl ContractDeclCheck {
    /// Additive JSON for the `--json` envelope (`contractDecls`), rendered here
    /// so `report` stays decoupled from the declaration types.
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let outcomes: Vec<CheckOutcome> = self
            .decl_outcomes
            .iter()
            .map(|(_, outcome)| *outcome)
            .collect();
        serde_json::json!({
            "findings": self.findings.iter().map(|f| serde_json::json!({
                "severity": f.severity.as_str(),
                "message": f.message,
                "code": f.code,
            })).collect::<Vec<_>>(),
            "gapSummary": gap_summary_json(&outcomes, self.findings.iter().filter_map(|f| f.code)),
            // LANG.FAILURE.TRICHOTOMY at check time, and the only spelling of
            // a violation here. A malformed directive is an error too: it
            // counts toward no declaration, but it fails the check, so the
            // fold alone would call a failing file `value`.
            "outcome": if self.violated {
                CheckOutcome::Error.as_str()
            } else {
                fold_outcomes(&outcomes).as_str()
            },
            "declarations": self
                .decl_outcomes
                .iter()
                .map(|(word, outcome)| declaration_json(word, *outcome))
                .collect::<Vec<_>>(),
        })
    }
}

/// Extract every top-level `[ body ] 'NAME' DEF` from `tokens`, returning
/// `(NAME, body-tokens, DEF-index)` triples in source order. Nested vectors
/// are respected; this reads the token stream only — it executes nothing.
fn collect_top_level_defs(tokens: &[Token]) -> Vec<(String, Vec<Token>, usize)> {
    let mut defs = Vec::new();
    // Record depth-0 `[ ... ]` spans as (open_index, close_index).
    let mut depth = 0i32;
    let mut open_at: Option<usize> = None;
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        match token {
            Token::VectorStart => {
                if depth == 0 {
                    open_at = Some(idx);
                }
                depth += 1;
            }
            Token::VectorEnd => {
                depth -= 1;
                if depth == 0 {
                    if let Some(open) = open_at.take() {
                        spans.push((open, idx));
                    }
                }
            }
            _ => {}
        }
    }

    for (open, close) in spans {
        // After the closing `]`, look for String(name) DEF.
        let j = close + 1;
        let Some(Token::String(name)) = tokens.get(j) else {
            continue;
        };
        let k = j + 1;
        if !is_def_symbol(tokens.get(k)) {
            continue;
        }
        let body = tokens[open + 1..close].to_vec();
        defs.push((name.to_string(), body, k));
    }

    defs
}

fn is_def_symbol(token: Option<&Token>) -> bool {
    matches!(token, Some(Token::Symbol(s))
        if crate::word_name::canonical_word_name(s).eq_ignore_ascii_case("DEF"))
}

/// The names whose binding the execution-free pass cannot settle. It
/// registers each plain top-level `[ body ] 'NAME' DEF` in order, so a name
/// is settled only when that is the one `DEF` of it in the source: a name
/// defined twice is called under either body depending on where the call
/// stands, and a `DEF` the pass does not read (inside a block, or after a
/// computed body) may bind it when it runs. A `DEF` whose name is not a
/// String literal could bind any name, so it unsettles every one.
#[derive(Debug, Default)]
pub(crate) struct UnsettledNames {
    names: HashSet<String>,
    every: bool,
}

impl UnsettledNames {
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.every || self.names.contains(&name.to_uppercase())
    }

    fn of(tokens: &[Token], defs: &[(String, Vec<Token>, usize)]) -> Self {
        let mut unsettled = UnsettledNames::default();
        let mut seen = HashSet::new();
        for (name, _, _) in defs {
            let upper = name.to_uppercase();
            if !seen.insert(upper.clone()) {
                unsettled.names.insert(upper);
            }
        }
        let read: HashSet<usize> = defs.iter().map(|(_, _, at)| *at).collect();
        for (idx, token) in tokens.iter().enumerate() {
            if read.contains(&idx) || !is_def_symbol(Some(token)) {
                continue;
            }
            match idx.checked_sub(1).map(|prev| &tokens[prev]) {
                Some(Token::String(name)) => {
                    unsettled.names.insert(name.to_uppercase());
                }
                _ => unsettled.every = true,
            }
        }
        unsettled
    }
}

/// Build an interpreter from `source` by registering its top-level word
/// definitions and imports **without executing any word body or top-level
/// code**, returning it with the user words it defined, in source order.
/// Shared by the `#:contract` checker and the `contract` reporter so both
/// see the identical execution-free environment. The names it cannot bind to
/// one body come back as [`UnsettledNames`]: what it registered for one of
/// them is only the last body it read.
pub(crate) fn build_definitions_interpreter(
    source: &str,
) -> (Interpreter, Vec<String>, UnsettledNames) {
    let mut interp = Interpreter::new();
    let mut names = Vec::new();
    let mut unsettled = UnsettledNames::default();
    if let Ok(tokens) = crate::tokenizer::tokenize(source) {
        let defs = collect_top_level_defs(&tokens);
        unsettled = UnsettledNames::of(&tokens, &defs);
        for (name, body, _) in defs {
            // A malformed body is not this pass's concern (the structural check
            // ran earlier); skip a definition that will not register.
            if crate::interpreter::execute_def::op_def_inner(&mut interp, &name, &body).is_ok() {
                let upper = name.to_uppercase();
                if !names.contains(&upper) {
                    names.push(upper);
                }
            }
        }
        // Registration writes naming warnings into the output buffer; discard
        // them so they never leak into a caller's findings.
        interp.output_buffer.clear();
    }
    (interp, names, unsettled)
}

/// Build a check interpreter from `source` (no execution), then check every
/// `#:contract` declaration against the inferred contract.
pub(crate) fn check_contract_decls(source: &str) -> ContractDeclCheck {
    let (decls, parse_errors) = parse_contract_directives(source);
    let mut findings: Vec<DeclFinding> = parse_errors
        .into_iter()
        .map(|message| DeclFinding {
            severity: Severity::Error,
            message,
            code: None,
        })
        .collect();

    if decls.is_empty() {
        return ContractDeclCheck {
            violated: !findings.is_empty(),
            findings,
            decl_outcomes: Vec::new(),
        };
    }

    let (mut interp, _names, unsettled) = build_definitions_interpreter(source);

    let mut decl_outcomes = Vec::with_capacity(decls.len());
    for decl in &decls {
        let before = findings.len();
        if unsettled.contains(&decl.name) {
            // Checking the last body read would verify a declaration against
            // a definition the calls before it never run.
            findings.push(DeclFinding {
                severity: Severity::Note,
                message: format!(
                    "`#:contract {}`: the word is defined more than once, or by a `DEF` \
                     the check does not read before running, so the check cannot tell \
                     which definition a call runs (unverified).",
                    decl.name
                ),
                code: Some(GapCode::UnmodelledControlFlow.as_str()),
            });
        } else {
            check_one(&mut interp, decl, &mut findings);
        }
        let new_findings = &findings[before..];
        let outcome = if new_findings.iter().any(|f| f.severity == Severity::Error) {
            CheckOutcome::Error
        } else if new_findings.iter().any(|f| f.severity == Severity::Note) {
            // Every conservative Note carries a code in practice (pitfall D);
            // `ConservativeSeed` guards only the defensive empty-gaps case.
            let gap = new_findings
                .iter()
                .find_map(|f| f.code)
                .and_then(GapCode::from_str)
                .unwrap_or(GapCode::ConservativeSeed);
            CheckOutcome::Nil(gap)
        } else {
            CheckOutcome::Value
        };
        decl_outcomes.push((decl.name.clone(), outcome));
    }

    ContractDeclCheck {
        violated: findings.iter().any(|f| f.severity == Severity::Error),
        findings,
        decl_outcomes,
    }
}

fn check_one(interp: &mut Interpreter, decl: &ContractDecl, findings: &mut Vec<DeclFinding>) {
    let Some(contract) = interp.infer_word_contract(&decl.name) else {
        findings.push(DeclFinding {
            severity: Severity::Error,
            message: format!("`#:contract {}`: no such word is defined.", decl.name),
            code: None,
        });
        return;
    };

    // Conservative inference cannot disprove a declaration, so a mismatch under
    // low confidence is a note (unverifiable), never a false error.
    let conservative = contract.confidence == ContractConfidence::Conservative;
    // The gap id behind a conservative mismatch. Every path to `Conservative`
    // also pushes a gap, but if one ever didn't, `None` is the safe direction
    // — never panic over an incomplete diagnostic (Phase 3 pitfall D).
    let code: Option<&'static str> = if conservative {
        contract.gaps.first().map(|g| g.as_str())
    } else {
        None
    };

    for (key, declared, inferred) in [("inputs", decl.inputs, 0), ("outputs", decl.outputs, 1)] {
        let Some(declared) = declared else {
            continue;
        };
        match &contract.flow {
            ContractFlow::Fixed { consumes, produces } => {
                let inferred = if inferred == 0 { *consumes } else { *produces };
                if inferred != declared {
                    findings.push(DeclFinding {
                        severity: Severity::Error,
                        message: format!(
                            "`#:contract {}`: declared `{key}={declared}` but inferred `{key}={inferred}`.",
                            decl.name
                        ),
                        code: None,
                    });
                }
            }
            // `variable` proves nothing against a count: a Core Word whose
            // arity depends on its operands (`EXEC`, `COLLECT`) makes the
            // inferred flow `Dynamic` even where this body always feeds it
            // the same operands (`[ 2 COLLECT ]` is 2 -> 1). So a fixed
            // declaration over it cannot be verified, never refuted.
            ContractFlow::Dynamic => findings.push(bound_finding(
                &decl.name,
                &format!("{key}={declared}"),
                &format!("{key}=variable"),
                true,
                code.or(Some(GapCode::UnmodelledControlFlow.as_str())),
            )),
        }
    }

    // `purity`, `partiality`, `field` and `determinism` are each a bound: the word may
    // be tighter than declared, never looser.
    if let Some(declared) = decl.purity.filter(|d| contract.purity > *d) {
        findings.push(bound_finding(
            &decl.name,
            &format!("purity={}", declared.as_spec_str()),
            &format!("purity={}", contract.purity.as_spec_str()),
            conservative,
            code,
        ));
    }
    if let Some(declared) = decl.partiality.filter(|d| contract.partiality > *d) {
        findings.push(bound_finding(
            &decl.name,
            &format!("partiality={}", declared.as_spec_str()),
            &format!("partiality={}", contract.partiality.as_spec_str()),
            conservative,
            code,
        ));
    }
    if let Some(declared) = decl.field.filter(|d| contract.field > *d) {
        findings.push(bound_finding(
            &decl.name,
            &format!("field={}", declared.as_spec_str()),
            &format!("field={}", contract.field.as_spec_str()),
            conservative,
            code,
        ));
    }
    if let Some(declared) = decl.determinism.filter(|d| contract.determinism > *d) {
        findings.push(bound_finding(
            &decl.name,
            &format!("determinism={}", declared.as_spec_str()),
            &format!("determinism={}", contract.determinism.as_spec_str()),
            conservative,
            code,
        ));
    }

    check_cost_decl(&decl.name, decl.cost, contract.cost, code, findings);
}

/// A declared bound the inferred contract exceeds: a violation when the
/// inference is complete, and only a note — unverifiable, never a false
/// error — when it is conservative.
fn bound_finding(
    name: &str,
    declared: &str,
    inferred: &str,
    conservative: bool,
    code: Option<&'static str>,
) -> DeclFinding {
    DeclFinding {
        severity: if conservative {
            Severity::Note
        } else {
            Severity::Error
        },
        message: format!(
            "`#:contract {name}`: declared `{declared}` but inferred `{inferred}`{}.",
            if conservative { " (unverified)" } else { "" }
        ),
        code: if conservative { code } else { None },
    }
}
