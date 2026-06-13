//! energyProxyScore — deterministic aggregation of the VTU observation
//! counters into a single structural-cost figure.
//!
//! Honesty contract (docs/quality/energy-proxy-score.md, and the standing
//! policy in docs/dev/virtual-tensor-unit-design.md):
//!
//! - The score is a **proxy**. It counts observed structural work — data
//!   movement across the flat/nested tensor boundary, output allocation,
//!   per-operation dispatch — with fixed integer weights. It does not
//!   measure, estimate, or assert energy consumption in joules.
//! - It is purely observational: computing it never changes execution
//!   semantics, and it is a pure function of `RuntimeMetrics` — the same
//!   program and input always produce the same score.
//! - The weights are versioned. Any change to a weight, to the formula, or
//!   to which counters participate must increment [`ENERGY_PROXY_VERSION`]
//!   and update the weight table in `docs/quality/energy-proxy-score.md`.
//!
//! The score exists so that "same meaning, more structural work" becomes a
//! CI-visible regression (`energy_proxy_regression_tests.rs`) instead of an
//! anecdote.

use super::interpreter_core::RuntimeMetrics;

/// Version of the scoring formula. Bump on any weight/formula change and
/// record the change in `docs/quality/energy-proxy-score.md`.
pub const ENERGY_PROXY_VERSION: u32 = 1;

// ── Cost weights (integer, per unit) ────────────────────────────────────
// Element-granular data movement dominates: every scalar carried across the
// flat/nested boundary or freshly allocated is charged per element. Per-
// operation dispatch overhead is charged per call at a higher unit weight
// but is independent of data size.
const W_FLATTENED_ELEMENT: u64 = 4;
const W_REBUILT_ELEMENT: u64 = 4;
const W_ALLOCATED_ELEMENT: u64 = 2;
const W_FLATTEN_OP: u64 = 16;
const W_REBUILD_OP: u64 = 16;
const W_BROADCAST_OP: u64 = 8;
const W_UNARY_FLAT_OP: u64 = 8;

// ── Efficiency deductions (integer, per use) ────────────────────────────
// Paths that do the same semantic work with less data movement (SIMD lanes,
// bulk HOF kernels, index-projected broadcasts) earn a fixed per-use
// deduction, applied with saturating subtraction so the score never
// underflows. Sparse counters are *candidates*, not realized skips, so they
// deliberately earn no deduction (no credit for unrealized work).
const D_SIMD_KERNEL_USE: u64 = 4;
const D_BULK_KERNEL_USE: u64 = 8;
const D_PROJECTED_BROADCAST: u64 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyProxyReport {
    /// Deterministic weighted aggregation of the VTU counters.
    pub score: u64,
    /// [`ENERGY_PROXY_VERSION`] at the time of scoring; consumers must not
    /// compare scores across different versions.
    pub proxy_version: u32,
    /// Mechanical observations derived from the counters: structural
    /// patterns that typically admit a cheaper equivalent program. Ordered
    /// deterministically; empty when nothing noteworthy was observed.
    pub suggestions: Vec<String>,
}

/// Pure scoring function: same `RuntimeMetrics`, same score.
pub fn energy_proxy_score(metrics: &RuntimeMetrics) -> u64 {
    let cost = metrics
        .vtu_tensor_flattened_elements
        .saturating_mul(W_FLATTENED_ELEMENT)
        .saturating_add(
            metrics
                .vtu_tensor_rebuilt_elements
                .saturating_mul(W_REBUILT_ELEMENT),
        )
        .saturating_add(
            metrics
                .vtu_allocated_elements
                .saturating_mul(W_ALLOCATED_ELEMENT),
        )
        .saturating_add(
            metrics
                .vtu_tensor_flatten_count
                .saturating_mul(W_FLATTEN_OP),
        )
        .saturating_add(
            metrics
                .vtu_tensor_rebuild_count
                .saturating_mul(W_REBUILD_OP),
        )
        .saturating_add(metrics.vtu_broadcast_count.saturating_mul(W_BROADCAST_OP))
        .saturating_add(metrics.vtu_unary_flat_count.saturating_mul(W_UNARY_FLAT_OP));

    let deduction = metrics
        .vtu_simd_kernel_use_count
        .saturating_mul(D_SIMD_KERNEL_USE)
        .saturating_add(
            metrics
                .vtu_bulk_kernel_use_count
                .saturating_mul(D_BULK_KERNEL_USE),
        )
        .saturating_add(
            metrics
                .vtu_projected_broadcast_count
                .saturating_mul(D_PROJECTED_BROADCAST),
        );

    cost.saturating_sub(deduction)
}

