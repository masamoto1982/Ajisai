//! The type rules of the Record Words in a typed body (LANG.CONTRACT.TYPES
//! over LANG.RECORDS.STRUCTURE). A Record type names its String keys in
//! order, so a rule needs the key written right there to know the field.

use super::{closing, data, join, show_token, Outcome, Ty};
use crate::types::Token;

/// The type rules of the Record Words (LANG.RECORDS.STRUCTURE), each over
/// a key written right there, so the field it names is known: `true` when
/// the Word was a Record Word on a Record, `false` to leave it to the
/// Vector rules.
pub(super) fn record_rule(w: &str, stack: &mut Vec<Ty>) -> Outcome<bool> {
    let key_of = |ty: &Ty| match ty {
        Ty::Str(Some(key)) => Ok(key.clone()),
        other => Err(format!(
            "{w} on a typed Record takes a key written right there as a String, not {}",
            other.show()
        )),
    };
    let field = |fields: &[(String, Ty)], key: &str| {
        fields
            .iter()
            .position(|(name, _)| name == key)
            .ok_or_else(|| format!("{w}: the Record has no field '{key}'"))
    };
    let depth = match w {
        "PUT" => 3,
        "KEYS" | "VALUES" => 1,
        _ => 2,
    };
    let Some(base) = stack.len().checked_sub(depth) else {
        return Err(format!("{w} would find too few values on the stack"));
    };
    let target = data(&stack[base])?;
    let Ty::Rec(mut fields) = target else {
        return match w {
            "GET" | "PUT" => Ok(false),
            "RECORD" => build_record(stack).map(|()| true),
            _ => Err(format!("{w} takes a Record, not {}", target.show())),
        };
    };
    let operands: Vec<Ty> = stack.drain(base..).collect();
    let result = match w {
        "GET" => fields[field(&fields, &key_of(&operands[1])?)?].1.clone(),
        "HAS?" => {
            key_of(&operands[1])?;
            Ty::Bool
        }
        "PUT" => {
            let key = key_of(&operands[1])?;
            let value = data(&operands[2])?;
            match fields.iter().position(|(name, _)| *name == key) {
                Some(at) => fields[at].1 = value,
                None => fields.push((key, value)),
            }
            Ty::Rec(fields)
        }
        "WITHOUT" => {
            let at = field(&fields, &key_of(&operands[1])?)?;
            fields.remove(at);
            Ty::Rec(fields)
        }
        "KEYS" => Ty::Vec(Box::new(Ty::Str(None))),
        "VALUES" => {
            let mut element = Ty::Empty;
            for (name, ty) in &fields {
                element = join(&element, ty).ok_or_else(|| {
                    format!("VALUES of a Record whose field '{name}' does not join the others")
                })?;
            }
            Ty::Vec(Box::new(element))
        }
        "MERGE" => {
            let Ty::Rec(right) = data(&operands[1])? else {
                return Err("MERGE takes two Records".to_string());
            };
            for (key, ty) in right {
                match fields.iter().position(|(name, _)| *name == key) {
                    Some(at) => fields[at].1 = ty,
                    None => fields.push((key, ty)),
                }
            }
            Ty::Rec(fields)
        }
        _ => return Err(format!("{w} takes a Vector of keys, not a Record")),
    };
    stack.push(result);
    Ok(true)
}

/// `RECORD` over keys written right there as Strings: each field takes the
/// type of the value at its position.
fn build_record(stack: &mut Vec<Ty>) -> Outcome<()> {
    let values = stack.pop().ok_or("RECORD would find the stack empty")?;
    let keys = stack.pop().ok_or("RECORD would find the stack empty")?;
    let Ty::Block(keys) = keys else {
        return Err("RECORD in a typed Word takes its keys written right there".to_string());
    };
    let names: Vec<String> = keys
        .iter()
        .map(|token| match token {
            Token::String(name) => Ok(name.to_string()),
            other => Err(format!(
                "RECORD's keys in a typed Word are Strings, not `{}`",
                show_token(other)
            )),
        })
        .collect::<Outcome<_>>()?;
    let types: Vec<Ty> = match &values {
        Ty::Block(tokens) => items(tokens)?,
        other => match data(other)? {
            Ty::Vec(element) => vec![*element; names.len()],
            other => {
                return Err(format!(
                    "RECORD takes a Vector of values, not {}",
                    other.show()
                ))
            }
        },
    };
    if types.len() != names.len() {
        return Err(format!(
            "RECORD is given {} keys and {} values",
            names.len(),
            types.len()
        ));
    }
    for (at, name) in names.iter().enumerate() {
        if names[..at].contains(name) {
            return Err(format!("RECORD is given the key '{name}' twice"));
        }
    }
    stack.push(Ty::Rec(names.into_iter().zip(types).collect()));
    Ok(())
}

/// The type of each item of a bracketed literal, one by one.
fn items(tokens: &[Token]) -> Outcome<Vec<Ty>> {
    let mut types = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        let end = match tokens[at] {
            Token::VectorStart => closing(tokens, at)?,
            _ => at,
        };
        let Ty::Vec(element) = data(&Ty::Block(tokens[at..=end].into()))? else {
            unreachable!("data answers a Vector for a block");
        };
        types.push(*element);
        at = end + 1;
    }
    Ok(types)
}

/// Two field lists with the same names in the same order, combined field by
/// field; `None` when the names differ or a field does not combine.
pub(super) fn zip_fields(
    f: &[(String, Ty)],
    g: &[(String, Ty)],
    combine: impl Fn(&Ty, &Ty) -> Option<Ty>,
) -> Option<Vec<(String, Ty)>> {
    if f.len() != g.len() {
        return None;
    }
    f.iter()
        .zip(g)
        .map(|((a, x), (b, y))| {
            (a == b)
                .then(|| combine(x, y))
                .flatten()
                .map(|t| (a.clone(), t))
        })
        .collect()
}
