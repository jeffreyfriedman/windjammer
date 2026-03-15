//! Test runner for Windjammer - discovers and runs *_test.wj files.
//!
//! Extracted from main.rs for use by cli/test.rs and wj binary when the cli feature is enabled.
//! Uses build_project for compilation (no ModuleCompiler dependency).

use crate::build_project;
use crate::config::{DependencySpec, WjConfig};
use crate::CompilationTarget;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Run tests (discovers and runs all test functions)
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
    let test_dir = path.unwrap_or_else(|| Path::new("."));

    if !test_dir.exists() {
        anyhow::bail!("Test path does not exist: {:?}", test_dir);
    }

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

    let temp_dir = std::env::temp_dir().join("windjammer-test");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

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

    let project_root = std::env::current_dir()?;
    generate_test_harness(&temp_dir, &all_tests, filter, &project_root)?;

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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let test_results = parse_test_output(&stdout, &stderr);

    if json {
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

        if std::env::var("WINDJAMMER_COVERAGE").is_ok() {
            println!("{} Generating coverage report...", "→".bright_blue().bold());
            generate_coverage_report(&temp_dir)?;
        }
    }

    if !output.status.success() {
        anyhow::bail!("Tests failed");
    }

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
    individual_results: HashMap<String, String>,
}

fn parse_test_output(stdout: &str, _stderr: &str) -> TestResults {
    let mut results = TestResults::default();

    for line in stdout.lines() {
        let line = line.trim();

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

        if line.contains("test result:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if part == &"passed;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        results.passed += n;
                    }
                }
                if part == &"failed;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        results.failed += n;
                    }
                }
                if part == &"ignored;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        results.ignored += n;
                    }
                }
            }
        }
    }

    results
}

fn discover_test_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut test_files = Vec::new();

    if dir.is_file() {
        if is_test_file(dir) {
            test_files.push(dir.to_path_buf());
        }
    } else {
        visit_dirs(dir, &mut test_files)?;
    }

    Ok(test_files)
}

fn visit_dirs(dir: &Path, test_files: &mut Vec<PathBuf>) -> Result<()> {
    use std::fs;

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
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

fn is_test_file(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();

        if !name_str.ends_with(".wj") {
            return false;
        }

        let in_tests_dir = path
            .components()
            .any(|c| c.as_os_str().to_string_lossy() == "tests_wj");

        let ends_with_test = name_str.ends_with("_test.wj");

        in_tests_dir || ends_with_test
    } else {
        false
    }
}

fn compile_test_file(test_file: &Path, _output_dir: &Path) -> Result<Vec<TestFunction>> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::fs;

    let source = fs::read_to_string(test_file)?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);

    let program = parser.parse().map_err(|e| {
        eprintln!("DEBUG: Parser error in file: {}", test_file.display());
        eprintln!("DEBUG: Error message: {}", e);
        anyhow::anyhow!("In file {}: {}", test_file.display(), e)
    })?;

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

#[derive(Debug, Clone)]
struct TestFunction {
    name: String,
    file: PathBuf,
}

