//! Declared types of User Words (LANG.CONTRACT.TYPES): the signature a
//! `#:contract` line may write after the Word's name, and the run-time check
//! at a typed Word's boundary.
//!
//! `#:contract SUMSQ [int] -> int` says SUMSQ takes one Vector of integers
//! that fit 64 bits and leaves one such integer. A type promises a range, not
//! a precision: nothing is rounded inside a typed Word, and every value a type
//! admits is a value the untyped language already has. So erasing every
//! signature from a program leaves an untyped program that answers the same
//! values — or the typed one stopped with `typeViolation`, because a value
//! fell outside the range its signature promised.
//!
//! The check runs where a typed Word is entered and where it returns: the
//! operands it is called with must fit its input types (the caller broke the
//! promise), and the values it leaves must fit its output types (the Word
//! broke it). A NIL fits every type, since an absence flows through typed
//! code as it flows through untyped code (LANG.VALUES.NIL).

use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueData};
use std::fmt;

/// One declared type. `Int` and `Rat` are ranges of the exact numbers, not
/// other numbers: an `Int` is an integer whose value fits `i64`, a `Rat` a
/// rational whose numerator and denominator in lowest terms each fit `i64`,
/// the two points over zero included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredType {
    Int,
    Rat,
    Bool,
    Str,
    Vector(Box<DeclaredType>),
    /// A Record whose keys are exactly these Strings, in any order, each
    /// holding a value of its type (LANG.RECORDS.STRUCTURE).
    Record(Vec<(String, DeclaredType)>),
    /// A table: a Record whose keys are exactly these Strings, each holding
    /// a Vector of its type, every Vector of one length.
    Table(Vec<(String, DeclaredType)>),
}

impl DeclaredType {
    /// A table's type as the Record of columns it is.
    pub fn columns(fields: &[(String, DeclaredType)]) -> DeclaredType {
        DeclaredType::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), DeclaredType::Vector(Box::new(ty.clone()))))
                .collect(),
        )
    }
}

impl fmt::Display for DeclaredType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeclaredType::Int => f.write_str("int"),
            DeclaredType::Rat => f.write_str("rat"),
            DeclaredType::Bool => f.write_str("bool"),
            DeclaredType::Str => f.write_str("str"),
            DeclaredType::Vector(element) => write!(f, "[{element}]"),
            DeclaredType::Record(fields) | DeclaredType::Table(fields) => {
                let fields: Vec<String> = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}:{ty}"))
                    .collect();
                let table = if matches!(self, DeclaredType::Table(_)) {
                    "table"
                } else {
                    ""
                };
                write!(f, "{table}{{{}}}", fields.join(" "))
            }
        }
    }
}

/// A Word's declared stack effect: the types it takes, deepest first, and the
/// types it leaves, deepest first — the order `#:contract` writes them in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub inputs: Vec<DeclaredType>,
    pub outputs: Vec<DeclaredType>,
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side = |types: &[DeclaredType]| {
            types
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        };
        let (inputs, outputs) = (side(&self.inputs), side(&self.outputs));
        match (inputs.is_empty(), outputs.is_empty()) {
            (true, true) => f.write_str("->"),
            (true, false) => write!(f, "-> {outputs}"),
            (false, true) => write!(f, "{inputs} ->"),
            (false, false) => write!(f, "{inputs} -> {outputs}"),
        }
    }
}

/// The signature at the head of a `#:contract` line's detail (what follows
/// the Word's name), and the `key=value` terms after it. A detail with no
/// `->` before its first `key=value` term declares no signature.
pub fn split_signature(detail: &str) -> std::result::Result<(Option<Signature>, String), String> {
    let mut sig_text = String::new();
    let mut rest = Vec::new();
    for term in detail.split_whitespace() {
        if !rest.is_empty() || term.contains('=') {
            rest.push(term);
        } else {
            sig_text.push_str(term);
            sig_text.push(' ');
        }
    }
    let rest = rest.join(" ");
    if sig_text.trim().is_empty() {
        return Ok((None, rest));
    }
    if !sig_text.contains("->") {
        return Err(format!(
            "`{}` is neither a `key=value` term nor a signature (a signature has `->`)",
            sig_text.trim()
        ));
    }
    parse_signature(&sig_text).map(|sig| (Some(sig), rest))
}

