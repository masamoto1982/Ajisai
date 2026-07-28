//! # ajisai-music
//!
//! Exact just-intonation music vocabulary for Ajisai, as an external package.
//!
//! Ajisai Core does not know this crate exists. There is no music feature
//! flag, no music value shape, no music role, no music stub, and no reserved
//! name. This package is built entirely out of the public extension surface:
//! it supplies words, and words are all it can supply.
//!
//! ## Why there is no equal temperament here
//!
//! Twelve-tone equal temperament puts the twelfth root of two in every
//! interval, and that number is not rational. Ajisai has no floating point and
//! no approximation, so a `MUSIC:EDO` word could only lie about its result or
//! carry an unevaluated symbol Ajisai has no shape for. Just intonation is
//! what an exact language can actually say: every interval here is a ratio of
//! integers, and a chain of them stays exact no matter how long it gets.
//!
//! That is a design consequence, not a limitation worked around.
//!
//! ## What a note is, and what this package cannot promise
//!
//! A note is `[ frequency beats ]` — an ordinary two-element vector, because a
//! vector is all a package can make. Ajisai's extension surface supplies
//! *words*, not domain types: this crate cannot add a value shape, cannot add a
//! Semantic Plane role, and therefore cannot make a note distinguishable from
//! any other two-element vector of numbers (`SPECIFICATION.md` §13).
//!
//! What it can do is check values, and it does: a frequency may not be
//! negative, a duration may not be negative, an interval and a tempo must both
//! be positive. So `[ -100 -3 ] MUSIC:PITCH` is refused. What it cannot do is
//! stop you building `[ 440 1 ]` by other means and calling it a note — and
//! that is a property of the boundary, stated here rather than glossed over.
//!
//! ## Stability
//!
//! This package sets its own policy, and it is not Ajisai Core's. These words
//! may change between minor versions. Nothing here is part of what it means to
//! be a conforming Ajisai implementation, and a host that never registers this
//! package is not missing anything the language promises.
//!
//! ## Use
//!
//! ```
//! use ajisai_core::Interpreter;
//!
//! let mut ajisai = Interpreter::new();
//! ajisai.register_package(ajisai_music::package()).unwrap();
//! // A perfect fifth above A440, exactly.
//! ajisai.execute("440 3 2 MUSIC:JUST").unwrap();
//! assert_eq!(ajisai.stack()[0].to_string(), "660");
//! ```

use ajisai_core::contract::{Arity, Body, Policy, StakSupport, TypeSpec, WordContract};
use ajisai_core::extension::Package;
use ajisai_core::{Error, Number, Result, Value};

const PACKAGE: &str = "ajisai-music";

/// The package, ready to register with an interpreter.
pub fn package() -> Package {
    Package::new(PACKAGE)
        .with(
            contract(
                "MUSIC:JUST",
                Shape {
                    stack_effect: "( base numerator denominator -- frequency )",
                    inn: 3,
                    out: 1,
                    input_types: &[TypeSpec::Number, TypeSpec::Number, TypeSpec::Number],
                    output_types: &[TypeSpec::Number],
                    stak: StakSupport::Unsupported,
                },
                "A frequency an exact ratio above a base frequency.",
            ),
            Body::Op(just),
        )
        .with(
            contract(
                "MUSIC:NOTE",
                Shape {
                    stack_effect: "( frequency beats -- note )",
                    inn: 2,
                    out: 1,
                    input_types: &[TypeSpec::Number, TypeSpec::Number],
                    output_types: &[TypeSpec::Vector],
                    stak: StakSupport::Unsupported,
                },
                "A note: a frequency paired with a duration in beats.",
            ),
            Body::Op(note),
        )
        .with(
            contract(
                "MUSIC:REST",
                Shape {
                    stack_effect: "( beats -- note )",
                    inn: 1,
                    out: 1,
                    input_types: &[TypeSpec::Number],
                    output_types: &[TypeSpec::Vector],
                    stak: StakSupport::MapEach,
                },
                "A silence of the given duration, written as a note at frequency 0.",
            ),
            Body::Op(rest),
        )
        .with(
            contract(
                "MUSIC:PITCH",
                Shape {
                    stack_effect: "( note -- frequency )",
                    inn: 1,
                    out: 1,
                    input_types: &[TypeSpec::Vector],
                    output_types: &[TypeSpec::Number],
                    stak: StakSupport::MapEach,
                },
                "A note's frequency.",
            ),
            Body::Op(pitch),
        )
        .with(
            contract(
                "MUSIC:BEATS",
                Shape {
                    stack_effect: "( note -- beats )",
                    inn: 1,
                    out: 1,
                    input_types: &[TypeSpec::Vector],
                    output_types: &[TypeSpec::Number],
                    stak: StakSupport::MapEach,
                },
                "A note's duration in beats.",
            ),
            Body::Op(beats),
        )
        .with(
            contract(
                "MUSIC:TRANSPOSE",
                Shape {
                    stack_effect: "( note numerator denominator -- note )",
                    inn: 3,
                    out: 1,
                    input_types: &[TypeSpec::Vector, TypeSpec::Number, TypeSpec::Number],
                    output_types: &[TypeSpec::Vector],
                    stak: StakSupport::Unsupported,
                },
                "The same note, its frequency multiplied by an exact ratio.",
            ),
            Body::Op(transpose),
        )
        .with(
            contract(
                "MUSIC:SECONDS",
                Shape {
                    stack_effect: "( note tempo -- seconds )",
                    inn: 2,
                    out: 1,
                    input_types: &[TypeSpec::Vector, TypeSpec::Number],
                    output_types: &[TypeSpec::Number],
                    stak: StakSupport::Unsupported,
                },
                "A note's duration in seconds at a tempo in beats per minute.",
            ),
            Body::Op(seconds),
        )
}

