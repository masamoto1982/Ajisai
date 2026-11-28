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

#[path = "type_check_records.rs"]
mod records;
use records::{record_rule, zip_fields};
#[path = "type_check_words.rs"]
mod words;
use words::{arith, closing, lift1, lift2, numeric, show_token, word_rule};

/// Where the walk finds the body of an untyped User Word a typed body calls.
pub(crate) type Bodies<'a> = &'a dyn Fn(&str) -> Option<Arc<[Token]>>;

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
    /// A Record of exactly these String keys, in this order.
    Rec(Vec<(String, Ty)>),
    /// The Vector `COLLECT` gathers, each position's type kept apart until
    /// it is read as data, so `RECORD` can give each field its own.
    Tuple(Vec<Ty>),
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
            DeclaredType::Record(fields) => Ty::Rec(
                fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), Ty::of(ty)))
                    .collect(),
            ),
        }
    }

    fn show(&self) -> String {
        match self {
            Ty::Int => "int".into(),
            Ty::Rat => "rat".into(),
            Ty::Bool => "bool".into(),
            Ty::Str(_) => "str".into(),
            Ty::Vec(element) => format!("[{}]", element.show()),
            Ty::Rec(fields) => {
                let fields: Vec<String> = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}:{}", ty.show()))
                    .collect();
                format!("{{{}}}", fields.join(" "))
            }
            Ty::Tuple(items) => {
                let items: Vec<String> = items.iter().map(Ty::show).collect();
                format!("[{}]", items.join(" "))
            }
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
        (Ty::Rec(f), Ty::Rec(g)) => Ty::Rec(zip_fields(f, g, join)?),
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
    bodies: Bodies<'a>,
    locals: HashMap<String, Ty>,
}

/// Check `body` against `sig`, or say why it does not type.
pub(crate) fn check_body(
    body: &[Token],
    sig: &Signature,
    signatures: &HashMap<String, Signature>,
    bodies: Bodies<'_>,
) -> Outcome<()> {
    let mut walk = Walk {
        signatures,
        bodies,
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
    let tokens = match ty {
        Ty::Block(tokens) => tokens,
        Ty::Tuple(items) => {
            let element = items.iter().try_fold(Ty::Empty, |a, b| join(&a, b));
            return element.map(|e| Ty::Vec(Box::new(e))).ok_or_else(|| {
                format!("COLLECT gathers {}, which do not join into one Vector type; give them to RECORD", ty.show())
            });
        }
        _ => return Ok(ty.clone()),
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

impl Walk<'_> {
    fn run(&mut self, body: &[Token], mut stack: Vec<Ty>) -> Outcome<Vec<Ty>> {
        let mut at = 0;
        while at < body.len() {
            match &body[at] {
                Token::Number(literal) if Self::collects(body, at) => {
                    let count = literal.value().and_then(|f| f.extract_i64_pair());
                    let Some((n, 1)) = count.filter(|&(n, _)| n >= 0) else {
                        return Err("COLLECT takes a count that is a whole number".into());
                    };
                    let base = stack
                        .len()
                        .checked_sub(usize::try_from(n).unwrap_or(usize::MAX))
                        .ok_or("COLLECT would find too few values on the stack")?;
                    let items = stack
                        .drain(base..)
                        .map(|t| data(&t))
                        .collect::<Outcome<_>>()?;
                    stack.push(Ty::Tuple(items));
                    at += 1;
                }
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

    /// A count written right before `COLLECT`, the one count a typed body's
    /// `COLLECT` takes: the walk must know how many values it gathers.
    fn collects(body: &[Token], at: usize) -> bool {
        matches!(body.get(at + 1), Some(Token::Symbol(name))
            if canonical_word_name(name.as_str()) == "COLLECT")
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

    /// An untyped User Word called from a typed body, walked with the types
    /// it is given in a frame of its own. The dictionary is acyclic
    /// (LANG.DICTIONARY.ACYCLIC), so the walk ends.
    fn untyped_call(&mut self, w: &str, stack: &mut Vec<Ty>) -> Outcome<()> {
        let Some(body) = (self.bodies)(w) else {
            return Err(format!(
                "{w} has no type rule yet; call it from an untyped Word instead"
            ));
        };
        let saved = std::mem::take(&mut self.locals);
        let left = self.run(&body, std::mem::take(stack));
        self.locals = saved;
        *stack = left.map_err(|why| format!("{w}, walked with the types it is given: {why}"))?;
        Ok(())
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
            "GET" | "PUT" | "HAS?" | "WITHOUT" | "KEYS" | "VALUES" | "MERGE" | "RECORD"
                if record_rule(w, stack)? => {}
            "GET" | "TAKE" | "DROP" => {
                let index = Self::pop_data(stack, w)?;
                let many = w == "GET" && matches!(&index, Ty::Vec(i) if numeric(i));
                if !(matches!(index, Ty::Int) || many) {
                    return Err(format!("{w} takes an int position, not {}", index.show()));
                }
                let vector = Self::pop_data(stack, w)?;
                let element = Self::element_of(&vector, w)?;
                stack.push(if w == "GET" && !many { element } else { vector });
            }
            "COLLECT" => {
                return Err(
                    "COLLECT in a typed Word takes its count written right before it".into(),
                )
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
                let value = match Self::pop(stack, w)? {
                    tuple @ Ty::Tuple(_) => tuple,
                    other => data(&other)?,
                };
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
            _ if word_rule(w, stack)? => {}
            _ => {
                let Some(sig) = self.signatures.get(w) else {
                    return self.untyped_call(w, stack);
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
