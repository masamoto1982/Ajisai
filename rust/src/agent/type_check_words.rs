//! The type rules of the order, text and absence Words in a typed body
//! (LANG.CONTRACT.TYPES): each one the Word's own domain read at the level of
//! types, lifted over Vectors where the Word itself lifts.

use super::{data, join, zip_fields, Outcome, Ty};
use crate::types::Token;

fn str_like(t: &Ty) -> bool {
    matches!(t, Ty::Str(_) | Ty::Empty)
}

fn scalar_like(t: &Ty) -> bool {
    numeric(t) || str_like(t) || matches!(t, Ty::Bool)
}

fn pop(stack: &mut Vec<Ty>, word: &str) -> Outcome<Ty> {
    data(
        &stack
            .pop()
            .ok_or_else(|| format!("{word} would find the stack empty"))?,
    )
}

/// `true` when `w` is one of these Words and its rule was applied.
pub(super) fn word_rule(w: &str, stack: &mut Vec<Ty>) -> Outcome<bool> {
    let text = || Ty::Str(None);
    let result = match w {
        "UPPER" | "LOWER" | "TRIM" => {
            let a = pop(stack, w)?;
            lift1(&a, w, |x| str_like(x).then(text))?
        }
        "STR" => {
            let a = pop(stack, w)?;
            lift1(&a, w, |x| scalar_like(x).then(text))?
        }
        "CHARS" => {
            let a = pop(stack, w)?;
            lift1(&a, w, |x| str_like(x).then(|| Ty::Vec(Box::new(text()))))?
        }
        "FORMAT" => {
            let digits = pop(stack, w)?;
            if !matches!(digits, Ty::Int) {
                return Err(format!(
                    "FORMAT takes an int count of digits, not {}",
                    digits.show()
                ));
            }
            let a = pop(stack, w)?;
            lift1(&a, w, |x| numeric(x).then(text))?
        }
        "SEARCH" | "TOKENIZE" => {
            let b = pop(stack, w)?;
            let a = pop(stack, w)?;
            let answer = if w == "SEARCH" {
                Ty::Int
            } else {
                Ty::Vec(Box::new(text()))
            };
            lift2(&a, &b, w, |x, y| {
                (str_like(x) && str_like(y)).then(|| answer.clone())
            })?
        }
        "REPLACE" => {
            let by = pop(stack, w)?;
            let what = pop(stack, w)?;
            let a = pop(stack, w)?;
            if !(str_like(&by) && str_like(&what)) {
                return Err("REPLACE takes two str to replace one by the other".to_string());
            }
            lift1(&a, w, |x| str_like(x).then(text))?
        }
        "JOIN" => match pop(stack, w)? {
            Ty::Vec(element) if str_like(&element) => text(),
            other => return Err(format!("JOIN takes [str], not {}", other.show())),
        },
        "ORDER" => match pop(stack, w)? {
            Ty::Vec(_) => Ty::Vec(Box::new(Ty::Int)),
            other => return Err(format!("ORDER takes a Vector, not {}", other.show())),
        },
        "FLATTEN" => match pop(stack, w)? {
            vector @ Ty::Vec(_) => Ty::Vec(Box::new(vector.split().0.clone())),
            other => return Err(format!("FLATTEN takes a Vector, not {}", other.show())),
        },
        "DEPTH" => {
            pop(stack, w)?;
            Ty::Int
        }
        "NIL?" => {
            pop(stack, w)?;
            Ty::Bool
        }
        _ => return Ok(false),
    };
    stack.push(result);
    Ok(true)
}

/// An element-wise Word over operands of any depth (LANG.COLLECTIONS.LIFT):
/// a Record lifts first, field by field, then a Vector element by element,
/// so the deeper operand decides the result's shape.
pub(super) fn lift2(
    a: &Ty,
    b: &Ty,
    word: &str,
    scalar: impl Fn(&Ty, &Ty) -> Option<Ty>,
) -> Outcome<Ty> {
    lifted2(a, b, &scalar)
        .ok_or_else(|| format!("{word} has no rule for {} and {}", a.show(), b.show()))
}

fn lifted2(a: &Ty, b: &Ty, scalar: &dyn Fn(&Ty, &Ty) -> Option<Ty>) -> Option<Ty> {
    let each = |fields: &[(String, Ty)], f: &dyn Fn(&Ty) -> Option<Ty>| {
        fields
            .iter()
            .map(|(name, ty)| f(ty).map(|t| (name.clone(), t)))
            .collect::<Option<Vec<_>>>()
            .map(Ty::Rec)
    };
    match (a, b) {
        (Ty::Rec(f), Ty::Rec(g)) => zip_fields(f, g, |x, y| lifted2(x, y, scalar)).map(Ty::Rec),
        (Ty::Rec(f), other) => each(f, &|x| lifted2(x, other, scalar)),
        (other, Ty::Rec(g)) => each(g, &|y| lifted2(other, y, scalar)),
        (Ty::Vec(x), Ty::Vec(y)) => lifted2(x, y, scalar).map(|t| Ty::Vec(Box::new(t))),
        (Ty::Vec(x), other) => lifted2(x, other, scalar).map(|t| Ty::Vec(Box::new(t))),
        (other, Ty::Vec(y)) => lifted2(other, y, scalar).map(|t| Ty::Vec(Box::new(t))),
        _ => scalar(a, b),
    }
}

pub(super) fn lift1(a: &Ty, word: &str, scalar: impl Fn(&Ty) -> Option<Ty>) -> Outcome<Ty> {
    lift2(a, &Ty::Empty, word, |x, _| scalar(x))
        .map_err(|_| format!("{word} has no rule for {}", a.show()))
}

pub(super) fn numeric(t: &Ty) -> bool {
    matches!(t, Ty::Int | Ty::Rat | Ty::Empty)
}

pub(super) fn arith(a: &Ty, b: &Ty) -> Option<Ty> {
    (numeric(a) && numeric(b)).then(|| join(a, b).unwrap_or(Ty::Rat))
}

pub(super) fn closing(tokens: &[Token], open: usize) -> Outcome<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token {
            Token::VectorStart => depth += 1,
            Token::VectorEnd => {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err("an unclosed `[`".to_string())
}

pub(super) fn show_token(token: &Token) -> String {
    match token {
        Token::Number(literal) => literal.lexeme().to_string(),
        Token::String(text) => format!("'{text}'"),
        Token::Symbol(name) => name.as_str().to_string(),
        Token::VectorStart => "[".into(),
        Token::VectorEnd => "]".into(),
        Token::Value(_) => "a value".into(),
    }
}
