//! Opt-in per-word contract declarations, checked against the *inferred*
//! contract (`crate::interpreter::word_contract`) before execution — the P2
//! "connect an opt-in declaration to a pre-execution check" step of
//! `docs/dev/external-evaluation-response-strategy.md`.
//!
//! Like the `#@` test directives, a `#:contract` directive is **tooling only**:
//! it adds no language semantics (canonical source: `SPECIFICATION.html`) and is
//! an ordinary comment to the interpreter. It states what a user word's contract
//! is expected to be; `check --contract` infers the word's actual contract
//! *without executing any word body* and reports a declaration the inference
//! contradicts.
//!
//! ```text
//! #:contract INC ( 1 -- 1 ) pure nil-free
//! #:contract NORMALIZE ( 1 -- 1 ) may-nil
//! ```
//!
//! Grammar: `#:contract NAME [ ( CONSUMES -- PRODUCES ) ] [purity] [nil]
//! [linearity] [space]`, where `purity` is one of `pure` / `observable` /
//! `effectful`, `nil` is `nil-free` / `may-nil`, `linearity` is
//! `linear` / `affine` / `droppable` (the resource-ownership axis; see
//! `docs/dev/structural-memory-safety-roadmap.md` Phase 1), and `space` is
//! `space:const` / `space:linear` / `space:superlinear` / `space:unbounded`
//! (the static-footprint axis; see `docs/dev/space-contract-design.md` Phase 2).
//! Each part is
//! optional; the fields left out are not checked. Because inference is
//! deliberately conservative (SPEC §7.14), an unprovable declaration is reported
//! as a `note`, never a false `error`.

use super::contract_linearity::{check_linearity, linearity_from_word, Linearity};
use super::contract_space::{check_space, space_from_word, SpaceClass};
use super::explain::Lang;
use super::plan_check::Severity;
use crate::interpreter::modules;
use crate::interpreter::word_contract::{
    ContractConfidence, ContractFlow, ContractPurity, NilBehavior,
};
use crate::interpreter::Interpreter;
use crate::types::Token;

/// A parsed `#:contract` declaration. Fields left unstated are `None` and are
/// not checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContractDecl {
    pub name: String,
    pub arity: Option<(u16, u16)>,
    pub purity: Option<ContractPurity>,
    /// `Some(true)` = declared `nil-free`; `Some(false)` = declared `may-nil`.
    pub nil_free: Option<bool>,
    /// Declared resource linearity, or `None` when the directive omits it.
    pub linearity: Option<Linearity>,
    /// Declared static-footprint class, or `None` when the directive omits it.
    pub space: Option<SpaceClass>,
    /// The original directive text, for diagnostics.
    pub raw: String,
}

/// One declaration-check finding, mirroring `plan_check::Finding`.
pub(crate) struct DeclFinding {
    pub severity: Severity,
    pub message: String,
}

/// Result of the declaration check over a whole file.
pub(crate) struct ContractDeclCheck {
    pub findings: Vec<DeclFinding>,
    /// True if any finding is an `error` (a declaration the inference
    /// contradicts). Drives the `check` exit code.
    pub violated: bool,
}

impl ContractDeclCheck {
    /// Additive JSON for the `--json` envelope (`contractDecls`), rendered here
    /// so `report` stays decoupled from the declaration types.
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "violated": self.violated,
            "findings": self.findings.iter().map(|f| serde_json::json!({
                "severity": f.severity.as_str(),
                "message": f.message,
            })).collect::<Vec<_>>(),
        })
    }
}

fn purity_from_word(word: &str) -> Option<ContractPurity> {
    match word {
        "pure" => Some(ContractPurity::Pure),
        "observable" => Some(ContractPurity::Observable),
        "effectful" => Some(ContractPurity::Effectful),
        _ => None,
    }
}

/// How impure a purity level is: pure < observable < effectful. An inferred
/// purity above the declared level is a violation.
fn purity_rank(purity: ContractPurity) -> u8 {
    match purity {
        ContractPurity::Pure => 0,
        ContractPurity::Observable => 1,
        ContractPurity::Effectful => 2,
    }
}

fn purity_label(purity: ContractPurity) -> &'static str {
    match purity {
        ContractPurity::Pure => "pure",
        ContractPurity::Observable => "observable",
        ContractPurity::Effectful => "effectful",
    }
}

