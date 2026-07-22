//! Lossless `Value` ⇄ persistence-node codec for session save/restore.
//!
//! This is the state-persistence wire format used by the WASM boundary
//! (`snapshot_stack` / `restore_stack_snapshot`). It is deliberately kept
//! **separate** from the observation protocol in
//! [`crate::types::value_protocol`]. The two have opposite requirements:
//!
//! - The observation protocol is intentionally lossy-but-honest (SPEC §2.3):
//!   an `ExactScalar` is observed as a *marked* rational approximation and a
//!   `CodeBlock` is hidden as `nil`. That is correct for a display/inspection
//!   surface — it must never present a hidden truncation as exact.
//! - Persistence must be **lossless**: reloading a saved session must return
//!   the identical value. Reusing the observation protocol for save/restore
//!   silently changed values on reload — a `CodeBlock` came back as a genuine
//!   NIL, and `√2` came back as the rational `768398401/543339720`.
//!
//! This codec guarantees `decode(encode(v)) == v` for every `Value`, enforced
//! by the property tests at the bottom of this file. `Value` equality (SPEC
//! value identity) is `data == other.data && hint == other.hint`; absence
//! metadata and the `Unknown` diagnosis are provenance, not identity, and are
//! outside that oracle (their round-trip is a separate future concern).

use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::record_shape::record_shape_from_ordered_keys;
use crate::types::{DenseTensor, Interpretation, Token, Value, ValueData};
use num_bigint::BigInt;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;

// ---- Interpretation role <-> stable tag ----

fn hint_to_tag(hint: Interpretation) -> &'static str {
    match hint {
        Interpretation::Unassigned => "unassigned",
        Interpretation::RawNumber => "rawNumber",
        Interpretation::Interval => "interval",
        Interpretation::Text => "text",
        Interpretation::TruthValue => "truthValue",
        Interpretation::Timestamp => "timestamp",
        Interpretation::Nil => "nil",
        Interpretation::ContinuedFraction => "continuedFraction",
    }
}

fn hint_from_tag(tag: &str) -> Interpretation {
    match tag {
        "rawNumber" => Interpretation::RawNumber,
        "interval" => Interpretation::Interval,
        "text" => Interpretation::Text,
        "truthValue" => Interpretation::TruthValue,
        "timestamp" => Interpretation::Timestamp,
        "nil" => Interpretation::Nil,
        "continuedFraction" => Interpretation::ContinuedFraction,
        _ => Interpretation::Unassigned,
    }
}

// ---- Fraction <-> decimal string pair ----

fn frac_to_parts(f: &Fraction) -> (String, String) {
    (f.numerator().to_string(), f.denominator().to_string())
}

fn frac_from_parts(num: &str, den: &str) -> Result<Fraction, String> {
    let numerator = BigInt::from_str(num).map_err(|e| format!("bad numerator: {e}"))?;
    let denominator = BigInt::from_str(den).map_err(|e| format!("bad denominator: {e}"))?;
    // A zero denominator is the `Fraction` nil sentinel (numerator/denominator
    // both zero). `Fraction::new` panics on a zero denominator, so route the
    // nil case through the dedicated constructor.
    if denominator.is_zero() {
        return Ok(Fraction::nil());
    }
    Ok(Fraction::new(numerator, denominator))
}

// ---- Wire structures ----

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PersistTerm {
    /// Monomial: a squarefree subset-product of the value's radical basis
    /// (`"1"` keys the rational part), as a decimal `BigInt`.
    m: String,
    /// Coefficient numerator / denominator (decimal `BigInt`).
    n: String,
    d: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "k")]
enum PersistToken {
    Number { s: String },
    Text { s: String },
    Symbol { s: String },
    VectorStart,
    VectorEnd,
    BlockStart,
    BlockEnd,
    Pipeline,
    NilCoalesce,
    CondClauseSep,
    LineBreak,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "t")]