fn generate_test_harness(
    output_dir: &Path,
    tests: &[TestFunction],
    filter: Option<&str>,
    project_root: &Path,
) -> Result<()> {
    use std::fs;

    let mut tests_by_file: HashMap<PathBuf, Vec<&TestFunction>> = HashMap::new();
    for test in tests {
        tests_by_file
            .entry(test.file.clone())
            .or_default()
            .push(test);
    }

    for (file, file_tests) in &tests_by_file {
        if let Some(filter_str) = filter {
            if !file_tests.iter().any(|t| t.name.contains(filter_str)) {
                continue;
            }
        }

        // Use build_project for single-file compilation (replaces compile_file)
        build_project(file, output_dir, CompilationTarget::Rust, true)?;

        let output_file = output_dir.join(format!(
            "{}.rs",
            file.file_stem().unwrap().to_string_lossy()
        ));
        let mut rust_code = fs::read_to_string(&output_file)?;

        for test in file_tests.iter() {
            let test_fn = format!("fn {}()", test.name);
            let test_attr = format!("#[test]\nfn {}()", test.name);
            rust_code = rust_code.replace(&test_fn, &test_attr);
        }

        fs::write(&output_file, rust_code)?;
    }

    let library_dependency = detect_and_compile_library(project_root, output_dir)?;

    let windjammer_runtime_path = find_windjammer_runtime_path()?;
    let test_runtime_path = output_dir.join("crates").join("windjammer-runtime");

    use colored::*;
    println!(
        "   {} Copying windjammer-runtime to test directory",
        "→".bright_blue().bold()
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

    let runtime_cargo_toml = test_runtime_path.join("Cargo.toml");
    if runtime_cargo_toml.exists() {
        let content = fs::read_to_string(&runtime_cargo_toml)?;
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

    let mut lib_rs = String::from("// Auto-generated test harness\n\n");
    for (file, _) in tests_by_file {
        let module_name = file.file_stem().unwrap().to_string_lossy();
        lib_rs.push_str(&format!("pub mod {};\n", module_name));
    }
    fs::write(output_dir.join("lib.rs"), lib_rs)?;

    Ok(())
}

fn detect_and_compile_library(
    project_root: &Path,
    test_output_dir: &Path,
) -> Result<Option<(String, PathBuf)>> {
    use std::fs;

    let config_path = if project_root.join("wj.toml").exists() {
        Some(project_root.join("wj.toml"))
    } else if project_root.join("windjammer.toml").exists() {
        Some(project_root.join("windjammer.toml"))
    } else {
        None
    };

    let config: Option<WjConfig> = if let Some(path) = config_path {
        match fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let src_wj_dir = project_root.join("src_wj");
    if !src_wj_dir.exists() || !src_wj_dir.is_dir() {
        return Ok(None);
    }

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

    let lib_output_dir = test_output_dir.join("lib");
    if lib_output_dir.exists() {
        fs::remove_dir_all(&lib_output_dir)?;
    }
    fs::create_dir_all(&lib_output_dir)?;

    use colored::*;
    println!(
        "   {} Compiling library: {}",
        "→".bright_blue().bold(),
        lib_name
    );

    match build_project(&src_wj_dir, &lib_output_dir, CompilationTarget::Rust, true) {
        Ok(_) => {
            if let Err(e) = generate_lib_rs_for_library(&lib_output_dir) {
                eprintln!("WARNING: Failed to generate lib.rs: {}", e);
            }
            if let Err(e) = copy_ffi_files_to_test_library(project_root, &lib_output_dir) {
                eprintln!("WARNING: Failed to copy FFI files: {}", e);
            }

            let project_cargo_toml = project_root.join("Cargo.toml");
            let actual_lib_name = if project_cargo_toml.exists() {
                match fs::read_to_string(&project_cargo_toml) {
                    Ok(content) => {
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

            let actual_package_name = if project_cargo_toml.exists() {
                match fs::read_to_string(&project_cargo_toml) {
                    Ok(content) => content.find("[package]").and_then(|pkg_start| {
                        content[pkg_start..]
                            .find("name = \"")
                            .and_then(|name_start| {
                                let abs_start = pkg_start + name_start + "name = \"".len();
                                content[abs_start..].find('"').map(|name_end| {
                                    content[abs_start..abs_start + name_end].to_string()
                                })
                            })
                    }),
                    Err(_) => None,
                }
            } else {
                None
            };

            let test_lib_name = actual_lib_name.unwrap_or_else(|| lib_name.replace('-', "_"));
            let test_lib_package_name =
                actual_package_name.unwrap_or_else(|| lib_name.replace('_', "-"));

            let cargo_toml_path = lib_output_dir.join("Cargo.toml");
            match fs::read_to_string(&cargo_toml_path) {
                Ok(mut cargo_toml) => {
                    if let Some(pkg_start) = cargo_toml.find("[package]") {
                        if let Some(name_start) = cargo_toml[pkg_start..].find("name = \"") {
                            let abs_name_start = pkg_start + name_start + "name = \"".len();
                            if let Some(name_end) = cargo_toml[abs_name_start..].find('"') {
                                let abs_name_end = abs_name_start + name_end;
                                cargo_toml
                                    .replace_range(abs_name_start..abs_name_end, &test_lib_package_name);
                            }
                        }
                    }

                    if let Some(lib_start) = cargo_toml.find("[lib]") {
                        if let Some(name_start) = cargo_toml[lib_start..].find("name = \"") {
                            let abs_name_start = lib_start + name_start + "name = \"".len();
                            if let Some(name_end) = cargo_toml[abs_name_start..].find('"') {
                                let abs_name_end = abs_name_start + name_end;
                                cargo_toml.replace_range(abs_name_start..abs_name_end, &test_lib_name);
                            }
                        }
                    }

                    let self_dep_pattern = format!("{} = {{", lib_name);
                    if let Some(start) = cargo_toml.find(&self_dep_pattern) {
                        if let Some(end) = cargo_toml[start..].find('\n') {
                            let line_end = start + end + 1;
                            cargo_toml.replace_range(start..line_end, "");
                        }
                    }

                    if let Some(cfg) = &config {
                        let mut deps_section = String::new();
                        for (dep_name, dep_spec) in &cfg.dependencies {
                            let wildcard_pattern = format!("{} = \"*\"", dep_name);
                            if cargo_toml.contains(&wildcard_pattern) {
                                cargo_toml =
                                    cargo_toml.replace(&format!("{}\n", wildcard_pattern), "");
                            } else if cargo_toml.contains(&format!("{} =", dep_name)) {
                                continue;
                            }

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
                                        let abs_path = project_root.join(p);
                                        deps_section.push_str(&format!(
                                            "path = \"{}\", ",
                                            abs_path.display()
                                        ));
                                    }
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
                                    if deps_section.ends_with(", ") {
                                        deps_section.truncate(deps_section.len() - 2);
                                    }
                                    deps_section.push_str(" }\n");
                                }
                            }
                        }

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
            Ok(Some((test_lib_package_name, lib_output_dir)))
        }
        Err(e) => {
            println!("   {} Library compilation failed: {}", "✗".red().bold(), e);
            Ok(None)
        }
    }
}

fn generate_lib_rs_for_library(lib_output_dir: &Path) -> Result<()> {
    use std::collections::HashSet;
    use std::fs;

    let mut dir_modules = HashSet::new();
    let mut file_modules = Vec::new();

    for entry in fs::read_dir(lib_output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if path.join("mod.rs").exists() {
                    dir_modules.insert(dir_name.to_string());
                }
            }
        } else if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs") && file_name != "lib.rs" {
                    let module_name = file_name.trim_end_matches(".rs");
                    file_modules.push(module_name.to_string());
                }
            }
        }
    }

    let mut modules: Vec<String> = dir_modules.iter().cloned().collect();
    for file_module in file_modules {
        if !dir_modules.contains(&file_module) {
            modules.push(file_module);
        }
    }

    if modules.is_empty() {
        return Ok(());
    }

    modules.retain(|m| m != "lib");

    if modules.is_empty() {
        return Ok(());
    }

    modules.sort();

    let mut lib_rs = String::from("// Auto-generated library entry point\n\n");
    for module in &modules {
        lib_rs.push_str(&format!("pub mod {};\n", module));
    }
    lib_rs.push_str("\n// Re-export for convenience\n");
    for module in &modules {
        lib_rs.push_str(&format!("pub use {}::*;\n", module));
    }
    fs::write(lib_output_dir.join("lib.rs"), lib_rs)?;

    Ok(())
}

fn copy_ffi_files_to_test_library(project_root: &Path, lib_output_dir: &Path) -> Result<()> {
    use colored::*;
    use std::fs;

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
        None => return Ok(()),
    };

    println!(
        "   {} Copying FFI files from {}",
        "→".bright_blue().bold(),
        src_ffi_dir.display()
    );

    let dest_ffi_dir = lib_output_dir.join("ffi");
    fs::create_dir_all(&dest_ffi_dir)?;
    copy_ffi_files_recursive(&src_ffi_dir, &dest_ffi_dir)?;

    let lib_rs_path = lib_output_dir.join("lib.rs");
    if lib_rs_path.exists() {
        let mut lib_rs_content = fs::read_to_string(&lib_rs_path)?;
        if !lib_rs_content.contains("pub mod ffi") {
            if let Some(reexport_pos) = lib_rs_content.find("// Re-export for convenience") {
                lib_rs_content
                    .insert_str(reexport_pos, "pub mod ffi; // FFI Rust implementations\n\n");
            } else {
                lib_rs_content.push_str("\npub mod ffi; // FFI Rust implementations\n");
            }
            fs::write(&lib_rs_path, lib_rs_content)?;
        }
    }

    println!("   {} FFI files copied successfully", "✓".green().bold());

    Ok(())
}

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
            let dest_subdir = dest.join(file_name);
            fs::create_dir_all(&dest_subdir)?;
            copy_ffi_files_recursive(&path, &dest_subdir)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
            || path.extension().and_then(|s| s.to_str()) == Some("wgsl")
        {
            let dest_file = dest.join(file_name);
            fs::copy(&path, &dest_file)?;
        }
    }

    Ok(())
}

