// wj build - Build Windjammer project
//
// This command compiles Windjammer source files to Rust.

use anyhow::Result;
use colored::*;
use std::path::Path;

/// `--metadata NAME=PATH` is repeatable; one flag may also list `NAME=PATH,NAME=PATH`.
fn parse_external_metadata_args(metadata: &[String]) -> Vec<(&str, &Path)> {
    metadata
        .iter()
        .flat_map(|s| s.split(','))
        .filter_map(|part| {
            let part = part.trim();
            let (name, path_str) = part.split_once('=')?;
            let name = name.trim();
            let path_str = path_str.trim();
            if name.is_empty() || path_str.is_empty() {
                return None;
            }
            Some((name, Path::new(path_str)))
        })
        .collect()
}

/// Build options for JavaScript target
pub struct BuildOptions {
    pub minify: bool,
    pub tree_shake: bool,
    pub source_maps: bool,
    pub polyfills: bool,
    pub v8_optimize: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn execute(
    path: &Path,
    output: Option<&Path>,
    release: bool,
    target_str: &str,
    options: BuildOptions,
    check: bool,
    raw_errors: bool,
    fix: bool,
    verbose: bool,
    quiet: bool,
    filter_file: Option<&Path>,
    filter_type: Option<&str>,
    library: bool,
    module_file: bool,
    run_cargo: bool,
    enable_lint: bool,
    no_generate_cargo_toml: bool,
    metadata: &[String],
    json_output: bool,
    ir_shadow_validate: bool,
) -> Result<()> {
    // Driver edge: the one place the environment is turned into explicit configuration.
    let config = crate::compiler_config::CompilerConfig::from_env()
        .with_shadow_validate_strict(ir_shadow_validate)
        .with_skip_cargo_toml_generation(no_generate_cargo_toml);
    let output_dir = output.unwrap_or_else(|| Path::new("./build"));

    if json_output {
        return build_with_json_output(
            path,
            output_dir,
            target_str,
            check,
            library,
            module_file,
            enable_lint,
            &config,
            metadata,
        );
    }

    println!(
        "{} Windjammer project from {:?} (target: {})",
        "Building".green().bold(),
        path,
        target_str
    );
    println!("Output: {:?}", output_dir);

    // Parse target string
    let target = match target_str.to_lowercase().as_str() {
        "rust" => crate::CompilationTarget::Rust,
        "javascript" | "js" | "typescript" | "ts" | "go" | "golang" => {
            use crate::codegen::backend::{CodegenConfig, Target};
            let target: Target = target_str.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let config = CodegenConfig {
                target,
                output_dir: output_dir.to_path_buf(),
                minify: options.minify,
                tree_shake: options.tree_shake,
                polyfills: options.polyfills,
                v8_optimize: options.v8_optimize,
                ..Default::default()
            };
            return build_portable(path, &config, options.source_maps);
        }
        "wasm" | "webassembly" => crate::CompilationTarget::Wasm,
        "wgsl" => {
            // Use WGSL backend for GPU shaders
            use crate::codegen::backend::{CodegenConfig, Target};
            let config = CodegenConfig {
                target: Target::Wgsl,
                output_dir: output_dir.to_path_buf(),
                ..Default::default()
            };
            return build_wgsl(path, &config);
        }
        _ => {
            anyhow::bail!(
                "Unknown target: {}. Use 'rust', 'go', 'javascript' (js), 'typescript' (ts), 'wasm', or 'wgsl'",
                target_str
            );
        }
    };

    let external_metadata = parse_external_metadata_args(metadata);

    crate::build_project_with_config(
        path,
        output_dir,
        target,
        enable_lint,
        library,
        &external_metadata,
        &config,
    )?;

    crate::build_utils::apply_library_build_post_steps(path, output_dir, library, module_file)?;

    println!("\n{} Transpilation complete!", "Success!".green().bold());

    // Run cargo check if requested
    if check {
        check_with_cargo(
            output_dir,
            raw_errors,
            fix,
            verbose,
            quiet,
            filter_file,
            filter_type,
        )?;
    }

    // Run cargo build automatically for Rust target (unless disabled)
    if (target_str == "rust") && run_cargo && !check {
        let profile = if release { "release" } else { "dev" };
        println!("\n{} Running cargo build ({})...", "⚙️".bold(), profile);

        let mut cargo = std::process::Command::new("cargo");
        cargo.arg("build");
        if release {
            cargo.arg("--release");
        }
        let cargo_status = cargo.current_dir(output_dir).status();

        match cargo_status {
            Ok(status) if status.success() => {
                println!("{} Cargo build complete!", "✅".green().bold());
                println!(
                    "\n{} Your Windjammer project is ready!",
                    "Success!".green().bold()
                );
                println!("Run your project with:");
                if release {
                    println!("  cd {:?} && cargo run --release", output_dir);
                } else {
                    println!("  cd {:?} && cargo run", output_dir);
                }
            }
            Ok(status) => {
                println!(
                    "{} Cargo build failed with exit code: {:?}",
                    "❌".red().bold(),
                    status.code()
                );
                println!("\nYou can:");
                println!("  • Fix the errors and run: cargo build");
                println!("  • Or use: wj build --no-run-cargo to skip cargo build");
                return Err(anyhow::anyhow!("Cargo build failed"));
            }
            Err(e) => {
                println!("{} Failed to run cargo: {}", "❌".red().bold(), e);
                println!("\nMake sure cargo is installed:");
                println!("  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh");
                return Err(anyhow::anyhow!("Failed to execute cargo: {}", e));
            }
        }
    } else if !run_cargo && target_str == "rust" {
        println!(
            "\n{} Transpilation complete (cargo build skipped)!",
            "Success!".green().bold()
        );
        println!("Run cargo build manually:");
        println!("  cd {:?} && cargo build", output_dir);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_with_json_output(
    path: &Path,
    output_dir: &Path,
    target_str: &str,
    _check: bool,
    library: bool,
    _module_file: bool,
    enable_lint: bool,
    config: &crate::compiler_config::CompilerConfig,
    metadata: &[String],
) -> Result<()> {
    use crate::cli::json_diagnostics::{JsonCompilationOutput, JsonDiagnostic, Severity};

    let external_metadata = parse_external_metadata_args(metadata);

    let target = match target_str.to_lowercase().as_str() {
        "rust" => crate::CompilationTarget::Rust,
        _ => {
            let mut output = JsonCompilationOutput::new_failure();
            output.add_error(format!(
                "JSON output mode only supports Rust target, got: {}",
                target_str
            ));
            output.emit_to_stdout();
            return Ok(());
        }
    };

    match crate::build_project_with_config(
        path,
        output_dir,
        target,
        enable_lint,
        library,
        &external_metadata,
        config,
    ) {
        Ok(()) => {
            let mut output = JsonCompilationOutput::new_success();
            if library {
                output.add_warning("Library mode: main() functions stripped".to_string());
            }
            output.emit_to_stdout();
        }
        Err(e) => {
            let mut output = JsonCompilationOutput::new_failure();
            output.diagnostics.push(JsonDiagnostic {
                code: None,
                severity: Severity::Error,
                message: format!("{}", e),
                span: None,
                maturity: None,
                repair_hint: None,
            });
            output.emit_to_stdout();
        }
    }

    Ok(())
}

/// Resolve the single `.wj` source of a portable-backend build (a file, or a directory with `main.wj`).
fn portable_source(path: &Path) -> Result<std::path::PathBuf> {
    if path.is_file() {
        return Ok(path.to_path_buf());
    }
    let main = path.join("main.wj");
    if main.is_file() {
        return Ok(main);
    }
    anyhow::bail!("expected a .wj file or a directory containing main.wj (multi-file projects are not supported by the Go/JS/TS backends yet): {}", path.display())
}

/// Build for Go / JavaScript / TypeScript: writes a ready-to-run directory
/// (`main.go`+`go.mod`+`wj_core.go`, or `main.mjs`/`main.ts`+`wj_core.mjs`+`package.json`).
fn build_portable(path: &Path, config: &crate::codegen::backend::CodegenConfig, source_maps: bool) -> Result<()> {
    use crate::codegen::backend::Target;
    use std::fs;

    if source_maps {
        anyhow::bail!("--source-maps is not supported by the Go/JS/TS backends (the previous implementation emitted placeholder maps)");
    }
    let file = portable_source(path)?;
    let source = fs::read_to_string(&file)?;
    let (parser, program) = crate::compiler::parse_wj_source(&file, &source)?;
    crate::compiler::emit_parser_warnings(&parser)?;
    let output = crate::codegen::generate(&program, config.target, Some(config.clone()))?;

    fs::create_dir_all(&config.output_dir)?;
    let entry = match config.target {
        Target::Go => "main.go",
        Target::TypeScript => "main.ts",
        _ => "main.mjs",
    };
    let entry_path = config.output_dir.join(entry);
    fs::write(&entry_path, &output.source)?;
    println!("  {} {:?}", "Generated".green(), entry_path);
    for (filename, content) in &output.additional_files {
        let file_path = config.output_dir.join(filename);
        fs::write(&file_path, content)?;
        println!("  {} {:?}", "Generated".green(), file_path);
    }
    println!("\n{} {} project is ready!", "Success!".green().bold(), config.target.as_str());
    let dir = &config.output_dir;
    match config.target {
        Target::Go => println!("Run with:\n  cd {:?} && go run .", dir),
        _ => println!("Run with:\n  cd {:?} && node {entry}", dir),
    }
    Ok(())
}

fn build_wgsl(path: &Path, config: &crate::codegen::backend::CodegenConfig) -> Result<()> {
    use std::fs;

    // Read source file
    let source = fs::read_to_string(path)?;

    // Detect .wjsl files - use WJSL transpiler (RFC syntax: @vertex, @fragment, etc.)
    // Note: .wjsl uses array<T, N> syntax; main Windjammer parser expects .wj syntax
    let (wgsl_source, additional_files) =
        if path.extension().and_then(|e| e.to_str()) == Some("wjsl") {
            let wgsl = crate::wjsl::transpile_wjsl(&source)?;
            (wgsl, Vec::new())
        } else {
            // Use Windjammer parser for .wj files
            use crate::codegen;
            use crate::lexer::Lexer;
            use crate::parser::Parser;

            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize_with_locations();
            let mut parser = Parser::new(tokens);
            let program = parser
                .parse()
                .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

            let output = codegen::generate(&program, config.target, Some(config.clone()))?;
            (output.source, output.additional_files)
        };

    // Create output directory
    fs::create_dir_all(&config.output_dir)?;

    // Determine output filename from input
    let input_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("shader");
    let output_path = config.output_dir.join(format!("{}.wgsl", input_stem));

    // Write WGSL output
    fs::write(&output_path, &wgsl_source)?;
    println!("  {} {:?}", "Generated".green(), output_path);

    // Write additional files if any
    for (filename, content) in &additional_files {
        let file_path = config.output_dir.join(filename);
        fs::write(&file_path, content)?;
        println!("  {} {:?}", "Generated".green(), file_path);
    }

    println!(
        "\n{} WGSL shader compilation complete!",
        "Success!".green().bold()
    );

    Ok(())
}

/// Run cargo build on the generated Rust code and display errors with source mapping
fn check_with_cargo(
    output_dir: &Path,
    show_raw_errors: bool,
    apply_fixes: bool,
    verbose: bool,
    quiet: bool,
    filter_file: Option<&Path>,
    filter_type: Option<&str>,
) -> Result<()> {
    use std::process::Command;

    // Error recovery loop: try up to 3 times if auto-fix is enabled
    let max_attempts = if apply_fixes { 3 } else { 1 };
    let mut last_error_count = 0;

    for attempt in 1..=max_attempts {
        if attempt > 1 {
            println!(
                "\n{} Retry {} of {}...",
                "Retrying".yellow().bold(),
                attempt,
                max_attempts
            );
        } else {
            println!("\n{} Rust compilation...", "Checking".cyan().bold());
        }

        let output = Command::new("cargo")
            .arg("build")
            .arg("--message-format=json")
            .current_dir(output_dir)
            .output()?;

        if output.status.success() {
            if attempt > 1 {
                println!(
                    "{} All errors fixed after {} attempt(s)!",
                    "Success!".green().bold(),
                    attempt
                );
            } else {
                println!("{} No Rust compilation errors!", "Success!".green().bold());
            }
            return Ok(());
        }

        // Combine stderr and stdout (cargo outputs to both)
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined_output = format!("{}{}", stderr, stdout);

        // If raw errors requested, show them and exit
        if show_raw_errors {
            println!("{} Rust compilation errors (raw):", "Error:".red().bold());
            println!("{}", combined_output);
            return Err(anyhow::anyhow!("Rust compilation failed"));
        }

        // Load all source maps from the output directory
        let source_maps = load_source_maps(output_dir)?;

        // Create error mapper with merged source maps
        let error_mapper = crate::error_mapper::ErrorMapper::new(source_maps);

        // Map rustc output to Windjammer diagnostics
        let mut wj_diagnostics = error_mapper.map_rustc_output(&combined_output);

        if wj_diagnostics.is_empty() {
            // Fallback: show raw output if we couldn't parse any diagnostics
            println!(
                "{} Could not parse Rust compilation errors. Showing raw output:",
                "Warning:".yellow().bold()
            );
            println!("{}", combined_output);
            return Err(anyhow::anyhow!("Rust compilation failed"));
        }

        // Apply filters
        if let Some(file_filter) = filter_file {
            wj_diagnostics.retain(|d| d.location.file == file_filter);
        }

        if let Some(type_filter) = filter_type {
            let filter_lower = type_filter.to_lowercase();
            wj_diagnostics.retain(|d| {
                matches!(
                    (&d.level, filter_lower.as_str()),
                    (crate::error_mapper::DiagnosticLevel::Error, "error")
                        | (crate::error_mapper::DiagnosticLevel::Warning, "warning")
                )
            });
        }

        // Group diagnostics by file
        let mut diagnostics_by_file: std::collections::HashMap<_, Vec<_>> =
            std::collections::HashMap::new();
        for diagnostic in &wj_diagnostics {
            diagnostics_by_file
                .entry(diagnostic.location.file.clone())
                .or_insert_with(Vec::new)
                .push(diagnostic);
        }

        // Count errors and warnings
        last_error_count = wj_diagnostics
            .iter()
            .filter(|d| matches!(d.level, crate::error_mapper::DiagnosticLevel::Error))
            .count();

        let warning_count = wj_diagnostics
            .iter()
            .filter(|d| matches!(d.level, crate::error_mapper::DiagnosticLevel::Warning))
            .count();

        // Display summary
        if quiet {
            // Quiet mode: only show counts
            if last_error_count > 0 {
                println!(
                    "\n{} {} error(s), {} warning(s)",
                    "Compilation failed:".red().bold(),
                    last_error_count,
                    warning_count
                );
            } else {
                println!(
                    "\n{} {} warning(s)",
                    "Compilation succeeded with warnings:".yellow().bold(),
                    warning_count
                );
            }
        } else {
            // Normal or verbose mode: show detailed output
            println!(
                "\n{} {} error(s), {} warning(s) found:\n",
                "Compilation failed:".red().bold(),
                last_error_count,
                warning_count
            );

            // Display diagnostics grouped by file
            for (file, file_diagnostics) in &diagnostics_by_file {
                println!("{} {}:", "In file".cyan().bold(), file.display());
                println!();

                for diagnostic in file_diagnostics {
                    let formatted = if verbose {
                        // Verbose mode: include all details
                        diagnostic.format()
                    } else {
                        // Normal mode: format as usual
                        diagnostic.format()
                    };
                    let colorized = colorize_diagnostic(&formatted, &diagnostic.level);
                    println!("{}", colorized);
                    println!(); // Blank line between errors
                }
            }
        }

        // Apply fixes if requested and not on last attempt
        if apply_fixes && attempt < max_attempts {
            println!("\n{} Applying automatic fixes...", "Fixing".green().bold());

            let fixes: Vec<_> = wj_diagnostics.iter().filter_map(|d| d.get_fix()).collect();

            if fixes.is_empty() {
                println!("{} No automatic fixes available", "Info:".cyan());
                // No fixes available, no point in retrying
                break;
            } else {
                println!("{} Found {} fixable error(s)", "Info:".cyan(), fixes.len());

                let applicator = crate::auto_fix::FixApplicator::new();
                match applicator.apply_fixes(&fixes) {
                    Ok(_) => {
                        println!(
                            "\n{} Applied {} fix(es)!",
                            "Success!".green().bold(),
                            fixes.len()
                        );
                        // Continue to next iteration to retry compilation
                        continue;
                    }
                    Err(e) => {
                        println!(
                            "{} Failed to apply some fixes: {}",
                            "Warning:".yellow().bold(),
                            e
                        );
                        // Failed to apply fixes, no point in retrying
                        break;
                    }
                }
            }
        } else {
            // No auto-fix or last attempt, break out of loop
            break;
        }
    }

    // If we get here, compilation failed
    Err(anyhow::anyhow!(
        "Rust compilation failed with {} error(s)",
        last_error_count
    ))
}

/// Load and merge all source maps from the output directory
fn load_source_maps(output_dir: &Path) -> Result<crate::source_map::SourceMap> {
    use std::fs;

    let mut merged_map = crate::source_map::SourceMap::new();
    let mut map_count = 0;
    let mut mapping_count = 0;

    // Find all .rs.map files in the output directory
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("map") {
                // Check if this is a .rs.map file (not just any .map file)
                if let Some(stem) = path.file_stem() {
                    if let Some(stem_str) = stem.to_str() {
                        if !stem_str.ends_with(".rs") {
                            continue;
                        }
                    }
                }

                // Load this source map
                if let Ok(map) = crate::source_map::SourceMap::load_from_file(&path) {
                    // Get the corresponding .rs file path
                    let rust_file = path.with_extension("").with_extension("rs");

                    // Merge all mappings from this source map
                    let mappings = map.mappings_for_rust_file(&rust_file);
                    for mapping in mappings {
                        merged_map.add_mapping(
                            mapping.rust_file.clone(),
                            mapping.rust_line,
                            mapping.rust_column,
                            mapping.wj_file.clone(),
                            mapping.wj_line,
                            mapping.wj_column,
                        );
                        mapping_count += 1;
                    }
                    map_count += 1;
                }
            }
        }
    }

    if map_count == 0 {
        println!(
            "{} No source maps found. Error locations may be inaccurate.",
            "Warning:".yellow().bold()
        );
    } else {
        println!(
            "{} Loaded {} source map(s) with {} mapping(s)",
            "Info:".cyan(),
            map_count,
            mapping_count
        );
    }

    Ok(merged_map)
}

/// Colorize diagnostic output based on level
fn colorize_diagnostic(text: &str, _level: &crate::error_mapper::DiagnosticLevel) -> String {
    use colored::*;

    let mut result = String::new();
    for line in text.lines() {
        if line.starts_with("error:") || line.starts_with("Error:") {
            result.push_str(&line.red().bold().to_string());
        } else if line.starts_with("warning:") || line.starts_with("Warning:") {
            result.push_str(&line.yellow().bold().to_string());
        } else if line.starts_with("help:") || line.starts_with("Help:") {
            result.push_str(&line.cyan().to_string());
        } else if line.starts_with("note:") || line.starts_with("Note:") {
            result.push_str(&line.blue().to_string());
        } else if line.contains("^") {
            // Error pointer line
            result.push_str(&line.red().to_string());
        } else if line.starts_with("  -->") || line.starts_with(" -->") {
            // Location line
            result.push_str(&line.cyan().to_string());
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Remove trailing newline if present
    if result.ends_with('\n') {
        result.pop();
    }

    result
}