fn parse_signature(text: &str) -> std::result::Result<Signature, String> {
    let spaced = text
        .replace("->", " -> ")
        .replace('[', " [ ")
        .replace(']', " ] ")
        .replace('{', " { ")
        .replace('}', " } ")
        .replace(':', " : ");
    let lexemes: Vec<&str> = spaced.split_whitespace().collect();
    let arrow = lexemes
        .iter()
        .position(|lexeme| *lexeme == "->")
        .ok_or("a signature has `->`")?;
    if lexemes[arrow + 1..].contains(&"->") {
        return Err("a signature has one `->`".to_string());
    }
    Ok(Signature {
        inputs: parse_types(&lexemes[..arrow])?,
        outputs: parse_types(&lexemes[arrow + 1..])?,
    })
}

fn parse_types(lexemes: &[&str]) -> std::result::Result<Vec<DeclaredType>, String> {
    let mut types = Vec::new();
    let mut at = 0;
    while at < lexemes.len() {
        let (ty, next) = parse_type(lexemes, at)?;
        types.push(ty);
        at = next;
    }
    Ok(types)
}

fn parse_type(lexemes: &[&str], at: usize) -> std::result::Result<(DeclaredType, usize), String> {
    match lexemes.get(at).copied() {
        Some("int") => Ok((DeclaredType::Int, at + 1)),
        Some("rat") => Ok((DeclaredType::Rat, at + 1)),
        Some("bool") => Ok((DeclaredType::Bool, at + 1)),
        Some("str") => Ok((DeclaredType::Str, at + 1)),
        Some("[") => {
            let (element, next) = parse_type(lexemes, at + 1)?;
            match lexemes.get(next).copied() {
                Some("]") => Ok((DeclaredType::Vector(Box::new(element)), next + 1)),
                _ => Err("a Vector type is one element type in `[ ]`".to_string()),
            }
        }
        Some("{") => parse_record(lexemes, at + 1),
        Some("table") if lexemes.get(at + 1) == Some(&"{") => {
            let (record, next) = parse_record(lexemes, at + 2)?;
            let DeclaredType::Record(fields) = record else {
                unreachable!("parse_record answers a Record type");
            };
            Ok((DeclaredType::Table(fields), next))
        }
        Some(other) => Err(format!(
            "unknown type `{other}` (expected `int`, `rat`, `bool`, `str`, `[T]`, `{{name:T …}}` or `table{{name:T …}}`)"
        )),
        None => Err("a `[` with no element type".to_string()),
    }
}

/// The fields of a Record type after its `{`, up to and past its `}`.
fn parse_record(
    lexemes: &[&str],
    mut at: usize,
) -> std::result::Result<(DeclaredType, usize), String> {
    let mut fields: Vec<(String, DeclaredType)> = Vec::new();
    loop {
        match (lexemes.get(at).copied(), lexemes.get(at + 1).copied()) {
            (Some("}"), _) => return Ok((DeclaredType::Record(fields), at + 1)),
            (Some(name), Some(":")) if !matches!(name, "{" | "[" | "]" | ":") => {
                if fields.iter().any(|(known, _)| known == name) {
                    return Err(format!("a Record type names the field `{name}` twice"));
                }
                let (ty, next) = parse_type(lexemes, at + 2)?;
                fields.push((name.to_string(), ty));
                at = next;
            }
            (None, _) => return Err("a `{` with no `}`".to_string()),
            (Some(other), _) => {
                return Err(format!(
                    "a Record type is `{{name:T …}}`, and `{other}` is not `name:`"
                ))
            }
        }
    }
}

