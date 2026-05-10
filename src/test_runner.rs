//! Test runner module for Windjammer test framework
//!
//! This module provides comprehensive test running infrastructure including:
//! - Test file discovery and compilation
//! - Test execution with parallel/sequential modes
//! - JSON output for CI integration
//! - Coverage report generation

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Import parent module functions needed by test runner
use crate::{build_project, CompilationTarget};

pub fn run_tests(
    path: Option<&Path>,
    filter: Option<&str>,
    nocapture: bool,
    parallel: bool,
    json: bool,
) -> Result<()> {
    use colored::*;
    use std::fs;
    use std::process::Command;
    use std::time::Instant;

    let start_time = Instant::now();

    // Determine test directory
    let test_dir = path.unwrap_or_else(|| Path::new("."));

    if !test_dir.exists() {
        anyhow::bail!("Test path does not exist: {:?}", test_dir);
    }

    // Discover test files
    if !json {
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
        if json {
            println!("{{\"error\": \"No test files found\", \"files\": [], \"tests\": []}}");
        } else {
            println!();
            println!("{} No test files found", "✗".red().bold());
            println!();
            println!("  {} Test files should:", "ℹ".blue());
            println!(
                "    • Be named {}  or {}",
                "*_test.wj".yellow(),
                "test_*.wj".yellow()
            );
            println!("    • Contain functions starting with {}", "test_".yellow());
            println!();
        }
        return Ok(());
    }

    if !json {
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
    if !json {
        println!("{} Compiling tests...", "→".bright_blue().bold());
    }

    let mut all_tests = Vec::new();

    for test_file in &test_files {
        let tests = compile_test_file(test_file, &temp_dir)?;
        all_tests.extend(tests);
    }

    if !json {
        println!(
            "{} Found {} test function(s)",
            "✓".green().bold(),
            all_tests.len().to_string().bright_white().bold()
        );
        println!();
    }

    // Generate test harness (pass project root for library detection)
    let project_root = std::env::current_dir()?;
    generate_test_harness(&temp_dir, &all_tests, filter, &project_root)?;

    // Run tests
    if !json {
        println!("{}", "─".repeat(50).bright_black());
        println!("{} Running tests...", "▶".bright_green().bold());
        println!("{}", "─".repeat(50).bright_black());
        println!();
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("test").current_dir(&temp_dir);

    if !parallel {
        cmd.arg("--").arg("--test-threads").arg("1");
    }

    if let Some(filter_str) = filter {
        cmd.arg("--").arg(filter_str);
    }

    if nocapture {
        if filter.is_none() {
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

    if json {
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

    // Clean up
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }

    Ok(())
}

#[derive(Default)]
struct TestResults {
    passed: usize,
    failed: usize,
    ignored: usize,
    individual_results: HashMap<String, String>, // test_name -> status
}

fn parse_test_output(stdout: &str, _stderr: &str) -> TestResults {
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

/// Discover test files in a directory
fn discover_test_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut test_files = Vec::new();

    if dir.is_file() {
        // Single file
        if is_test_file(dir) {
            test_files.push(dir.to_path_buf());
        }
    } else {
        // Directory - search recursively
        visit_dirs(dir, &mut test_files)?;
    }

    Ok(test_files)
}

/// Visit directories recursively to find test files
fn visit_dirs(dir: &Path, test_files: &mut Vec<PathBuf>) -> Result<()> {
    use std::fs;

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip target, build, and hidden directories
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') || name_str == "target" || name_str == "build" {
                        continue;
                    }
                }
                visit_dirs(&path, test_files)?;
            } else if is_test_file(&path) {
                test_files.push(path);
            }
        }
    }

    Ok(())
}

/// Check if a file is a test file
/// TDD FIX: Only discover test files in tests_wj/ directories or files ending in _test.wj
/// THE WINDJAMMER WAY: Avoid false positives by checking directory structure
fn is_test_file(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();

        // Must end with .wj
        if !name_str.ends_with(".wj") {
            return false;
        }

        // Check if file is in tests/ directory OR ends with _test.wj
        let in_tests_dir = path
            .components()
            .any(|c| c.as_os_str().to_string_lossy() == "tests");

        let ends_with_test = name_str.ends_with("_test.wj");

        in_tests_dir || ends_with_test
    } else {
        false
    }
}

