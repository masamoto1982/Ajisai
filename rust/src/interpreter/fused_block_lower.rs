//! Lowering a block's compiled plan to the straight-line ops of a fused walk
//! (`fused_block`), User Word calls included.
//!
//! A User Word called from a block is inlined: its body's plan is lowered in
//! place, in a frame of its own. Nothing else about the call is observable on
//! a run that finishes, so a call costs what the interpreted route charges
//! for it and no more:
//!
//! - one execution step for the call itself (`execute_word_core` charges it
//!   before running the body), carried in `steps_per_run`;
//! - one compiled-plan lookup, a cache hit — or, for the first call of a Word
//!   whose plan is not current, a miss, a build and an epoch. The plan is
//!   built here without storing it; a committed walk stores it, so a walk that
//!   is abandoned leaves the next interpreted call to build it, as it would
//!   have.
//!
//! The call stack, the depth counter and failure attribution exist for runs
//! that fail, and a fused walk that meets a failure is abandoned. A Word body
//! is a barrier frame: it reads only the names it binds itself, so a name it
//! did not bind is the ordinary walk's to report, and the call depth is
//! bounded exactly as `execute_word_core` bounds it.

use std::collections::HashMap;
use std::sync::Arc;

use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::compiled_plan::{arc_plan, compile_word_definition, CompiledOp};
use crate::interpreter::fused_block::{Compare, FusedBlock, Op, Plain};
use crate::interpreter::interpreter_core::MAX_USER_WORD_DEPTH;
use crate::interpreter::{is_plan_valid, CompiledPlan, Interpreter};
use crate::kernel::generated::WordId;
use crate::types::{Token, Value};

/// The op `id` lowers to: a hand-written one, or the plain law of a Word the
/// contract admits (`fusion_contract`). `fusion_contract_tests` holds every
/// hand-written one to its Word's contract too.
pub(crate) fn word_op(id: WordId) -> Option<Op> {
    Some(match id {
        WordId::Add => Op::Arith(ExactArithmeticSchema::Add),
        WordId::Sub => Op::Arith(ExactArithmeticSchema::Sub),
        WordId::Mul => Op::Arith(ExactArithmeticSchema::Mul),
        WordId::Div => Op::Arith(ExactArithmeticSchema::Div),
        WordId::Lt => Op::Compare(Compare::Lt),
        WordId::Gt => Op::Compare(Compare::Gt),
        WordId::Eq => Op::Compare(Compare::Eq),
        WordId::Min => Op::Extremum { max: false },
        WordId::Max => Op::Extremum { max: true },
        WordId::Floor => Op::Floor,
        WordId::Round => Op::Round,
        WordId::Not => Op::Not,
        WordId::And => Op::And,
        WordId::Select => Op::Select,
        WordId::Pow => Op::Pow,
        _ => Op::Kernel(crate::interpreter::fusion_contract::kernel(id)?),
    })
}

/// The single name a `BIND` name operand gives, when it is one the
/// interpreter would accept. Anything it would refuse, or a destructuring
/// list, is the ordinary walk's to run.
pub(crate) fn bindable_name(interp: &Interpreter, value: &Value) -> Option<String> {
    let names = crate::interpreter::bindings::binding_names(value).ok()?;
    let [name] = names.as_slice() else {
        return None;
    };
    interp.check_bindable_name(name).ok()?;
    Some(name.to_uppercase())
}

struct Lowering<'a> {
    interp: &'a Interpreter,
    ops: Vec<Op>,
    /// Binding slots handed out so far, across every frame.
    slots: usize,
    /// Values on the block's stack after the ops so far.
    depth: usize,
    calls: u64,
    builds: Vec<(String, Arc<CompiledPlan>)>,
    reads_outer: bool,
}

