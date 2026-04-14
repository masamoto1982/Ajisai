/// Perf-regression smoke tests.
///
/// These tests verify that the quantized execution path is actually taken for
/// the key higher-order functions, and that the cache invalidation machinery
/// works correctly after re-definitions.
///
/// They also print lightweight timing/metrics summaries when run with
/// `-- --nocapture` so you can see hit rates and quantized-usage counts
/// without a full bench harness.
use crate::interpreter::Interpreter;

fn run_code(code: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        interp.execute(code).await.expect("code should execute");
    });
    interp
}

fn run_code_timed(code: &str) -> (Interpreter, std::time::Duration) {
    let mut interp = Interpreter::new();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let start = std::time::Instant::now();
    rt.block_on(async {
        interp.execute(code).await.expect("code should execute");
    });
    let elapsed = start.elapsed();
    (interp, elapsed)
}

fn print_summary(label: &str, interp: &Interpreter, elapsed: std::time::Duration) {
    let m = interp.runtime_metrics();
    let total_plan = m.compiled_plan_cache_hit_count + m.compiled_plan_cache_miss_count;
    let hit_rate = if total_plan > 0 {
        m.compiled_plan_cache_hit_count as f64 / total_plan as f64 * 100.0
    } else {
        0.0
    };
    let total_quant = m.quantized_block_build_count;
    let quant_rate = if total_quant > 0 {
        m.quantized_block_use_count as f64 / total_quant as f64 * 100.0
    } else {
        0.0
    };
    println!(
        "[BENCH] {} | elapsed={:?} | plan_hit_rate={:.1}% | quant_rate={:.1}%",
        label, elapsed, hit_rate, quant_rate
    );
}

// ---------------------------------------------------------------------------
// CompiledPlan cache smoke tests
// ---------------------------------------------------------------------------

#[test]
fn bench_user_word_repeated() {
    let code = "{ [ 1 ] + } 'INC' DEF [ 1 ] INC [ 1 ] INC [ 1 ] INC [ 1 ] INC";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("user_word_repeated", &interp, elapsed);
    assert!(m.compiled_plan_build_count >= 1, "plan should be compiled at least once");
    assert!(m.compiled_plan_cache_hit_count >= 3, "3 of 4 calls should be cache hits");
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "user_word_repeated took too long: {:?}",
        elapsed
    );
}

#[test]
fn bench_redef_invalidates_plan() {
    let code = "{ [ 1 ] + } 'INC' DEF [ 1 ] INC { [ 2 ] + } 'INC' DEF [ 1 ] INC";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("redef_invalidation", &interp, elapsed);
    assert!(m.compiled_plan_cache_miss_count >= 1, "cache miss expected after redef");
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "redef_invalidates_plan took too long: {:?}",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// MAP — quantized path
// ---------------------------------------------------------------------------

#[test]
fn bench_map_increment() {
    let code = "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 1 ] + } MAP";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("map_increment", &interp, elapsed);
    assert!(m.quantized_block_use_count >= 10, "all 10 elements should use quantized kernel");
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "map_increment took too long: {:?}",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// FILTER — quantized predicate path
// ---------------------------------------------------------------------------

#[test]
fn bench_filter_positive() {
    let code = "[ -5 -4 -3 -2 -1 0 1 2 3 4 5 ] { [ 0 ] <= NOT } FILTER";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("filter_positive", &interp, elapsed);
    assert!(m.quantized_block_use_count >= 1, "FILTER should use quantized predicate kernel");
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "filter_positive took too long: {:?}",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// ANY / ALL / COUNT — quantized predicate path
// ---------------------------------------------------------------------------

#[test]
fn bench_any_quantized() {
    let code = "[ 1 2 3 4 5 ] { [ 3 ] = } ANY";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("any_quantized", &interp, elapsed);
    assert!(m.quantized_block_use_count >= 1, "ANY should use quantized predicate kernel");
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "any_quantized took too long: {:?}",
        elapsed
    );
}

#[test]
fn bench_all_quantized() {
    let code = "[ 1 2 3 4 5 ] { [ 0 ] <= NOT } ALL";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("all_quantized", &interp, elapsed);
    assert!(m.quantized_block_use_count >= 1, "ALL should use quantized predicate kernel");
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "all_quantized took too long: {:?}",
        elapsed
    );
}

#[test]
fn bench_count_quantized() {
    // { [ 5 ] <= NOT } = elem > 5
    let code = "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 5 ] <= NOT } COUNT";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("count_quantized", &interp, elapsed);
    assert!(m.quantized_block_use_count >= 1, "COUNT should use quantized predicate kernel");
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "count_quantized took too long: {:?}",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// FOLD / SCAN — quantized fold path
// ---------------------------------------------------------------------------

#[test]
fn bench_fold_sum() {
    let code = "[ 1 2 3 4 5 6 7 8 9 10 ] [ 0 ] { + } FOLD";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("fold_sum", &interp, elapsed);
    assert!(m.quantized_block_use_count >= 1, "FOLD should use quantized fold kernel");
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "fold_sum took too long: {:?}",
        elapsed
    );
}

#[test]
fn bench_scan_prefix_sums() {
    let code = "[ 1 2 3 4 5 ] [ 0 ] { + } SCAN";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("scan_prefix_sums", &interp, elapsed);
    assert!(m.quantized_block_use_count >= 1, "SCAN should use quantized fold kernel");
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "scan_prefix_sums took too long: {:?}",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// Comprehensive hit-rate check
// ---------------------------------------------------------------------------

/// Run the same user-word many times and verify the cache hit rate is high.
#[test]
fn bench_high_hit_rate() {
    // 8 calls after 1 definition → expect ≥ 7 cache hits (first call builds).
    let code = "{ [ 1 ] + } 'INC' DEF \
                [ 1 ] INC [ 1 ] INC [ 1 ] INC [ 1 ] INC \
                [ 1 ] INC [ 1 ] INC [ 1 ] INC [ 1 ] INC";
    let (interp, elapsed) = run_code_timed(code);
    let m = interp.runtime_metrics();
    print_summary("high_hit_rate", &interp, elapsed);
    let total = m.compiled_plan_cache_hit_count + m.compiled_plan_cache_miss_count;
    assert!(total >= 8, "should have at least 8 plan lookups");
    assert!(
        m.compiled_plan_cache_hit_count >= 7,
        "hit rate should be ≥ 7/8 after warm-up, got hits={}",
        m.compiled_plan_cache_hit_count
    );
    assert!(
        elapsed < std::time::Duration::from_millis(200),
        "high_hit_rate took too long: {:?}",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// child runtime
// ---------------------------------------------------------------------------

#[test]
fn bench_child_runtime_restart() {
    let _interp = run_code("{ [ 1 ] } SPAWN AWAIT");
}