/// Compile a test file and extract test functions
fn compile_test_file(test_file: &Path, _output_dir: &Path) -> Result<Vec<TestFunction>> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::fs;

    let source = fs::read_to_string(test_file)?;

    // Lex and parse
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);

    // TDD DEBUG: Add file context to parser errors
    let program = parser.parse().map_err(|e| {
        eprintln!("DEBUG: Parser error in file: {}", test_file.display());
        eprintln!("DEBUG: Error message: {}", e);
        anyhow::anyhow!("In file {}: {}", test_file.display(), e)
    })?;

    // Find test functions
    let mut tests = Vec::new();
    for item in &program.items {
        if let crate::parser::Item::Function { decl: func, .. } = item {
            if func.name.starts_with("test_") {
                tests.push(TestFunction {
                    name: func.name.clone(),
                    file: test_file.to_path_buf(),
                });
            }
        }
    }

    Ok(tests)
}

/// Test function metadata
#[derive(Debug, Clone)]
struct TestFunction {
    name: String,
    file: PathBuf,
}

/// Detect and compile the library being tested (if it exists)
/// Returns (library_name, library_path) for Cargo dependency, or None
fn detect_and_compile_library(
    project_root: &Path,
    test_output_dir: &Path,
) -> Result<Option<(String, PathBuf)>> {
    use std::fs;

    // Look for wj.toml or windjammer.toml
    let config_path = if project_root.join("wj.toml").exists() {
        Some(project_root.join("wj.toml"))
    } else if project_root.join("windjammer.toml").exists() {
        Some(project_root.join("windjammer.toml"))
    } else {
        None
    };

    let config = if let Some(path) = config_path {
        match fs::read_to_string(&path) {
            Ok(content) => toml::from_str::<crate::config::WjConfig>(&content).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    // Check if there's a library to compile
    let src_dir = project_root.join("src");
    if !src_dir.exists() || !src_dir.is_dir() {
        return Ok(None); // No library to compile
    }

    // Get library name from config or infer from directory
    let lib_name = config
        .as_ref()
        .and_then(|c| {
            if !c.package.name.is_empty() {
                Some(c.package.name.clone())
            } else {
                c.project.as_ref().map(|p| p.name.clone())
            }
        })
        .or_else(|| {
            project_root
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.replace('-', "_"))
        })
        .unwrap_or_else(|| "lib".to_string());

    // Create library output directory (clean it first to avoid stale files)
    let lib_output_dir = test_output_dir.join("lib");
    if lib_output_dir.exists() {
        fs::remove_dir_all(&lib_output_dir)?;
    }
    fs::create_dir_all(&lib_output_dir)?;

    // Compile the library
    use colored::*;
    println!(
        "   {} Compiling library: {}",
        "→".bright_blue().bold(),
        lib_name
    );

    // Use build_project to compile the library
    eprintln!("DEBUG: About to call build_project");
    match build_project(&src_dir, &lib_output_dir, CompilationTarget::Rust, true) {
        Ok(_) => {
            eprintln!("DEBUG: build_project returned Ok");
            // Generate lib.rs entry point for the compiled library
            // build_project generates Rust files but doesn't create lib.rs
            if let Err(e) = generate_lib_rs_for_library(&lib_output_dir) {
                eprintln!("WARNING: Failed to generate lib.rs: {}", e);
                // Continue anyway - the library might still work
            } else {
                eprintln!("DEBUG: generate_lib_rs_for_library succeeded");
            }

            // TDD FIX: Copy FFI files from src/ffi to test library
            // THE WINDJAMMER WAY: Dynamic, robust FFI integration
            // This enables tests to work with full FFI functionality
            if let Err(e) = copy_ffi_files_to_test_library(project_root, &lib_output_dir) {
                eprintln!("WARNING: Failed to copy FFI files: {}", e);
                // Continue anyway - tests might not need FFI
            } else {
                eprintln!("DEBUG: FFI files copied successfully");
            }

            eprintln!("DEBUG: About to fix Cargo.toml");

            // TDD FIX: Use the project's actual lib name, not a _testlib suffix
            // THE WINDJAMMER WAY: Test library name must match project lib name so imports work
            // Bug: Tests were failing with E0433: unresolved module windjammer_game_core
            // Root Cause: Test library was using *_testlib suffix, breaking imports
            // Fix: Read the actual [lib] name from project's Cargo.toml

            // Read project's Cargo.toml to get the actual lib name
            let project_cargo_toml = project_root.join("Cargo.toml");
            let actual_lib_name = if project_cargo_toml.exists() {
                match fs::read_to_string(&project_cargo_toml) {
                    Ok(content) => {
                        // Parse [lib] name from Cargo.toml
                        if let Some(lib_section_start) = content.find("[lib]") {
                            if let Some(name_start) = content[lib_section_start..].find("name = \"")
                            {
                                let abs_start = lib_section_start + name_start + "name = \"".len();
                                content[abs_start..].find('"').map(|name_end| {
                                    content[abs_start..abs_start + name_end].to_string()
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            } else {
                None
            };

            // TDD FIX: Use actual library package name from project's Cargo.toml
            // THE WINDJAMMER WAY: Test dependencies must use real library names
            // Bug: Tests failing with E0433: unresolved module windjammer_game_core
            // Root Cause: Test Cargo.toml uses windjammer-game-core-testlib, but tests import windjammer_game_core
            // Fix: Read actual [package] name from project's Cargo.toml and use that for test dependency

            let actual_package_name = if project_cargo_toml.exists() {
                match fs::read_to_string(&project_cargo_toml) {
                    Ok(content) => {
                        // Parse [package] name from Cargo.toml
                        content.find("[package]").and_then(|pkg_start| {
                            content[pkg_start..]
                                .find("name = \"")
                                .and_then(|name_start| {
                                    let abs_start = pkg_start + name_start + "name = \"".len();
                                    content[abs_start..].find('"').map(|name_end| {
                                        content[abs_start..abs_start + name_end].to_string()
                                    })
                                })
                        })
                    }
                    Err(_) => None,
                }
            } else {
                None
            };

            // Use actual lib name from Cargo.toml, or infer from package name
            let test_lib_name = actual_lib_name.unwrap_or_else(|| lib_name.replace('-', "_"));
            // Use actual package name from Cargo.toml, or infer from directory (NO -testlib suffix!)
            let test_lib_package_name =
                actual_package_name.unwrap_or_else(|| lib_name.replace('_', "-"));

            // Fix the generated Cargo.toml to use the correct library name and add user dependencies
            let cargo_toml_path = lib_output_dir.join("Cargo.toml");

            println!(
                "   {} Reading Cargo.toml from: {}",
                "→".blue().bold(),
                cargo_toml_path.display()
            );

            match fs::read_to_string(&cargo_toml_path) {
                Ok(mut cargo_toml) => {
                    // Replace [package] name with unique test library name
                    if let Some(pkg_start) = cargo_toml.find("[package]") {
                        if let Some(name_start) = cargo_toml[pkg_start..].find("name = \"") {
                            let abs_name_start = pkg_start + name_start + "name = \"".len();
                            if let Some(name_end) = cargo_toml[abs_name_start..].find('"') {
                                let abs_name_end = abs_name_start + name_end;
                                cargo_toml.replace_range(
                                    abs_name_start..abs_name_end,
                                    &test_lib_package_name,
                                );
                            }
                        }
                    }

                    // Replace [lib] name with unique test library name
                    if let Some(lib_start) = cargo_toml.find("[lib]") {
                        if let Some(name_start) = cargo_toml[lib_start..].find("name = \"") {
                            let abs_name_start = lib_start + name_start + "name = \"".len();
                            if let Some(name_end) = cargo_toml[abs_name_start..].find('"') {
                                let abs_name_end = abs_name_start + name_end;
                                cargo_toml
                                    .replace_range(abs_name_start..abs_name_end, &test_lib_name);
                            }
                        }
                    }

                    // Remove self-referential dependency (library importing itself)
                    let self_dep_pattern = format!("{} = {{", lib_name);
                    if let Some(start) = cargo_toml.find(&self_dep_pattern) {
                        // Find the end of this dependency line
                        if let Some(end) = cargo_toml[start..].find('\n') {
                            let line_end = start + end + 1;
                            cargo_toml.replace_range(start..line_end, "");
                        }
                    }

                    // Add user dependencies from wj.toml (replace wildcards with proper specs)
                    if let Some(cfg) = &config {
                        let mut deps_section = String::new();
                        for (dep_name, dep_spec) in &cfg.dependencies {
                            // If dependency exists with wildcard version, replace it
                            let wildcard_pattern = format!("{} = \"*\"", dep_name);
                            if cargo_toml.contains(&wildcard_pattern) {
                                // Remove the wildcard line
                                cargo_toml =
                                    cargo_toml.replace(&format!("{}\n", wildcard_pattern), "");
                            }
                            // If dependency already exists with a proper spec, skip it
                            else if cargo_toml.contains(&format!("{} =", dep_name)) {
                                continue;
                            }

                            use crate::config::DependencySpec;
                            match dep_spec {
                                DependencySpec::Simple(version) => {
                                    deps_section
                                        .push_str(&format!("{} = \"{}\"\n", dep_name, version));
                                }
                                DependencySpec::Detailed {
                                    version,
                                    features,
                                    path,
                                    git,
                                    branch,
                                } => {
                                    deps_section.push_str(&format!("{} = {{ ", dep_name));
                                    if let Some(v) = version {
                                        deps_section.push_str(&format!("version = \"{}\", ", v));
                                    }
                                    if let Some(p) = path {
                                        // Make path relative to project root
                                        let abs_path = project_root.join(p);
                                        deps_section.push_str(&format!(
                                            "path = \"{}\", ",
                                            path_to_toml_string(&abs_path)
                                        ));
                                    }
                                    // Add desktop feature for windjammer-ui
                                    if dep_name == "windjammer-ui" && !features.is_some() {
                                        deps_section.push_str("features = [\"desktop\"], ");
                                    }
                                    if let Some(g) = git {
                                        deps_section.push_str(&format!("git = \"{}\", ", g));
                                    }
                                    if let Some(b) = branch {
                                        deps_section.push_str(&format!("branch = \"{}\", ", b));
                                    }
                                    if let Some(f) = features {
                                        deps_section.push_str(&format!("features = {:?}, ", f));
                                    }
                                    // Remove trailing comma and space
                                    if deps_section.ends_with(", ") {
                                        deps_section.truncate(deps_section.len() - 2);
                                    }
                                    deps_section.push_str(" }\n");
                                }
                            }
                        }

                        // Insert dependencies before [lib] section
                        if !deps_section.is_empty() {
                            if let Some(lib_pos) = cargo_toml.find("[lib]") {
                                cargo_toml.insert_str(lib_pos, &deps_section);
                            }
                        }
                    }

                    if let Err(e) = fs::write(&cargo_toml_path, &cargo_toml) {
                        eprintln!("WARNING: Failed to write Cargo.toml: {}", e);
                    } else {
                        println!(
                            "   {} Updated library Cargo.toml (package: {}, lib: {})",
                            "✓".green().bold(),
                            test_lib_package_name,
                            test_lib_name
                        );
                    }
                }
                Err(e) => {
                    eprintln!("WARNING: Failed to read Cargo.toml: {}", e);
                }
            }

            println!("   {} Library compiled successfully", "✓".green().bold());
            // Return the test library package name (with -testlib suffix) for Cargo dependency
            Ok(Some((test_lib_package_name, lib_output_dir)))
        }
        Err(e) => {
            println!("   {} Library compilation failed: {}", "✗".red().bold(), e);
            // Don't fail the entire test run - just continue without library dependency
            Ok(None)
        }
    }
}

/// Generate lib.rs entry point for a compiled library
/// This creates a proper Rust library crate structure that tests can import from
fn generate_lib_rs_for_library(lib_output_dir: &Path) -> Result<()> {
    use std::collections::HashSet;
    use std::fs;

    // Find all modules (directories with mod.rs AND top-level .rs files)
    let mut dir_modules = HashSet::new();
    let mut file_modules = Vec::new();

    for entry in fs::read_dir(lib_output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Directory modules (must have mod.rs)
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if path.join("mod.rs").exists() {
                    dir_modules.insert(dir_name.to_string());
                }
            }
        } else if path.is_file() {
            // Top-level .rs files (but not lib.rs or Cargo.toml)
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs") && file_name != "lib.rs" {
                    // Extract module name (remove .rs extension)
                    let module_name = file_name.trim_end_matches(".rs");
                    file_modules.push(module_name.to_string());
                }
            }
        }
    }

    // TDD FIX: When both window.rs and window/mod.rs exist, prefer directory module
    // This prevents E0761: file for module `window` found at both locations
    // Directory modules take precedence (standard Rust convention)
    let mut modules: Vec<String> = dir_modules.iter().cloned().collect();

    // Add file modules that don't conflict with directory modules
    for file_module in file_modules {
        if !dir_modules.contains(&file_module) {
            modules.push(file_module);
        }
    }

    if modules.is_empty() {
        return Ok(()); // No modules to export
    }

    // TDD FIX: Filter out "lib" module to prevent E0761 conflict
    // "lib" is a reserved name for the library itself, not a module to import
    // This prevents: error[E0761]: file for module `lib` found at both "lib.rs" and "lib/mod.rs"
    // Also exclude "mod" — `pub mod mod;` is invalid (keyword); mod.rs is the barrel, not a child module
    modules.retain(|m| m != "lib" && m != "mod");

    if modules.is_empty() {
        return Ok(()); // No modules to export after filtering
    }

    modules.sort();

    // Generate lib.rs content
    let mut lib_rs = String::from("// Auto-generated library entry point\n\n");

    // Declare all modules
    for module in &modules {
        lib_rs.push_str(&format!("pub mod {};\n", module));
    }

    lib_rs.push_str("\n// Re-export for convenience\n");
    for module in &modules {
        lib_rs.push_str(&format!("pub use {}::*;\n", module));
    }

    // Write lib.rs
    fs::write(lib_output_dir.join("lib.rs"), lib_rs)?;

    Ok(())
}

/// TDD FIX: Copy FFI files from project src/ffi to test library output
/// THE WINDJAMMER WAY: Dynamic, robust FFI integration for tests
///
/// This function:
/// 1. Checks if project has src/ffi directory
/// 2. Recursively copies all .rs files
/// 3. Copies shader files (.wgsl) if they exist
/// 4. Updates lib.rs to include pub mod ffi
/// 5. Returns Ok even if no FFI files exist (optional feature)
fn copy_ffi_files_to_test_library(project_root: &Path, lib_output_dir: &Path) -> Result<()> {
    use colored::*;
    use std::fs;

    // TDD FIX: Check for FFI directory in multiple locations
    // THE WINDJAMMER WAY: Support both src/ffi (library) and ffi/ (game engine) layouts
    // 1. Check ffi/ at project root (game engine layout)
    // 2. Check src/ffi/ (library layout)
    let ffi_locations = [
        project_root.join("ffi"),
        project_root.join("src").join("ffi"),
    ];

    let src_ffi_dir = ffi_locations
        .iter()
        .find(|path| path.exists() && path.is_dir())
        .cloned();

    let src_ffi_dir = match src_ffi_dir {
        Some(dir) => dir,
        None => {
            // No FFI directory - this is fine, not all projects need FFI
            return Ok(());
        }
    };

    println!(
        "   {} Copying FFI files from {}",
        "→".bright_blue().bold(),
        src_ffi_dir.display()
    );

    // Create ffi directory in lib output
    let dest_ffi_dir = lib_output_dir.join("ffi");
    fs::create_dir_all(&dest_ffi_dir)?;

    // Recursively copy all .rs files from src/ffi
    copy_ffi_files_recursive(&src_ffi_dir, &dest_ffi_dir)?;

    // Update lib.rs to include ffi module
    let lib_rs_path = lib_output_dir.join("lib.rs");
    if lib_rs_path.exists() {
        let mut lib_rs_content = fs::read_to_string(&lib_rs_path)?;

        // Check if ffi module is already declared
        if !lib_rs_content.contains("pub mod ffi") {
            // Insert ffi module declaration after other module declarations
            // but before re-exports
            if let Some(reexport_pos) = lib_rs_content.find("// Re-export for convenience") {
                lib_rs_content
                    .insert_str(reexport_pos, "pub mod ffi; // FFI Rust implementations\n\n");
            } else {
                // No re-export comment found, append at end
                lib_rs_content.push_str("\npub mod ffi; // FFI Rust implementations\n");
            }

            fs::write(&lib_rs_path, lib_rs_content)?;
            println!(
                "   {} Updated lib.rs to include FFI module",
                "✓".green().bold()
            );
        }
    }

    // TDD FIX: Copy FFI dependencies from project Cargo.toml to test library Cargo.toml
    // This ensures that FFI code that uses external crates (like wgpu) can compile
    copy_ffi_dependencies_to_test_library(project_root, lib_output_dir)?;

    println!("   {} FFI files copied successfully", "✓".green().bold());

    Ok(())
}

/// TDD FIX: Copy FFI dependencies from project Cargo.toml to test library Cargo.toml
/// THE WINDJAMMER WAY: Dynamic, robust dependency management for FFI
fn copy_ffi_dependencies_to_test_library(project_root: &Path, lib_output_dir: &Path) -> Result<()> {
    use colored::*;
    use std::fs;

    // Read project's Cargo.toml
    let project_cargo_toml = project_root.join("Cargo.toml");
    if !project_cargo_toml.exists() {
        // No Cargo.toml - no FFI dependencies to copy
        return Ok(());
    }

    let cargo_toml_content = fs::read_to_string(&project_cargo_toml)?;
    let cargo_toml: toml::Value = toml::from_str(&cargo_toml_content)?;

    // Extract dependencies
    let mut ffi_deps = Vec::new();
    if let Some(deps) = cargo_toml.get("dependencies").and_then(|v| v.as_table()) {
        for (dep_name, dep_spec) in deps {
            // Skip windjammer-runtime (already added by test framework)
            // Skip dependencies that are paths to local crates
            if dep_name == "windjammer-runtime" {
                continue;
            }

            // Add all other dependencies (these are typically external crates needed by FFI)
            ffi_deps.push((dep_name.clone(), dep_spec.clone()));
        }
    }

    if ffi_deps.is_empty() {
        // No FFI dependencies to copy
        return Ok(());
    }

    println!(
        "   {} Copying {} FFI dependencies to test library",
        "→".bright_blue().bold(),
        ffi_deps.len()
    );

    // Read test library's Cargo.toml
    let test_cargo_toml_path = lib_output_dir.join("Cargo.toml");
    let mut test_cargo_toml_content = fs::read_to_string(&test_cargo_toml_path)?;

    // Add FFI dependencies to the [dependencies] section
    for (dep_name, dep_spec) in ffi_deps {
        // Skip if dependency already exists
        if test_cargo_toml_content.contains(&format!("{} =", dep_name)) {
            continue;
        }

        // Format dependency spec as TOML
        let dep_line = if let Some(version_str) = dep_spec.as_str() {
            // Simple version string: dep = "1.0"
            format!("{} = \"{}\"", dep_name, version_str)
        } else if let Some(table) = dep_spec.as_table() {
            // Complex dependency with features, etc.
            let mut parts = Vec::new();
            if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
                parts.push(format!("version = \"{}\"", version));
            }
            if let Some(features) = table.get("features") {
                // Handle features array more carefully
                if let Ok(features_str) = toml::to_string(features) {
                    parts.push(format!("features = {}", features_str.trim()));
                } else if let Some(arr) = features.as_array() {
                    // Manual array formatting as fallback
                    let feature_list: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| format!("\"{}\"", s))
                        .collect();
                    parts.push(format!("features = [{}]", feature_list.join(", ")));
                }
            }
            if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                // Make path absolute for test library
                let abs_path = project_root.join(path);
                parts.push(format!("path = \"{}\"", path_to_toml_string(&abs_path)));
            }
            if parts.is_empty() {
                // No useful parts extracted, skip this dependency
                continue;
            }
            format!("{} = {{ {} }}", dep_name, parts.join(", "))
        } else {
            continue;
        };

        // Find [dependencies] section and add the dependency
        if let Some(deps_pos) = test_cargo_toml_content.find("[dependencies]") {
            // Find the end of the [dependencies] section (next [section] or EOF)
            let after_deps = &test_cargo_toml_content[deps_pos + "[dependencies]".len()..];
            if let Some(next_section_pos) = after_deps.find("\n[") {
                // Insert before next section
                let insert_pos = deps_pos + "[dependencies]".len() + next_section_pos;
                test_cargo_toml_content.insert_str(insert_pos, &format!("\n{}", dep_line));
            } else {
                // Append at end of file
                test_cargo_toml_content.push_str(&format!("\n{}\n", dep_line));
            }
        }
    }

    // Write updated Cargo.toml
    fs::write(&test_cargo_toml_path, test_cargo_toml_content)?;

    println!(
        "   {} FFI dependencies added to test library Cargo.toml",
        "✓".green().bold()
    );

    Ok(())
}

