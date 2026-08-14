//! Where a collection Word's element work is charged — the collection meter's
//! boundary.
//!
//! A collection Word loops inside Rust. `executionSteps` prices word count and
//! charges it one step; the work meter prices arithmetic and it performs none;
//! `materializedElements` bounds how big its operand may be but says nothing
//! about what is done to it. So `[ 0 99999 ] RANGE UNIQUE` spent 45 seconds as
//! one step of a hundred-thousand-step budget, and every declared ceiling
//! stayed silent.
//!
//! What is charged is not the element count. Three measurements
//! (`examples/collection_word_calibration`, and
//! `docs/dev/collection-word-billing-2026-08-13.md` for the tables) rule that
//! out:
//!
//!  * the same 16,000-element `UNIQUE` costs 0.52 ms or 682 ms depending only
//!    on how many *distinct* values the data holds — a factor of 1,300 the
//!    element count cannot see, when `UNIQUE` finds its bucket by scanning
//!    every distinct value seen so far;
//!  * an element that is itself a 64-element vector costs 41x more to probe
//!    when the elements agree on their first 63 positions than when they differ
//!    at the first, so "one element" is not a unit of work;
//!  * an algebraic element costs 3.0 µs to compare against 5.8 ns for a machine
//!    word, because deciding it rebases two radical bases.
//!
//! So the charge is *operations × the price of one operation*, with the price
//! measured from the operand ([`ElementCost`]) and the count supplied by the
//! Word. Copies and comparison sorts know their count before they start and are
//! charged at the entry, exactly as arithmetic is.
//!
//! `UNIQUE`, `TALLY` and `GROUP` used to be the exception: a linear scan
//! against every distinct value found so far, which is where the first
//! bullet's 1,300x came from, and which meant the honest price of the scan
//! itself was `n × distinct` — quadratic on the data the scan cannot see in
//! advance. `Value: Hash` (canonical for every domain the scan visits,
//! including a bounded interval refinement for algebraic elements — see
//! `types::exact::algebraic::Algebraic::hash`) turned that scan into a
//! `HashMap` lookup, so the per-element price this module charges is now
//! fixed — [`ElementCost::probe`], the cost of hashing the element once —
//! not scaled by how many distinct values already exist. See [`ScanMeter`]
//! for what stays a per-element pre-charge rather than a single entry
//! charge: the *count* of elements is known up front, but committing to
//! *which* elements survive is not, so a refusal still lands before the
//! scan runs past the budget rather than after.

use crate::error::{AjisaiError, ResourceProgress, Result, PROGRESS_UNIT_ELEMENTS};
use crate::interpreter::arithmetic_meter::measure_operand;
use crate::interpreter::runtime_limits::{ElementCost, OperandWork, COLLECTION_COPY_UNITS};
use crate::interpreter::Interpreter;
use crate::types::Value;

/// What one `HashMap` lookup costs beyond the leaf-width probe it hashes, in
/// collection units — the de-quadraticization follow-up's constant
/// (`docs/dev/collection-word-dequadraticization-2026-08-14.md`).
///
/// `probe_units` alone undercharged at scale: at 1,000,000 all-distinct
/// elements (the playground's `materializedElements` ceiling), it charged
/// 2,277 units/ms of real wall time, *below* the 2,814 units/ms floor
/// `examples/work_meter_calibration` already accepts on an unbounded numeric
/// path — table growth and cache pressure a flat per-leaf charge cannot see.
/// This adds a second fixed charge to every element hashed (not only the
/// ones retained), which restores the margin. Reuses `COLLECTION_COPY_UNITS`'s
/// value for the reason `collection-word-billing-2026-08-13.md` §4 gives it:
/// a `HashMap` insert is itself a copy of the key into the table.
const COLLECTION_HASH_UNITS: u64 = COLLECTION_COPY_UNITS;

