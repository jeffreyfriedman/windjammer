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
mod options;
mod test_discovery;
pub mod test_execution;
mod test_extraction;
mod test_harness;
mod test_reporting;
mod util;

pub mod interpreted;
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

    use test_execution::generate_test_harness;
    use test_harness::select_files;
    use test_reporting::{generate_coverage_report, parse_test_output};

    let start_time = Instant::now();
    let project_root = std::env::current_dir()?;
    layout_detect::apply_inferred_test_options(&project_root, &mut opts);

    // Determine test directory
    let test_dir = opts.path.as_deref().unwrap_or_else(|| Path::new("."));

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

    let discovery = test_discovery::discover(test_dir)?;
    if !discovery.errors.is_empty() {
        anyhow::bail!("{}", discovery.error_summary());
    }

    if opts.list {
        print_listing(&discovery, opts.json);
        return Ok(());
    }

    if discovery.test_count() == 0 {
        return report_no_tests(test_dir, opts.allow_empty, opts.json);
    }

    let selected = select_files(&discovery.files, opts.filter.as_deref());
    if selected.is_empty() {
        let filter = opts.filter.as_deref().unwrap_or_default();
        if opts.allow_empty {
            println!("No tests match filter {filter:?} (--allow-empty).");
            return Ok(());
        }
        anyhow::bail!(
            "filter {filter:?} matched none of the {} discovered test(s); \
             use `wj test --list` to see them, or --allow-empty",
            discovery.test_count()
        );
    }
    let test_files: Vec<&std::path::PathBuf> = selected.iter().map(|f| &f.path).collect();
    let all_tests: Vec<_> = selected.iter().flat_map(|f| f.tests.clone()).collect();

    if !opts.json {
        println!(
            "{} Found {} test file(s), {} test function(s)",
            "✓".green().bold(),
            test_files.len().to_string().bright_white().bold(),
            all_tests.len().to_string().bright_white().bold()
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

    // Compile test files and generate the harness crate
    if !opts.json {
        println!("{} Compiling tests...", "→".bright_blue().bold());
    }
    generate_test_harness(&temp_dir, &selected, &project_root, &opts)?;

    // Run tests
    if !opts.json {
        println!("{}", "─".repeat(50).bright_black());
        println!("{} Running tests...", "▶".bright_green().bold());
        println!("{}", "─".repeat(50).bright_black());
        println!();
    }

    let _ = crate::cargo_cache::reclaim();
    let mut cmd = Command::new("cargo");
    cmd.arg("test").current_dir(&temp_dir);
    // Ephemeral test crates share the verify tree (often identical package names).
    crate::cargo_cache::configure_cargo_command(&mut cmd, crate::cargo_cache::TargetKind::Verify);

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
        print_json_envelope(test_dir);
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
            let module_name = selected
                .iter()
                .find(|f| f.path == test.file)
                .map(|f| f.module_name.as_str())
                .unwrap_or_default();
            let full_test_name = format!("{}::{}", module_name, test.display_name());
            let status = test_results
                .individual_results
                .get(&full_test_name)
                .or_else(|| test_results.individual_results.get(&test.display_name()))
                .map(|s| s.as_str())
                .unwrap_or("unknown");

            println!(
                "    {{\"name\": \"{}\", \"file\": \"{}\", \"status\": \"{}\"}}{}",
                test.display_name(),
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

/// `--list`: print discovered files and tests without compiling anything.
fn print_listing(discovery: &test_discovery::Discovery, json: bool) {
    if json {
        let files: Vec<String> = discovery
            .files
            .iter()
            .map(|f| {
                let tests: Vec<String> = f
                    .tests
                    .iter()
                    .map(|t| format!("\"{}\"", t.display_name()))
                    .collect();
                format!(
                    "{{\"file\": \"{}\", \"module\": \"{}\", \"tests\": [{}]}}",
                    f.path.display(),
                    f.module_name,
                    tests.join(", ")
                )
            })
            .collect();
        println!("{{\"files\": [{}], \"total_tests\": {}}}", files.join(", "), discovery.test_count());
        return;
    }
    for f in &discovery.files {
        println!("{} (module {})", f.path.display(), f.module_name);
        for t in &f.tests {
            println!("    {}", t.display_name());
        }
    }
    println!(
        "{} test(s) in {} file(s)",
        discovery.test_count(),
        discovery.files.len()
    );
}

/// Nothing to run: an error (non-zero exit) unless `--allow-empty` was given.
fn report_no_tests(dir: &Path, allow_empty: bool, json: bool) -> Result<()> {
    let message = format!(
        "no tests found under {}. Tests are `@test` functions or `test_*` functions in any .wj \
         file (top level, inline `mod`, or receiver-less `impl` functions); files named \
         `*_test.wj`/`test_*.wj` or under `tests/` are always scanned.",
        dir.display()
    );
    if allow_empty {
        if json {
            println!("{{\"files\": [], \"tests\": [], \"note\": \"no tests found\"}}");
        } else {
            println!("{message} (--allow-empty: exiting successfully)");
        }
        return Ok(());
    }
    anyhow::bail!("{message} Pass --allow-empty if an empty test set is expected.")
}

/// Opening of `wj test --json`: schema version, command and the shared-schema front-end
/// diagnostics (docs/diagnostics-schema.json) for the tested sources.
fn print_json_envelope(test_dir: &Path) {
    let diagnostics = crate::check::check_path(test_dir, &crate::check::CheckOptions::default())
        .map(|c| c.report.diagnostics)
        .unwrap_or_default();
    println!("{{");
    println!("  \"schema_version\": {},", crate::diagnostics::SCHEMA_VERSION);
    println!("  \"command\": \"test\",");
    println!(
        "  \"diagnostics\": {},",
        serde_json::to_string(&diagnostics).unwrap_or_else(|_| "[]".to_string())
    );
}
