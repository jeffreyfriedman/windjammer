//! Dispatch handlers for the `wj` binary (`cargo`-friendly shim crate splits parsing vs logic).

use crate::wj_cli_args::{Cli, Commands};
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
                interpret_file(&path)?;
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
        } => {
            windjammer::run_tests(
                path.as_deref(),
                filter.as_deref(),
                nocapture,
                parallel,
                json,
            )?;
        }
        Commands::Fmt { check } => {
            windjammer::cli::fmt::execute(check)?;
        }
        Commands::Lint { path, strict } => {
            windjammer::cli::lint::execute(&path, strict)?;
        }
        Commands::Check => {
            windjammer::cli::check::execute()?;
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
        Commands::Docs { output, format } => cmd_generate_docs(&output, &format)?,
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
    let wgsl = windjammer::shader::compile_shader(&source)?;
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

/// Interpret a .wj file directly using the Windjammerscript tree-walking interpreter.
fn interpret_file(file: &std::path::Path) -> Result<()> {
    use colored::*;

    if !file.exists() {
        anyhow::bail!("File not found: {:?}", file);
    }
    if file.extension().is_none_or(|ext| ext != "wj") {
        anyhow::bail!("File must have .wj extension: {:?}", file);
    }

    println!(
        "{} {:?} (Windjammerscript interpreter)",
        "Interpreting".green().bold(),
        file
    );

    let source = std::fs::read_to_string(file)?;
    let mut lex = windjammer::lexer::Lexer::new(&source);
    let tokens = lex.tokenize_with_locations();
    let mut parse = windjammer::parser::Parser::new_with_source(
        tokens,
        file.to_string_lossy().to_string(),
        source.clone(),
    );
    let program = parse
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    let mut interp = windjammer::interpreter::Interpreter::new();
    match interp.run(&program) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{} {}", "Runtime error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

/// Run the Windjammerscript REPL
fn run_repl() -> Result<()> {
    use colored::*;
    use std::io::{BufRead, Write};

    println!(
        "{} {} {}",
        "Windjammerscript".cyan().bold(),
        "REPL".white().bold(),
        "(type 'exit' or Ctrl-D to quit)".dimmed()
    );
    println!(
        "{}",
        "Tip: Any code here can be compiled with `wj build` for production.".dimmed()
    );
    println!();

    let stdin = std::io::stdin();
    let mut accumulated_source = String::new();
    let mut line_buffer = String::new();
    let mut in_block = false;
    let mut brace_depth: i32 = 0;

    loop {
        if in_block {
            print!("{} ", "...".dimmed());
        } else {
            print!("{} ", "wj>".green().bold());
        }
        std::io::stdout().flush()?;

        line_buffer.clear();
        let bytes_read = stdin.lock().read_line(&mut line_buffer)?;
        if bytes_read == 0 {
            println!();
            break;
        }

        let line = line_buffer.trim_end();

        if line == "exit" || line == "quit" {
            break;
        }

        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                _ => {}
            }
        }

        accumulated_source.push_str(line);
        accumulated_source.push('\n');

        if brace_depth > 0 {
            in_block = true;
            continue;
        }

        in_block = false;
        brace_depth = 0;

        let source = if accumulated_source.contains("fn main()") {
            accumulated_source.clone()
        } else {
            format!("fn main() {{\n{}\n}}", accumulated_source)
        };

        let mut lex = windjammer::lexer::Lexer::new(&source);
        let tokens = lex.tokenize_with_locations();
        let mut parse =
            windjammer::parser::Parser::new_with_source(tokens, "repl".to_string(), source.clone());

        match parse.parse() {
            Ok(program) => {
                let mut interp = windjammer::interpreter::Interpreter::new();
                match interp.run(&program) {
                    Ok(val) => {
                        let display = val.to_display_string();
                        if display != "()" {
                            println!("{}", display);
                        }
                    }
                    Err(e) => {
                        eprintln!("{} {}", "Error:".red().bold(), e);
                    }
                }
            }
            Err(e) => {
                eprintln!("{} {}", "Parse error:".red().bold(), e);
            }
        }

        accumulated_source.clear();
    }

    println!("{}", "Goodbye!".dimmed());
    Ok(())
}
