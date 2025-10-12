//! Stress Tests for Salsa Performance
//!
//! Tests the database under heavy load to ensure
//! it remains stable and performant.

use std::time::{Duration, Instant};
use tower_lsp::lsp_types::Url;
use windjammer_lsp::database::WindjammerDatabase;

/// Test rapid consecutive edits (simulating fast typing)
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_rapid_edits() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///test.wj").unwrap();

    let start = Instant::now();

    // Simulate 1000 rapid edits
    for i in 0..1000 {
        let content = format!("fn func_{i}() {{}}\nfn helper() {{}}");
        let file = db.set_source_text(uri.clone(), content);
        let _program = db.get_program(file);
    }

    let elapsed = start.elapsed();

    println!("1000 edits took: {:?}", elapsed);
    println!("Average per edit: {:?}", elapsed / 1000);

    // Should complete in reasonable time (< 50ms total, ~50μs per edit)
    assert!(
        elapsed < Duration::from_millis(50),
        "1000 edits took too long: {:?}",
        elapsed
    );
}

/// Test very large file (10,000 lines)
#[test]
#[ignore] // Timing-sensitive, may vary by machine
#[ignore] // Timing-sensitive, may vary by machine
fn stress_large_file() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///large.wj").unwrap();

    // Generate 10,000-line file
    let mut source = String::new();
    for i in 0..10000 {
        source.push_str(&format!("fn func_{i}() {{ return {i}; }}\n"));
    }

    println!("Generated {} bytes", source.len());

    let start = Instant::now();
    let file = db.set_source_text(uri, source);
    let program = db.get_program(file);
    let first_parse = start.elapsed();

    println!("First parse of 10k lines: {:?}", first_parse);
    assert_eq!(program.items.len(), 10000);

    // Query again (should be cached)
    let start = Instant::now();
    let program2 = db.get_program(file);
    let cached = start.elapsed();

    println!("Cached query: {:?}", cached);

    // Cached should be MUCH faster
    assert!(
        cached < Duration::from_micros(100),
        "Cached query too slow: {:?}",
        cached
    );

    // Should be same pointer (memoized)
    assert!(std::ptr::eq(program, program2));
}

/// Test many files simultaneously
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_many_files() {
    let mut db = WindjammerDatabase::new();

    let start = Instant::now();

    // Create 1000 files
    for i in 0..1000 {
        let uri = Url::parse(&format!("file:///file{i}.wj")).unwrap();
        let content = format!("fn file_{i}_func() {{}}\nfn helper() {{}}");
        let file = db.set_source_text(uri, content);
        let _program = db.get_program(file);
    }

    let create_time = start.elapsed();
    println!("Created 1000 files in: {:?}", create_time);

    // Query all again (should all be cached)
    let start = Instant::now();
    for i in 0..1000 {
        let uri = Url::parse(&format!("file:///file{i}.wj")).unwrap();
        let content = format!("fn file_{i}_func() {{}}\nfn helper() {{}}");
        let file = db.set_source_text(uri, content);
        let _program = db.get_program(file);
    }
    let query_time = start.elapsed();

    println!("Queried 1000 cached files in: {:?}", query_time);
    println!("Average per file: {:?}", query_time / 1000);

    // Cached queries should be very fast (< 10ms total, ~10μs per file)
    assert!(
        query_time < Duration::from_millis(10),
        "Cached queries too slow: {:?}",
        query_time
    );
}

/// Test alternating between many versions
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_version_churn() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///test.wj").unwrap();

    let versions = vec![
        "fn version_a() {}",
        "fn version_b() {}",
        "fn version_c() {}",
        "fn version_d() {}",
        "fn version_e() {}",
    ];

    let start = Instant::now();

    // Cycle through versions many times
    for _ in 0..200 {
        for version in &versions {
            let file = db.set_source_text(uri.clone(), version.to_string());
            let _program = db.get_program(file);
        }
    }

    let elapsed = start.elapsed();
    println!("1000 version changes: {:?}", elapsed);

    // Should handle churn efficiently
    assert!(
        elapsed < Duration::from_millis(100),
        "Version churn too slow: {:?}",
        elapsed
    );
}

