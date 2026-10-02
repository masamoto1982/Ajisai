//! Fused execution of a straight-line arithmetic block over a Vector of
//! rationals — the `MAP`, `FOLD` and `SCAN` inner loop without the dispatcher.
//!
//! A higher-order Word runs its block once per element, and each run pays for
//! the whole dispatch apparatus: a scratch stack of boxed `Value`s, a binding
//! frame, the declared NIL contract, the lift check, the meter, the NIL trace.
//! For a block like `[ 2 MUL 1 ADD ]` that apparatus is almost all of the time
//! — about 1,450 instructions per element for one integer addition.
//!
//! A block made only of rational literals and `ADD`/`SUB`/`MUL`/`DIV` can do
//! none of the things that apparatus exists for: it binds nothing, reads no
//! dictionary, mints no NIL and lifts over no container, so long as every
//! operand it meets is a plain rational. Such a block is lowered here to a
//! small program and run against the elements directly, in one of two tiers:
//!
//! - **integer**: every element, the seed and every literal an integer that
//!   fits a machine word, and no `DIV`. The walk runs on `i64` with checked
//!   arithmetic, reading a dense Tensor's numerator column in place.
//! - **rational**: anything else in the subset, on `Fraction`s.
//!
//! The fused run is *speculative*. It touches no interpreter state while it
//! runs; it computes what the interpreted walk would have charged — the same
//! steps, the same numeric work, the same metrics, the same epochs — and
//! commits all of it only if the whole walk finishes without anything the
//! fused form cannot reproduce exactly. An integer overflow falls to the
//! rational tier; a division by zero (a NIL projection), a result past the
//! size ceiling or a budget the walk would exhaust answers `None`, and the
//! caller runs the ordinary walk from the start. Because a block is pure
//! (LANG.DICTIONARY.ACYCLIC, no effects), running it twice — once abandoned —
//! is unobservable; and because the committed charges are the interpreted
//! walk's own, which route ran is unobservable too (LANG.AUTHORITY.FREEDOM).
//! `fused_block_tests` holds the routes equal.

use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::compiled_plan::CompiledOp;
use crate::interpreter::runtime_limits::{
    binary_numeric_work, fraction_result_bits, fraction_work_bits,
};
use crate::interpreter::{CompiledPlan, Interpreter};
use crate::kernel::generated::WordId;
use crate::types::fraction::{Fraction, FractionRepr};
use crate::types::DenseTensor;
use crate::types::{Value, ValueData};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
enum FusedOp {
    Push(Fraction),
    Apply(ExactArithmeticSchema),
}

/// The integer tier's form of the same op.
#[derive(Debug, Clone, Copy)]
enum IntOp {
    Push(i64),
    Add,
    Sub,
    Mul,
}

/// The deepest stack the integer tier keeps in registers; a block that needs
/// more runs on the rational tier.
const INT_STACK: usize = 16;

/// A block lowered to straight-line rational arithmetic.
#[derive(Debug, Clone)]
pub(crate) struct FusedBlock {
    ops: Vec<FusedOp>,
    /// The same ops for the integer tier, when every literal is an integer,
    /// there is no `DIV`, and the stack stays within [`INT_STACK`].
    int_ops: Option<Vec<IntOp>>,
    /// `ADD`/`SUB`/`MUL`/`DIV` per run: each is one dispatched Word, so one
    /// execution step and one scalar-fastpath hit on the interpreted route.
    applies: u64,
}

/// Which higher-order Word is walking, and so what it answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FusedWalk {
    /// One result per element, the element alone on the block's stack.
    Map,
    /// The accumulator and the element on the block's stack; the last
    /// accumulator is the answer.
    Fold,
    /// As `Fold`, answering every accumulator.
    Scan,
}

/// A plain rational: the only operand the fused form reproduces exactly. A
/// NIL lane, an irrational, a Boolean or a container goes the ordinary way.
fn plain_rational(value: &Value) -> Option<&Fraction> {
    match &value.data {
        ValueData::Scalar(f) if value.absence.is_none() && !f.is_nil() => Some(f),
        _ => None,
    }
}

