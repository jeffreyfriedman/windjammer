//! Performance Regression Benchmarks
//!
//! This benchmark suite tracks performance metrics over time to detect regressions.
//! Run these benchmarks before and after major changes to ensure performance is maintained.
//!
//! ## Usage
//!
//! ```bash
//! # Baseline (before changes)
//! cargo bench --bench regression_benchmarks -- --save-baseline master
//!
//! # After changes
//! cargo bench --bench regression_benchmarks -- --baseline master
//! ```
//!
//! ## Metrics Tracked
//!
//! - Lexer throughput (tokens/sec)
//! - Parser throughput (AST nodes/sec)
//! - Codegen throughput (lines/sec)
//! - End-to-end compilation time
//! - Memory usage

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::codegen::CodeGenerator;
use windjammer::analyzer::SignatureRegistry;
use windjammer::CompilationTarget;

const SMALL_PROGRAM: &str = r#"
fn add(x: int, y: int) -> int {
    x + y
}
"#;

const MEDIUM_PROGRAM: &str = r#"
fn fibonacci(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn factorial(n: int) -> int {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

fn main() {
    let fib = fibonacci(10)
    let fact = factorial(5)
    println!("fib={}, fact={}", fib, fact)
}
"#;

/// Benchmark lexer performance
fn bench_lexer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer_throughput");
    
    for (name, source) in &[("small", SMALL_PROGRAM), ("medium", MEDIUM_PROGRAM)] {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), source, |b, &source| {
            b.iter(|| {
                let mut lexer = Lexer::new(source);
                black_box(lexer.tokenize());
            });
        });
    }
    
    group.finish();
}

/// Benchmark parser performance
fn bench_parser_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_throughput");
    
    for (name, source) in &[("small", SMALL_PROGRAM), ("medium", MEDIUM_PROGRAM)] {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        
        group.throughput(Throughput::Elements(tokens.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &tokens, |b, tokens| {
            b.iter(|| {
                let mut parser = Parser::new(tokens.clone());
                black_box(parser.parse());
            });
        });
    }
    
    group.finish();
}

/// Benchmark code generator performance
fn bench_codegen_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("codegen_throughput");
    
    for (name, source) in &[("small", SMALL_PROGRAM), ("medium", MEDIUM_PROGRAM)] {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        
        if let Ok(program) = parser.parse() {
            let items = program.items.len();
            group.throughput(Throughput::Elements(items as u64));
            group.bench_with_input(BenchmarkId::from_parameter(name), &program, |b, program| {
                b.iter(|| {
                    let signatures = SignatureRegistry::new();
                    let mut generator = CodeGenerator::new_for_module(
                        signatures,
                        CompilationTarget::Wasm
                    );
                    black_box(generator.generate_program(program, &[]));
                });
            });
        }
    }
    
    group.finish();
}

/// Benchmark end-to-end compilation
fn bench_e2e_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");
    
    for (name, source) in &[("small", SMALL_PROGRAM), ("medium", MEDIUM_PROGRAM)] {
        group.bench_with_input(BenchmarkId::from_parameter(name), source, |b, &source| {
            b.iter(|| {
                let mut lexer = Lexer::new(source);
                let tokens = lexer.tokenize();
                
                let mut parser = Parser::new(tokens);
                if let Ok(program) = parser.parse() {
                    let signatures = SignatureRegistry::new();
                    let mut generator = CodeGenerator::new_for_module(
                        signatures,
                        CompilationTarget::Wasm
                    );
                    black_box(generator.generate_program(&program, &[]));
                }
            });
        });
    }
    
    group.finish();
}

/// Benchmark scaling with program size
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");
    
    for size in &[10, 50, 100, 500] {
        let source = generate_n_functions(*size);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| {
                let mut lexer = Lexer::new(source);
                let tokens = lexer.tokenize();
                let mut parser = Parser::new(tokens);
                black_box(parser.parse());
            });
        });
    }
    
    group.finish();
}

fn generate_n_functions(n: usize) -> String {
    let mut code = String::new();
    for i in 0..n {
        code.push_str(&format!(
            "fn func_{}(x: int) -> int {{\n    x + {}\n}}\n\n",
            i, i
        ));
    }
    code
}

criterion_group!(
    benches,
    bench_lexer_throughput,
    bench_parser_throughput,
    bench_codegen_throughput,
    bench_e2e_compilation,
    bench_scaling,
);
criterion_main!(benches);

