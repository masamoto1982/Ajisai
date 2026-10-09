//! Parsing of the `#:contract` directive lines a source declares for its own
//! Words (LANG.CONTRACT.CHECK); `contract_decl` checks what this reads.

use super::contract_decl::ContractDecl;
use super::contract_gap::{parse_cost_terms, CostDecl};
use crate::coreword_registry::FieldClosure;
use crate::interpreter::word_contract::{ContractDeterminism, ContractPartiality, ContractPurity};

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
            field: None,
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
        "field" => {
            decl.field = Some(value(
                key,
                v,
                FieldClosure::from_spec_str(v),
                "`closed` or `leaving`",
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
         `partiality=…`, `field=…`, `determinism=…`, or `cost steps=… numeric=… collection=…`)"
    )
}