fn small_integer(f: &Fraction) -> Option<i64> {
    match f.repr {
        FractionRepr::Small(n, 1) => Some(n),
        _ => None,
    }
}

fn integer(n: i64) -> Fraction {
    Fraction::from_repr(FractionRepr::Small(n, 1))
}

/// Every element of `target` as a plain rational, or `None` if any is not.
///
/// A one-dimensional dense Tensor — what `RANGE` and every arithmetic Word
/// over a Vector build — is read straight from its columns; `Value::child`
/// would box each lane as a `Value` only for this to unbox it again.
fn rational_elements(target: &Value) -> Option<Vec<Fraction>> {
    if let ValueData::Tensor { data, shape } = &target.data {
        if shape.len() == 1 {
            return (0..data.len())
                .map(|i| data.get_small_fraction(i))
                .collect();
        }
    }
    (0..target.len())
        .map(|i| target.child(i).and_then(|v| plain_rational(&v).cloned()))
        .collect()
}

/// Every element of `target` as a machine-word integer, borrowed in place
/// from a pure-integer dense Tensor's numerator column, or `None`.
///
/// `is_pure_integer` is false for a Tensor with an absent lane (its
/// denominator is 0), so a column it vouches for holds no NIL.
fn integer_elements(target: &Value) -> Option<Cow<'_, [i64]>> {
    match &target.data {
        ValueData::Tensor { data, shape } if shape.len() == 1 && data.is_pure_integer => {
            Some(Cow::Borrowed(&data.numerators))
        }
        ValueData::Vector(items) => items
            .iter()
            .map(|v| plain_rational(v).and_then(small_integer))
            .collect::<Option<Vec<_>>>()
            .map(Cow::Owned),
        _ => None,
    }
}

/// The dense Tensor `Value::from_vector_promoted` builds for a run of plain
/// scalars that each fit a machine word, built from the rationals directly.
/// Anything else answers the boxed values, for `from_vector_promoted` to
/// decide.
fn promote_rationals(results: Vec<Fraction>) -> std::result::Result<Value, Vec<Value>> {
    if !results.is_empty() && results.iter().all(|f| f.extract_i64_pair().is_some()) {
        let shape = vec![results.len()];
        if let Some(tensor) = DenseTensor::from_fractions(results, shape.clone()) {
            return Ok(dense_value(tensor, shape));
        }
        unreachable!("every lane fits a machine word");
    }
    Err(results.into_iter().map(Value::from_fraction).collect())
}

fn dense_value(tensor: DenseTensor, shape: Vec<usize>) -> Value {
    Value::new(
        ValueData::Tensor {
            data: Arc::new(tensor),
            shape: Arc::new(shape),
        },
        None,
    )
}

/// The same promotion for integer results: `from_fractions` of `n/1` lanes
/// is exactly these columns.
fn promote_integers(results: Vec<i64>) -> Value {
    let shape = vec![results.len()];
    let ones = vec![1; results.len()];
    dense_value(
        DenseTensor::from_columns(results, ones, shape.clone(), true, BTreeMap::new()),
        shape,
    )
}

/// What a fused walk would charge, to be committed only once it has finished.
struct Charges {
    runs: u64,
    steps: usize,
    work: u64,
}

