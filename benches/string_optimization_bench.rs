/// Benchmarks for String Parameter Optimization
///
/// Measures the performance improvement of using &str instead of &String
/// for string parameters that don't need &String.
///
/// Expected results:
/// - Functions using &str: ~0% overhead (string literals pass directly)
/// - Functions using &String: ~5-10% overhead (conversion + allocation)

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// Simulates a function that only reads the string (can use &str)
fn log_str(msg: &str) {
    // Simulate some work with the string
    let _ = black_box(msg.len());
}

// Simulates a function that needs &String (e.g., passes to Vec<String>::contains)
fn check_contains_string(items: &Vec<String>, search: &String) -> bool {
    black_box(items.contains(search))
}

// Simulates the same with &str (if possible)
fn check_contains_str(items: &Vec<String>, search: &str) -> bool {
    // Note: This doesn't actually work for Vec<String>::contains in Rust
    // But shows what the performance would be if it did
    black_box(items.iter().any(|s| s.as_str() == search))
}

fn benchmark_str_vs_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_params");

    // Benchmark 1: &str parameter (no allocation)
    group.bench_function("log_with_str", |b| {
        b.iter(|| {
            log_str(black_box("Hello, World!"));
        });
    });

    // Benchmark 2: &String parameter (requires allocation)
    group.bench_function("log_with_string_conversion", |b| {
        b.iter(|| {
            let msg = black_box("Hello, World!".to_string());
            let _ = black_box(msg.len());
        });
    });

    // Benchmark 3: Vec contains with &String
    let items: Vec<String> = (0..100).map(|i| format!("item_{}", i)).collect();
    let search = "item_50".to_string();
    
    group.bench_function("contains_with_string", |b| {
        b.iter(|| {
            check_contains_string(black_box(&items), black_box(&search));
        });
    });

    // Benchmark 4: Vec contains with &str (manual iteration)
    group.bench_function("contains_with_str_workaround", |b| {
        b.iter(|| {
            check_contains_str(black_box(&items), black_box("item_50"));
        });
    });

    group.finish();
}

fn benchmark_allocation_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_overhead");

    // Measure the cost of string allocation itself
    group.bench_function("string_allocation", |b| {
        b.iter(|| {
            let _s = black_box("Hello, World!".to_string());
        });
    });

    // Measure passing a string literal (zero cost)
    group.bench_function("string_literal", |b| {
        b.iter(|| {
            let _s: &str = black_box("Hello, World!");
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_str_vs_string, benchmark_allocation_overhead);
criterion_main!(benches);
