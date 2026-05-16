//! Parsing `cargo test` output and optional coverage reporting.

use anyhow::Result;
use colored::*;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use super::util::copy_dir_recursive;

#[derive(Default)]
pub(crate) struct TestResults {
    pub(crate) passed: usize,
    pub(crate) failed: usize,
    pub(crate) ignored: usize,
    pub(crate) individual_results: HashMap<String, String>, // test_name -> status
}

pub(crate) fn parse_test_output(stdout: &str, _stderr: &str) -> TestResults {
    let mut results = TestResults::default();

    // Parse individual test results
    for line in stdout.lines() {
        let line = line.trim();

        // Parse individual test lines: "test module::test_name ... ok"
        if line.starts_with("test ")
            && (line.contains(" ... ok")
                || line.contains(" ... FAILED")
                || line.contains(" ... ignored"))
        {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[0] == "test" {
                let test_name = parts[1].to_string();
                let status = if line.contains(" ... ok") {
                    "passed"
                } else if line.contains(" ... FAILED") {
                    "failed"
                } else if line.contains(" ... ignored") {
                    "ignored"
                } else {
                    "unknown"
                };
                results
                    .individual_results
                    .insert(test_name, status.to_string());
            }
        }

        // Parse summary line for aggregate counts
        if line.contains("test result:") {
            // Example: "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if part == &"passed;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        results.passed += n; // Sum instead of replace
                    }
                }
                if part == &"failed;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        results.failed += n; // Sum instead of replace
                    }
                }
                if part == &"ignored;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        results.ignored += n; // Sum instead of replace
                    }
                }
            }
        }
    }

    results
}

/// Generate coverage report using cargo-llvm-cov
pub(crate) fn generate_coverage_report(test_dir: &Path) -> Result<()> {
    // Check if cargo-llvm-cov is installed
    let check = Command::new("cargo")
        .arg("llvm-cov")
        .arg("--version")
        .output();

    if check.is_err() || !check.unwrap().status.success() {
        println!("{} cargo-llvm-cov not found", "⚠".yellow());
        println!("Install with: cargo install cargo-llvm-cov");
        println!("Skipping coverage report...");
        return Ok(());
    }

    // Generate coverage
    let output = Command::new("cargo")
        .arg("llvm-cov")
        .arg("test")
        .arg("--html")
        .current_dir(test_dir)
        .output()?;

    if output.status.success() {
        // Copy coverage report to project directory
        let source_dir = test_dir.join("target/llvm-cov");
        let dest_dir = std::path::Path::new("target/llvm-cov");

        if source_dir.exists() {
            // Create destination directory
            std::fs::create_dir_all(dest_dir)?;

            // Copy the coverage report
            if let Err(e) = copy_dir_recursive(&source_dir, dest_dir) {
                println!("{} Failed to copy coverage report: {}", "⚠".yellow(), e);
            } else {
                println!("{} Coverage report generated", "✓".green());
                println!("  Open: target/llvm-cov/html/index.html");
            }
        } else {
            println!(
                "{} Coverage report not found at expected location",
                "⚠".yellow()
            );
        }
    } else {
        println!("{} Coverage generation failed", "✗".red());
        print!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}
