//! Fused execution of a straight-line block over a Vector — the `MAP`,
//! `FILTER`, `FOLD` and `SCAN` inner loop without the dispatcher.
//!
//! A higher-order Word runs its block once per element, and each run pays for
//! the whole dispatch apparatus: a scratch stack of boxed `Value`s, a binding
//! frame, the declared NIL contract, the lift check, the meter, the NIL trace.
//! For a block like `[ 2 MUL 1 ADD ]` that apparatus is almost all of the time
//! — about 1,450 instructions per element for one integer addition.
//!
//! A block made only of the Words below can do none of the things that
//! apparatus exists for, so long as every operand it meets is a plain
//! rational or Boolean: it mints no NIL, lifts over no container, reads no
//! dictionary and binds only within its own frame.
//!
//! - literals, `TRUE`, `FALSE`, and names bound by `BIND` — in the block, or
//!   in a frame the block can see;
//! - `ADD` `SUB` `MUL` `DIV`, `LT` `GT` `EQ`, `FLOOR` `ROUND`, `NOT` `AND`
//!   `SELECT`, and `'NAME' BIND`.
//!
//! Such a block is lowered here and run against the elements directly, in one
//! of three tiers: `fused_block_int` when every value is a machine-word
//! integer or a Boolean, `fused_block_rat` when every number is a rational
//! whose halves each fit a machine word (types known before the walk starts
//! in both), and `fused_block_general` for anything else in the subset.
//!
//! The fused run is *speculative*. It touches no interpreter state while it
//! runs; it computes what the interpreted walk would have charged — the same
//! steps, the same numeric work, the same metrics, the same epochs — and
//! commits all of it only if the whole walk finishes without anything the
//! fused form cannot reproduce exactly. An integer overflow falls to the
//! general tier; a division by zero (a NIL projection), an operand of the
//! wrong domain (an ERROR), a result past the size ceiling or a budget the
//! walk would exhaust answers `None`, and the caller runs the ordinary walk
//! from the start. Because a block is pure (LANG.DICTIONARY.ACYCLIC, no
//! effects), running it twice — once abandoned — is unobservable; and because
//! the committed charges are the interpreted walk's own, which route ran is
//! unobservable too (LANG.AUTHORITY.FREEDOM). `fused_block_tests` holds the
//! routes equal.

use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::compiled_plan::CompiledOp;
use crate::interpreter::{CompiledPlan, Interpreter};
use crate::kernel::generated::WordId;
use crate::types::fraction::Fraction;
use crate::types::{Token, Value, ValueData};
use std::collections::HashMap;

/// A plain value: the only kind the fused form reproduces exactly. Each one
/// is rebuilt by the constructor that built it on the interpreted route
/// (`Value::from_fraction`, `Value::from_bool`), so nothing is lost by holding
/// it unboxed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Plain {
    Num(Fraction),
    Bool(bool),
}

impl Plain {
    pub(crate) fn of(value: &Value) -> Option<Self> {
        if value.absence.is_some() {
            return None;
        }
        match &value.data {
            ValueData::Scalar(f) if !f.is_nil() => Some(Plain::Num(f.clone())),
            ValueData::Boolean(b) => Some(Plain::Bool(*b)),
            _ => None,
        }
    }