fn path_to_toml_string(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn find_windjammer_runtime_path() -> Result<PathBuf> {
    use std::env;

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir);
        let runtime_path = manifest_path.join("crates/windjammer-runtime");
        if runtime_path.join("Cargo.toml").exists() {
            return Ok(runtime_path);
        }
    }

    let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if current.join("crates/windjammer-runtime/Cargo.toml").exists() {
        return Ok(current.join("crates/windjammer-runtime"));
    }

    for _ in 0..10 {
        if let Some(parent) = current.parent() {
            if parent
                .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return Ok(parent.join("windjammer/crates/windjammer-runtime"));
            }
            if parent.join("crates/windjammer-runtime/Cargo.toml").exists() {
                return Ok(parent.join("crates/windjammer-runtime"));
            }
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    let fallback_path = PathBuf::from("./crates/windjammer-runtime");
    if fallback_path.exists() {
        if let Ok(canonical) = fallback_path.canonicalize() {
            return Ok(canonical);
        }
    }

    Ok(fallback_path)
}

fn generate_coverage_report(test_dir: &Path) -> Result<()> {
    use colored::*;
    use std::process::Command;

    let check = Command::new("cargo")
        .arg("llvm-cov")
        .arg("--version")
        .output();

    if check.is_err() || !check.unwrap().status.success() {
        println!("{} cargo-llvm-cov not found", "⚠".yellow());
        println!("Install with: cargo install cargo-llvm-cov");
        return Ok(());
    }

    let output = Command::new("cargo")
        .arg("llvm-cov")
        .arg("test")
        .arg("--html")
        .current_dir(test_dir)
        .output()?;

    if output.status.success() {
        let source_dir = test_dir.join("target/llvm-cov");
        let dest_dir = std::path::Path::new("target/llvm-cov");
        if source_dir.exists() {
            std::fs::create_dir_all(dest_dir)?;
            if let Err(e) = copy_dir_recursive(&source_dir, dest_dir) {
                println!("{} Failed to copy coverage report: {}", "⚠".yellow(), e);
            } else {
                println!("{} Coverage report generated", "✓".green());
                println!("  Open: target/llvm-cov/html/index.html");
            }
        }
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
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
