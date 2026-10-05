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
//! - `ADD` `SUB` `MUL` `DIV`, `LT` `GT` `EQ`, `MIN` `MAX`, `FLOOR` `ROUND`,
//!   `NOT` `AND` `SELECT`, and `'NAME' BIND`;
//! - User Words whose bodies are made of these (`fused_block_lower`).
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
use crate::interpreter::{CompiledPlan, Interpreter};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

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
    /// `MIN` (`max: false`) or `MAX`: the operand the order picks, the left
    /// on a tie. A step, and no fast-path hit or work, as the interpreted
    /// route charges it.
    Extremum {
        max: bool,
    },
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
    pub(crate) fn is_word(&self) -> bool {
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
    /// Words dispatched per run, User Word calls included.
    pub(crate) steps_per_run: u64,
    /// User Word calls per run, each one a compiled-plan lookup.
    pub(crate) calls_per_run: u64,
    /// The plans of called User Words that had none current: the interpreted
    /// walk builds each on its first call, so a committed fused walk stores
    /// them and counts those builds.
    pub(crate) builds: Vec<(String, std::sync::Arc<CompiledPlan>)>,
    /// Whether a name bound outside the block was read into it as a
    /// constant (`Op::Push`), which ties the lowering to that binding.
    pub(crate) reads_outer: bool,
    /// The integer tier's compiled forms of the block.
    pub(crate) int_programs: crate::interpreter::fused_block_int::IntPrograms,
}

/// What a fused walk would charge, committed only once it has finished.
pub(crate) struct Charges {
    pub(crate) runs: u64,
    pub(crate) steps: usize,
    pub(crate) work: u64,
    pub(crate) fastpath: u64,
}

impl FusedBlock {
    /// Lower `plan` for a walk that starts the block on `inputs` values, or
    /// `None` when any op is outside the fused subset or the block would
    /// underflow (an ERROR the ordinary route reports). See
    /// `fused_block_lower`, which also inlines the User Words it calls.
    pub(crate) fn compile(
        plan: &CompiledPlan,
        interp: &Interpreter,
        inputs: usize,
    ) -> Option<Self> {
        crate::interpreter::fused_block_lower::lower(plan, interp, inputs)
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
        // A one-lane seed (`[ 0 ]`) is walked as its lane; `lane_mixed` is
        // what that changes in the charges (`one_lane_seed`).
        let mut lane_mixed: Option<u64> = None;
        let seed = match walk {
            FusedWalk::Map | FusedWalk::Filter => None,
            FusedWalk::Fold | FusedWalk::Scan => {
                let seed = seed?;
                match Plain::of(seed) {
                    Some(plain) => Some(plain),
                    None => {
                        let lane = one_lane_seed(interp, seed)?;
                        lane_mixed = Some(self.lane_mixed_per_run()?);
                        Some(lane)
                    }
                }
            }
        };
        let (mut value, mut charges) =
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
        if let Some(mixed) = lane_mixed {
            charges.fastpath = charges
                .fastpath
                .checked_sub(mixed.checked_mul(charges.runs)?)?;
            value = rewrap_one_lane(walk, &value)?;
        }

        let builds = self.builds.len() as u64;
        let hits = self
            .calls_per_run
            .checked_mul(charges.runs)?
            .checked_sub(builds)?;

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
        // Each User Word call looks its plan up: the first call of a Word
        // with none current builds one (a miss, a build and an epoch), every
        // other call is a hit.
        for (name, plan) in &self.builds {
            interp.store_compiled_plan_for_word(name, plan.clone());
        }
        let metrics = &mut interp.runtime_metrics;
        metrics.compiled_plan_cache_miss_count += builds;
        metrics.compiled_plan_build_count += builds;
        metrics.compiled_plan_cache_hit_count += hits;
        interp.global_epoch += charges.runs + builds;
        interp.execution_epoch = interp.global_epoch;
        Some(value)
    }

