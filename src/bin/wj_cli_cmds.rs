//! Dispatch handlers for the `wj` binary (`cargo`-friendly shim crate splits parsing vs logic).

use crate::wj_cli_args::{CacheCommand, Cli, Commands};
use anyhow::Result;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::New { name, template } => {
            windjammer::cli::new::handle_new_command(&name, &template)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        Commands::Build {
            path,
            output,
            release,
            target,
            defer_drop,
            defer_drop_threshold,
            minify,
            tree_shake,
            source_maps,
            polyfills,
            v8_optimize,
            check,
            raw_errors,
            fix,
            verbose,
            quiet,
            filter_file,
            filter_type,
            library,
            module_file,
            no_cargo,
            no_lint,
            no_generate_cargo_toml,
            metadata,
            json,
            ir_shadow_validate,
        } => {
            // TODO: Pass defer_drop config to compiler
            let _ = (defer_drop, defer_drop_threshold);
            windjammer::cli::build::execute(
                &path,
                output.as_deref(),
                release,
                &target,
                windjammer::cli::build::BuildOptions {
                    minify,
                    tree_shake,
                    source_maps,
                    polyfills,
                    v8_optimize,
                },
                check,
                raw_errors,
                fix,
                verbose,
                quiet,
                filter_file.as_deref(),
                filter_type.as_deref(),
                library,
                module_file,
                !no_cargo,
                !no_lint,
                no_generate_cargo_toml,
                &metadata,
                json,
                ir_shadow_validate,
            )?;
        }
        Commands::Run {
            path,
            args,
            target,
            interpret,
            defer_drop,
            defer_drop_threshold,
        } => {
            let _ = (defer_drop, defer_drop_threshold);
            if interpret {
                interpret_file(&path, args)?;
            } else {
                windjammer::cli::run::execute(&path, &args, &target)?;
            }
        }
        Commands::Repl {} => {
            run_repl()?;
        }
        Commands::Test {
            path,
            filter,
            nocapture,
            parallel,
            json,
            library,
            module_file,
            output,
            use_build_dir,
            use_project_cargo,
            no_runtime_copy,
            copy_runtime,
            runtime_path,
            no_generate_cargo_toml,
            list,
            allow_empty,
            interpret,
            doc,
            compiled,
        } => {
            // ADR-0018: doctests run first (interpreter; `--compiled` also through Rust).
            let doc_count = run_doctests(path.as_deref(), filter.as_deref(), compiled)?;
            if doc {
                return Ok(());
            }
            if interpret {
                return interpret_tests(path.as_deref(), filter.as_deref());
            }
            let allow_empty = allow_empty || doc_count > 0;
            windjammer::run_tests_with_options(windjammer::TestRunOptions {
                path,
                filter,
                nocapture,
                parallel,
                json,
                library,
                module_file,
                output,
                use_build_dir,
                use_project_cargo,
                no_runtime_copy,
                copy_runtime,
                runtime_path,
                no_generate_cargo_toml,
                list,
                allow_empty,
            })?;
        }
        Commands::Fmt { check, rust, paths } => {
            windjammer::cli::fmt::execute(check, &paths, rust)?;
        }
        Commands::Fix { path, check, docs } => {
            windjammer::cli::fix::execute(&path, check, docs)?;
        }
        Commands::Lint { path, strict } => {
            windjammer::cli::lint::execute(&path, strict)?;
        }
        Commands::Check { path, json } => {
            windjammer::cli::check::run(path.as_deref(), json)?;
        }
        Commands::Add {
            package,
            version,
            features,
            path,
        } => {
            windjammer::cli::add::execute(
                &package,
                version.as_deref(),
                features.as_deref(),
                path.as_deref(),
            )?;
        }
        Commands::Remove { package } => {
            windjammer::cli::remove::execute(&package)?;
        }

        Commands::Update { check, force } => {
            windjammer::cli::update::execute(check, force)?;
        }
        Commands::Stats { clear, verbose: _ } => {
            if clear {
                let mut stats = windjammer::error_statistics::load_or_create_stats();
                stats.clear();
                windjammer::error_statistics::save_stats(&stats)?;
                println!("Statistics cleared!");
            } else {
                let stats = windjammer::error_statistics::load_or_create_stats();
                println!("{}", stats.format());
            }
        }
        Commands::Docs { query, limit, json, output, format } => {
            if query.is_empty() {
                cmd_generate_docs(&output, &format)?
            } else {
                windjammer::agent_index::docs_search::run_cli(&query.join(" "), limit, json)
            }
        }
        Commands::ExplainOwnership {
            file,
            function,
            json,
        } => windjammer::cli::explain_ownership::execute(&file, function.as_deref(), json)?,
        Commands::Explain { code } => cmd_explain(&code)?,
        Commands::AgentIndex { output } => {
            windjammer::agent_index::generate_agent_index(&output)?;
        }
        Commands::ShaderCompile { input, output } => cmd_shader_compile(&input, output.as_deref())?,
        Commands::Errors { file, output } => cmd_errors_tui(&file, &output)?,
        Commands::ValidateWjsl { path } => cmd_validate_wjsl(&path)?,
        Commands::Clean { all } => {
            windjammer::cli::clean::execute(all)?;
        }
        Commands::Cache { action } => match action {
            CacheCommand::Path { verify } => {
                windjammer::cli::cache::execute_path(verify)?;
            }
            CacheCommand::Status => {
                windjammer::cli::cache::execute_status()?;
            }
            CacheCommand::Prune { aggressive } => {
                windjammer::cli::cache::execute_prune(aggressive)?;
            }
            CacheCommand::Setup => {
                windjammer::cli::cache::execute_setup()?;
            }
        },
        Commands::SelfInstall => {
            windjammer::cli::self_install::execute()?;
        }
        Commands::Plugin(plugin_args) => {
            if plugin_args.is_empty() {
                anyhow::bail!("Plugin name required. Usage: wj <plugin> <args>");
            }

            let plugin_name = &plugin_args[0];
            let args = &plugin_args[1..];

            let exit_code = windjammer::plugin::execute_plugin(plugin_name, args)?;
            std::process::exit(exit_code);
        }
    }

    Ok(())
}