/// Parse the `#:contract` directives out of `source`. Malformed directives are
/// returned as error messages so a typo never passes silently (mirroring the
/// `#@` test-directive contract).
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
            arity: None,
            purity: None,
            nil_free: None,
            linearity: None,
            space: None,
            raw: raw.clone(),
        };
        let mut malformed: Option<String> = None;

        let rest: Vec<&str> = words.collect();
        let mut i = 0;
        while i < rest.len() {
            match rest[i] {
                "(" => {
                    // ( CONSUMES -- PRODUCES )
                    let close = rest[i..].iter().position(|w| *w == ")").map(|p| i + p);
                    let Some(close) = close else {
                        malformed = Some(format!("`#:contract {name}`: unclosed `(` in arity"));
                        break;
                    };
                    let inner = &rest[i + 1..close];
                    match parse_arity(inner) {
                        Ok(arity) => decl.arity = Some(arity),
                        Err(e) => {
                            malformed = Some(format!("`#:contract {name}`: {e}"));
                            break;
                        }
                    }
                    i = close + 1;
                }
                "nil-free" => {
                    decl.nil_free = Some(true);
                    i += 1;
                }
                "may-nil" => {
                    decl.nil_free = Some(false);
                    i += 1;
                }
                other => {
                    if let Some(p) = purity_from_word(other) {
                        decl.purity = Some(p);
                    } else if let Some(l) = linearity_from_word(other) {
                        decl.linearity = Some(l);
                    } else if let Some(s) = space_from_word(other) {
                        decl.space = Some(s);
                    } else {
                        malformed = Some(format!(
                            "`#:contract {name}`: unknown term `{other}` (expected `( c -- p )`, `pure`/`observable`/`effectful`, `nil-free`/`may-nil`, `linear`/`affine`/`droppable`, or `space:const`/`space:linear`/`space:superlinear`/`space:unbounded`)"
                        ));
                        break;
                    }
                    i += 1;
                }
            }
        }

        match malformed {
            Some(e) => errors.push(e),
            None => decls.push(decl),
        }
    }

    (decls, errors)
}

/// Parse the `c -- p` interior of an arity clause.
fn parse_arity(inner: &[&str]) -> Result<(u16, u16), String> {
    let sep = inner
        .iter()
        .position(|w| *w == "--")
        .ok_or_else(|| "arity needs `--` between consumes and produces".to_string())?;
    let consumes_words = &inner[..sep];
    let produces_words = &inner[sep + 1..];
    let consumes = parse_count(consumes_words, "consumes")?;
    let produces = parse_count(produces_words, "produces")?;
    Ok((consumes, produces))
}

fn parse_count(words: &[&str], side: &str) -> Result<u16, String> {
    match words {
        [] => Ok(0),
        [single] => single
            .parse::<u16>()
            .map_err(|_| format!("{side} count `{single}` is not a non-negative integer")),
        _ => Err(format!(
            "{side} side must be a single count (got `{}`)",
            words.join(" ")
        )),
    }
}

