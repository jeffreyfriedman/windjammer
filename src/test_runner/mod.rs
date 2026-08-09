//! Test runner module for Windjammer test framework
//!
//! This module provides comprehensive test running infrastructure including:
//! - Test file discovery and compilation
//! - Test execution with parallel/sequential modes
//! - JSON output for CI integration
//! - Coverage report generation

use anyhow::Result;
use std::path::Path;

mod layout_detect;
mod test_discovery;
pub mod test_execution;
mod test_reporting;
mod util;
mod options;

pub use options::TestRunOptions;
pub use test_execution::rewrite_test_crate_imports;
pub use util::{copy_dir_recursive, path_to_toml_string};

pub fn run_tests(
    path: Option<&Path>,
    filter: Option<&str>,
    nocapture: bool,
    parallel: bool,
    json: bool,
) -> Result<()> {
    run_tests_with_options(TestRunOptions::from_legacy(
        path, filter, nocapture, parallel, json,
    ))
}

pub fn run_tests_with_options(mut opts: TestRunOptions) -> Result<()> {
    use colored::*;
    use std::fs;
    use std::process::Command;
    use std::time::Instant;

    use test_discovery::{compile_test_file, discover_test_files};
    use test_execution::generate_test_harness;
    use test_reporting::{generate_coverage_report, parse_test_output};

    let start_time = Instant::now();
    let project_root = std::env::current_dir()?;
    layout_detect::apply_inferred_test_options(&project_root, &mut opts);

    // Determine test directory
    let test_dir = opts
        .path
        .as_deref()
        .unwrap_or_else(|| Path::new("."));

    if !test_dir.exists() {
        anyhow::bail!("Test path does not exist: {:?}", test_dir);
    }

    // Discover test files
    if !opts.json {
        println!();
        println!(
            "{}",
            "╭─────────────────────────────────────────────╮".cyan()
        );
        println!(
            "{}",
            "│  🧪  Windjammer Test Framework            │"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "╰─────────────────────────────────────────────╯".cyan()
        );
        println!();
        println!("{} Discovering tests...", "→".bright_blue().bold());
    }

    let test_files = discover_test_files(test_dir)?;

    if test_files.is_empty() {
        if opts.json {
            println!("{{\"error\": \"No test files found\", \"files\": [], \"tests\": []}}");
        } else {
            println!();
            println!("{} No test files found", "✗".red().bold());
            println!();
            println!("  {} Test files should:", "ℹ".blue());
            println!(
                "    • Live under {} with names like {}",
                "tests/".yellow(),
                "*_test.wj".yellow()
            );
            println!(
                "    • Mark tests with {} or name functions {}",
                "@test".yellow(),
                "test_*".yellow()
            );
            println!();
        }
        return Ok(());
    }

    if !opts.json {
        println!(
            "{} Found {} test file(s)",
            "✓".green().bold(),
            test_files.len().to_string().bright_white().bold()
        );
        for file in &test_files {
            println!(
                "    {} {}",
                "•".bright_black(),
                file.display().to_string().bright_white()
            );
        }
        println!();
    }

    let temp_dir = std::env::temp_dir().join(format!(
        "windjammer-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    // Compile test files
    if !opts.json {
        println!("{} Compiling tests...", "→".bright_blue().bold());
    }

    let mut all_tests = Vec::new();

    for test_file in &test_files {
        let tests = compile_test_file(test_file, &temp_dir)?;
        all_tests.extend(tests);
    }

    if !opts.json {
        println!(
            "{} Found {} test function(s)",
            "✓".green().bold(),
            all_tests.len().to_string().bright_white().bold()
        );
        println!();
    }

    // Generate test harness (pass project root for library detection)
    generate_test_harness(&temp_dir, &all_tests, opts.filter.as_deref(), &project_root, &opts)?;

    // Run tests
    if !opts.json {
        println!("{}", "─".repeat(50).bright_black());
        println!("{} Running tests...", "▶".bright_green().bold());
        println!("{}", "─".repeat(50).bright_black());
        println!();
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("test")
        .current_dir(&temp_dir)
        .env_remove("CARGO_TARGET_DIR");

    // Dogfood `--use-build-dir` / project-Cargo layouts path-depend on crates whose
    // `build.rs` tip-transpiles unless SKIP_WJ_REGEN is set. Default it for Cargo
    // children so prebuilt outbound trees are not invalidated mid-test.
    if opts.use_build_dir.is_some() || opts.use_project_cargo {
        cmd.env("SKIP_WJ_REGEN", "1");
    }

    if !opts.parallel {
        cmd.arg("--").arg("--test-threads").arg("1");
    }

    if let Some(filter_str) = opts.filter.as_deref() {
        cmd.arg("--").arg(filter_str);
    }

    if opts.nocapture {
        if opts.filter.is_none() {
            cmd.arg("--");
        }
        cmd.arg("--nocapture");
    }

    let output = cmd.output()?;
    let duration = start_time.elapsed();

    // Parse test output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let test_results = parse_test_output(&stdout, &stderr);

    if opts.json {
        // JSON output for tooling
        println!("{{");
        println!("  \"success\": {},", output.status.success());
        println!("  \"duration_ms\": {},", duration.as_millis());
        println!("  \"test_files\": {},", test_files.len());
        println!("  \"total_tests\": {},", all_tests.len());
        println!("  \"passed\": {},", test_results.passed);
        println!("  \"failed\": {},", test_results.failed);
        println!("  \"ignored\": {},", test_results.ignored);
        println!("  \"files\": [");
        for (i, file) in test_files.iter().enumerate() {
            println!(
                "    \"{}\"{}",
                file.display(),
                if i < test_files.len() - 1 { "," } else { "" }
            );
        }
        println!("  ],");
        println!("  \"tests\": [");
        for (i, test) in all_tests.iter().enumerate() {
            // Look up the status for this test
            // The test name in cargo output is "module::test_name"
            let full_test_name = format!(
                "{}::{}",
                test.file.file_stem().unwrap().to_string_lossy(),
                test.name
            );
            let status = test_results
                .individual_results
                .get(&full_test_name)
                .or_else(|| test_results.individual_results.get(&test.name))
                .map(|s| s.as_str())
                .unwrap_or("unknown");

            println!(
                "    {{\"name\": \"{}\", \"file\": \"{}\", \"status\": \"{}\"}}{}",
                test.name,
                test.file.display(),
                status,
                if i < all_tests.len() - 1 { "," } else { "" }
            );
        }
        println!("  ]");
        println!("}}");
    } else {
        // Pretty output for humans
        print!("{}", stdout);
        print!("{}", stderr);

        println!();
        println!("{}", "─".repeat(50).bright_black());

        if output.status.success() {
            println!();
            println!(
                "{} {} All tests passed! {}",
                "✓".green().bold(),
                "🎉".bright_white(),
                "✓".green().bold()
            );
            println!();
            println!(
                "  {} {} passed",
                "✓".green(),
                test_results.passed.to_string().bright_white().bold()
            );
            if test_results.ignored > 0 {
                println!(
                    "  {} {} ignored",
                    "○".yellow(),
                    test_results.ignored.to_string().bright_white()
                );
            }
            println!(
                "  {} Completed in {}",
                "⏱".bright_blue(),
                format!("{:.2}s", duration.as_secs_f64())
                    .bright_white()
                    .bold()
            );
        } else {
            println!();
            println!(
                "{} {} Tests failed {}",
                "✗".red().bold(),
                "⚠".bright_yellow(),
                "✗".red().bold()
            );
            println!();
            println!(
                "  {} {} passed",
                "✓".green(),
                test_results.passed.to_string().bright_white()
            );
            println!(
                "  {} {} failed",
                "✗".red().bold(),
                test_results.failed.to_string().bright_white().bold()
            );
            if test_results.ignored > 0 {
                println!(
                    "  {} {} ignored",
                    "○".yellow(),
                    test_results.ignored.to_string().bright_white()
                );
            }
            println!(
                "  {} Completed in {}",
                "⏱".bright_blue(),
                format!("{:.2}s", duration.as_secs_f64()).bright_white()
            );
        }

        println!();
        println!("{}", "─".repeat(50).bright_black());
        println!();

        // Check for coverage flag in environment
        if std::env::var("WINDJAMMER_COVERAGE").is_ok() {
            println!("{} Generating coverage report...", "→".bright_blue().bold());
            generate_coverage_report(&temp_dir)?;
        }
    }

    if !output.status.success() {
        anyhow::bail!("Tests failed");
    }

    // Clean up (unless tests need to inspect the temp tree)
    if std::env::var("WJ_TEST_KEEP_TEMP").is_ok() {
        eprintln!("WJ_TEST_KEEP_TEMP: {}", temp_dir.display());
    } else if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }

    Ok(())
}
