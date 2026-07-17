//! Call-site shape inline cache (IC) for compiled builtin calls.
//!
//! The idea is borrowed from hidden-class/shape-based dispatch in JavaScript
//! engines: a call site that has only ever seen one operand shape can skip the
//! generic dispatch chain and jump straight to the specialized route, guarded
//! by a cheap re-check of the operands. The cache is *routing state only* —
//! every route revalidates its operands before producing a value, so a stale
//! or racy cache entry can never change an observable result, only which
//! equivalent route computes it.
//!
//! States (one `AtomicU8` per compiled call site):
//! - `UNSEEN`: the site has not executed yet; probe the scalar fast path.
//! - `SCALAR`: every execution so far completed on the scalar fast path;
//!   keep probing it first.
//! - `GENERIC`: the site has seen operands the scalar fast path rejected;
//!   stop probing and go straight to the generic executor (which still
//!   contains its own fast-path attempt after the NIL check, preserving
//!   exact baseline behavior).
//!
//! Non-canonical optimization: no surface syntax or value-semantics change.
//! The canonical definition of every word remains `SPECIFICATION.html`.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::builtins::BuiltinExecutorKey;
use crate::error::Result;

use super::{arithmetic, comparison, Interpreter};

const IC_UNSEEN: u8 = 0;
const IC_SCALAR: u8 = 1;
const IC_GENERIC: u8 = 2;

/// Per-call-site monomorphic shape cache. Shared across plan clones via the
/// `Arc<CompiledCall>` that owns it; atomic so a hedged race between the
/// compiled and plain paths can only flip routing, never corrupt a value.
#[derive(Debug)]
pub struct ShapeIc {
    state: AtomicU8,
}

impl Default for ShapeIc {
    fn default() -> Self {
        Self {
            state: AtomicU8::new(IC_UNSEEN),
        }
    }
}

impl ShapeIc {
    fn is_generic(&self) -> bool {
        self.state.load(Ordering::Relaxed) == IC_GENERIC
    }

    fn note_scalar_hit(&self) {
        self.state.store(IC_SCALAR, Ordering::Relaxed);
    }

    fn note_generic(&self) {
        self.state.store(IC_GENERIC, Ordering::Relaxed);
    }
}

/// The binary scalar operations the shape IC can route. Resolved once at
/// compile time from the builtin's executor key so the runtime never has to
/// re-classify the word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeIcOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Neq,
}

impl ShapeIcOp {
    /// Which executor keys have a scalar fast path the IC can target.
    pub fn from_executor_key(key: BuiltinExecutorKey) -> Option<Self> {
        match key {
            BuiltinExecutorKey::Add => Some(ShapeIcOp::Add),
            BuiltinExecutorKey::Sub => Some(ShapeIcOp::Sub),
            BuiltinExecutorKey::Mul => Some(ShapeIcOp::Mul),
            BuiltinExecutorKey::Div => Some(ShapeIcOp::Div),
            BuiltinExecutorKey::Lt => Some(ShapeIcOp::Lt),
            BuiltinExecutorKey::Le => Some(ShapeIcOp::Le),
            BuiltinExecutorKey::Gt => Some(ShapeIcOp::Gt),
            BuiltinExecutorKey::Gte => Some(ShapeIcOp::Ge),
            BuiltinExecutorKey::Eq => Some(ShapeIcOp::Eq),
            BuiltinExecutorKey::Neq => Some(ShapeIcOp::Neq),
            _ => None,
        }
    }
}

/// Try to complete `op` through the scalar fast path, guided by the site's
/// shape cache. Returns `Ok(true)` when the operation fully completed (value
/// pushed, hints recorded, metrics counted) and `Ok(false)` when the caller
/// must run the generic executor.
///
/// Safety of the shortcut: the scalar fast paths only accept operands that can
/// never be operational NIL (bare scalars and singleton numeric wrappers), so
/// skipping the generic route's NIL-passthrough pre-check for a completed fast
/// path is observationally identical. On any rejection the generic route runs
/// from its very beginning, NIL check included.
pub(crate) fn try_shape_ic_call(
    interp: &mut Interpreter,
    ic: &ShapeIc,
    op: ShapeIcOp,
) -> Result<bool> {
    if !interp.shape_ic_enabled || !interp.scalar_fastpath_enabled || ic.is_generic() {
        return Ok(false);
    }

    let completed = match op {
        ShapeIcOp::Add => arithmetic::scalar_fastpath_add(interp)?,
        ShapeIcOp::Sub => arithmetic::scalar_fastpath_sub(interp)?,
        ShapeIcOp::Mul => arithmetic::scalar_fastpath_mul(interp)?,
        ShapeIcOp::Div => arithmetic::scalar_fastpath_div(interp)?,
        ShapeIcOp::Lt => comparison::scalar_fastpath_lt(interp),
        ShapeIcOp::Le => comparison::scalar_fastpath_le(interp),
        ShapeIcOp::Gt => comparison::scalar_fastpath_gt(interp),
        ShapeIcOp::Ge => comparison::scalar_fastpath_ge(interp),
        ShapeIcOp::Eq => comparison::scalar_fastpath_eq(interp),
        ShapeIcOp::Neq => comparison::scalar_fastpath_neq(interp),
    };

    if completed {
        ic.note_scalar_hit();
        interp.runtime_metrics.shape_ic_hit_count =
            interp.runtime_metrics.shape_ic_hit_count.saturating_add(1);
    } else {
        ic.note_generic();
        interp.runtime_metrics.shape_ic_miss_count =
            interp.runtime_metrics.shape_ic_miss_count.saturating_add(1);
    }
    Ok(completed)
}
