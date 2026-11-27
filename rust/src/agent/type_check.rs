//! The pre-execution type check of a typed User Word (LANG.CONTRACT.TYPES).
//!
//! The body is walked once with the declared input types on an abstract
//! stack, every Word it calls replaced by its type rule, and what is left is
//! held to the declared output types. Nothing runs. Phase one deliberately
//! covers a small vocabulary — the numbers, truth, Vectors and the
//! higher-order Words over a literal block — and a Word outside it is a
//! finding, not a pass: a typed Word whose body cannot be typed is not
//! verified, and the repair is to move that part into an untyped Word.

use crate::interpreter::word_types::{DeclaredType, Signature};
use crate::types::Token;
use crate::word_name::canonical_word_name;
use std::collections::HashMap;
use std::sync::Arc;

/// A type on the abstract stack. `Block` is a bracketed literal not yet read
/// as code or as data; `Empty` is the element of an empty Vector or a NIL,
/// which fits every type.
#[derive(Debug, Clone, PartialEq)]
enum Ty {
    Int,
    Rat,
    Bool,
    Str(Option<String>),
    Vec(Box<Ty>),
    Empty,
    Block(Arc<[Token]>),
}

impl Ty {
    fn of(declared: &DeclaredType) -> Ty {
        match declared {
            DeclaredType::Int => Ty::Int,
            DeclaredType::Rat => Ty::Rat,
            DeclaredType::Bool => Ty::Bool,
            DeclaredType::Str => Ty::Str(None),
            DeclaredType::Vector(element) => Ty::Vec(Box::new(Ty::of(element))),
        }
    }

    fn show(&self) -> String {
        match self {
            Ty::Int => "int".into(),
            Ty::Rat => "rat".into(),
            Ty::Bool => "bool".into(),
            Ty::Str(_) => "str".into(),
            Ty::Vec(element) => format!("[{}]", element.show()),
            Ty::Empty => "nil".into(),
            Ty::Block(_) => "a block".into(),
        }
    }

    /// The scalar at the bottom of the Vectors and how many Vectors deep.
    fn split(&self) -> (&Ty, usize) {
        match self {
            Ty::Vec(element) => {
                let (scalar, depth) = element.split();
                (scalar, depth + 1)
            }
            other => (other, 0),
        }
    }

    fn wrap(scalar: Ty, depth: usize) -> Ty {
        (0..depth).fold(scalar, |ty, _| Ty::Vec(Box::new(ty)))
    }
}

/// The least type both fit, if there is one: `int` widens to `rat`.
fn join(a: &Ty, b: &Ty) -> Option<Ty> {
    Some(match (a, b) {
        (Ty::Empty, other) | (other, Ty::Empty) => other.clone(),
        (Ty::Int, Ty::Int) => Ty::Int,
        (Ty::Int | Ty::Rat, Ty::Int | Ty::Rat) => Ty::Rat,
        (Ty::Bool, Ty::Bool) => Ty::Bool,
        (Ty::Str(_), Ty::Str(_)) => Ty::Str(None),
        (Ty::Vec(x), Ty::Vec(y)) => Ty::Vec(Box::new(join(x, y)?)),
        _ => return None,
    })
}

fn fits(actual: &Ty, expected: &Ty) -> bool {
    join(actual, expected).as_ref() == Some(expected)
        || matches!((actual, expected), (Ty::Str(_), Ty::Str(_)))
}

type Outcome<T> = std::result::Result<T, String>;

struct Walk<'a> {
    signatures: &'a HashMap<String, Signature>,
    locals: HashMap<String, Ty>,
}

/// Check `body` against `sig`, or say why it does not type.
pub(crate) fn check_body(
    body: &[Token],
    sig: &Signature,
    signatures: &HashMap<String, Signature>,
) -> Outcome<()> {
    let mut walk = Walk {
        signatures,
        locals: HashMap::new(),
    };
    let start: Vec<Ty> = sig.inputs.iter().map(Ty::of).collect();
    let left = walk.run(body, start)?;
    let expected: Vec<Ty> = sig.outputs.iter().map(Ty::of).collect();
    if left.len() != expected.len() {
        return Err(format!(
            "the body leaves {} value(s) where `{sig}` leaves {}",
            left.len(),
            expected.len()
        ));
    }
    for (position, (actual, want)) in left.iter().zip(&expected).enumerate() {
        let actual = data(actual)?;
        if !fits(&actual, want) {
            return Err(format!(
                "output {} is {} where `{sig}` declares {}",
                position + 1,
                actual.show(),
                want.show()
            ));
        }
    }
    Ok(())
}

