use crate::types::{FlowToken, Value};

/// Judgment returned by the linear consumption optimization hook.
///
/// This API is informational only — it does not alter execution behavior.
/// Operators call [`check_in_place_candidate`] to identify values eligible
/// for future in-place mutation optimizations without violating the
/// fractional-dataflow mass-conservation invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InPlaceJudgment {
    /// Safe to update in place: `remaining == total`, no bifurcation links,
    /// and the backing allocation has exactly one owner (Rc strong count == 1
    /// for Vector/Record; inherently safe for Scalar/Nil).
    Safe,
    /// The backing allocation is shared (Rc strong count > 1).
    /// In-place mutation would silently corrupt other references.
    Aliased,
    /// The FlowToken has been partially consumed (`remaining < total`), is
    /// part of a bifurcation tree, or has a non-unit mass ratio.
    /// Mass-conservation semantics require the original value to be preserved.
    PartiallyConsumed,
    /// No FlowToken context is available for this value.
    /// Safety cannot be determined without flow tracking.
    NoFlowContext,
}

/// Hook: determine whether `value` is a safe in-place update candidate.
///
/// This function encodes the linear-type safety check for the fractional-
/// dataflow model. Operators SHOULD call this before choosing between a
/// copy-and-transform path and a potential in-place transform path.
///
/// Conditions for [`InPlaceJudgment::Safe`]:
/// - `flow.remaining == flow.total` (full mass present; nothing consumed yet)
/// - `flow.parent_flow_id.is_none()` and `flow.child_flow_ids.is_empty()`
///   (value is not part of a bifurcation tree)
/// - `flow.mass_ratio == (1, 1)` (unshared, full-unit mass)
/// - `value.is_uniquely_owned()` (Rc strong count == 1 for Vector/Record,
///   or inherently safe for Scalar/Nil)
///
/// The hook does NOT change what operations execute or how results are
/// stored. Its return value is advisory. Operators are expected to act on
/// `Safe` judgments once the corresponding optimization pass is activated.
#[inline]
pub(crate) fn check_in_place_candidate(
    value: &Value,
    flow: Option<&FlowToken>,
) -> InPlaceJudgment {
    let Some(flow) = flow else {
        return InPlaceJudgment::NoFlowContext;
    };
    if !flow.is_reusable_allocation() {
        return InPlaceJudgment::PartiallyConsumed;
    }
    if !value.is_uniquely_owned() {
        return InPlaceJudgment::Aliased;
    }
    InPlaceJudgment::Safe
}
