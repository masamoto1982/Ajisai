use crate::interpreter::{Interpreter, RuntimeMetrics};
use std::time::{Duration, Instant};

/// Upper bound for total loop time.  Each iteration rebuilds a tokio runtime
/// and a fresh `Interpreter`, which dominates execution cost — so this is a
/// generous ceiling meant to catch catastrophic regressions (e.g. fallback
/// disabling the quantized path), not to gate on micro-benchmark variance.
const PERF_LOOP_SOFT_LIMIT: Duration = Duration::from_secs(60);

fn run_code(code: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        interp.execute(code).await.expect("code should execute");
    });
    interp
}

fn run_loop(
    label: &str,
    iterations: usize,
    expected_quant_calls_per_iter: u64,
    code: &str,
) -> (Duration, RuntimeMetrics) {
    // Warm up a single interpreter so plan/quantized caches can be reused
    // across iterations — this is what JIT-style caches are designed for,
    // and running a fresh interpreter per iteration would make hit rates
    // meaningless.
    let mut interp = Interpreter::new();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    // One warm-up run to populate caches; not timed.
    rt.block_on(async {
        interp.execute(code).await.expect("warm-up execution");
    });
    let baseline = interp.runtime_metrics();

    let start = Instant::now();
    for _ in 0..iterations {
        rt.block_on(async {
            interp.execute(code).await.expect("code should execute");
        });
    }
    let elapsed = start.elapsed();

    // Delta metrics from after-warmup to end-of-loop.
    let final_m = interp.runtime_metrics();
    let delta = RuntimeMetrics {
        compiled_plan_build_count: final_m
            .compiled_plan_build_count
            .saturating_sub(baseline.compiled_plan_build_count),
        compiled_plan_cache_hit_count: final_m
            .compiled_plan_cache_hit_count
            .saturating_sub(baseline.compiled_plan_cache_hit_count),
        compiled_plan_cache_miss_count: final_m
            .compiled_plan_cache_miss_count
            .saturating_sub(baseline.compiled_plan_cache_miss_count),
        quantized_block_build_count: final_m
            .quantized_block_build_count
            .saturating_sub(baseline.quantized_block_build_count),
        quantized_block_use_count: final_m
            .quantized_block_use_count
            .saturating_sub(baseline.quantized_block_use_count),
        ..Default::default()
    };

    let plan_total =
        delta.compiled_plan_cache_hit_count + delta.compiled_plan_cache_miss_count;
    let hit_rate = if plan_total > 0 {
        (delta.compiled_plan_cache_hit_count as f64 / plan_total as f64) * 100.0
    } else {
        0.0
    };
    let expected_total_quant = (iterations as u64) * expected_quant_calls_per_iter.max(1);
    let quant_rate = (delta.quantized_block_use_count as f64 / expected_total_quant as f64)
        * 100.0;

    println!(
        "[perf] {label} x{iterations}: {:.1}ms (quantized: {:.1}%, plan hit rate: {:.1}%, hits: {}/{})",
        elapsed.as_secs_f64() * 1000.0,
        quant_rate,
        hit_rate,
        delta.compiled_plan_cache_hit_count,
        plan_total,
    );

    #[cfg(feature = "trace-compile")]
    eprintln!(
        "[metrics] plan_build={} plan_hit={} plan_miss={}",
        delta.compiled_plan_build_count,
        delta.compiled_plan_cache_hit_count,
        delta.compiled_plan_cache_miss_count
    );

    #[cfg(feature = "trace-compile")]
    eprintln!(
        "[metrics] quant_build={} quant_use={}",
        delta.quantized_block_build_count,
        delta.quantized_block_use_count
    );

    (elapsed, delta)
}

#[test]
fn perf_filter_map_fold_reports_metrics() {
    let (filter_elapsed, filter_metrics) = run_loop(
        "FILTER",
        1000,
        11,
        "[ -5 -4 -3 -2 -1 0 1 2 3 4 5 ] { [ 0 ] <= NOT } FILTER",
    );
    let (map_elapsed, map_metrics) = run_loop(
        "MAP",
        1000,
        10,
        "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 1 ] + } MAP",
    );
    let (fold_elapsed, fold_metrics) = run_loop(
        "FOLD",
        500,
        10,
        "[ 1 2 3 4 5 6 7 8 9 10 ] [ 0 ] { + } FOLD",
    );

    assert!(filter_elapsed < PERF_LOOP_SOFT_LIMIT);
    assert!(map_elapsed < PERF_LOOP_SOFT_LIMIT);
    assert!(fold_elapsed < PERF_LOOP_SOFT_LIMIT);

    assert!(filter_metrics.quantized_block_use_count >= 1);
    assert!(map_metrics.quantized_block_use_count >= 1);
    assert!(fold_metrics.quantized_block_use_count >= 1);

    // With a reused interpreter, quantized kernel must be reused across iterations.
    assert!(
        filter_metrics.quantized_block_use_count >= 1000,
        "expected quantized kernel reuse across iterations, got {}",
        filter_metrics.quantized_block_use_count
    );
}

#[test]
fn perf_scan_any_all_count_reports_quantized_usage() {
    let (_scan_elapsed, scan_metrics) = run_loop("SCAN", 500, 5, "[ 1 2 3 4 5 ] [ 0 ] { + } SCAN");
    let (_any_elapsed, any_metrics) = run_loop("ANY", 1000, 3, "[ 1 2 3 4 5 ] { [ 3 ] = } ANY");
    let (_all_elapsed, all_metrics) = run_loop("ALL", 1000, 5, "[ 1 2 3 4 5 ] { [ 0 ] <= NOT } ALL");
    let (_count_elapsed, count_metrics) = run_loop(
        "COUNT",
        1000,
        10,
        "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 5 ] <= NOT } COUNT",
    );

    assert!(scan_metrics.quantized_block_use_count >= 1);
    assert!(any_metrics.quantized_block_use_count >= 1);
    assert!(all_metrics.quantized_block_use_count >= 1);
    assert!(count_metrics.quantized_block_use_count >= 1);
}

#[test]
fn perf_redefinition_still_invalidates_plan() {
    let interp = run_code("{ [ 1 ] + } 'INC' DEF [ 1 ] INC { [ 2 ] + } 'INC' DEF [ 1 ] INC");
    let m = interp.runtime_metrics();
    assert!(m.compiled_plan_cache_miss_count >= 1);
}