    pub(crate) fn into_value(self) -> Value {
        match self {
            Plain::Num(f) => Value::from_fraction(f),
            Plain::Bool(b) => Value::from_bool(b),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Compare {
    Lt,
    Gt,
    Eq,
}

/// One op of a lowered block. Every op but `Push` and `Load` is one
/// dispatched Word, and so one execution step.
#[derive(Debug, Clone)]
pub(crate) enum Op {
    /// A literal, or the value of a name bound outside the block: free.
    Push(Plain),
    /// `TRUE`/`FALSE`: a Word, so a step (`CompiledOp::PushWordLiteral`).
    PushWord(bool),
    Arith(ExactArithmeticSchema),
    Compare(Compare),
    Floor,
    Round,
    Not,
    And,
    Select,
    /// `'NAME' BIND` into the block's own frame, slot-numbered.
    Bind(usize),
    /// A name the block bound earlier in the same run: free, as a binding
    /// read is (`execute_word_core` answers it before charging).
    Load(usize),
}

impl Op {
    fn is_word(&self) -> bool {
        !matches!(self, Op::Push(_) | Op::Load(_))
    }
}

/// Which higher-order Word is walking, and so what it answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FusedWalk {
    /// One result per element, the element alone on the block's stack.
    Map,
    /// The elements whose result is TRUE, the element alone on the stack.
    Filter,
    /// The accumulator and the element on the block's stack; the last
    /// accumulator is the answer.
    Fold,
    /// As `Fold`, answering every accumulator.
    Scan,
}

/// A block lowered to straight-line ops.
#[derive(Debug, Clone)]
pub(crate) struct FusedBlock {
    pub(crate) ops: Vec<Op>,
    /// How many names the block binds.
    pub(crate) slots: usize,
    /// Words dispatched per run.
    pub(crate) steps_per_run: u64,
}

/// What a fused walk would charge, committed only once it has finished.
pub(crate) struct Charges {
    pub(crate) runs: u64,
    pub(crate) steps: usize,
    pub(crate) work: u64,
    pub(crate) fastpath: u64,
}

fn word_op(id: WordId) -> Option<Op> {
    Some(match id {
        WordId::Add => Op::Arith(ExactArithmeticSchema::Add),
        WordId::Sub => Op::Arith(ExactArithmeticSchema::Sub),
        WordId::Mul => Op::Arith(ExactArithmeticSchema::Mul),
        WordId::Div => Op::Arith(ExactArithmeticSchema::Div),
        WordId::Lt => Op::Compare(Compare::Lt),
        WordId::Gt => Op::Compare(Compare::Gt),
        WordId::Eq => Op::Compare(Compare::Eq),
        WordId::Floor => Op::Floor,
        WordId::Round => Op::Round,
        WordId::Not => Op::Not,
        WordId::And => Op::And,
        WordId::Select => Op::Select,
        _ => return None,
    })
}

/// The single name a `BIND` name operand gives, when it is one the
/// interpreter would accept. Anything it would refuse, or a destructuring
/// list, is the ordinary walk's to run.
fn bindable_name(interp: &Interpreter, value: &Value) -> Option<String> {
    let names = crate::interpreter::bindings::binding_names(value).ok()?;
    let [name] = names.as_slice() else {
        return None;
    };
    interp.check_bindable_name(name).ok()?;
    Some(name.to_uppercase())
}

impl FusedBlock {
    /// Lower `plan` for a walk that starts the block on `inputs` values, or
    /// `None` when any op is outside the fused subset or the block would
    /// underflow (an ERROR the ordinary route reports).
    ///
    /// A name the plan could not resolve is a `FallbackToken`: it lowers to a
    /// `Load` when the block bound it earlier, and to a `Push` of its value
    /// when a frame the block can see holds it. That value is fixed for the
    /// whole walk, since a block binds only in the frame it opens per run.
    pub(crate) fn compile(
        plan: &CompiledPlan,
        interp: &Interpreter,
        inputs: usize,
    ) -> Option<Self> {
        let source = &plan.line.ops;
        let mut ops = Vec::with_capacity(source.len());
        let mut local: HashMap<String, usize> = HashMap::new();
        let mut depth = inputs;
        let mut i = 0;
        while i < source.len() {
            let (op, pops, pushes) = match &source[i] {
                CompiledOp::PushLiteral(value) => match Plain::of(value) {
                    Some(plain) => (Op::Push(plain), 0, 1),
                    // `'NAME' BIND`: the name is folded into the op.
                    None => {
                        let CompiledOp::CallBuiltin(call) = source.get(i + 1)? else {
                            return None;
                        };
                        if call.word?.id != WordId::Bind {
                            return None;
                        }
                        let name = bindable_name(interp, value)?;
                        let next = local.len();
                        let slot = *local.entry(name).or_insert(next);
                        i += 1;
                        (Op::Bind(slot), 1, 0)
                    }
                },
                CompiledOp::PushWordLiteral(value, _) => match Plain::of(value)? {
                    Plain::Bool(b) => (Op::PushWord(b), 0, 1),
                    Plain::Num(_) => return None,
                },
                CompiledOp::CallBuiltin(call) => {
                    let op = word_op(call.word?.id)?;
                    let (pops, pushes) = match op {
                        Op::Floor | Op::Round | Op::Not => (1, 1),
                        Op::Select => (3, 1),
                        _ => (2, 1),
                    };
                    (op, pops, pushes)
                }
                CompiledOp::FallbackToken(Token::Symbol(name)) => {
                    let name = crate::word_name::canonical_word_name(name);
                    match local.get(name.as_ref()) {
                        Some(slot) => (Op::Load(*slot), 0, 1),
                        None => (Op::Push(Plain::of(&interp.lookup_binding(&name)?)?), 0, 1),
                    }
                }
                _ => return None,
            };
            if depth < pops {
                return None;
            }
            depth = depth - pops + pushes;
            ops.push(op);
            i += 1;
        }
        if depth == 0 {
            return None;
        }
        let steps_per_run = ops.iter().filter(|op| op.is_word()).count() as u64;
        Some(Self {
            ops,
            slots: local.len(),
            steps_per_run,
        })
    }

    /// Run the walk over `target`, starting from `seed` for `Fold`/`Scan`.
    ///
    /// Answers the value the Word leaves on the stack, having committed the
    /// interpreted walk's charges, or `None` — with nothing touched — when the
    /// ordinary walk must run instead.
    pub(crate) fn run(
        &self,
        interp: &mut Interpreter,
        walk: FusedWalk,
        target: &Value,
        seed: Option<&Value>,
    ) -> Option<Value> {
        if !interp.scalar_fastpath_enabled || !interp.fused_block_enabled {
            return None;
        }
        let seed = match walk {
            FusedWalk::Map | FusedWalk::Filter => None,
            FusedWalk::Fold | FusedWalk::Scan => Some(Plain::of(seed?)?),
        };
        let (value, charges) =
            crate::interpreter::fused_block_int::run(self, interp, walk, target, seed.as_ref())
                .or_else(|| {
                    crate::interpreter::fused_block_rat::run(
                        self,
                        interp,
                        walk,
                        target,
                        seed.as_ref(),
                    )
                })
                .or_else(|| {
                    crate::interpreter::fused_block_general::run(
                        self,
                        interp,
                        walk,
                        target,
                        seed.as_ref(),
                    )
                })?;

        #[cfg(test)]
        FUSED_RUNS.with(|c| c.set(c.get() + 1));
        // Commit what the interpreted walk would have: a step per dispatched
        // Word, a fastpath hit per scalar-pair arithmetic or comparison, the
        // work, and one execution epoch per run.
        interp.execution_step_count += charges.steps;
        interp.numeric_work_used += charges.work;
        interp.runtime_metrics.scalar_fastpath_count = interp
            .runtime_metrics
            .scalar_fastpath_count
            .saturating_add(charges.fastpath);
        interp.global_epoch += charges.runs;
        interp.execution_epoch = interp.global_epoch;
        Some(value)
    }

    /// The steps `runs` runs take, if the step ceiling lets all of them run.
    /// A walk the ceiling would stop part way is the ordinary walk's to stop.
    pub(crate) fn steps_within_ceiling(&self, interp: &Interpreter, runs: u64) -> Option<usize> {
        let steps = usize::try_from(runs.checked_mul(self.steps_per_run)?).ok()?;
        (interp.execution_step_count.checked_add(steps)? <= interp.max_execution_steps)
            .then_some(steps)
    }
}

#[cfg(test)]
thread_local! {
    static FUSED_RUNS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Fused walks committed on this thread, for tests that pin which route ran.
#[cfg(test)]
pub(crate) fn fused_runs_on_this_thread() -> u64 {
    FUSED_RUNS.with(|c| c.get())
}
