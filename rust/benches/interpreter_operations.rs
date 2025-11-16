// benches/interpreter_operations.rs
//
// インタプリタ操作のベンチマーク
// 実際のコード実行のパフォーマンスを測定

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ajisai_core::AjisaiInterpreter;
use tokio::runtime::Runtime;

/// ベンチマーク1: ベクタ操作 - GET
fn bench_vector_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("vector_get", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                interp.execute("[1 2 3 4 5 6 7 8 9 10]").await.ok();
                black_box(interp.execute("[5] GET").await).ok();
            });
        });
    });
}

/// ベンチマーク2: ベクタ操作 - CONCAT
fn bench_vector_concat(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("vector_concat", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                interp.execute("[1 2 3 4 5]").await.ok();
                interp.execute("[6 7 8 9 10]").await.ok();
                black_box(interp.execute("CONCAT").await).ok();
            });
        });
    });
}

/// ベンチマーク3: 算術演算 - 加算
fn bench_arithmetic_add(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("arithmetic_add");

    // 要素ごと加算
    group.bench_function("elementwise", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                interp.execute("[1 2 3 4 5]").await.ok();
                interp.execute("[10 20 30 40 50]").await.ok();
                black_box(interp.execute("+").await).ok();
            });
        });
    });

    // ブロードキャスト
    group.bench_function("broadcast", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                interp.execute("[1 2 3 4 5]").await.ok();
                interp.execute("[10]").await.ok();
                black_box(interp.execute("+").await).ok();
            });
        });
    });

    group.finish();
}

/// ベンチマーク4: 高階関数 - MAP
fn bench_map(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("map", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                interp.execute("[ '[2] *' ] 'DOUBLE' DEF").await.ok();
                interp.execute("[1 2 3 4 5 6 7 8 9 10]").await.ok();
                black_box(interp.execute("'DOUBLE' MAP").await).ok();
            });
        });
    });
}

/// ベンチマーク5: カスタムワード実行
fn bench_custom_word(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("custom_word_execution", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                interp.execute("[ '[2] * [1] +' ] 'CALC' DEF").await.ok();
                interp.execute("[5]").await.ok();
                black_box(interp.execute("CALC").await).ok();
            });
        });
    });
}

criterion_group!(
    benches,
    bench_vector_get,
    bench_vector_concat,
    bench_arithmetic_add,
    bench_map,
    bench_custom_word
);
criterion_main!(benches);
