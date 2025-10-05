use criterion::{black_box, criterion_group, criterion_main, Criterion};

// This benchmark compares common algorithms:
// Since Windjammer transpiles to Rust, the runtime performance should be identical.
// This benchmark serves as a baseline reference.

fn fibonacci_recursive(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        n => fibonacci_recursive(n - 1) + fibonacci_recursive(n - 2),
    }
}

fn fibonacci_iterative(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut a = 0;
    let mut b = 1;
    for _ in 1..n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    b
}

fn sum_array(arr: &[i64]) -> i64 {
    arr.iter().sum()
}

fn filter_and_map(arr: &[i64]) -> Vec<i64> {
    arr.iter()
        .filter(|&&x| x % 2 == 0)
        .map(|&x| x * 2)
        .collect()
}

fn benchmark_fibonacci(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci");
    
    group.bench_function("recursive_n20", |b| {
        b.iter(|| fibonacci_recursive(black_box(20)));
    });
    
    group.bench_function("iterative_n1000", |b| {
        b.iter(|| fibonacci_iterative(black_box(1000)));
    });
    
    group.finish();
}

fn benchmark_array_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("array_operations");
    
    let data: Vec<i64> = (0..1000).collect();
    
    group.bench_function("sum_1000_elements", |b| {
        b.iter(|| sum_array(black_box(&data)));
    });
    
    group.bench_function("filter_and_map_1000_elements", |b| {
        b.iter(|| filter_and_map(black_box(&data)));
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_fibonacci, benchmark_array_operations);
criterion_main!(benches);