    /// For a walk whose accumulator is a one-lane Tensor: how many of each
    /// run's arithmetic Words pair that lane with a plain scalar, or `None`
    /// when the lane reaches anything but `ADD`/`SUB`/`MUL`/`DIV`, `BIND` and
    /// a bound name, or the run's result is not the lane.
    ///
    /// Those four Words lift over the lane and answer a one-lane Tensor, so
    /// the walk computes on the lane alone. What differs is the route each
    /// pairing takes: two scalars, or two one-lane Tensors, take the scalar
    /// fast path and count a hit; a lane beside a scalar takes the column
    /// kernel (`dense_kernels`) and counts none. The work is the same, since
    /// a one-lane Tensor and a machine-word scalar both measure one lane of
    /// one limb (`measure_operand`). Every other Word treats a one-lane
    /// Tensor differently from its lane — `EQ` compares whole values, `LT`
    /// answers a Vector of Booleans — so the walk declines it.
    fn lane_mixed_per_run(&self) -> Option<u64> {
        // The accumulator, then the element.
        let mut stack = vec![true, false];
        let mut slots = vec![false; self.slots];
        let mut mixed = 0;
        for op in &self.ops {
            match op {
                Op::Push(_) | Op::PushWord(_) => stack.push(false),
                Op::Load(slot) => stack.push(slots[*slot]),
                Op::Bind(slot) => slots[*slot] = stack.pop()?,
                Op::Arith(_) => {
                    let (b, a) = (stack.pop()?, stack.pop()?);
                    mixed += u64::from(a != b);
                    stack.push(a || b);
                }
                Op::Compare(_) | Op::And | Op::Extremum { .. } => {
                    let (b, a) = (stack.pop()?, stack.pop()?);
                    if a || b {
                        return None;
                    }
                    stack.push(false);
                }
                Op::Floor | Op::Round | Op::Not => {
                    if stack.pop()? {
                        return None;
                    }
                    stack.push(false);
                }
                Op::Select => {
                    let (m, f, t) = (stack.pop()?, stack.pop()?, stack.pop()?);
                    if m || f || t {
                        return None;
                    }
                    stack.push(false);
                }
            }
        }
        stack.last().copied()?.then_some(mixed)
    }

    /// The steps `runs` runs take, if the step ceiling lets all of them run.
    /// A walk the ceiling would stop part way is the ordinary walk's to stop.
    pub(crate) fn steps_within_ceiling(&self, interp: &Interpreter, runs: u64) -> Option<usize> {
        let steps = usize::try_from(runs.checked_mul(self.steps_per_run)?).ok()?;
        (interp.execution_step_count.checked_add(steps)? <= interp.max_execution_steps)
            .then_some(steps)
    }
}

/// The lane of a one-lane seed — a dense Tensor of shape `[1]` holding a
/// machine-word rational, which is what `[ 0 ]` is — when the column kernels
/// that answer the interpreted walk's lane arithmetic are on.
fn one_lane_seed(interp: &Interpreter, seed: &Value) -> Option<Plain> {
    if !interp.dense_kernels_enabled || seed.absence.is_some() {
        return None;
    }
    let ValueData::Tensor { data, shape } = &seed.data else {
        return None;
    };
    if shape.as_slice() != [1] || data.len() != 1 || data.absences().next().is_some() {
        return None;
    }
    let lane = data.get_small_fraction(0)?;
    (!lane.is_nil()).then_some(Plain::Num(lane))
}

/// The one-lane value the interpreted walk holds for `lane`: a one-lane
/// Tensor while both halves fit a machine word, as the column kernel and the
/// scalar fast path build it (`DenseTensor::from_fractions`) — the lane's pair
/// as its two columns, pure when its denominator is 1, no absences — and, once
/// either half outgrows one, the lift's Vector of that one scalar. The form is
/// a function of the value alone: a lane that grows past a word and shrinks
/// back is a Tensor again.
fn one_lane(lane: Fraction) -> Option<Value> {
    let Some((n, d)) = lane.extract_i64_pair() else {
        return Some(Value::from_vector(vec![Value::from_fraction(lane)]));
    };
    let data = crate::types::DenseTensor::from_columns([n], [d], [1], d == 1, Default::default());
    Some(Value::new(
        ValueData::Tensor {
            data: std::sync::Arc::new(data),
            shape: std::sync::Arc::new(vec![1]),
        },
        None,
    ))
}

/// The answer of a lane walk, as the interpreted walk gives it: `FOLD`'s
/// last accumulator a one-lane Tensor, `SCAN`'s every accumulator one, in a
/// Vector built as `higher_order_fold` builds it.
fn rewrap_one_lane(walk: FusedWalk, value: &Value) -> Option<Value> {
    let lane = |v: &Value| match Plain::of(v)? {
        Plain::Num(f) => one_lane(f),
        Plain::Bool(_) => None,
    };
    match walk {
        FusedWalk::Fold => lane(value),
        FusedWalk::Scan => {
            let lanes = (0..value.len())
                .map(|i| lane(&value.child(i)?))
                .collect::<Option<Vec<_>>>()?;
            Some(Value::from_vector(lanes))
        }
        FusedWalk::Map | FusedWalk::Filter => None,
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