/// A bracketed literal read as data: the Vector type its elements give.
fn data(ty: &Ty) -> Outcome<Ty> {
    let Ty::Block(tokens) = ty else {
        return Ok(ty.clone());
    };
    let mut element = Ty::Empty;
    let mut at = 0;
    while at < tokens.len() {
        let (item, next) = match &tokens[at] {
            Token::Number(literal) => (number(literal), at + 1),
            Token::String(_) => (Ty::Str(None), at + 1),
            Token::Symbol(name) if canonical_word_name(name.as_str()) == "NIL" => {
                (Ty::Empty, at + 1)
            }
            Token::VectorStart => {
                let close = closing(tokens, at)?;
                (data(&Ty::Block(tokens[at + 1..close].into()))?, close + 1)
            }
            other => {
                return Err(format!(
                    "the Vector literal holds `{}`, which has no type",
                    show_token(other)
                ))
            }
        };
        element = join(&element, &item)
            .ok_or_else(|| "a Vector literal mixes types that do not join".to_string())?;
        at = next;
    }
    Ok(Ty::Vec(Box::new(element)))
}

fn number(literal: &crate::types::NumberLiteral) -> Ty {
    match literal.value().and_then(|f| f.extract_i64_pair()) {
        Some((_, 1)) => Ty::Int,
        _ => Ty::Rat,
    }
}