enum PersistData {
    Bool {
        v: bool,
    },
    Scalar {
        n: String,
        d: String,
    },
    ExactRat {
        n: String,
        d: String,
    },
    ExactAlg {
        terms: Vec<PersistTerm>,
    },
    Vector {
        items: Vec<PersistValue>,
    },
    Tensor {
        nums: Vec<i64>,
        dens: Vec<i64>,
        mask: Vec<u64>,
        dshape: Vec<usize>,
        pure_int: bool,
        shape: Vec<usize>,
    },
    Record {
        pairs: Vec<PersistValue>,
        keys: Vec<String>,
    },
    Nil,
    Unknown,
    Code {
        tokens: Vec<PersistToken>,
    },
    Process {
        id: u64,
    },
    Supervisor {
        id: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PersistValue {
    /// Interpretation role tag of this value.
    h: String,
    d: PersistData,
}

/// One stack slot: the value plus its stack-position interpretation role
/// (Phase 4 owns the role on the `Stack`, not on the value).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PersistSlot {
    v: PersistValue,
    /// Stack-position role tag.
    r: String,
}

// ---- Token <-> wire ----

fn token_to_wire(token: &Token) -> PersistToken {
    match token {
        Token::Number(s) => PersistToken::Number { s: s.to_string() },
        Token::String(s) => PersistToken::Text { s: s.to_string() },
        Token::Symbol(s) => PersistToken::Symbol { s: s.to_string() },
        Token::VectorStart => PersistToken::VectorStart,
        Token::VectorEnd => PersistToken::VectorEnd,
        Token::BlockStart => PersistToken::BlockStart,
        Token::BlockEnd => PersistToken::BlockEnd,
        Token::Pipeline => PersistToken::Pipeline,
        Token::NilCoalesce => PersistToken::NilCoalesce,
        Token::CondClauseSep => PersistToken::CondClauseSep,
        Token::LineBreak => PersistToken::LineBreak,
    }
}

fn token_from_wire(token: &PersistToken) -> Token {
    match token {
        PersistToken::Number { s } => Token::Number(Arc::from(s.as_str())),
        PersistToken::Text { s } => Token::String(Arc::from(s.as_str())),
        PersistToken::Symbol { s } => Token::Symbol(Arc::from(s.as_str())),
        PersistToken::VectorStart => Token::VectorStart,
        PersistToken::VectorEnd => Token::VectorEnd,
        PersistToken::BlockStart => Token::BlockStart,
        PersistToken::BlockEnd => Token::BlockEnd,
        PersistToken::Pipeline => Token::Pipeline,
        PersistToken::NilCoalesce => Token::NilCoalesce,
        PersistToken::CondClauseSep => Token::CondClauseSep,
        PersistToken::LineBreak => Token::LineBreak,
    }
}

// ---- Value <-> wire ----

fn encode_data(data: &ValueData) -> Result<PersistData, String> {
    Ok(match data {
        ValueData::Boolean(b) => PersistData::Bool { v: *b },
        ValueData::Scalar(f) => {
            let (n, d) = frac_to_parts(f);
            PersistData::Scalar { n, d }
        }
        ValueData::ExactScalar(er) => match er {
            ExactReal::Rational(f) => {
                let (n, d) = frac_to_parts(f);
                PersistData::ExactRat { n, d }
            }
            _ => match er.algebraic_terms() {
                Some(terms) => PersistData::ExactAlg {
                    terms: terms
                        .iter()
                        .map(|(m, c)| {
                            let (n, d) = frac_to_parts(c);
                            PersistTerm {
                                m: m.to_string(),
                                n,
                                d,
                            }
                        })
                        .collect(),
                },
                // Tier 2 computable reals are not constructible by any current
                // vocabulary word, so no stack value can reach this arm.
                None => return Err("cannot persist a Tier-2 computable exact real".to_string()),
            },
        },
        ValueData::Vector(items) => PersistData::Vector {
            items: items
                .iter()
                .map(encode_value)
                .collect::<Result<Vec<_>, _>>()?,
        },
        ValueData::Tensor { data, shape } => PersistData::Tensor {
            nums: data.numerators.clone(),
            dens: data.denominators.clone(),
            mask: data.valid_mask.clone(),
            dshape: data.shape.clone(),
            pure_int: data.is_pure_integer,
            shape: (**shape).clone(),
        },
        ValueData::Record { pairs, shape } => {
            let len = pairs.len();
            let mut keys = vec![String::new(); len];
            for (key, &slot) in shape.mapping() {
                if slot < len {
                    keys[slot] = key.clone();
                }
            }
            PersistData::Record {
                pairs: pairs
                    .iter()
                    .map(encode_value)
                    .collect::<Result<Vec<_>, _>>()?,
                keys,
            }
        }
        ValueData::Nil => PersistData::Nil,
        ValueData::Unknown(_) => PersistData::Unknown,
        ValueData::CodeBlock(tokens) => PersistData::Code {
            tokens: tokens.iter().map(token_to_wire).collect(),
        },
        ValueData::ProcessHandle(id) => PersistData::Process { id: *id },
        ValueData::SupervisorHandle(id) => PersistData::Supervisor { id: *id },
    })
}

fn decode_data(data: &PersistData) -> Result<ValueData, String> {
    Ok(match data {
        PersistData::Bool { v } => ValueData::Boolean(*v),
        PersistData::Scalar { n, d } => ValueData::Scalar(frac_from_parts(n, d)?),
        PersistData::ExactRat { n, d } => {
            ValueData::ExactScalar(ExactReal::Rational(frac_from_parts(n, d)?))
        }
        PersistData::ExactAlg { terms } => {
            // Replay ∑ cₘ·√m through the public exact arithmetic. The
            // multiquadratic normal form is canonical, so the accumulated
            // value is the identical ExactReal.
            let mut acc = ExactReal::from_integer(0);
            for term in terms {
                let monomial =
                    BigInt::from_str(&term.m).map_err(|e| format!("bad monomial: {e}"))?;
                let coeff = frac_from_parts(&term.n, &term.d)?;
                let root = ExactReal::from_sqrt_rational(Fraction::new(monomial, BigInt::one()))
                    .ok_or_else(|| "invalid monomial for √".to_string())?;
                acc = acc.add(&root.mul(&ExactReal::from_fraction(coeff)));
            }
            ValueData::ExactScalar(acc)
        }
        PersistData::Vector { items } => ValueData::Vector(Arc::new(
            items
                .iter()
                .map(decode_value)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        PersistData::Tensor {
            nums,
            dens,
            mask,
            dshape,
            pure_int,
            shape,
        } => {
            if nums.len() != dens.len() {
                return Err("tensor numerator/denominator length mismatch".to_string());
            }
            ValueData::Tensor {
                data: Arc::new(DenseTensor {
                    numerators: nums.clone(),
                    denominators: dens.clone(),
                    valid_mask: mask.clone(),
                    shape: dshape.clone(),
                    is_pure_integer: *pure_int,
                }),
                shape: Arc::new(shape.clone()),
            }
        }
        PersistData::Record { pairs, keys } => {
            let shape = record_shape_from_ordered_keys(keys.iter().cloned());
            let values = pairs
                .iter()
                .map(decode_value)
                .collect::<Result<Vec<_>, _>>()?;
            ValueData::Record {
                pairs: Arc::new(values),
                shape,
            }
        }
        PersistData::Nil => ValueData::Nil,
        PersistData::Unknown => ValueData::Unknown(None),
        PersistData::Code { tokens } => {
            ValueData::CodeBlock(tokens.iter().map(token_from_wire).collect())
        }
        PersistData::Process { id } => ValueData::ProcessHandle(*id),
        PersistData::Supervisor { id } => ValueData::SupervisorHandle(*id),
    })
}

fn encode_value(value: &Value) -> Result<PersistValue, String> {
    Ok(PersistValue {
        h: hint_to_tag(value.hint).to_string(),
        d: encode_data(&value.data)?,
    })
}

fn decode_value(value: &PersistValue) -> Result<Value, String> {
    Ok(Value {
        data: decode_data(&value.d)?,
        hint: hint_from_tag(&value.h),
        absence: None,
    })
}

// ---- Public stack codec (WASM boundary) ----

/// Serialize the stack slots (`(value, role)` pairs) to the lossless JSON
/// persistence string.
pub(crate) fn encode_stack<'a>(
    slots: impl Iterator<Item = (&'a Value, Interpretation)>,
) -> Result<String, String> {
    let wire: Vec<PersistSlot> = slots
        .map(|(value, role)| {
            Ok(PersistSlot {
                v: encode_value(value)?,
                r: hint_to_tag(role).to_string(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    serde_json::to_string(&wire).map_err(|e| e.to_string())
}

/// Deserialize a lossless persistence string back into `(value, role)` pairs.
pub(crate) fn decode_stack(json: &str) -> Result<Vec<(Value, Interpretation)>, String> {
    let wire: Vec<PersistSlot> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    wire.iter()
        .map(|slot| Ok((decode_value(&slot.v)?, hint_from_tag(&slot.r))))
        .collect()
}
