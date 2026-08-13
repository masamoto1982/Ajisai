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
//!    element count cannot see;
//!  * an element that is itself a 64-element vector costs 41x more to probe
//!    when the elements agree on their first 63 positions than when they differ
//!    at the first, so "one element" is not a unit of work;
//!  * an algebraic element costs 3.0 µs to compare against 5.8 ns for a machine
//!    word, because deciding it rebases two radical bases.
//!
//! So the charge is *operations × the price of one operation*, with the price
//! measured from the operand ([`ElementCost`]) and the count supplied by the
//! Word. Copies and comparison sorts know their count before they start and are
//! charged at the entry, exactly as arithmetic is. Equality scans do not — the
//! count depends on the data — and are charged per element, before that
//! element's scan, against the number of distinct values found so far. That
//! number is an upper bound on the probes the element will perform, so a
//! refusal still happens before the work rather than after it.

use crate::error::Result;
use crate::interpreter::arithmetic_meter::measure_operand;
use crate::interpreter::runtime_limits::{ElementCost, OperandWork};
use crate::interpreter::Interpreter;
use crate::types::Value;

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
        return Err(crate::error::AjisaiError::ResourceLimitExceeded {
            resource: crate::error::ResourceLimit::CollectionWork,
            limit: interp.runtime_limits.max_collection_work,
            observed: Some(interp.collection_work_used),
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

/// The running charge for an equality scan — `UNIQUE`, `TALLY`, `GROUP`,
/// `INDEX-OF`.
///
/// The one place in either meter that does not charge everything at the entry,
/// and the reason is a property of the operation rather than an exception made
/// for it. Arithmetic can pre-charge because the cost of `a * b` is a function
/// of the operands' *shape*, which is known before it runs. A scan's cost is a
/// function of the operands' *content*: how many distinct values the data
/// holds, which is what the scan is for. Charging the worst case (`n²/2`
/// probes) up front would refuse the programs these Words exist to serve — a
/// per-class tally, a vocabulary, a grouping key — which are cheap precisely
/// because their distinct count is small; measured, `d=1` at 16,000 elements is
/// 1,300x cheaper than `d=n`.
///
/// What "refuse rather than measure" actually requires is that no more than the
/// budget is ever spent, and [`Self::charge_scan_of`] satisfies it by charging
/// an upper bound on each element's probes *before* that element is scanned.
pub(crate) struct ScanMeter {
    probe_units: u64,
    copy_units: u64,
}

impl ScanMeter {
    /// Price a scan over `items`.
    pub(crate) fn new(items: &[Value]) -> Self {
        let cost = element_cost_of_slice(items);
        Self {
            probe_units: cost.probe(),
            copy_units: cost.copy(),
        }
    }

    /// Charge, before scanning one element, for probing `candidates` of them.
    ///
    /// `candidates` is the number of distinct values found so far, which is the
    /// most probes this element can perform. An element that matches early is
    /// over-charged — by 2x on average over a uniform match, and not at all
    /// when every element is distinct or every element is equal, the two ends
    /// the ceiling exists to separate.
    pub(crate) fn charge_scan_of(&self, interp: &mut Interpreter, candidates: usize) -> Result<()> {
        charge(interp, self.probe_units.saturating_mul(candidates as u64))
    }

    /// Charge for retaining one element in the result.
    pub(crate) fn charge_retained(&self, interp: &mut Interpreter) -> Result<()> {
        charge(interp, self.copy_units)
    }
}