fn closing(tokens: &[Token], open: usize) -> Outcome<usize> {
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

fn show_token(token: &Token) -> String {
    match token {
        Token::Number(literal) => literal.lexeme().to_string(),
        Token::String(text) => format!("'{text}'"),
        Token::Symbol(name) => name.as_str().to_string(),
        Token::VectorStart => "[".into(),
        Token::VectorEnd => "]".into(),
        Token::Value(_) => "a value".into(),
    }
}

/// An element-wise Word over operands of any depth (LANG.COLLECTIONS.LIFT):
/// the deeper operand decides the result's depth.
fn lift2(a: &Ty, b: &Ty, word: &str, scalar: impl Fn(&Ty, &Ty) -> Option<Ty>) -> Outcome<Ty> {
    let ((sa, da), (sb, db)) = (a.split(), b.split());
    let result = scalar(sa, sb)
        .ok_or_else(|| format!("{word} has no rule for {} and {}", a.show(), b.show()))?;
    Ok(Ty::wrap(result, da.max(db)))
}

fn lift1(a: &Ty, word: &str, scalar: impl Fn(&Ty) -> Option<Ty>) -> Outcome<Ty> {
    let (s, depth) = a.split();
    let result = scalar(s).ok_or_else(|| format!("{word} has no rule for {}", a.show()))?;
    Ok(Ty::wrap(result, depth))
}

fn numeric(t: &Ty) -> bool {
    matches!(t, Ty::Int | Ty::Rat | Ty::Empty)
}

fn arith(a: &Ty, b: &Ty) -> Option<Ty> {
    (numeric(a) && numeric(b)).then(|| join(a, b).unwrap_or(Ty::Rat))
}

impl Walk<'_> {
    fn run(&mut self, body: &[Token], mut stack: Vec<Ty>) -> Outcome<Vec<Ty>> {
        let mut at = 0;
        while at < body.len() {
            match &body[at] {
                Token::Number(literal) => stack.push(number(literal)),
                Token::String(text) => stack.push(Ty::Str(Some(text.to_string()))),
                Token::VectorStart => {
                    let close = closing(body, at)?;
                    stack.push(Ty::Block(body[at + 1..close].into()));
                    at = close;
                }
                Token::VectorEnd => return Err("an unopened `]`".to_string()),
                Token::Value(_) => return Err("a value with no source spelling".to_string()),
                Token::Symbol(name) => self.word(name.as_str(), &mut stack)?,
            }
            at += 1;
        }
        Ok(stack)
    }

    fn pop(stack: &mut Vec<Ty>, word: &str) -> Outcome<Ty> {
        stack
            .pop()
            .ok_or_else(|| format!("{word} would find the stack empty"))
    }

    fn pop_data(stack: &mut Vec<Ty>, word: &str) -> Outcome<Ty> {
        data(&Self::pop(stack, word)?)
    }

    fn pop_block(stack: &mut Vec<Ty>, word: &str) -> Outcome<Arc<[Token]>> {
        match Self::pop(stack, word)? {
            Ty::Block(tokens) => Ok(tokens),
            other => Err(format!(
                "{word} in a typed Word takes a block written right there, not {}",
                other.show()
            )),
        }
    }

    fn element_of(ty: &Ty, word: &str) -> Outcome<Ty> {
        match ty {
            Ty::Vec(element) => Ok((**element).clone()),
            other => Err(format!("{word} takes a Vector, not {}", other.show())),
        }
    }

    /// The one value a block leaves from `operands`.
    fn block(&mut self, tokens: &[Token], operands: Vec<Ty>, word: &str) -> Outcome<Ty> {
        let saved = self.locals.clone();
        let left = self.run(tokens, operands);
        self.locals = saved;
        let mut left = left?;
        match (left.pop(), left.is_empty()) {
            (Some(result), true) => data(&result),
            _ => Err(format!(
                "the block of {word} does not leave exactly one value"
            )),
        }
    }

    fn word(&mut self, name: &str, stack: &mut Vec<Ty>) -> Outcome<()> {
        let word = canonical_word_name(name);
        if let Some(ty) = self.locals.get(word.as_ref()) {
            stack.push(ty.clone());
            return Ok(());
        }
        let w = word.as_ref();
        match w {
            "TRUE" | "FALSE" => stack.push(Ty::Bool),
            "NIL" => stack.push(Ty::Empty),
            "ADD" | "SUB" | "MUL" | "MIN" | "MAX" | "DIV" | "EQ" | "LT" | "GT" | "AND" => {
                let b = Self::pop_data(stack, w)?;
                let a = Self::pop_data(stack, w)?;
                let result = match w {
                    "DIV" => lift2(&a, &b, w, |x, y| arith(x, y).map(|_| Ty::Rat))?,
                    "EQ" => lift2(&a, &b, w, |x, y| join(x, y).map(|_| Ty::Bool))?,
                    "LT" | "GT" => lift2(&a, &b, w, |x, y| arith(x, y).map(|_| Ty::Bool))?,
                    "AND" => lift2(&a, &b, w, |x, y| {
                        matches!((x, y), (Ty::Bool | Ty::Empty, Ty::Bool | Ty::Empty))
                            .then_some(Ty::Bool)
                    })?,
                    _ => lift2(&a, &b, w, arith)?,
                };
                stack.push(result);
            }
            "NOT" => {
                let a = Self::pop_data(stack, w)?;
                let result = lift1(&a, w, |x| {
                    matches!(x, Ty::Bool | Ty::Empty).then_some(Ty::Bool)
                })?;
                stack.push(result);
            }
            "FLOOR" | "ROUND" => {
                let a = Self::pop_data(stack, w)?;
                stack.push(lift1(&a, w, |x| numeric(x).then_some(Ty::Int))?);
            }
            "SELECT" => {
                let mask = Self::pop_data(stack, w)?;
                let otherwise = Self::pop_data(stack, w)?;
                let then = Self::pop_data(stack, w)?;
                let branches = lift2(&then, &otherwise, w, join)?;
                let (m, mask_depth) = mask.split();
                if !matches!(m, Ty::Bool | Ty::Empty) {
                    return Err(format!("SELECT's mask is {}, not bool", mask.show()));
                }
                let (scalar, depth) = branches.split();
                stack.push(Ty::wrap(scalar.clone(), depth.max(mask_depth)));
            }
            "LENGTH" => match Self::pop_data(stack, w)? {
                Ty::Vec(_) | Ty::Str(_) => stack.push(Ty::Int),
                other => return Err(format!("LENGTH takes a Vector, not {}", other.show())),
            },
            "GET" | "TAKE" | "DROP" => {
                let index = Self::pop_data(stack, w)?;
                if !matches!(index, Ty::Int) {
                    return Err(format!("{w} takes an int position, not {}", index.show()));
                }
                let vector = Self::pop_data(stack, w)?;
                let element = Self::element_of(&vector, w)?;
                stack.push(if w == "GET" { element } else { vector });
            }
            "REVERSE" | "SORT" | "UNIQUE" => {
                let vector = Self::pop_data(stack, w)?;
                Self::element_of(&vector, w)?;
                stack.push(vector);
            }
            "CONCAT" => {
                let b = Self::pop_data(stack, w)?;
                let a = Self::pop_data(stack, w)?;
                Self::element_of(&a, w)?;
                Self::element_of(&b, w)?;
                stack.push(join(&a, &b).ok_or_else(|| {
                    format!("CONCAT of {} and {} does not join", a.show(), b.show())
                })?);
            }
            "RANGE" => {
                let end = Self::pop_data(stack, w)?;
                let start = Self::pop_data(stack, w)?;
                if !(matches!(start, Ty::Int) && matches!(end, Ty::Int)) {
                    return Err("RANGE takes two ints".to_string());
                }
                stack.push(Ty::Vec(Box::new(Ty::Int)));
            }
            "MAP" | "FILTER" => {
                let block = Self::pop_block(stack, w)?;
                let vector = Self::pop_data(stack, w)?;
                let element = Self::element_of(&vector, w)?;
                let result = self.block(&block, vec![element], w)?;
                if w == "MAP" {
                    stack.push(Ty::Vec(Box::new(result)));
                } else if matches!(result, Ty::Bool | Ty::Empty) {
                    stack.push(vector);
                } else {
                    return Err(format!(
                        "FILTER's block answers {}, not bool",
                        result.show()
                    ));
                }
            }
            "FOLD" | "SCAN" => {
                let block = Self::pop_block(stack, w)?;
                let mut acc = Self::pop_data(stack, w)?;
                let vector = Self::pop_data(stack, w)?;
                let element = Self::element_of(&vector, w)?;
                // Twice at most: a seed of `int` the block widens to `rat`
                // settles on `rat` the second time round.
                for _ in 0..2 {
                    let next = self.block(&block, vec![acc.clone(), element.clone()], w)?;
                    if fits(&next, &acc) {
                        break;
                    }
                    acc = join(&acc, &next).ok_or_else(|| {
                        format!("{w}'s block turns {} into {}", acc.show(), next.show())
                    })?;
                }
                let settled = self.block(&block, vec![acc.clone(), element], w)?;
                if !fits(&settled, &acc) {
                    return Err(format!("{w}'s accumulator does not settle on one type"));
                }
                stack.push(if w == "SCAN" {
                    Ty::Vec(Box::new(acc))
                } else {
                    acc
                });
            }
            "BIND" => {
                let names = Self::pop(stack, w)?;
                let value = Self::pop_data(stack, w)?;
                match names {
                    Ty::Str(Some(name)) => {
                        self.locals.insert(name.to_uppercase(), value);
                    }
                    Ty::Block(tokens) => {
                        let element = Self::element_of(&value, w)?;
                        for token in tokens.iter() {
                            let Token::String(name) = token else {
                                return Err(
                                    "BIND destructures into names written as Strings".into()
                                );
                            };
                            self.locals.insert(name.to_uppercase(), element.clone());
                        }
                    }
                    _ => return Err("BIND in a typed Word takes a name written right there".into()),
                }
            }
            _ => {
                let Some(sig) = self.signatures.get(w) else {
                    return Err(format!(
                        "{w} has no type rule yet; call it from an untyped Word instead"
                    ));
                };
                let base = stack
                    .len()
                    .checked_sub(sig.inputs.len())
                    .ok_or_else(|| format!("{w} would find too few values on the stack"))?;
                for (position, declared) in sig.inputs.iter().enumerate() {
                    let actual = data(&stack[base + position])?;
                    if !fits(&actual, &Ty::of(declared)) {
                        return Err(format!(
                            "{w} takes {declared} as input {}, and is given {}",
                            position + 1,
                            actual.show()
                        ));
                    }
                }
                stack.truncate(base);
                stack.extend(sig.outputs.iter().map(Ty::of));
            }
        }
        Ok(())
    }
}