/// A word's shape: everything about it that is not its name or its prose.
struct Shape {
    stack_effect: &'static str,
    inn: u8,
    out: u8,
    input_types: &'static [TypeSpec],
    output_types: &'static [TypeSpec],
    stak: StakSupport,
}

fn contract(name: &'static str, shape: Shape, summary: &'static str) -> WordContract {
    let Shape {
        stack_effect,
        inn,
        out,
        input_types,
        output_types,
        stak,
    } = shape;
    WordContract {
        name,
        stack_effect,
        arity: Arity::Fixed { inn, out },
        input_types,
        output_types,
        // Music has no reading for an absent or undetermined pitch, so both
        // are refused rather than propagated into a frequency.
        nil_policy: Policy::REFUSES,
        unknown_policy: Policy::REFUSES,
        stak,
        // Nothing here reads the Semantic Plane. A note is a vector, and this
        // package cannot add a role to say more than that; see the module
        // documentation on what a package can and cannot do.
        role_required: None,
        summary,
    }
}

fn number(word: &str, value: &Value) -> Result<Number> {
    value
        .as_number()
        .cloned()
        .ok_or_else(|| Error::TypeMismatch {
            word: word.to_string(),
            expected: "number".to_string(),
            found: value.type_name().to_string(),
        })
}

/// A quantity that must not be negative: a frequency, or a duration.
fn non_negative(word: &str, what: &str, value: &Value) -> Result<Number> {
    let n = number(word, value)?;
    if n.is_negative() {
        return Err(Error::TypeMismatch {
            word: word.to_string(),
            expected: format!("a {what} that is not negative"),
            found: n.to_string(),
        });
    }
    Ok(n)
}

/// A quantity that must be strictly positive: an interval, or a tempo.
fn positive(word: &str, what: &str, n: Number) -> Result<Number> {
    if n.is_negative() || n.is_zero() {
        return Err(Error::TypeMismatch {
            word: word.to_string(),
            expected: format!("a positive {what}"),
            found: n.to_string(),
        });
    }
    Ok(n)
}

/// Read a note: a two-element vector of frequency and beats.
fn read_note(word: &str, value: &Value) -> Result<(Number, Number)> {
    let items = value.as_vector().ok_or_else(|| Error::TypeMismatch {
        word: word.to_string(),
        expected: "note".to_string(),
        found: value.type_name().to_string(),
    })?;
    if items.len() != 2 {
        return Err(Error::TypeMismatch {
            word: word.to_string(),
            expected: "a note of [ frequency beats ]".to_string(),
            found: format!("a vector of {} element(s)", items.len()),
        });
    }
    Ok((
        non_negative(word, "frequency", &items[0])?,
        non_negative(word, "duration", &items[1])?,
    ))
}

fn ratio(word: &str, numerator: &Value, denominator: &Value) -> Result<Number> {
    let n = number(word, numerator)?;
    let d = number(word, denominator)?;
    let ratio = n.checked_div(&d).ok_or(Error::DivisionByZero)?;
    positive(word, "interval", ratio)
}

fn just(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [base, numerator, denominator] = args else {
        return Err(arity(word, 3, args.len()));
    };
    let interval = ratio(word, numerator, denominator)?;
    let base = non_negative(word, "frequency", base)?;
    Ok(vec![Value::number(&base * &interval)])
}

fn note(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [frequency, beats] = args else {
        return Err(arity(word, 2, args.len()));
    };
    let frequency = non_negative(word, "frequency", frequency)?;
    let beats = non_negative(word, "duration", beats)?;
    Ok(vec![Value::vector(vec![
        Value::number(frequency),
        Value::number(beats),
    ])])
}

fn rest(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [beats] = args else {
        return Err(arity(word, 1, args.len()));
    };
    Ok(vec![Value::vector(vec![
        Value::integer(0),
        Value::number(non_negative(word, "duration", beats)?),
    ])])
}

fn pitch(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [note] = args else {
        return Err(arity(word, 1, args.len()));
    };
    Ok(vec![Value::number(read_note(word, note)?.0)])
}

fn beats(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [note] = args else {
        return Err(arity(word, 1, args.len()));
    };
    Ok(vec![Value::number(read_note(word, note)?.1)])
}

fn transpose(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [note, numerator, denominator] = args else {
        return Err(arity(word, 3, args.len()));
    };
    let (frequency, beats) = read_note(word, note)?;
    let interval = ratio(word, numerator, denominator)?;
    Ok(vec![Value::vector(vec![
        Value::number(&frequency * &interval),
        Value::number(beats),
    ])])
}

fn seconds(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [note, tempo] = args else {
        return Err(arity(word, 2, args.len()));
    };
    let (_, beats) = read_note(word, note)?;
    let tempo = positive(word, "tempo", number(word, tempo)?)?;
    let per_beat = Number::integer(60)
        .checked_div(&tempo)
        .ok_or(Error::DivisionByZero)?;
    Ok(vec![Value::number(&beats * &per_beat)])
}

fn arity(word: &str, needed: usize, found: usize) -> Error {
    Error::StackUnderflow {
        word: word.to_string(),
        needed,
        found,
    }
}