impl FusedBlock {
    /// Lower `plan` for a walk that starts the block on `inputs` values, or
    /// `None` when any op is outside the straight-line rational subset.
    ///
    /// The last op must be an `Apply`: its result is built by
    /// `Value::from_fraction` on both routes, whereas a trailing literal would
    /// be that literal's own `Value`, which the fused form does not keep.
    pub(crate) fn compile(plan: &CompiledPlan, inputs: usize) -> Option<Self> {
        let mut ops = Vec::with_capacity(plan.line.ops.len());
        let mut int_ops = Some(Vec::with_capacity(plan.line.ops.len()));
        let mut depth = inputs;
        let mut applies = 0u64;
        for op in &plan.line.ops {
            match op {
                CompiledOp::PushLiteral(value) => {
                    let f = plain_rational(value)?;
                    depth += 1;
                    if depth > INT_STACK {
                        int_ops = None;
                    }
                    match (small_integer(f), int_ops.as_mut()) {
                        (Some(n), Some(int)) => int.push(IntOp::Push(n)),
                        _ => int_ops = None,
                    }
                    ops.push(FusedOp::Push(f.clone()));
                }
                CompiledOp::CallBuiltin(call) => {
                    let (schema, int_op) = match call.word?.id {
                        WordId::Add => (ExactArithmeticSchema::Add, Some(IntOp::Add)),
                        WordId::Sub => (ExactArithmeticSchema::Sub, Some(IntOp::Sub)),
                        WordId::Mul => (ExactArithmeticSchema::Mul, Some(IntOp::Mul)),
                        WordId::Div => (ExactArithmeticSchema::Div, None),
                        _ => return None,
                    };
                    // An underflow is an ERROR the ordinary route reports.
                    if depth < 2 {
                        return None;
                    }
                    depth -= 1;
                    applies += 1;
                    match (int_op, int_ops.as_mut()) {
                        (Some(op), Some(int)) => int.push(op),
                        _ => int_ops = None,
                    }
                    ops.push(FusedOp::Apply(schema));
                }
                _ => return None,
            }
        }
        if !matches!(ops.last(), Some(FusedOp::Apply(_))) {
            return None;
        }
        Some(Self {
            ops,
            int_ops,
            applies,
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
            FusedWalk::Map => None,
            FusedWalk::Fold | FusedWalk::Scan => Some(plain_rational(seed?)?),
        };
        let (value, charges) = self
            .run_integer(interp, walk, target, seed)
            .or_else(|| self.run_rational(interp, walk, target, seed))?;

        #[cfg(test)]
        FUSED_RUNS.with(|c| c.set(c.get() + 1));
        // Commit what the interpreted walk would have: a step and a fastpath
        // hit per applied Word, the work, and one execution epoch per run.
        interp.execution_step_count += charges.steps;
        interp.numeric_work_used += charges.work;
        interp.runtime_metrics.scalar_fastpath_count = interp
            .runtime_metrics
            .scalar_fastpath_count
            .saturating_add(charges.runs * self.applies);
        interp.global_epoch += charges.runs;
        interp.execution_epoch = interp.global_epoch;
        Some(value)
    }

    /// The steps `runs` runs take, if the step ceiling lets all of them run.
    /// A walk the ceiling would stop part way is the ordinary walk's to stop.
    fn steps_within_ceiling(&self, interp: &Interpreter, runs: u64) -> Option<usize> {
        let steps = usize::try_from(runs.checked_mul(self.applies)?).ok()?;
        (interp.execution_step_count.checked_add(steps)? <= interp.max_execution_steps)
            .then_some(steps)
    }

    /// The integer tier, or `None` when the walk is not all machine-word
    /// integers or overflows one.
    ///
    /// Every operand here is a `Small` fraction, which the meter prices at one
    /// limb, so each applied Word costs `binary_numeric_work(1, 1)` and the
    /// whole walk's work is known before it starts. Every result fits 64 bits,
    /// so the size ceiling can refuse one only when it is set below that.
    fn run_integer(
        &self,
        interp: &Interpreter,
        walk: FusedWalk,
        target: &Value,
        seed: Option<&Fraction>,
    ) -> Option<(Value, Charges)> {
        let ops = self.int_ops.as_deref()?;
        if interp.runtime_limits.max_bigint_bits < 64 {
            return None;
        }
        let seed = match seed {
            Some(f) => Some(small_integer(f)?),
            None => None,
        };
        let elements = integer_elements(target)?;
        let runs = elements.len() as u64;
        let steps = self.steps_within_ceiling(interp, runs)?;
        let work = (steps as u64).checked_mul(binary_numeric_work(1, 1))?;
        if interp.numeric_work_used.checked_add(work)? > interp.runtime_limits.max_numeric_work {
            return None;
        }

        let block = |stack: &mut [i64; INT_STACK], mut depth: usize| -> Option<i64> {
            for op in ops {
                match *op {
                    IntOp::Push(n) => {
                        stack[depth] = n;
                        depth += 1;
                    }
                    IntOp::Add | IntOp::Sub | IntOp::Mul => {
                        let (a, b) = (stack[depth - 2], stack[depth - 1]);
                        stack[depth - 2] = match *op {
                            IntOp::Add => a.checked_add(b),
                            IntOp::Sub => a.checked_sub(b),
                            _ => a.checked_mul(b),
                        }?;
                        depth -= 1;
                    }
                }
            }
            Some(stack[depth - 1])
        };

        let mut stack = [0i64; INT_STACK];
        let value = match (walk, seed) {
            (FusedWalk::Map, _) => {
                let mut results = Vec::with_capacity(elements.len());
                for &x in elements.iter() {
                    stack[0] = x;
                    results.push(block(&mut stack, 1)?);
                }
                promote_integers(results)
            }
            (FusedWalk::Fold, Some(mut acc)) => {
                for &x in elements.iter() {
                    stack[0] = acc;
                    stack[1] = x;
                    acc = block(&mut stack, 2)?;
                }
                Value::from_fraction(integer(acc))
            }
            (FusedWalk::Scan, Some(mut acc)) => {
                let mut results = Vec::with_capacity(elements.len());
                for &x in elements.iter() {
                    stack[0] = acc;
                    stack[1] = x;
                    acc = block(&mut stack, 2)?;
                    results.push(Value::from_fraction(integer(acc)));
                }
                Value::from_vector(results)
            }
            (FusedWalk::Fold | FusedWalk::Scan, None) => return None,
        };
        Some((value, Charges { runs, steps, work }))
    }

    /// The rational tier: any plain rationals, charged op by op as the
    /// interpreted route charges them.
    fn run_rational(
        &self,
        interp: &Interpreter,
        walk: FusedWalk,
        target: &Value,
        seed: Option<&Fraction>,
    ) -> Option<(Value, Charges)> {
        let elements = rational_elements(target)?;
        let runs = elements.len() as u64;
        let steps = self.steps_within_ceiling(interp, runs)?;
        let work_budget = interp
            .runtime_limits
            .max_numeric_work
            .checked_sub(interp.numeric_work_used)?;
        let mut work = 0u64;

        let mut accumulator = seed.cloned();
        let mut results: Vec<Fraction> = Vec::with_capacity(match walk {
            FusedWalk::Fold => 0,
            FusedWalk::Map | FusedWalk::Scan => elements.len(),
        });
        let mut stack: Vec<Fraction> = Vec::with_capacity(self.ops.len() + 2);
        for element in elements {
            stack.clear();
            if let Some(acc) = accumulator.take() {
                stack.push(acc);
            }
            stack.push(element);
            for op in &self.ops {
                match op {
                    FusedOp::Push(f) => stack.push(f.clone()),
                    FusedOp::Apply(schema) => {
                        let b = stack.pop()?;
                        let a = stack.pop()?;
                        work = work.saturating_add(binary_numeric_work(
                            fraction_work_bits(&a),
                            fraction_work_bits(&b),
                        ));
                        if work > work_budget {
                            return None;
                        }
                        // A zero divisor projects a reasoned NIL, and the NIL
                        // trace that records it is the ordinary route's.
                        let r = schema.fraction(&a, &b).ok()?;
                        if interp
                            .runtime_limits
                            .check_algebraic_size(0, fraction_result_bits(&r))
                            .is_err()
                        {
                            return None;
                        }
                        stack.push(r);
                    }
                }
            }
            let result = stack.pop()?;
            match walk {
                FusedWalk::Map => results.push(result),
                FusedWalk::Scan => {
                    results.push(result.clone());
                    accumulator = Some(result);
                }
                FusedWalk::Fold => accumulator = Some(result),
            }
        }

        let value = match walk {
            FusedWalk::Map => {
                promote_rationals(results).unwrap_or_else(Value::from_vector_promoted)
            }
            FusedWalk::Scan => {
                Value::from_vector(results.into_iter().map(Value::from_fraction).collect())
            }
            FusedWalk::Fold => Value::from_fraction(accumulator?),
        };
        Some((value, Charges { runs, steps, work }))
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