fn cmd_generate_docs(output: &std::path::Path, format: &str) -> Result<()> {
    use std::fs;

    println!("Generating error documentation...");

    let catalog = windjammer::error_catalog::ErrorCatalog::new();

    fs::create_dir_all(output)?;

    match format {
        "html" => {
            let html = catalog.generate_html();
            let path = output.join("index.html");
            fs::write(&path, html)?;
            println!("✓ Generated HTML documentation: {}", path.display());
        }
        "markdown" | "md" => {
            let md = catalog.generate_markdown();
            let path = output.join("errors.md");
            fs::write(&path, md)?;
            println!("✓ Generated Markdown documentation: {}", path.display());
        }
        "json" => {
            let path = output.join("errors.json");
            catalog.save_json(&path)?;
            println!("✓ Generated JSON catalog: {}", path.display());
        }
        _ => {
            anyhow::bail!(
                "Unknown format: {}. Use 'html', 'markdown', or 'json'",
                format
            );
        }
    }

    println!("\n📚 Error documentation generated successfully!");
    println!("   {} errors documented", catalog.errors.len());
    println!("   {} categories", catalog.categories.len());

    Ok(())
}

fn cmd_explain(code: &str) -> Result<()> {
    let registry = windjammer::error_codes::get_registry();

    if let Some(wj_code) = registry.get(code) {
        println!("{}", registry.format_explanation(&wj_code.code));
    } else if let Some(wj_code) = registry.map_rust_code(code) {
        println!("{}", registry.format_explanation(&wj_code.code));
    } else {
        use colored::*;
        println!("{}", format!("Error code '{}' not found", code).red());
        println!("\n{}", "Available Windjammer error codes:".yellow());

        let mut codes: Vec<_> = registry.all_codes();
        codes.sort_by_key(|c| c.code.as_str());

        for error_code in codes {
            println!("  {} - {}", error_code.code.cyan(), error_code.title);
        }

        println!("\n{}", "Usage:".yellow());
        println!("  wj explain WJ0001");
        println!("  wj explain E0425  (Rust error code)");
    }

    Ok(())
}