/// Lower `plan` for a walk that starts the block on `inputs` values, or
/// `None` when any op is outside the fused subset or the block would
/// underflow (an ERROR the ordinary route reports).
///
/// A name the plan could not resolve is a `FallbackToken`: it lowers to a
/// `Load` when its frame bound it earlier, and — in the block's own frame —
/// to a `Push` of its value when a frame the block can see holds it. That
/// value is fixed for the whole walk, since a block binds only in the frame
/// it opens per run.
pub(crate) fn lower(
    plan: &CompiledPlan,
    interp: &Interpreter,
    inputs: usize,
) -> Option<FusedBlock> {
    let mut lowering = Lowering {
        interp,
        ops: Vec::with_capacity(plan.line.ops.len()),
        slots: 0,
        depth: inputs,
        calls: 0,
        builds: Vec::new(),
        reads_outer: false,
    };
    lowering.line(&plan.line.ops, &mut HashMap::new(), true, interp.call_depth)?;
    if lowering.depth == 0 {
        return None;
    }
    let words = lowering.ops.iter().filter(|op| op.is_word()).count() as u64;
    Some(FusedBlock {
        ops: lowering.ops,
        slots: lowering.slots,
        steps_per_run: words + lowering.calls,
        calls_per_run: lowering.calls,
        builds: lowering.builds,
        reads_outer: lowering.reads_outer,
        int_programs: Default::default(),
    })
}

impl Lowering<'_> {
    fn line(
        &mut self,
        source: &[CompiledOp],
        frame: &mut HashMap<String, usize>,
        sees_outer: bool,
        call_depth: usize,
    ) -> Option<()> {
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
                        let name = bindable_name(self.interp, value)?;
                        let slot = match frame.get(&name) {
                            Some(slot) => *slot,
                            None => {
                                self.slots += 1;
                                frame.insert(name, self.slots - 1);
                                self.slots - 1
                            }
                        };
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
                        Op::Kernel(kernel) => (kernel.arity, 1),
                        _ => (2, 1),
                    };
                    (op, pops, pushes)
                }
                CompiledOp::FallbackToken(Token::Symbol(name)) => {
                    let name = crate::word_name::canonical_word_name(name);
                    match frame.get(name.as_ref()) {
                        Some(slot) => (Op::Load(*slot), 0, 1),
                        None if sees_outer => {
                            self.reads_outer = true;
                            (
                                Op::Push(Plain::of(&self.interp.lookup_binding(&name)?)?),
                                0,
                                1,
                            )
                        }
                        None => return None,
                    }
                }
                CompiledOp::CallUserWord(name) => {
                    self.call(name, call_depth)?;
                    i += 1;
                    continue;
                }
                // `[ ... ] LENGTH` on a literal Vector is the
                // constant `quickened::try_length_call` answers, for one
                // step and nothing else.
                CompiledOp::PushVectorLiteral(value) => {
                    let CompiledOp::CallBuiltin(call) = source.get(i + 1)? else {
                        return None;
                    };
                    if call.word?.id != WordId::Length
                        || value.absence.is_some()
                        || !value.is_vector()
                        || value.is_nil()
                    {
                        return None;
                    }
                    let len = crate::types::fraction::Fraction::from(value.len() as i64);
                    i += 1;
                    (Op::Const(Plain::Num(len)), 0, 1)
                }
                _ => return None,
            };
            if self.depth < pops {
                return None;
            }
            self.depth = self.depth - pops + pushes;
            self.ops.push(op);
            i += 1;
        }
        Some(())
    }

    /// Inline a call of the User Word `name` (`execute_word_core`'s route
    /// for a definition with a body).
    fn call(&mut self, name: &str, call_depth: usize) -> Option<()> {
        if call_depth + 1 > MAX_USER_WORD_DEPTH {
            return None;
        }
        let def = self.interp.definition_of(name)?;
        if def.body.is_empty() {
            return None;
        }
        let plan = match def.compiled_plan.as_ref() {
            Some(plan) if is_plan_valid(plan, self.interp) => plan.clone(),
            _ => match self.builds.iter().find(|(built, _)| built == name) {
                Some((_, plan)) => plan.clone(),
                None => {
                    let plan = arc_plan(compile_word_definition(&def, self.interp));
                    self.builds.push((name.to_string(), plan.clone()));
                    plan
                }
            },
        };
        self.calls += 1;
        self.line(&plan.line.ops, &mut HashMap::new(), false, call_depth + 1)
    }
}