/// Charge `units` of collection work and fail — diagnosably, before the loop
/// that would spend it runs — if the cumulative total crosses
/// `runtime_limits.max_collection_work`.
///
/// A free function here rather than a method beside `charge_numeric_work`,
/// which is the shape the arithmetic meter has: the counter is the
/// interpreter's, but deciding what a collection Word owes is this module's
/// whole subject, and every caller below is in this file. Nothing outside it
/// charges the meter, for the reason the arithmetic meter was consolidated into
/// one entry — a price scattered over the implementations is a price the next
/// implementation forgets.
pub(crate) fn charge(interp: &mut Interpreter, units: u64) -> Result<()> {
    interp.collection_work_used = interp.collection_work_used.saturating_add(units);
    if interp.collection_work_used > interp.runtime_limits.max_collection_work {
        return Err(AjisaiError::ResourceLimitExceeded {
            resource: crate::error::ResourceLimit::CollectionWork,
            limit: interp.runtime_limits.max_collection_work,
            observed: Some(interp.collection_work_used),
            // Filled in by `ScanMeter` for the operations charged as they go.
            // A pre-charged copy or sort has nothing to add: its `observed`
            // already includes the whole operation's cost, so it says how far
            // over the request was without any help.
            progress: None,
        });
    }
    Ok(())
}

/// The price of one element of a vector `value`.
///
/// Measured through `measure_operand`, the same reading the arithmetic meter
/// takes, so the two meters cannot end up with different opinions about how
/// wide an operand is. It is O(1) for a dense tensor and O(elements) for a
/// boxed vector — the same order as the Word being priced, and bounded by the
/// same `materializedElements` ceiling. A Word that is *not* O(elements) —
/// `LENGTH` reads a count — must not call this, and does not.
pub(crate) fn element_cost(value: &Value) -> ElementCost {
    ElementCost::measure(measure_operand(value), value.len())
}

/// The price of one element of an already-materialized element list.
///
/// Same measure as [`element_cost`], for the Words that reach the meter holding
/// the elements rather than the vector they came from.
pub(crate) fn element_cost_of_slice(items: &[Value]) -> ElementCost {
    let work = items
        .iter()
        .map(measure_operand)
        .reduce(OperandWork::join)
        .unwrap_or(OperandWork::leaf(1));
    ElementCost::measure(work, items.len())
}

/// Charge for copying `count` elements of `value` into a result.
///
/// The whole cost is known before the copy runs, so it is taken at the entry,
/// exactly as the arithmetic meter's charge is. `count` is separate from
/// `value.len()` for the Words that copy only a *part* of what they were handed
/// (`TAKE` a prefix, `GET` a selection): pricing those by the operand's length
/// would charge for elements they never touch.
pub(crate) fn charge_copy_of(interp: &mut Interpreter, value: &Value, count: usize) -> Result<()> {
    let units = element_cost(value).copies(count);
    charge(interp, units)
}

/// Charge for copying every element of the vector on top of the stack.
///
/// `portion` maps the vector's length to the number of elements the Word will
/// actually copy: the identity for `REVERSE`, a clamped count for `TAKE`. A
/// non-vector top is not charged — that is a structure error, and the Word
/// reports it for itself.
pub(crate) fn charge_stacktop_copy(
    interp: &mut Interpreter,
    portion: impl FnOnce(usize) -> usize,
) -> Result<()> {
    let units = match interp.stack.last() {
        Some(value) if value.is_vector() => element_cost(value).copies(portion(value.len())),
        _ => return Ok(()),
    };
    charge(interp, units)
}

/// Charge for materializing `count` fresh elements of unit width.
///
/// The generative Words (`RANGE`, `FILL`, `RANDOM`) have no operand to measure
/// — the elements do not exist yet — but they allocate the same boxed values a
/// copy does, and a program can ask for them repeatedly. Priced as a copy of
/// `count` machine-word scalars, which is what they are.
pub(crate) fn charge_materialization(interp: &mut Interpreter, count: usize) -> Result<()> {
    let cost = ElementCost {
        leaves: 1,
        width: 1,
    };
    charge(interp, cost.copies(count))
}

/// Charge for a comparison sort over `items` at its worst case, plus the copy
/// of the result.
pub(crate) fn charge_comparison_sort(interp: &mut Interpreter, items: &[Value]) -> Result<()> {
    let units = element_cost_of_slice(items).comparison_sort(items.len());
    charge(interp, units)
}

