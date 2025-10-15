//! Benchmark for Salsa Incremental Compilation
//!
//! Measures compilation speed with and without caching:
//! - Cold compilation (first time)
//! - Hot compilation (no changes)
//! - Incremental compilation (single function changed)
//!
//! Expected results:
//! - Hot builds: < 100ms
//! - Incremental builds: 10-20x faster than cold builds

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use windjammer::compiler_database::*;

const SAMPLE_CODE: &str = r#"
fn factorial(n: int) -> int {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

fn fibonacci(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn sum_range(start: int, end: int) -> int {
    let mut total = 0
    for i in start..end {
        total += i
    }
    total
}

fn main() {
    let fact = factorial(10)
    let fib = fibonacci(10)
    let sum = sum_range(1, 100)
    println!("Factorial: {}", fact)
    println!("Fibonacci: {}", fib)
    println!("Sum: {}", sum)
}
"#;

const SAMPLE_CODE_MODIFIED: &str = r#"
fn factorial(n: int) -> int {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

fn fibonacci(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn sum_range(start: int, end: int) -> int {
    let mut total = 0
    for i in start..end {
        total += i * 2  // CHANGED: multiply by 2
    }
    total
}

fn main() {
    let fact = factorial(10)
    let fib = fibonacci(10)
    let sum = sum_range(1, 100)
    println!("Factorial: {}", fact)
    println!("Fibonacci: {}", fib)
    println!("Sum: {}", sum)
}
"#;

fn bench_cold_compilation(c: &mut Criterion) {
    c.bench_function("cold_compilation", |b| {
        b.iter(|| {
            // Create fresh database for each iteration
            let db = CompilerDatabase::new();
            
            // Create source input
            let input = SourceInput::new(&db, "test.wj".into(), SAMPLE_CODE.to_string());
            
            // Full compilation pipeline
            let tokens = tokenize(&db, input);
            let parsed = parse_tokens(&db, tokens);
            let typed = analyze_types(&db, parsed);
            let optimized = optimize_program(&db, typed);
            let rust_code = generate_rust(&db, optimized);
            
            black_box(rust_code.code(&db));
        });
    });
}

fn bench_hot_compilation(c: &mut Criterion) {
    c.bench_function("hot_compilation_no_changes", |b| {
        // Create database once
        let db = CompilerDatabase::new();
        let input = SourceInput::new(&db, "test.wj".into(), SAMPLE_CODE.to_string());
        
        // Warm up the cache
        let tokens = tokenize(&db, input);
        let parsed = parse_tokens(&db, tokens);
        let typed = analyze_types(&db, parsed);
        let optimized = optimize_program(&db, typed);
        let _rust_code = generate_rust(&db, optimized);
        
        b.iter(|| {
            // Recompile with same input (should hit cache)
            let tokens = tokenize(&db, input);
            let parsed = parse_tokens(&db, tokens);
            let typed = analyze_types(&db, parsed);
            let optimized = optimize_program(&db, typed);
            let rust_code = generate_rust(&db, optimized);
            
            black_box(rust_code.code(&db));
        });
    });
}

fn bench_incremental_compilation(c: &mut Criterion) {
    c.bench_function("incremental_compilation_one_function_changed", |b| {
        b.iter_batched(
            || {
                // Setup: compile original version
                let db = CompilerDatabase::new();
                let input = SourceInput::new(&db, "test.wj".into(), SAMPLE_CODE.to_string());
                
                let tokens = tokenize(&db, input);
                let parsed = parse_tokens(&db, tokens);
                let typed = analyze_types(&db, parsed);
                let optimized = optimize_program(&db, typed);
                let _rust_code = generate_rust(&db, optimized);
                
                db
            },
            |db| {
                // Benchmark: recompile with modified code
                let input_modified =
                    SourceInput::new(&db, "test.wj".into(), SAMPLE_CODE_MODIFIED.to_string());
                
                let tokens = tokenize(&db, input_modified);
                let parsed = parse_tokens(&db, tokens);
                let typed = analyze_types(&db, parsed);
                let optimized = optimize_program(&db, typed);
                let rust_code = generate_rust(&db, optimized);
                
                black_box(rust_code.code(&db));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("compilation_pipeline");
    
    group.bench_function("with_salsa", |b| {
        let db = CompilerDatabase::new();
        let input = SourceInput::new(&db, "test.wj".into(), SAMPLE_CODE.to_string());
        
        b.iter(|| {
            let tokens = tokenize(&db, input);
            let parsed = parse_tokens(&db, tokens);
            
            black_box(parsed.program(&db));
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_cold_compilation,
    bench_hot_compilation,
    bench_incremental_compilation,
    bench_full_pipeline
);
criterion_main!(benches);

