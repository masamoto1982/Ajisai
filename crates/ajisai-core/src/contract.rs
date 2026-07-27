//! Word contracts.
//!
//! Every word in Ajisai Core carries a machine-readable contract, and there is
//! exactly one registry holding them. The contract is small on purpose: it
//! records what the contract lint and the documentation actually read, and
//! nothing else. There is no confidence lattice, no resource linearity, no
//! complexity class, no backend suitability, no content-addressed identity.
//!
//! A contract is not a proof. `docs/contracts.md` states the limit plainly:
//! the lint reports obvious inconsistencies between declared stack effects and
//! types. It never claims a program will succeed.

use std::fmt;

use crate::error::Result;
use crate::interpreter::Interpreter;
use crate::value::Value;

/// How many values a word draws and leaves.
///
/// `Fixed` is the honest name for what used to be dressed up as conserved
/// mass: a count in and a count out. `Dynamic` means the count depends on
/// runtime values, and the lint says so instead of guessing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arity {
    Fixed { inn: u8, out: u8 },
    Dynamic,
}

impl Arity {
    pub fn fixed(&self) -> Option<(usize, usize)> {
        match *self {
            Arity::Fixed { inn, out } => Some((inn as usize, out as usize)),
            Arity::Dynamic => None,
        }
    }
}

/// The kind of value a position accepts or yields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeSpec {
    /// Any value at all.
    Any,
    Number,
    Boolean,
    /// `TRUE`, `FALSE`, or `UNKNOWN`.
    TruthValue,
    Vector,
    Quote,
    /// A vector carrying the `TEXT` role.
    Text,
}

impl fmt::Display for TypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TypeSpec::Any => "any",
            TypeSpec::Number => "number",
            TypeSpec::Boolean => "boolean",
            TypeSpec::TruthValue => "truth value",
            TypeSpec::Vector => "vector",
            TypeSpec::Quote => "quote",
            TypeSpec::Text => "text",
        })
    }
}

/// How a word stands towards `NIL`, or towards `UNKNOWN`.
///
/// Two terms, because two terms are what the contract lint reads: whether the
/// value is refused on the way in, and whether the word can put it into the
/// flow. An earlier draft of this type distinguished "propagates" from
/// "accepts", which read well and changed nothing — no caller could act on the
/// difference — so it is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy {
    /// The word raises an error when this value reaches an input position.
    pub rejects: bool,
    /// The word can put this value into the flow.
    pub may_produce: bool,
}

impl Policy {
    /// Taken as an ordinary operand if it arrives, and never created. Covers
    /// both a word with no inputs and an observation predicate that settles
    /// the question rather than propagating.
    pub const INERT: Policy = Policy {
        rejects: false,
        may_produce: false,
    };
    /// Passes through, or is created here. Arithmetic carries `NIL`; a
    /// comparison creates `UNKNOWN`.
    pub const CARRIES: Policy = Policy {
        rejects: false,
        may_produce: true,
    };
    /// Refused on the way in.
    pub const REFUSES: Policy = Policy {
        rejects: true,
        may_produce: false,
    };
}

/// Whether running the word can change anything outside the flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Reads and writes only the flow. A `VENT` that blocks a pure unit is
    /// observationally identical to never having written it.
    Pure,
    /// Changes the dictionary. `VENT` blocking such a unit is still safe —
    /// the unit is never run — but the lint reports it, because a blocked
    /// definition is usually a mistake.
    Dictionary,
}

/// A word's machine-readable contract.
#[derive(Clone, Debug)]
pub struct WordContract {
    pub name: &'static str,
    /// The canonical stack-effect notation, e.g. `( a b -- c )`.
    pub stack_effect: &'static str,
    pub arity: Arity,
    pub input_types: &'static [TypeSpec],
    pub output_types: &'static [TypeSpec],
    pub nil_policy: Policy,
    pub unknown_policy: Policy,
    pub effect: Effect,
    pub summary: &'static str,
}

/// A pure operand-to-result function.
///
/// Words written this way are the reason `TOP`/`STAK` and `EAT`/`KEEP` are
/// implemented once. The operand layer in [`Interpreter`] selects the
/// operands, calls this, and commits the results; the word itself never sees a
/// mode and never repeats the four-way branch.
pub type OpFn = fn(&str, &[Value]) -> Result<Vec<Value>>;

/// A word that needs the interpreter itself.
pub type FullFn = fn(&mut Interpreter) -> Result<()>;

/// How a word runs.
#[derive(Clone, Copy)]
pub enum Body {
    /// Pure operand-to-result. Modes apply through the common operand layer.
    Op(OpFn),
    /// Needs the interpreter — dynamic arity, the dictionary, or a quote.
    /// These words reject any armed mode, because there is no operand region
    /// for the common layer to select.
    Full(FullFn),
    /// Handled directly by the evaluator: the mode words and `VENT`. They are
    /// listed in the registry so that documentation, completion, and the lint
    /// see one vocabulary, but they are not dispatched like ordinary words.
    Directive,
}

/// A registered word: its contract and its implementation.
#[derive(Clone)]
pub struct Word {
    pub contract: WordContract,
    pub body: Body,
    /// The package that owns the word. Ajisai Core owns `"ajisai-core"`.
    pub package: &'static str,
}

impl Word {
    pub fn name(&self) -> &'static str {
        self.contract.name
    }
}

/// Parse a stack-effect notation into its operand counts.
///
/// The notation and [`Arity`] are two views of the same fact, so the registry
/// test checks them against each other rather than trusting both.
pub fn notation_arity(notation: &str) -> Option<(usize, usize)> {
    let inner = notation.trim().strip_prefix('(')?.strip_suffix(')')?;
    let (before, after) = inner.split_once("--")?;
    Some((
        before.split_whitespace().count(),
        after.split_whitespace().count(),
    ))
}
