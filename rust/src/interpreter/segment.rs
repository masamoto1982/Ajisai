//! Typed straight-line segments: a run of a line lowered to a small typed
//! bytecode and run on unboxed machine words, the dispatch skipped whole.
//!
//! Every Word a line dispatches pays for the dispatch: a `Value` built for
//! each operand and result, the stack pushed and popped, the declared NIL
//! contract, the lift, the meter, the trace. `quickened` already skips most
//! of that for one scalar Word at a time; what it cannot skip is the
//! per-Word round trip through boxed stack slots, and names and User Word
//! calls between the Words still take the full route. A segment covers the
//! whole run instead:
//!
//! - literals, `TRUE`, `FALSE`;
//! - `ADD` `SUB` `MUL` `DIV`, `LT` `GT` `EQ`, `MIN` `MAX`, `FLOOR` `ROUND`,
//!   `NOT` `AND` `SELECT` (`quickened::apply`, the one definition both use);
//! - `'NAME' BIND`, and a bound name read;
//! - a call of a User Word whose body is made of these, inlined
//!   (`segment_lower`).
//!
//! It holds every value as a `Slot` — a rational whose halves fit a machine
//! word, or a truth value — and runs speculatively, like a fused walk
//! (`fused_block`): it reads its operands and the names it needs, computes
//! on its own stack, and touches nothing. Only when the whole run has
//! finished does it commit, all at once, exactly what the dispatched route
//! would have left behind:
//!
//! - the stack: the operands it consumed gone, its results pushed;
//! - the names its own frame bound, at their last values (`bind_local`);
//! - one execution step per Word and per User Word call, the numeric work
//!   and the fast-path hits each Word is charged (`quickened::apply`);
//! - for each User Word call, its compiled-plan lookup: a hit, or — for the
//!   first call of a Word with no current plan — a miss, a build, an epoch,
//!   and the plan stored.
//!
//! Anything it cannot reproduce exactly sends the run back to the dispatched
//! route from the segment's first op, nothing touched: an operand or a name
//! that is not a plain machine-word value, a result that leaves a machine
//! word, a zero divisor, an operand outside a Word's domain, a step, work or
//! size ceiling the run would reach, a call deeper than the depth guard
//! allows, or a dictionary that moved since the segment was lowered. A
//! segment is pure, so a run abandoned and run again is unobservable, and
//! what it commits is what the dispatch charges, so which route ran is
//! unobservable too (LANG.AUTHORITY.FREEDOM). `segment_tests` holds the two
//! equal.

use std::sync::Arc;

use smallvec::SmallVec;

use crate::interpreter::compiled_plan::{arc_plan, compile_word_definition};
use crate::interpreter::interpreter_core::MAX_USER_WORD_DEPTH;
use crate::interpreter::quickened::{apply, Kind, Slot};
use crate::interpreter::{is_plan_valid, CompiledPlan, Interpreter};

/// One op of a segment.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SegOp {
    /// A literal: free.
    Push(Slot),
    /// `TRUE`/`FALSE`: a Word, so a step. `checked` when the route it
    /// stands for runs the nesting check after it, as the token walk's
    /// dispatch does and a compiled `PushWordLiteral` does not.
    PushWord { value: bool, checked: bool },
    /// A scalar Word: a step, and what `quickened::apply` charges.
    Word(Kind),
    /// `'NAME' BIND` into a slot: a step.
    Bind(u32),
    /// A bound name read: free, as a binding read is.
    Load(u32),
}

/// A lowered run of a line (`segment_lower`).
#[derive(Debug)]
pub(crate) struct Segment {
    pub(crate) ops: Vec<SegOp>,
    /// Values the run consumes from the stack beneath it.
    pub(crate) inputs: usize,
    pub(crate) slots: usize,
    /// Names the run reads before its own frame binds them, and the slot
    /// each is read into when the run starts.
    pub(crate) reads: Vec<(String, u32)>,
    /// Names the run's own frame binds, and the slot holding each one's
    /// last value. A called Word's frame ends with the call, so its names
    /// are not here.
    pub(crate) binds: Vec<(String, u32)>,
    /// Words dispatched, User Word calls included.
    pub(crate) steps: usize,
    /// Each User Word called, once, and how many calls in all.
    pub(crate) callees: Vec<String>,
    pub(crate) calls: u64,
    /// How many User Word frames deep the inlined calls go.
    pub(crate) call_depth: usize,
    /// The dictionary the run was lowered against: the Words it inlined and
    /// the names it found bindable.
    pub(crate) dictionary_epoch: u64,
}

/// Where a segment sits in a compiled line: `ops[start..end]`.
#[derive(Debug, Clone)]
pub(crate) struct LineSegment {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) code: Arc<Segment>,
}