/// Mechanical suggestions: each rule reads only the counters and states the
/// structural observation plus the program-level change that removes it.
/// Wording stays structural ("data movement", "round-trips") — never an
/// energy-outcome claim.
pub fn energy_proxy_suggestions(metrics: &RuntimeMetrics) -> Vec<String> {
    let mut out = Vec::new();

    if metrics.vtu_fusion_candidate_count > 0 {
        out.push(format!(
            "fusionCandidateCount={}: adjacent elementwise stages were classified as fusable; \
             merging them into a single block avoids intermediate flatten/rebuild round-trips",
            metrics.vtu_fusion_candidate_count
        ));
    }

    let round_trips = metrics.vtu_tensor_flatten_count + metrics.vtu_tensor_rebuild_count;
    if metrics.vtu_tensor_rebuild_count >= 2 && round_trips >= 4 {
        out.push(format!(
            "tensorFlattenCount={} / tensorRebuildCount={}: values cross the flat/nested \
             boundary repeatedly; chaining tensor words (or bulk HOF paths) keeps data flat \
             between stages",
            metrics.vtu_tensor_flatten_count, metrics.vtu_tensor_rebuild_count
        ));
    }

    if metrics.vtu_sparse_candidate_elements > 0
        && metrics.vtu_sparse_skippable_zero_elements * 2 >= metrics.vtu_sparse_candidate_elements
    {
        out.push(format!(
            "sparseSkippableZeroElements={} of sparseCandidateElements={}: half or more of the \
             candidate lanes are zero; restructuring to avoid moving zero lanes reduces data \
             movement",
            metrics.vtu_sparse_skippable_zero_elements, metrics.vtu_sparse_candidate_elements
        ));
    }

    if metrics.vtu_rejected_block_count > 0 {
        out.push(format!(
            "rejectedBlockCount={}: quantized blocks were rejected as VTU candidates; build with \
             the trace-quant feature to see the rejection reasons",
            metrics.vtu_rejected_block_count
        ));
    }

    out
}

pub fn energy_proxy_report(metrics: &RuntimeMetrics) -> EnergyProxyReport {
    EnergyProxyReport {
        score: energy_proxy_score(metrics),
        proxy_version: ENERGY_PROXY_VERSION,
        suggestions: energy_proxy_suggestions(metrics),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_metrics_score_zero_with_no_suggestions() {
        let report = energy_proxy_report(&RuntimeMetrics::default());
        assert_eq!(report.score, 0);
        assert_eq!(report.proxy_version, ENERGY_PROXY_VERSION);
        assert!(report.suggestions.is_empty());
    }

    #[test]
    fn score_is_a_pure_function_of_the_counters() {
        let metrics = RuntimeMetrics {
            vtu_tensor_flattened_elements: 100,
            vtu_tensor_flatten_count: 3,
            vtu_allocated_elements: 50,
            vtu_broadcast_count: 2,
            ..Default::default()
        };
        assert_eq!(energy_proxy_score(&metrics), energy_proxy_score(&metrics));
        // 100*4 + 3*16 + 50*2 + 2*8 = 400 + 48 + 100 + 16
        assert_eq!(energy_proxy_score(&metrics), 564);
    }

    #[test]
    fn efficiency_paths_deduct_and_never_underflow() {
        let metrics = RuntimeMetrics {
            vtu_simd_kernel_use_count: 1_000_000,
            vtu_bulk_kernel_use_count: 1_000_000,
            vtu_projected_broadcast_count: 1_000_000,
            ..Default::default()
        };
        assert_eq!(
            energy_proxy_score(&metrics),
            0,
            "deduction must saturate at zero"
        );

        let mixed = RuntimeMetrics {
            vtu_allocated_elements: 100,   // cost 200
            vtu_simd_kernel_use_count: 10, // deduction 40
            ..Default::default()
        };
        assert_eq!(energy_proxy_score(&mixed), 160);
    }

    #[test]
    fn sparse_candidates_earn_no_deduction() {
        // Candidates are observations, not realized skips: they must not
        // lower the score (no credit for unrealized work).
        let base = RuntimeMetrics {
            vtu_allocated_elements: 100,
            ..Default::default()
        };
        let with_sparse = RuntimeMetrics {
            vtu_sparse_candidate_count: 5,
            vtu_sparse_candidate_elements: 1000,
            vtu_sparse_skippable_zero_elements: 900,
            ..base
        };
        assert_eq!(energy_proxy_score(&base), energy_proxy_score(&with_sparse));
    }

    #[test]
    fn suggestion_rules_fire_mechanically() {
        let metrics = RuntimeMetrics {
            vtu_fusion_candidate_count: 2,
            vtu_tensor_flatten_count: 3,
            vtu_tensor_rebuild_count: 2,
            vtu_sparse_candidate_elements: 10,
            vtu_sparse_skippable_zero_elements: 9,
            vtu_rejected_block_count: 1,
            ..Default::default()
        };
        let suggestions = energy_proxy_suggestions(&metrics);
        assert_eq!(suggestions.len(), 4);
        assert!(suggestions[0].contains("fusionCandidateCount=2"));
        assert!(suggestions[1].contains("tensorRebuildCount=2"));
        assert!(suggestions[2].contains("sparseSkippableZeroElements=9"));
        assert!(suggestions[3].contains("rejectedBlockCount=1"));
    }
}
