// wj - Unified Windjammer CLI
//
// This is the new unified CLI for Windjammer that provides a simpler,
// more cohesive development experience.
//
// Usage:
//   wj new <name>        Create a new project
//   wj build <file>      Build Windjammer project
//   wj run <file>        Compile and execute
//   wj test              Run tests
//   wj fmt               Format code
//   wj lint              Run linter
//   wj check             Type check without building

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "wj")]
#[command(about = "Windjammer - A simple language that transpiles to Rust", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Windjammer project
    New {
        /// Project name
        #[arg(value_name = "NAME")]
        name: String,

        /// Project template (cli, web, lib, wasm)
        #[arg(short, long, default_value = "cli")]
        template: String,
    },

    /// Build a Windjammer project
    Build {
        /// Path to .wj file or directory
        #[arg(value_name = "FILE")]
        path: PathBuf,

        /// Output directory (default: ./build)
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,

        /// Build in release mode
        #[arg(short, long)]
        release: bool,

        /// Target platform (rust, javascript, wasm)
        #[arg(short, long, value_name = "TARGET", default_value = "rust")]
        target: String,

        /// Defer drop optimization mode (auto, always, never)
        #[arg(long, value_name = "MODE", default_value = "auto")]
        defer_drop: String,

        /// Defer drop threshold in bytes (default: 102400 = 100KB)
        #[arg(long, value_name = "BYTES")]
        defer_drop_threshold: Option<usize>,

        /// Minify JavaScript output (JS target only)
        #[arg(long)]
        minify: bool,

        /// Enable tree shaking (dead code elimination)
        #[arg(long)]
        tree_shake: bool,

        /// Generate source maps
        #[arg(long)]
        source_maps: bool,

        /// Include polyfills for older browsers (JS target only)
        #[arg(long)]
        polyfills: bool,

        /// Apply V8 optimizations (JS target only)
        #[arg(long)]
        v8_optimize: bool,

        /// Run cargo build after transpilation and show errors
        #[arg(long)]
        check: bool,

        /// Show raw Rust errors instead of translated Windjammer errors
        #[arg(long)]
        raw_errors: bool,

        /// Automatically apply fixes for fixable errors
        #[arg(long)]
        fix: bool,

        /// Show verbose error output (includes all notes and suggestions)
        #[arg(short, long)]
        verbose: bool,

        /// Show only error count (suppress detailed output)
        #[arg(short, long)]
        quiet: bool,

        /// Filter errors by file path
        #[arg(long, value_name = "PATH")]
        filter_file: Option<PathBuf>,

        /// Filter errors by type (error, warning)
        #[arg(long, value_name = "TYPE")]
        filter_type: Option<String>,

        /// Library mode: exclude test main() functions from output
        #[arg(long)]
        library: bool,

        /// Auto-generate mod.rs with pub mod declarations and re-exports
        #[arg(long)]
        module_file: bool,

        /// Skip cargo build after transpilation (transpile only)
        #[arg(long)]
        no_cargo: bool,
    },

    /// Compile and run a Windjammer file
    Run {
        /// Path to .wj file
        #[arg(value_name = "FILE")]
        path: PathBuf,

        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Target platform (rust, javascript, wasm)
        #[arg(short, long, value_name = "TARGET", default_value = "rust")]
        target: String,

        /// Interpret directly (Windjammerscript mode) — no compilation, instant execution.
        /// Same .wj source can later be compiled with `wj build` for production.
        #[arg(long)]
        interpret: bool,

        /// Defer drop optimization mode (auto, always, never)
        #[arg(long, value_name = "MODE", default_value = "auto")]
        defer_drop: String,

        /// Defer drop threshold in bytes (default: 102400 = 100KB)
        #[arg(long, value_name = "BYTES")]
        defer_drop_threshold: Option<usize>,
    },

    /// Start the Windjammerscript REPL (interactive interpreter)
    Repl {},

    /// Run tests
    Test {
        /// Directory or file containing tests (defaults to current directory)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Run only tests matching this pattern
        #[arg(short, long)]
        filter: Option<String>,

        /// Show output from passing tests
        #[arg(long)]
        nocapture: bool,

        /// Run tests in parallel (default: true)
        #[arg(long, default_value = "true")]
        parallel: bool,

        /// Output results as JSON for tooling
        #[arg(long)]
        json: bool,
    },

    /// Format Windjammer code
    Fmt {
        /// Check formatting without applying changes
        #[arg(long)]
        check: bool,
    },

    /// Run linter (clippy)
    Lint {
        /// Automatically fix warnings
        #[arg(long)]
        fix: bool,
    },

    /// Type check without building
    Check,

    /// Add a dependency to wj.toml
    Add {
        /// Package name
        #[arg(value_name = "PACKAGE")]
        package: String,

        /// Package version
        #[arg(short, long)]
        version: Option<String>,

        /// Comma-separated list of features
        #[arg(short, long)]
        features: Option<String>,

        /// Local path dependency
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Remove a dependency from wj.toml
    Remove {
        /// Package name
        #[arg(value_name = "PACKAGE")]
        package: String,
    },

    /// Update Windjammer to the latest version
    Update {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,

        /// Force reinstall even if already up to date
        #[arg(long)]
        force: bool,
    },

    /// Show error statistics
    Stats {
        /// Clear statistics
        #[arg(long)]
        clear: bool,

        /// Show detailed statistics
        #[arg(short, long)]
        verbose: bool,
    },

    /// Interactive error navigator (TUI)
    Errors {
        /// File to check
        file: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "./build")]
        output: PathBuf,
    },

    /// Generate error documentation
    Docs {
        /// Output directory for generated docs
        #[arg(short, long, default_value = "./docs/errors")]
        output: PathBuf,

        /// Format (html, markdown, json)
        #[arg(short, long, default_value = "html")]
        format: String,
    },

    /// Explain an error code
    Explain {
        /// Error code to explain (e.g., WJ0001, E0425)
        code: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

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
        } => {
            // TODO: Pass defer_drop config to compiler
            // For now, just ignore these flags - defer drop is always auto
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
                !no_cargo, // run_cargo = !no_cargo
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
            // TODO: Pass defer_drop config to compiler
            // For now, just ignore these flags - defer drop is always auto
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
        Commands::Lint { fix } => {
            windjammer::cli::lint::execute(fix)?;
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
        Commands::Docs { output, format } => {
            use std::fs;

            println!("Generating error documentation...");

            let catalog = windjammer::error_catalog::ErrorCatalog::new();

            // Create output directory
            fs::create_dir_all(&output)?;

            match format.as_str() {
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
        }
        Commands::Explain { code } => {
            let registry = windjammer::error_codes::get_registry();

            // Try as Windjammer code first
            if let Some(wj_code) = registry.get(&code) {
                println!("{}", registry.format_explanation(&wj_code.code));
            }
            // Try as Rust code
            else if let Some(wj_code) = registry.map_rust_code(&code) {
                println!("{}", registry.format_explanation(&wj_code.code));
            }
            // Not found
            else {
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
        }
        Commands::Errors { file, output } => {
            use colored::*;

            println!("Building and checking {}...", file.display());

            // Build the project first (without checking)
            let build_options = windjammer::cli::build::BuildOptions {
                minify: false,
                tree_shake: false,
                source_maps: true,
                polyfills: false,
                v8_optimize: false,
            };

            windjammer::cli::build::execute(
                file.as_path(),
                Some(output.as_path()),
                false, // release
                "rust",
                build_options,
                true,  // check - run cargo check
                false, // raw_errors
                false, // fix
                false, // verbose
                true,  // quiet - suppress normal output
                None,  // filter_file
                None,  // filter_type
                false, // library
                false, // module_file
                false, // run_cargo - not needed when check=true
            )
            .ok(); // Ignore errors, we'll get them from the TUI

            // TODO: Add a public API to get diagnostics from build module
            // For now, show a message
            println!(
                "{}",
                "✓ TUI mode coming soon! Use 'wj build --check' for now.".yellow()
            );
            println!(
                "{}",
                "  The TUI infrastructure is ready, just needs diagnostics API.".dimmed()
            );
        }
    }

    Ok(())
}

/// Interpret a .wj file directly using the Windjammerscript tree-walking interpreter.
fn interpret_file(file: &std::path::Path) -> anyhow::Result<()> {
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
fn run_repl() -> anyhow::Result<()> {
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
        "Tip: Any code here can be compiled with `wj build` for production."
            .dimmed()
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
        let mut parse = windjammer::parser::Parser::new_with_source(
            tokens,
            "repl".to_string(),
            source.clone(),
        );

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