/// The running charge for a hash-keyed scan — `UNIQUE`, `TALLY`, `GROUP`.
///
/// The one place in either meter that does not charge everything at the
/// entry, and the reason is a property of the operation rather than an
/// exception made for it. Arithmetic can pre-charge because the cost of
/// `a * b` is a function of the operands' *shape*, known before it runs. A
/// hash-keyed scan's per-element cost is also a function of shape — hashing
/// one element visits its leaves once, the same width [`ElementCost::probe`]
/// already prices for a comparison — but the *count* of elements it will
/// need to hash is exactly the vector length, so this charges that fixed
/// per-element price before each element rather than the whole scan at the
/// entry. That still refuses before the work runs, one element early rather
/// than the whole vector early: a `HashMap` lookup and insert is O(1)
/// amortized on top of the hash itself, so nothing here scales with how many
/// distinct values have been found — unlike the linear-scan design this
/// replaced, where a probe was charged against every distinct value seen so
/// far and a vocabulary of size `d` cost `n × d` (`docs/dev/collection-word-billing-2026-08-13.md`
/// priced that; the de-quadraticization follow-up re-priced it here).
///
/// An algebraic element's hash is not free either: [`crate::types::exact::algebraic::Algebraic::hash`]
/// runs a bounded interval refinement to find its representation-independent
/// bucket, comparable in cost to the one comparison [`ALGEBRAIC_ELEMENT_UNITS`](crate::interpreter::runtime_limits::ALGEBRAIC_ELEMENT_UNITS)
/// already prices — [`ElementCost::probe`] already folds that constant into
/// an algebraic leaf's width, so no separate charge is needed here.
pub(crate) struct ScanMeter {
    probe_units: u64,
    copy_units: u64,
    total: u64,
}

impl ScanMeter {
    /// Price a scan over `items`.
    pub(crate) fn new(items: &[Value]) -> Self {
        let cost = element_cost_of_slice(items);
        Self {
            probe_units: cost.probe(),
            copy_units: cost.copy(),
            total: items.len() as u64,
        }
    }

    /// Charge, before scanning element number `completed`, for hashing it —
    /// the fixed per-element price of finding its bucket, however many
    /// distinct buckets already exist.
    ///
    /// `completed` is how many elements are already behind it, and it is what
    /// makes a refusal here actionable. Because this meter charges as it goes,
    /// `observed` on the way out is always a hair over the limit however far
    /// over the request really was, so it cannot say how much smaller the input
    /// needs to be. `completed` can: the budget bought exactly this many
    /// elements of this data. See `error::ResourceProgress`.
    ///
    /// `probe_units` prices hashing the element's own leaves; `COLLECTION_HASH_UNITS`
    /// prices the `HashMap` machinery around that hash — table growth and
    /// cache pressure that a flat per-leaf charge cannot see, and that
    /// re-measuring at scale showed matters.
    pub(crate) fn charge_scan_of(&self, interp: &mut Interpreter, completed: usize) -> Result<()> {
        let units = self.probe_units.saturating_add(COLLECTION_HASH_UNITS);
        charge(interp, units).map_err(|error| self.with_progress(error, completed))
    }

    /// Charge for retaining one element in the result.
    pub(crate) fn charge_retained(&self, interp: &mut Interpreter, completed: usize) -> Result<()> {
        charge(interp, self.copy_units).map_err(|error| self.with_progress(error, completed))
    }

    /// Attach how far the scan got to the ceiling it just crossed.
    fn with_progress(&self, error: AjisaiError, completed: usize) -> AjisaiError {
        match error {
            AjisaiError::ResourceLimitExceeded {
                resource,
                limit,
                observed,
                progress: None,
            } => AjisaiError::ResourceLimitExceeded {
                resource,
                limit,
                observed,
                progress: Some(ResourceProgress {
                    completed: completed as u64,
                    total: self.total,
                    unit: PROGRESS_UNIT_ELEMENTS,
                }),
            },
            other => other,
        }
    }
}