/// Extract every top-level `{ body } 'NAME' DEF` from `tokens`, returning
/// `(NAME, body-tokens)` pairs in source order. Nested blocks are respected;
/// blocks that are not the operand of a top-level `DEF` are ignored. This reads
/// the token stream only — it executes nothing.
fn collect_top_level_defs(tokens: &[Token]) -> Vec<(String, Vec<Token>)> {
    let mut defs = Vec::new();
    // Record depth-0 `{ ... }` spans as (open_index, close_index).
    let mut depth = 0i32;
    let mut open_at: Option<usize> = None;
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        match token {
            Token::BlockStart => {
                if depth == 0 {
                    open_at = Some(idx);
                }
                depth += 1;
            }
            Token::BlockEnd => {
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
        // After the closing `}`, skip line breaks and look for String(name) DEF.
        let mut j = close + 1;
        while matches!(tokens.get(j), Some(Token::LineBreak)) {
            j += 1;
        }
        let Some(Token::String(name)) = tokens.get(j) else {
            continue;
        };
        let mut k = j + 1;
        while matches!(tokens.get(k), Some(Token::LineBreak)) {
            k += 1;
        }
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

/// Bring the file's `'MODULE' IMPORT` modules into scope so module-word
/// dependencies resolve during inference. Uses the same non-executing module
/// restore the session-load path uses — no word body runs.
fn restore_imports(interp: &mut Interpreter, tokens: &[Token]) {
    for (i, token) in tokens.iter().enumerate() {
        let Token::String(name) = token else {
            continue;
        };
        let follows_import = tokens[i + 1..]
            .iter()
            .find(|t| !matches!(t, Token::LineBreak))
            .is_some_and(|t| {
                matches!(t, Token::Symbol(s)
                if { let c = crate::core_word_aliases::canonicalize_core_word_name(s);
                     c.eq_ignore_ascii_case("IMPORT") || c.eq_ignore_ascii_case("IMPORT-ONLY") })
            });
        if follows_import {
            modules::restore_module(interp, &name.to_uppercase());
        }
    }
}

/// Build an interpreter from `source` by registering its top-level word
/// definitions and imports **without executing any word body or top-level
/// code**, returning it alongside the user words it defined, in source order.
/// Shared by the `#:contract` checker and the `contract` reporter so both see
/// the identical execution-free environment the inference runs against.
pub(crate) fn build_definitions_interpreter(source: &str) -> (Interpreter, Vec<String>) {
    let mut interp = Interpreter::new();
    let mut names = Vec::new();
    if let Ok(tokens) = crate::tokenizer::tokenize(source) {
        restore_imports(&mut interp, &tokens);
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

/// Build a check interpreter from `source` by registering its top-level word
/// definitions (and imports) without executing any word body or top-level code,
/// then check every `#:contract` declaration against the inferred contract.
pub(crate) fn check_contract_decls(source: &str, lang: Lang) -> ContractDeclCheck {
    let (decls, parse_errors) = parse_contract_directives(source);
    let mut findings: Vec<DeclFinding> = parse_errors
        .into_iter()
        .map(|message| DeclFinding {
            severity: Severity::Error,
            message,
        })
        .collect();

    if decls.is_empty() {
        return ContractDeclCheck {
            violated: !findings.is_empty(),
            findings,
        };
    }

    let (mut interp, _names) = build_definitions_interpreter(source);

    for decl in &decls {
        check_one(&mut interp, decl, lang, &mut findings);
    }

    ContractDeclCheck {
        violated: findings.iter().any(|f| f.severity == Severity::Error),
        findings,
    }
}

fn check_one(
    interp: &mut Interpreter,
    decl: &ContractDecl,
    lang: Lang,
    findings: &mut Vec<DeclFinding>,
) {
    let Some(contract) = interp.infer_word_contract(&decl.name) else {
        findings.push(DeclFinding {
            severity: Severity::Error,
            message: match lang {
                Lang::Ja => format!("`#:contract {}`: その名前の語が見つかりません。", decl.name),
                Lang::En => format!("`#:contract {}`: no such word is defined.", decl.name),
            },
        });
        return;
    };

    // Conservative inference cannot disprove a declaration, so a mismatch under
    // low confidence is a note (unverifiable), never a false error.
    let conservative = contract.confidence == ContractConfidence::Conservative;

    if let Some((dc, dp)) = decl.arity {
        match &contract.flow {
            ContractFlow::Fixed { consumes, produces } => {
                if *consumes != dc || *produces != dp {
                    findings.push(DeclFinding {
                        severity: Severity::Error,
                        message: arity_msg(lang, &decl.name, dc, dp, *consumes, *produces),
                    });
                }
            }
            ContractFlow::Dynamic => findings.push(DeclFinding {
                severity: if conservative {
                    Severity::Note
                } else {
                    Severity::Error
                },
                message: match lang {
                    Lang::Ja => format!(
                        "`#:contract {}`: 固定アリティ ( {} -- {} ) を宣言していますが、推論は可変アリティです{}。",
                        decl.name, dc, dp,
                        if conservative { "(確証なし)" } else { "" }
                    ),
                    Lang::En => format!(
                        "`#:contract {}`: declared fixed arity ( {} -- {} ) but the inferred arity is dynamic{}.",
                        decl.name, dc, dp,
                        if conservative { " (unverified)" } else { "" }
                    ),
                },
            }),
        }
    }

    if let Some(declared) = decl.purity {
        if purity_rank(contract.purity) > purity_rank(declared) {
            findings.push(DeclFinding {
                severity: if conservative {
                    Severity::Note
                } else {
                    Severity::Error
                },
                message: match lang {
                    Lang::Ja => format!(
                        "`#:contract {}`: `{}` を宣言していますが、推論は `{}` です{}。",
                        decl.name,
                        purity_label(declared),
                        purity_label(contract.purity),
                        if conservative { "(確証なし)" } else { "" }
                    ),
                    Lang::En => format!(
                        "`#:contract {}`: declared `{}` but inferred `{}`{}.",
                        decl.name,
                        purity_label(declared),
                        purity_label(contract.purity),
                        if conservative { " (unverified)" } else { "" }
                    ),
                },
            });
        }
    }

    if decl.nil_free == Some(true) {
        // `nil-free` means the word never *manufactures* absence. Only
        // `MayCreate` (a CreatesNil word like DIV in the flow) does that;
        // `Propagates` merely carries an input NIL through (ADD1 is nil-free yet
        // propagates), and Rejects/Consumes/NeverCreates never mint a NIL.
        let may_create = matches!(contract.nil_behavior, NilBehavior::MayCreate);
        if may_create {
            findings.push(DeclFinding {
                severity: if conservative {
                    Severity::Note
                } else {
                    Severity::Error
                },
                message: match lang {
                    Lang::Ja => format!(
                        "`#:contract {}`: `nil-free` を宣言していますが、推論では NIL を生成/透過し得ます{}。",
                        decl.name,
                        if conservative { "(確証なし)" } else { "" }
                    ),
                    Lang::En => format!(
                        "`#:contract {}`: declared `nil-free` but inference shows it can create or propagate NIL{}.",
                        decl.name,
                        if conservative { " (unverified)" } else { "" }
                    ),
                },
            });
        }
    }

    if let Some(linearity) = decl.linearity {
        check_linearity(interp, decl, linearity, lang, findings);
    }

    if let Some(space) = decl.space {
        check_space(decl, space, &contract, lang, findings);
    }
}

fn arity_msg(lang: Lang, name: &str, dc: u16, dp: u16, ic: u16, ip: u16) -> String {
    match lang {
        Lang::Ja => format!(
            "`#:contract {name}`: アリティ ( {dc} -- {dp} ) を宣言していますが、推論は ( {ic} -- {ip} ) です。"
        ),
        Lang::En => format!(
            "`#:contract {name}`: declared arity ( {dc} -- {dp} ) but inferred ( {ic} -- {ip} )."
        ),
    }
}