/// Where `value` first falls outside `ty`, as a phrase naming the lane, or
/// `None` when it fits.
pub fn first_misfit(value: &Value, ty: &DeclaredType) -> Option<String> {
    let scalar_misfit = |fits: bool| (!fits).then(|| format!("{value} is not {ty}"));
    match (&value.data, ty) {
        (ValueData::Nil(_), _) => None,
        (ValueData::Scalar(f), DeclaredType::Int) => {
            scalar_misfit(matches!(f.extract_i64_pair(), Some((_, 1))))
        }
        (ValueData::Scalar(f), DeclaredType::Rat) => scalar_misfit(f.extract_i64_pair().is_some()),
        (ValueData::Boolean(_), DeclaredType::Bool) | (ValueData::Text(_), DeclaredType::Str) => {
            None
        }
        (ValueData::Truths(_), DeclaredType::Vector(element))
            if **element == DeclaredType::Bool =>
        {
            None
        }
        (ValueData::Vector(items), DeclaredType::Vector(element)) => {
            items.iter().enumerate().find_map(|(index, item)| {
                first_misfit(item, element).map(|why| format!("element {index}: {why}"))
            })
        }
        (ValueData::Tensor { data, shape }, DeclaredType::Vector(_)) => {
            let mut element = ty;
            for _ in shape.iter() {
                match element {
                    DeclaredType::Vector(inner) => element = inner,
                    _ => return Some(format!("{value} nests deeper than {ty}")),
                }
            }
            // A dense column holds machine-word pairs in lowest terms, so
            // every lane is a `rat`, and one with no denominator but 1 is an
            // `int` throughout: the common case is answered without a walk.
            match element {
                DeclaredType::Rat => return None,
                DeclaredType::Int if data.is_pure_integer => return None,
                _ => {}
            }
            let fits = |index: usize| {
                matches!(
                    (element, data.fraction_at(index).extract_i64_pair()),
                    (DeclaredType::Int, Some((_, 1))) | (DeclaredType::Rat, Some(_))
                )
            };
            (0..data.len())
                .find(|index| !fits(*index))
                .map(|index| format!("lane {index}: {} is not {element}", data.fraction_at(index)))
        }
        (ValueData::Record(record), DeclaredType::Record(fields)) => {
            // The keys are a set: each named once, in any order.
            let items: Option<Vec<&Value>> = (record.len() == fields.len())
                .then(|| {
                    fields
                        .iter()
                        .map(|(name, _)| record.get(&Value::from_string(name)))
                        .collect()
                })
                .flatten();
            let Some(items) = items else {
                let keys: Vec<String> = record.keys().iter().map(ToString::to_string).collect();
                return Some(format!(
                    "a Record with keys [ {} ] is not {ty}",
                    keys.join(" ")
                ));
            };
            fields.iter().zip(items).find_map(|((name, field), item)| {
                first_misfit(item, field).map(|why| format!("field {name}: {why}"))
            })
        }
        (ValueData::Record(record), DeclaredType::Table(fields)) => {
            if let Some(why) = first_misfit(value, &DeclaredType::columns(fields)) {
                return Some(why);
            }
            let mut lengths = record
                .keys()
                .iter()
                .zip(record.values())
                .filter(|(_, column)| !column.is_nil());
            let (first_key, first) = lengths.next()?;
            lengths
                .find(|(_, column)| column.len() != first.len())
                .map(|(key, column)| {
                    format!(
                        "column {key} is {} long where column {first_key} is {}",
                        column.len(),
                        first.len()
                    )
                })
        }
        _ => Some(format!("{value} is not {ty}")),
    }
}

/// The `typeViolation` a misfit answers.
fn violation(word: &str, message: String) -> AjisaiError {
    AjisaiError::TypeViolation {
        word: word.to_string(),
        message,
    }
}

impl crate::interpreter::Interpreter {
    /// The operands a typed Word is about to take, against its input types.
    /// A stack too short is left to the body, which reports the underflow it
    /// always has.
    pub(crate) fn check_typed_inputs(&self, word: &str, sig: &Signature) -> Result<()> {
        let count = sig.inputs.len();
        let Some(base) = self.stack.len().checked_sub(count) else {
            return Ok(());
        };
        for (position, ty) in sig.inputs.iter().enumerate() {
            if let Some(why) = first_misfit(&self.stack[base + position], ty) {
                return Err(violation(
                    word,
                    format!(
                        "input {} of `{sig}` was given a value outside {ty}: {why}",
                        position + 1
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The values a typed Word left, against its output types. `depth_before`
    /// is the stack's length before its operands were taken.
    pub(crate) fn check_typed_outputs(
        &self,
        word: &str,
        sig: &Signature,
        depth_before: usize,
    ) -> Result<()> {
        let base = depth_before.saturating_sub(sig.inputs.len());
        if self.stack.len() != base + sig.outputs.len() {
            return Err(violation(
                word,
                format!(
                    "`{sig}` leaves {} value(s), but the body left {}",
                    sig.outputs.len(),
                    self.stack.len().saturating_sub(base)
                ),
            ));
        }
        for (position, ty) in sig.outputs.iter().enumerate() {
            if let Some(why) = first_misfit(&self.stack[base + position], ty) {
                return Err(violation(
                    word,
                    format!(
                        "output {} of `{sig}` left the range of {ty}: {why}",
                        position + 1
                    ),
                ));
            }
        }
        Ok(())
    }
}
