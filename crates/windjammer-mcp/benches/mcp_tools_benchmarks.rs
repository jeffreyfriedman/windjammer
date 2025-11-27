//! Performance benchmarks for MCP tools
//!
//! Run with: cargo bench --package windjammer-mcp

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;
use windjammer_mcp::tools::{
    analyze_types, parse_code, refactor_extract_function, refactor_inline_variable,
    refactor_rename_symbol,
};

fn benchmark_parse_code(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("parse_code_small", |b| {
        b.to_async(&runtime).iter(|| async {
            let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
            let args = serde_json::json!({
                "code": black_box("fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}")
            });

            parse_code::handle(db, args).await.unwrap()
        });
    });

    c.bench_function("parse_code_large", |b| {
        let large_code = (0..100)
            .map(|i| format!("fn function_{}() {{ let x = {}; }}\n", i, i))
            .collect::<String>();

        b.to_async(&runtime).iter(|| async {
            let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
            let args = serde_json::json!({
                "code": black_box(&large_code)
            });

            parse_code::handle(db, args).await.unwrap()
        });
    });
}

fn benchmark_analyze_types(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("analyze_types_simple", |b| {
        b.to_async(&runtime).iter(|| async {
            let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
            let args = serde_json::json!({
                "code": black_box("fn add(a: int, b: int) -> int { a + b }")
            });

            analyze_types::handle(db, args).await.unwrap()
        });
    });
}

fn benchmark_refactoring_tools(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("extract_function", |b| {
        b.to_async(&runtime).iter(|| async {
            let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
            let args = serde_json::json!({
                "code": black_box("fn main() {\n    let x = 1;\n    let y = 2;\n    println!(\"{}\", x + y);\n}"),
                "range": {
                    "start": { "line": 1, "column": 4 },
                    "end": { "line": 2, "column": 17 }
                },
                "function_name": "calculate_sum"
            });

            refactor_extract_function::handle(db, args).await.unwrap()
        });
    });

    c.bench_function("inline_variable", |b| {
        b.to_async(&runtime).iter(|| async {
            let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
            let args = serde_json::json!({
                "code": black_box("fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}"),
                "position": { "line": 1, "column": 8 }
            });

            refactor_inline_variable::handle(db, args).await.unwrap()
        });
    });

    c.bench_function("rename_symbol", |b| {
        b.to_async(&runtime).iter(|| async {
            let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
            let args = serde_json::json!({
                "code": black_box("fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}"),
                "position": { "line": 1, "column": 8 },
                "new_name": "value"
            });

            refactor_rename_symbol::handle(db, args).await.unwrap()
        });
    });
}

criterion_group!(
    benches,
    benchmark_parse_code,
    benchmark_analyze_types,
    benchmark_refactoring_tools
);
criterion_main!(benches);
