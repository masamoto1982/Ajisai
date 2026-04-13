use crate::interpreter::Interpreter;

fn run_code(code: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        interp.execute(code).await.expect("code should execute");
    });
    interp
}

#[test]
fn bench_user_word_repeated() {
    let code = "{ [ 1 ] + } 'INC' DEF [ 1 ] INC [ 1 ] INC [ 1 ] INC [ 1 ] INC";
    let interp = run_code(code);
    assert!(interp.runtime_metrics().compiled_plan_build_count >= 1);
}

#[test]
fn bench_map_increment() {
    let code = "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 1 ] + } MAP";
    let interp = run_code(code);
    assert!(interp.runtime_metrics().quantized_block_use_count >= 1);
}

#[test]
fn bench_filter_positive() {
    let _interp = run_code("[ -2 -1 0 1 2 3 ] { [ 0 ] < NOT } FILTER");
}

#[test]
fn bench_fold_sum() {
    let _interp = run_code("[ 1 2 3 4 5 ] [ 0 ] { + } FOLD");
}

#[test]
fn bench_redef_invalidation() {
    let code = "{ [ 1 ] + } 'INC' DEF [ 1 ] INC { [ 2 ] + } 'INC' DEF [ 1 ] INC";
    let interp = run_code(code);
    assert!(interp.runtime_metrics().compiled_plan_cache_miss_count >= 1);
}

#[test]
fn bench_child_runtime_restart() {
    let _interp = run_code("{ [ 1 ] } SPAWN AWAIT");
}