/// Recursively copy .rs and .wgsl files from source to destination
fn copy_ffi_files_recursive(src: &Path, dest: &Path) -> Result<()> {
    use std::fs;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = match path.file_name() {
            Some(name) => name,
            None => continue,
        };

        if path.is_dir() {
            // Recursively copy subdirectories (e.g., shaders/)
            let dest_subdir = dest.join(file_name);
            fs::create_dir_all(&dest_subdir)?;
            copy_ffi_files_recursive(&path, &dest_subdir)?;
        } else if path.is_file() {
            // Copy .rs files and .wgsl shader files
            if let Some(file_name_str) = file_name.to_str() {
                if file_name_str.ends_with(".rs") || file_name_str.ends_with(".wgsl") {
                    let dest_file = dest.join(file_name);
                    fs::copy(&path, &dest_file)?;
                }
            }
        }
    }

    Ok(())
}

/// Convert a path to TOML-safe format (forward slashes, no Windows \\?\ prefix)
/// Windows canonicalize() adds \\?\ prefix; backslashes cause TOML parse errors
pub fn path_to_toml_string(path: &Path) -> String {
    let s = path.display().to_string();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.replace('\\', "/")
}

/// Find windjammer-runtime path using robust search logic
fn find_windjammer_runtime_path() -> Result<PathBuf> {
    use std::env;

    // TDD FIX: Use CARGO_MANIFEST_DIR when available (set during cargo test/build)
    // This gives us the windjammer compiler's directory directly
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir);
        let runtime_path = manifest_path.join("crates/windjammer-runtime");
        if runtime_path.join("Cargo.toml").exists() {
            return Ok(runtime_path);
        }
    }

    // Start from current directory and search upward
    let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Try current directory first (if we're in windjammer repo)
    if current
        .join("crates/windjammer-runtime/Cargo.toml")
        .exists()
    {
        return Ok(current.join("crates/windjammer-runtime"));
    }

    // Search up to 10 levels (increased from 5 for deeper project structures)
    for _ in 0..10 {
        if let Some(parent) = current.parent() {
            // Check for windjammer/crates/windjammer-runtime (nested structure)
            if parent
                .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return Ok(parent.join("windjammer/crates/windjammer-runtime"));
            }

            // Check for crates/windjammer-runtime (flat structure)
            if parent.join("crates/windjammer-runtime/Cargo.toml").exists() {
                return Ok(parent.join("crates/windjammer-runtime"));
            }

            // Check current level for windjammer/crates (sibling search)
            if current
                .join("../windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return Ok(current.join("../windjammer/crates/windjammer-runtime"));
            }

            // Check for ../../windjammer/crates (deeper sibling)
            if current
                .join("../../windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return Ok(current.join("../../windjammer/crates/windjammer-runtime"));
            }

            // Check for ../../../windjammer/crates (even deeper)
            if current
                .join("../../../windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return Ok(current.join("../../../windjammer/crates/windjammer-runtime"));
            }

            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Fallback: try relative paths and canonicalize them
    let candidates = vec![
        PathBuf::from("../windjammer/crates/windjammer-runtime"),
        PathBuf::from("../../windjammer/crates/windjammer-runtime"),
        PathBuf::from("../../../windjammer/crates/windjammer-runtime"),
        PathBuf::from("./crates/windjammer-runtime"),
    ];

    for candidate in candidates {
        if candidate.join("Cargo.toml").exists() {
            // Canonicalize to get absolute path
            if let Ok(canonical) = candidate.canonicalize() {
                return Ok(canonical);
            }
            return Ok(candidate);
        }
    }

    // Last resort: assume it's in the workspace (try to make it absolute)
    let fallback_path = PathBuf::from("./crates/windjammer-runtime");
    if fallback_path.exists() {
        // Try to canonicalize to get absolute path
        if let Ok(canonical) = fallback_path.canonicalize() {
            return Ok(canonical);
        }
    }

    // Final fallback: return relative path and hope for the best
    Ok(fallback_path)
}

/// Generate Rust test harness from Windjammer tests
fn generate_test_harness(
    output_dir: &Path,
    tests: &[TestFunction],
    filter: Option<&str>,
    project_root: &Path,
) -> Result<()> {
    use std::collections::HashMap;
    use std::fs;

    // Group tests by file
    let mut tests_by_file: HashMap<PathBuf, Vec<&TestFunction>> = HashMap::new();
    for test in tests {
        tests_by_file
            .entry(test.file.clone())
            .or_default()
            .push(test);
    }

    // Compile each test file using the existing infrastructure
    for (file, file_tests) in &tests_by_file {
        // Skip if filter doesn't match
        if let Some(filter_str) = filter {
            if !file_tests.iter().any(|t| t.name.contains(filter_str)) {
                continue;
            }
        }

        // Compile the file to Rust
        build_project(file, output_dir, CompilationTarget::Rust, false)?;

        // Read the generated Rust code
        let output_file = output_dir.join(format!(
            "{}.rs",
            file.file_stem().unwrap().to_string_lossy()
        ));
        let mut rust_code = fs::read_to_string(&output_file)?;

        // Add test attributes to test functions
        for test in file_tests.iter() {
            let test_fn = format!("fn {}()", test.name);
            let test_attr = format!("#[test]\nfn {}()", test.name);
            rust_code = rust_code.replace(&test_fn, &test_attr);
        }

        // Write back
        fs::write(&output_file, rust_code)?;
    }

    // Detect and compile the library if it exists
    let library_dependency = detect_and_compile_library(project_root, output_dir)?;

    // TDD FIX: Copy windjammer-runtime to test directory so tests can find it
    // THE WINDJAMMER WAY: Self-contained test environments
    let windjammer_runtime_path = find_windjammer_runtime_path()?;
    let test_runtime_path = output_dir.join("crates").join("windjammer-runtime");

    // Create crates directory and copy windjammer-runtime
    use colored::*;
    println!(
        "   {} Copying windjammer-runtime to test directory",
        "→".bright_blue().bold()
    );
    println!(
        "   {} Source: {}",
        "→".bright_blue().bold(),
        windjammer_runtime_path.display()
    );
    println!(
        "   {} Dest: {}",
        "→".bright_blue().bold(),
        test_runtime_path.display()
    );
    fs::create_dir_all(output_dir.join("crates"))
        .map_err(|e| anyhow::anyhow!("Failed to create crates directory: {}", e))?;

    if !windjammer_runtime_path.exists() {
        anyhow::bail!(
            "windjammer-runtime source path does not exist: {}",
            windjammer_runtime_path.display()
        );
    }

    copy_dir_recursive(&windjammer_runtime_path, &test_runtime_path)
        .map_err(|e| anyhow::anyhow!("Failed to copy windjammer-runtime: {}", e))?;

    // TDD FIX: Patch windjammer-runtime's Cargo.toml to remove workspace inheritance
    // When copied to a temp directory, there's no workspace root, so all fields must be explicit
    let runtime_cargo_toml = test_runtime_path.join("Cargo.toml");
    if runtime_cargo_toml.exists() {
        let content = fs::read_to_string(&runtime_cargo_toml)?;
        // Replace all workspace-inherited fields with explicit values
        let patched = content
            .replace("version.workspace = true", "version = \"0.1.0\"")
            .replace("version = { workspace = true }", "version = \"0.1.0\"")
            .replace("edition.workspace = true", "edition = \"2021\"")
            .replace("edition = { workspace = true }", "edition = \"2021\"")
            .replace("authors.workspace = true", "authors = []")
            .replace("authors = { workspace = true }", "authors = []")
            .replace("license.workspace = true", "license = \"MIT\"")
            .replace("license = { workspace = true }", "license = \"MIT\"");
        fs::write(&runtime_cargo_toml, patched)?;
    }

    println!(
        "   {} windjammer-runtime copied successfully",
        "✓".green().bold()
    );

    let library_dep_str = if let Some((lib_name, lib_path)) = library_dependency {
        format!(
            "\n{} = {{ path = \"{}\" }}",
            lib_name,
            path_to_toml_string(&lib_path)
        )
    } else {
        String::new()
    };

    let cargo_toml = format!(
        r#"[package]
name = "windjammer-tests"
version = "0.1.0"
edition = "2021"

[dependencies]
windjammer-runtime = {{ path = "crates/windjammer-runtime" }}
smallvec = "1.13"{}

[lib]
name = "windjammer_tests"
path = "lib.rs"
"#,
        library_dep_str
    );
    fs::write(output_dir.join("Cargo.toml"), cargo_toml)?;

    // Create lib.rs that includes all test modules
    let mut lib_rs = String::from("// Auto-generated test harness\n\n");
    for (file, _) in tests_by_file {
        let module_name = file.file_stem().unwrap().to_string_lossy();
        lib_rs.push_str(&format!("pub mod {};\n", module_name));
    }
    fs::write(output_dir.join("lib.rs"), lib_rs)?;

    Ok(())
}

/// Generate coverage report using cargo-llvm-cov
fn generate_coverage_report(test_dir: &Path) -> Result<()> {
    use colored::*;
    use std::process::Command;

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

/// Recursively copy a directory
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
