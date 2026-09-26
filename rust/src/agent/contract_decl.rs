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
//! [partiality=Q] [determinism=D] [cost AXIS=CLASS...]`. Every key is the
//! field of the same name in a contract Record (`CONTRACT`,
//! `spec/words.json`) and every value one that field admits: `purity` is
//! `pure`/`effectful`, `partiality` `total`/`partial`/`projecting`,
//! `determinism` `deterministic`/`stateRelative`/`hostRelative`, and each
//! `cost` axis (`steps`/`numeric`/`collection`) a class
//! `const`/`linear`/`superlinear`/`unbounded`
//! (`docs/dev/cost-contract-design.md`). `inputs` and `outputs` must equal
//! the inferred counts; every other value is an upper bound the inferred one
//! must not exceed. Each part is optional; fields left out are not checked.
//! Inference is deliberately conservative (SPEC LANG.CONTRACT.REGISTRY), so an
//! unprovable declaration is a `note`, never a false `error`.

use super::contract_cost::{check_cost_decl, parse_cost_terms, CostDecl};
use super::contract_gap::GapCode;
use super::contract_gap::{declaration_json, fold_outcomes, gap_summary_json, CheckOutcome};
use crate::interpreter::word_contract::{
    ContractConfidence, ContractDeterminism, ContractFlow, ContractPartiality, ContractPurity,
};
use crate::interpreter::Interpreter;
use crate::types::Token;

/// A parsed `#:contract` declaration. Fields left unstated are `None` and are
/// not checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContractDecl {
    pub name: String,
    pub inputs: Option<u16>,
    pub outputs: Option<u16>,
    pub purity: Option<ContractPurity>,
    pub partiality: Option<ContractPartiality>,
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
    /// True if any finding is an `error`. Drives the `check` exit code;
    /// `outcome`/`declarations` below are read-only projections (pitfall C).
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
            "violated": self.violated,
            "findings": self.findings.iter().map(|f| serde_json::json!({
                "severity": f.severity.as_str(),
                "message": f.message,
                "code": f.code,
            })).collect::<Vec<_>>(),
            "gapSummary": gap_summary_json(&outcomes, self.findings.iter().filter_map(|f| f.code)),
            // Phase 4: LANG.FAILURE.TRICHOTOMY at check time; `findings`/
            // `violated` stay as the legacy projection of the same result.
            "outcome": fold_outcomes(&outcomes).as_str(),
            "declarations": self
                .decl_outcomes
                .iter()
                .map(|(word, outcome)| declaration_json(word, *outcome))
                .collect::<Vec<_>>(),
        })
    }
}

/// Parse the `#:contract` directives out of `source`. Malformed directives
/// are returned as error messages so a typo never passes silently.
pub(crate) fn parse_contract_directives(source: &str) -> (Vec<ContractDecl>, Vec<String>) {
    let mut decls = Vec::new();
    let mut errors = Vec::new();

    for raw_line in source.lines() {
        let Some(body) = raw_line.trim_start().strip_prefix("#:contract") else {
            continue;
        };
        let raw = body.trim().to_string();
        let mut words = body.split_whitespace();
        let Some(name) = words.next() else {
            errors.push("empty `#:contract` directive (expected a word name)".to_string());
            continue;
        };

        let mut decl = ContractDecl {
            name: name.to_uppercase(),
            inputs: None,
            outputs: None,
            purity: None,
            partiality: None,
            determinism: None,
            cost: CostDecl::default(),
            raw: raw.clone(),
        };
        let mut malformed: Option<String> = None;

        let rest: Vec<&str> = words.collect();
        let mut i = 0;
        while i < rest.len() {
            if rest[i] == "cost" {
                match parse_cost_terms(&rest, i + 1, name, &mut decl.cost) {
                    Ok(next_i) => i = next_i,
                    Err(e) => {
                        malformed = Some(e);
                        break;
                    }
                }
                continue;
            }
            if let Err(e) = parse_term(rest[i], &mut decl) {
                malformed = Some(format!("`#:contract {name}`: {e}"));
                break;
            }
            i += 1;
        }

        match malformed {
            Some(e) => errors.push(e),
            None => decls.push(decl),
        }
    }

    (decls, errors)
}

/// One `key=value` term, written in the contract Record's own field names
/// and values.
fn parse_term(term: &str, decl: &mut ContractDecl) -> Result<(), String> {
    fn value<T>(key: &str, v: &str, parsed: Option<T>, admits: &str) -> Result<T, String> {
        parsed.ok_or_else(|| format!("`{key}` is {admits}, got `{v}`"))
    }
    let Some((key, v)) = term.split_once('=') else {
        return Err(unknown_term(term));
    };
    match key {
        "inputs" | "outputs" => {
            let count = value(key, v, v.parse::<u16>().ok(), "a non-negative integer")?;
            if key == "inputs" {
                decl.inputs = Some(count);
            } else {
                decl.outputs = Some(count);
            }
        }
        "purity" => {
            decl.purity = Some(value(
                key,
                v,
                ContractPurity::from_spec_str(v),
                "`pure` or `effectful`",
            )?)
        }
        "partiality" => {
            decl.partiality = Some(value(
                key,
                v,
                ContractPartiality::from_spec_str(v),
                "`total`, `partial` or `projecting`",
            )?)
        }
        "determinism" => {
            decl.determinism = Some(value(
                key,
                v,
                ContractDeterminism::from_spec_str(v),
                "`deterministic`, `stateRelative` or `hostRelative`",
            )?)
        }
        _ => return Err(unknown_term(term)),
    }
    Ok(())
}

fn unknown_term(term: &str) -> String {
    format!(
        "unknown term `{term}` (expected `inputs=N`, `outputs=N`, `purity=…`, \
         `partiality=…`, `determinism=…`, or `cost steps=… numeric=… collection=…`)"
    )
}

/// Extract every top-level `[ body ] 'NAME' DEF` from `tokens`, returning
/// `(NAME, body-tokens)` pairs in source order. Nested vectors are respected;
/// this reads the token stream only — it executes nothing.
fn collect_top_level_defs(tokens: &[Token]) -> Vec<(String, Vec<Token>)> {
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
        let is_def = matches!(tokens.get(k), Some(Token::Symbol(s))
            if crate::core_word_aliases::canonicalize_core_word_name(s).eq_ignore_ascii_case("DEF"));
        if !is_def {
            continue;
        }
        let body = tokens[open + 1..close].to_vec();
        defs.push((name.to_string(), body));
    }

    defs
}

/// Build an interpreter from `source` by registering its top-level word
/// definitions and imports **without executing any word body or top-level
/// code**, returning it with the user words it defined, in source order.
/// Shared by the `#:contract` checker and the `contract` reporter so both
/// see the identical execution-free environment.
pub(crate) fn build_definitions_interpreter(source: &str) -> (Interpreter, Vec<String>) {
    let mut interp = Interpreter::new();
    let mut names = Vec::new();
    if let Ok(tokens) = crate::tokenizer::tokenize(source) {
        for (name, body) in collect_top_level_defs(&tokens) {
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
    (interp, names)
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

    let (mut interp, _names) = build_definitions_interpreter(source);

    let mut decl_outcomes = Vec::with_capacity(decls.len());
    for decl in &decls {
        let before = findings.len();
        check_one(&mut interp, decl, &mut findings);
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
            ContractFlow::Dynamic => findings.push(bound_finding(
                &decl.name,
                &format!("{key}={declared}"),
                &format!("{key}=variable"),
                conservative,
                code,
            )),
        }
    }

    // `purity`, `partiality` and `determinism` are each a bound: the word may
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