/// Test rapid open/close of files
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_file_churn() {
    let mut db = WindjammerDatabase::new();

    let start = Instant::now();

    // Open and "close" (just stop referencing) many files
    for round in 0..10 {
        for i in 0..100 {
            let uri = Url::parse(&format!("file:///round{round}_file{i}.wj")).unwrap();
            let content = format!("fn func_{i}() {{}}");
            let file = db.set_source_text(uri, content);
            let _program = db.get_program(file);
        }
        // Files from previous rounds are no longer referenced
        // (in real usage, they'd be GC'd)
    }

    let elapsed = start.elapsed();
    println!("1000 file opens: {:?}", elapsed);

    // Should handle file churn
    assert!(
        elapsed < Duration::from_millis(200),
        "File churn too slow: {:?}",
        elapsed
    );
}

/// Test deeply nested structures
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_complex_ast() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///complex.wj").unwrap();

    // Generate deeply nested code
    let mut source = String::from("fn main() {");
    for i in 0..100 {
        source.push_str(&format!("\n    let x{i} = {i};"));
    }
    source.push_str("\n}");

    let start = Instant::now();
    let file = db.set_source_text(uri, source);
    let program = db.get_program(file);
    let parse_time = start.elapsed();

    println!("Complex AST parse: {:?}", parse_time);
    assert_eq!(program.items.len(), 1);

    // Query again (cached)
    let start = Instant::now();
    let _program2 = db.get_program(file);
    let cached = start.elapsed();

    println!("Cached complex AST: {:?}", cached);
    assert!(cached < Duration::from_micros(100));
}

/// Test rapid keystroke simulation
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_typing_simulation() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///test.wj").unwrap();

    // Simulate typing "fn main() {}"
    let keystrokes = vec![
        "f",
        "fn",
        "fn ",
        "fn m",
        "fn ma",
        "fn mai",
        "fn main",
        "fn main(",
        "fn main()",
        "fn main() ",
        "fn main() {",
        "fn main() {}",
    ];

    let start = Instant::now();

    // Repeat typing sequence 100 times
    for _ in 0..100 {
        for keystroke in &keystrokes {
            let file = db.set_source_text(uri.clone(), keystroke.to_string());
            let _program = db.get_program(file);
        }
    }

    let elapsed = start.elapsed();
    println!("1200 keystroke edits: {:?}", elapsed);
    println!("Per keystroke: {:?}", elapsed / 1200);

    // Should handle rapid typing (< 60ms total, ~50μs per keystroke)
    assert!(
        elapsed < Duration::from_millis(60),
        "Typing simulation too slow: {:?}",
        elapsed
    );
}

/// Test concurrent-like access (sequential but rapid)
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_rapid_file_switching() {
    let mut db = WindjammerDatabase::new();

    // Create 10 files
    let files: Vec<_> = (0..10)
        .map(|i| {
            let uri = Url::parse(&format!("file:///file{i}.wj")).unwrap();
            let content = format!("fn func_{i}() {{}}");
            db.set_source_text(uri, content)
        })
        .collect();

    // Parse all once
    for &file in &files {
        let _program = db.get_program(file);
    }

    let start = Instant::now();

    // Rapidly switch between files (like rapid tab switching)
    for _ in 0..1000 {
        for &file in &files {
            let _program = db.get_program(file);
        }
    }

    let elapsed = start.elapsed();
    println!("10,000 cached file queries: {:?}", elapsed);
    println!("Per query: {:?}", elapsed / 10000);

    // Should be extremely fast (< 5ms total, ~500ns per query)
    assert!(
        elapsed < Duration::from_millis(5),
        "Rapid file switching too slow: {:?}",
        elapsed
    );
}