fn cmd_shader_compile(input: &std::path::Path, output: Option<&std::path::Path>) -> Result<()> {
    let source = std::fs::read_to_string(input)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", input.display(), e))?;
    let wgsl = windjammer::wjsl::compile_legacy_shader(&source)?;
    match output {
        Some(path) => {
            std::fs::write(path, &wgsl)?;
            println!("Compiled {} -> {}", input.display(), path.display());
        }
        None => {
            print!("{}", wgsl);
        }
    }
    Ok(())
}

fn cmd_errors_tui(file: &std::path::Path, output: &std::path::Path) -> Result<()> {
    use colored::*;

    println!("Building and checking {}...", file.display());

    let build_options = windjammer::cli::build::BuildOptions {
        minify: false,
        tree_shake: false,
        source_maps: true,
        polyfills: false,
        v8_optimize: false,
    };

    windjammer::cli::build::execute(
        file,
        Some(output),
        false,
        "rust",
        build_options,
        true,
        false,
        false,
        false,
        true,
        None,
        None,
        false,
        false,
        false,
        true,
        false,
        &[],
        false,
        false,
    )
    .ok();

    println!(
        "{}",
        "✓ TUI mode coming soon! Use 'wj build --check' for now.".yellow()
    );
    println!(
        "{}",
        "  The TUI infrastructure is ready, just needs diagnostics API.".dimmed()
    );

    Ok(())
}

fn cmd_validate_wjsl(path: &std::path::Path) -> Result<()> {
    use colored::*;
    use std::path::Path;

    println!("{}", "Validating WJSL shaders...".bold());

    let mut errors = 0u32;
    let mut validated = 0u32;

    fn find_wjsl_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    find_wjsl_files(&p, files);
                } else if p.extension().is_some_and(|e| e == "wjsl") {
                    files.push(p);
                }
            }
        }
    }

    let mut wjsl_files = Vec::new();
    find_wjsl_files(path, &mut wjsl_files);
    wjsl_files.sort();

    for file in &wjsl_files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  {} {}: {}", "✗".red(), file.display(), e);
                errors += 1;
                continue;
            }
        };

        let base_dir = file.parent().unwrap_or(Path::new("."));
        match windjammer::wjsl::transpile_wjsl_with_includes(&source, base_dir) {
            Ok(_) => {
                validated += 1;
            }
            Err(e) => {
                eprintln!("  {} {}: {}", "✗".red(), file.display(), e);
                errors += 1;
            }
        }
    }

    if errors > 0 {
        eprintln!(
            "\n{} {} shader(s) validated, {} error(s)",
            "WJSL validation failed:".red().bold(),
            validated,
            errors
        );
        std::process::exit(1);
    } else {
        println!(
            "  {} {} shader(s) validated successfully",
            "✓".green(),
            validated
        );
    }

    Ok(())
}

/// Interpret a .wj file or project directory directly (no cargo, no rustc).
fn interpret_file(file: &std::path::Path, args: Vec<String>) -> Result<()> {
    windjammer::cli_execution::report_build_lints(file);
    let code = windjammer::interpreter::runner::run_path(file, args);
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

/// Run the Windjammerscript REPL (state persists between inputs).
fn run_repl() -> Result<()> {
    windjammer::interpreter::repl::run_repl_stdio();
    Ok(())
}

/// `wj test --interpret`: run `@test` / `test_*` functions without compiling.
/// Run the doctests under `path`; exits non-zero on a failure. Returns how many ran.
fn run_doctests(path: Option<&std::path::Path>, filter: Option<&str>, compiled: bool) -> Result<usize> {
    let root = path.unwrap_or_else(|| std::path::Path::new("."));
    let exe = std::env::current_exe()?;
    let results = windjammer::doctest::run::run_doctests(root, filter, compiled.then_some(exe.as_path()));
    if results.is_empty() {
        return Ok(0);
    }
    if windjammer::doctest::run::report(&results) > 0 {
        std::process::exit(1);
    }
    Ok(results.len())
}

fn interpret_tests(path: Option<&std::path::Path>, filter: Option<&str>) -> Result<()> {
    let root = path.unwrap_or_else(|| std::path::Path::new("."));
    let results = windjammer::interpreter::runner::run_tests(root, filter);
    let code = windjammer::interpreter::runner::report_tests(&results);
    if results.is_empty() {
        eprintln!("no test files (*_test.wj) found under {}", root.display());
        std::process::exit(1);
    }
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}
