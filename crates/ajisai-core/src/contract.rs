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
use crate::role::Role;
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

/// Whether, and how, `STAK` reads the word across a whole flow.
///
/// This is declared per word, and it is not derived from the word's arity.
/// Deriving it was a mistake of exactly the kind this language removed when it
/// deleted Flow Mass Conservation: a count of operands is not a meaning, and
/// "two in, one out" does not entail that folding the word across a flow says
/// anything. `1 1 1 STAK EQ` under the derived rule computed
/// `EQ(EQ(1, 1), 1)` — `EQ(TRUE, 1)` — and answered `FALSE` about three equal
/// values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StakSupport {
    /// The word is applied to every cell of the flow, in order, and the
    /// results are concatenated. Requires one input.
    MapEach,
    /// The word is folded left across the whole flow. Requires a **closed**
    /// operation: two inputs, one output, and an output type identical to the
    /// first input type, so that each result is a legitimate operand for the
    /// next step. `words::mod` holds this as a test rather than a convention.
    FoldLeft,
    /// The word has no defensible reading across a whole flow. Refusing is
    /// better than inventing one.
    Unsupported,
}

/// A word's machine-readable contract.
#[derive(Clone, Debug)]
pub struct WordContract {
    pub name: &'static str,
    /// The canonical stack-effect notation, e.g. `( a b -- c )`.
    pub stack_effect: &'static str,
    /// The true stack effect — how many values the word draws and leaves.
    /// This says nothing about how the word is dispatched; see [`Body`].
    pub arity: Arity,
    pub input_types: &'static [TypeSpec],
    pub output_types: &'static [TypeSpec],
    pub nil_policy: Policy,
    pub unknown_policy: Policy,
    pub stak: StakSupport,
    /// The role an input must carry, where the word reads the Semantic Plane.
    /// `None` for every word that does not — which is all of them but the
    /// dictionary words. See `SPECIFICATION.md` §6.3.
    pub role_required: Option<(usize, Role)>,
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

/// How a word is dispatched.
///
/// This is an implementation fact, not a stack effect. `MAP` has a perfectly
/// definite effect of `( vector quote -- vector )` and is dispatched as
/// [`Body::Full`] only because it needs the interpreter to run a quote; the two
/// were conflated in an earlier draft, which made the lint go blind at every
/// higher-order word for no reason.
#[derive(Clone, Copy)]
pub enum Body {
    /// Pure operand-to-result. `STAK` is available to these words, because the
    /// common layer can call them repeatedly.
    Op(OpFn),
    /// Needs the interpreter — to run a quote, to read the flow's depth, or to
    /// reach the dictionary. `KEEP` still applies when the arity is fixed;
    /// `STAK` does not, because the common layer cannot re-drive the word.
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