/// Test memory stability under load
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_memory_stability() {
    let mut db = WindjammerDatabase::new();

    // Create many large files
    for round in 0..10 {
        for i in 0..100 {
            let uri = Url::parse(&format!("file:///round{round}_file{i}.wj")).unwrap();

            // Each file has 100 functions
            let mut content = String::new();
            for j in 0..100 {
                content.push_str(&format!("fn func_{i}_{j}() {{ return {j}; }}\n"));
            }

            let file = db.set_source_text(uri, content);
            let _program = db.get_program(file);
        }
    }

    // Test passes if no OOM
    // 10 rounds × 100 files × 100 functions = 100,000 functions parsed
    println!("Successfully handled 100,000 function definitions");
}

/// Test incremental edit performance
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_incremental_edits() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///test.wj").unwrap();

    // Start with base content
    let mut content = "fn func_0() {}\n".to_string();

    let start = Instant::now();

    // Add one function at a time (500 functions)
    for i in 1..500 {
        content.push_str(&format!("fn func_{i}() {{}}\n"));
        let file = db.set_source_text(uri.clone(), content.clone());
        let program = db.get_program(file);
        assert_eq!(program.items.len(), i + 1);
    }

    let elapsed = start.elapsed();
    println!("500 incremental edits: {:?}", elapsed);
    println!("Per edit: {:?}", elapsed / 500);

    // Currently re-parses entire file, but should still be fast
    assert!(
        elapsed < Duration::from_millis(250),
        "Incremental edits too slow: {:?}",
        elapsed
    );
}

/// Test cache effectiveness
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_cache_hit_rate() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///test.wj").unwrap();

    let content = "fn main() {}".to_string();
    let file = db.set_source_text(uri, content);

    // First parse
    let start = Instant::now();
    let _program = db.get_program(file);
    let first = start.elapsed();

    // Cached queries
    let start = Instant::now();
    for _ in 0..10000 {
        let _program = db.get_program(file);
    }
    let cached = start.elapsed();

    let avg_cached = cached / 10000;

    println!("First parse: {:?}", first);
    println!("10,000 cached: {:?}", cached);
    println!("Average cached: {:?}", avg_cached);

    // Cached should be at least 100x faster
    let speedup = first.as_nanos() / avg_cached.as_nanos();
    println!("Speedup: {}x", speedup);

    assert!(speedup >= 100, "Cache not effective enough: {}x", speedup);
}

/// Test error recovery under stress
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_error_recovery() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///test.wj").unwrap();

    let invalid_sources = vec![
        "fn }}}",
        "struct {{{",
        "impl }}",
        "fn main(",
        "let x = ",
        "if {",
        "while }",
        "}}}}}}}}",
    ];

    // Should handle all errors gracefully
    for (i, invalid) in invalid_sources.iter().enumerate() {
        let file = db.set_source_text(uri.clone(), invalid.to_string());
        let program = db.get_program(file);
        // Should return empty program, not panic
        println!("Error case {}: {} items", i, program.items.len());
    }
}

/// Test unicode stress
#[test]
#[ignore] // Timing-sensitive, may vary by machine
fn stress_unicode_heavy() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///unicode.wj").unwrap();

    // Generate file with lots of unicode
    let mut content = String::new();
    for i in 0..100 {
        content.push_str(&format!(
            "fn func_{i}() {{ println(\"🎉 Hello, 世界 {i} αβγ\"); }}\n"
        ));
    }

    let start = Instant::now();
    let file = db.set_source_text(uri, content);
    let program = db.get_program(file);
    let elapsed = start.elapsed();

    println!("Unicode parse: {:?}", elapsed);
    assert_eq!(program.items.len(), 100);

    // Should handle unicode efficiently
    assert!(
        elapsed < Duration::from_millis(10),
        "Unicode parsing too slow: {:?}",
        elapsed
    );
}
