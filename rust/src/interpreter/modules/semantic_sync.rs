//! Semantic-plane resynchronization for the module-word execution path.
//!
//! SPEC §12.1: an interpretation role is decided once, at value construction,
//! and rendering is a pure function of `(data, role)`. The semantic plane
//! (SPEC §5.2) keys roles by stack position, so a word that pops operands and
//! pushes results at the same positions must not let the operands' roles leak
//! onto the results (e.g. `'{"a":1}' JSON@PARSE` must not render the parsed
//! Record under the consumed input string's `Text` role).
//!
//! Core words repair the plane per word via `apply_word_hint_override`; module
//! words share one execution chokepoint (`execute_module_word`), so the repair
//! is done there, path-level and word-agnostic: fingerprint every stack slot
//! before the executor runs, and afterwards reset the role of every slot whose
//! value changed to the role the new value was constructed with (`Value.hint`).
//! Unchanged slots keep their plane role, preserving position-scoped casts
//! such as `>CF` (which retags a slot without rebuilding its value) and NIL
//! passthrough.

use crate::types::exact::{Algebraic, ExactReal};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};
use std::sync::Arc;

use crate::interpreter::Interpreter;

/// Cheap per-slot identity for one stack position. Heap-backed values are
/// identified by their `Arc` payload pointer (module words rebuild changed
/// values, so pointer identity means "left untouched"); inline scalars are
/// identified by value. `Opaque` marks kinds with no cheap identity — they
/// compare as changed, which degrades to re-deriving the role from the
/// value's own construction-time hint (safe, never a stale leak).
pub(super) enum SlotFingerprint {
    Nil,
    Boolean(bool),
    Scalar(Fraction),
    ExactRational(Fraction),
    /// A Tier 1 algebraic slot, identified by its stored normal form.
    /// Structural identity (`same_representation`) is the cheap check
    /// here: a rebuilt value with a different basis granularity compares
    /// as changed, which safely re-derives the role.
    ExactAlgebraic(Algebraic),
    Vector(*const (), Interpretation),
    Tensor(*const (), Interpretation),
    Record(*const (), Interpretation),
    ProcessHandle(u64),
    SupervisorHandle(u64),
    Opaque,
}

fn fingerprint(value: &Value) -> SlotFingerprint {
    match &value.data {
        ValueData::Nil => SlotFingerprint::Nil,
        ValueData::Boolean(b) => SlotFingerprint::Boolean(*b),
        ValueData::Scalar(f) => SlotFingerprint::Scalar(f.clone()),
        ValueData::ExactScalar(ExactReal::Rational(f)) => SlotFingerprint::ExactRational(f.clone()),
        ValueData::ExactScalar(ExactReal::Algebraic(a)) => {
            SlotFingerprint::ExactAlgebraic(a.clone())
        }
        // A Tier 2 process has no cheap value identity; Opaque compares
        // as changed, which safely re-derives the role.
        ValueData::ExactScalar(ExactReal::Computable(_)) => SlotFingerprint::Opaque,
        ValueData::Vector(v) => SlotFingerprint::Vector(Arc::as_ptr(v).cast(), value.hint),
        ValueData::Tensor { data, .. } => {
            SlotFingerprint::Tensor(Arc::as_ptr(data).cast(), value.hint)
        }
        ValueData::Record { pairs, .. } => {
            SlotFingerprint::Record(Arc::as_ptr(pairs).cast(), value.hint)
        }
        ValueData::ProcessHandle(id) => SlotFingerprint::ProcessHandle(*id),
        ValueData::SupervisorHandle(id) => SlotFingerprint::SupervisorHandle(*id),
        ValueData::CodeBlock(_) => SlotFingerprint::Opaque,
    }
}

fn slot_unchanged(before: &SlotFingerprint, now: &Value) -> bool {
    match (before, &now.data) {
        (SlotFingerprint::Nil, ValueData::Nil) => true,
        (SlotFingerprint::Boolean(b), ValueData::Boolean(n)) => b == n,
        (SlotFingerprint::Scalar(f), ValueData::Scalar(n)) => f == n,
        (SlotFingerprint::ExactRational(f), ValueData::ExactScalar(ExactReal::Rational(n))) => {
            f == n
        }
        (SlotFingerprint::ExactAlgebraic(a), ValueData::ExactScalar(ExactReal::Algebraic(n))) => {
            a.same_representation(n)
        }
        (SlotFingerprint::Vector(p, hint), ValueData::Vector(v)) => {
            *hint == now.hint && std::ptr::eq(*p, Arc::as_ptr(v).cast())
        }
        (SlotFingerprint::Tensor(p, hint), ValueData::Tensor { data, .. }) => {
            *hint == now.hint && std::ptr::eq(*p, Arc::as_ptr(data).cast())
        }
        (SlotFingerprint::Record(p, hint), ValueData::Record { pairs, .. }) => {
            *hint == now.hint && std::ptr::eq(*p, Arc::as_ptr(pairs).cast())
        }
        (SlotFingerprint::ProcessHandle(id), ValueData::ProcessHandle(n)) => id == n,
        (SlotFingerprint::SupervisorHandle(id), ValueData::SupervisorHandle(n)) => id == n,
        _ => false,
    }
}

/// Fingerprint every stack slot before a module word executes.
pub(super) fn snapshot_stack_slots(stack: &[Value]) -> Vec<SlotFingerprint> {
    stack.iter().map(fingerprint).collect()
}

/// After a module word executed successfully, re-derive the semantic-plane
/// role of every stack slot whose value is not the one fingerprinted before
/// execution — from the new value's construction-time role (`Value.hint`,
/// SPEC §12.1) — while leaving untouched slots' plane roles intact.
pub(super) fn resync_changed_slots(interp: &mut Interpreter, before: &[SlotFingerprint]) {
    let stack_len = interp.stack.len();
    interp.semantic_registry.normalize_to_stack_len(stack_len);
    for i in 0..stack_len {
        let value = &interp.stack[i];
        let unchanged = before
            .get(i)
            .is_some_and(|slot| slot_unchanged(slot, value));
        if !unchanged {
            let hint = value.hint;
            interp.semantic_registry.update_hint_at(i, hint);
        }
    }
}