impl Segment {
    /// Run the segment against the top of the stack and commit it, or answer
    /// `false` having touched nothing.
    pub(crate) fn try_run(&self, interp: &mut Interpreter) -> bool {
        let committed = self.run(interp).is_some();
        #[cfg(test)]
        if committed {
            SEGMENT_RUNS.with(|c| c.set(c.get() + 1));
        }
        committed
    }

    fn run(&self, interp: &mut Interpreter) -> Option<()> {
        if !interp.segments_enabled
            || !interp.scalar_fastpath_enabled
            || interp.runtime_limits.max_bigint_bits < 64
            || interp.dictionary_epoch != self.dictionary_epoch
            || interp.execution_step_count.checked_add(self.steps)? > interp.max_execution_steps
            || interp.call_depth + self.call_depth > MAX_USER_WORD_DEPTH
        {
            return None;
        }

        let values = interp.stack.as_slice();
        let base = values.len().checked_sub(self.inputs)?;
        let mut stack: SmallVec<[Slot; 16]> = SmallVec::new();
        for value in &values[base..] {
            stack.push(Slot::of(value)?);
        }
        let mut slots: SmallVec<[Slot; 8]> = SmallVec::from_elem(Slot::Bool(false), self.slots);
        for (name, slot) in &self.reads {
            slots[*slot as usize] = Slot::of(&interp.lookup_binding(name)?)?;
        }

        let (mut work, mut fastpath) = (0u64, 0u64);
        // The stack's height after the last op the dispatch follows with the
        // nesting check (`check_fresh_nesting`), which marks everything
        // beneath it checked.
        let mut checked_below: Option<usize> = None;
        for op in &self.ops {
            match *op {
                SegOp::Push(slot) => stack.push(slot),
                SegOp::Load(slot) => stack.push(slots[slot as usize]),
                SegOp::PushWord { value, checked } => {
                    stack.push(Slot::Bool(value));
                    if checked {
                        checked_below = Some(stack.len());
                    }
                }
                SegOp::Bind(slot) => {
                    slots[slot as usize] = stack.pop()?;
                    checked_below = Some(stack.len());
                }
                SegOp::Word(kind) => {
                    let at = stack.len().checked_sub(kind.arity())?;
                    let answer = apply(kind, &stack[at..])?;
                    stack.truncate(at);
                    stack.push(answer.value);
                    work += answer.work;
                    fastpath += answer.fastpath;
                    checked_below = Some(stack.len());
                }
            }
        }
        if interp.numeric_work_used.checked_add(work)? > interp.runtime_limits.max_numeric_work {
            return None;
        }
        // The first check the dispatch makes reads every value written since
        // the last one, the unchecked ones beneath the operands included;
        // what the run itself makes is plain and nests nothing.
        let unchecked = interp.stack.fresh_start().min(base);
        let deepest = values[unchecked..base]
            .iter()
            .map(|v| v.nesting())
            .max()
            .unwrap_or(0);
        interp
            .runtime_limits
            .check_nesting_depth(deepest as usize)
            .ok()?;

        // Each call looks its Word's plan up; one with none current is built
        // by its first call.
        let mut builds: SmallVec<[(&str, Arc<CompiledPlan>); 2]> = SmallVec::new();
        for name in &self.callees {
            let def = interp.user_words.get(name.as_str())?;
            if !matches!(&def.compiled_plan, Some(plan) if is_plan_valid(plan, interp)) {
                builds.push((name, arc_plan(compile_word_definition(def, interp))));
            }
        }

        interp.execution_step_count += self.steps;
        interp.numeric_work_used += work;
        let metrics = &mut interp.runtime_metrics;
        metrics.scalar_fastpath_count = metrics.scalar_fastpath_count.saturating_add(fastpath);
        let built = builds.len() as u64;
        metrics.compiled_plan_cache_miss_count += built;
        metrics.compiled_plan_build_count += built;
        metrics.compiled_plan_cache_hit_count += self.calls - built;
        if built > 0 {
            interp.global_epoch += built;
            interp.execution_epoch = interp.global_epoch;
        }
        for (name, plan) in builds {
            interp.store_compiled_plan_for_word(name, plan);
        }
        for (name, slot) in &self.binds {
            interp.bind_local(name.clone(), slots[*slot as usize].into_value());
        }
        interp.stack.truncate(base);
        for slot in stack {
            interp.stack.push(slot.into_value());
        }
        // Where the dispatch's last check left the mark. Only pushes follow
        // it, and a push never lowers the mark below the slot it fills. With
        // no check in the run there were no pops either, so the pushes above
        // have already marked what the dispatch's pushes would have.
        if let Some(height) = checked_below {
            interp.stack.set_fresh_start(base + height);
        }
        Some(())
    }
}

#[cfg(test)]
thread_local! {
    static SEGMENT_RUNS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Segments committed on this thread, for tests that pin which route ran.
#[cfg(test)]
pub(crate) fn segment_runs_on_this_thread() -> u64 {
    SEGMENT_RUNS.with(|c| c.get())
}
