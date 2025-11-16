// benches/value_operations.rs
//
// 値操作のベンチマーク
// 型システムの過剰なラッピングによるパフォーマンスへの影響を測定

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ajisai_core::AjisaiInterpreter;
use tokio::runtime::Runtime;

/// ベンチマーク1: 数値の作成とスタックへのプッシュ
fn bench_number_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("number_creation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                black_box(interp.execute("[42]").await).ok();
            });
        });
    });
}

/// ベンチマーク2: ベクタの作成
fn bench_vector_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("vector_creation");

    for size in [1, 10, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let mut interp = AjisaiInterpreter::new();
                    let code = format!("[{}]", (1..=size).map(|n| n.to_string()).collect::<Vec<_>>().join(" "));
                    black_box(interp.execute(&code).await).ok();
                });
            });
        });
    }
    group.finish();
}

/// ベンチマーク3: 文字列の作成
fn bench_string_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("string_creation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                black_box(interp.execute("['hello']").await).ok();
            });
        });
    });
}

/// ベンチマーク4: 複数の値の作成（混在）
fn bench_mixed_values(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("mixed_values", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                black_box(interp.execute("[42] ['text'] [true] [false]").await).ok();
            });
        });
    });
}

/// ベンチマーク5: ネストされたベクタ
fn bench_nested_vectors(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("nested_vectors");

    for depth in [1, 3, 5].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, &depth| {
            b.iter(|| {
                rt.block_on(async {
                    let mut interp = AjisaiInterpreter::new();
                    let mut code = String::from("1");
                    for _ in 0..depth {
                        code = format!("[{}]", code);
                    }
                    black_box(interp.execute(&code).await).ok();
                });
            });
        });
    }
    group.finish();
}

/// ベンチマーク6: スタック操作のオーバーヘッド
fn bench_stack_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("stack_push_pop", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut interp = AjisaiInterpreter::new();
                for i in 0..50 {
                    black_box(interp.execute(&format!("[{}]", i)).await).ok();
                }
            });
        });
    });
}

criterion_group!(
    benches,
    bench_number_creation,
    bench_vector_creation,
    bench_string_creation,
    bench_mixed_values,
    bench_nested_vectors,
    bench_stack_operations
);
criterion_main!(benches);
