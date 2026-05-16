//! CLI execution utilities
//!
//! This module handles:
//! - Running compiled Windjammer programs
//! - Interpreting Windjammer files directly (tree-walking interpreter)
//! - REPL (Read-Eval-Print Loop) for interactive development

use anyhow::Result;
use std::path::Path;

use crate::CompilationTarget;
use crate::{build_project, interpreter, lexer, parser};

pub fn run_file(file: &Path, target: CompilationTarget, args: &[String]) -> Result<()> {
    use colored::*;
    use std::fs;
    use std::process::Command;

    // Validate that the file exists and is a .wj file
    if !file.exists() {
        anyhow::bail!("File not found: {:?}", file);
    }
    if file.extension().is_none_or(|ext| ext != "wj") {
        anyhow::bail!("File must have .wj extension: {:?}", file);
    }

    // Auto-detect target based on imports ONLY if no explicit target was provided
    // (default_value in CLI is "rust", so we can't distinguish between explicit and default)
    // For now, skip auto-detection if user provided --target flag
    // TODO: Better way to detect if user explicitly provided --target

    println!(
        "{} {:?} (target: {:?})",
        "Running".green().bold(),
        file,
        target
    );

    // Create a temporary build directory
    let temp_dir = std::env::temp_dir().join(format!(
        "windjammer-run-{}",
        file.file_stem().and_then(|s| s.to_str()).unwrap_or("app")
    ));

    // Clean up any previous build
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    // Build the project
    build_project(file, &temp_dir, target, true)?;

    // Handle execution based on target
    match target {
        CompilationTarget::Wasm => {
            // WASM apps need to be built with wasm-pack and served
            println!("\n{} WASM app...", "Building".cyan().bold());
            println!("To run this WASM app:");
            println!("  1. cd {:?}", temp_dir);
            println!("  2. wasm-pack build --target web");
            println!("  3. Serve the generated HTML file");
            println!("\nOr use the pre-built counter example:");
            println!("  cd crates/windjammer-ui");
            println!("  wasm-pack build --target web");
            println!("  # Then serve examples/counter_wasm.html");

            // Don't clean up so user can inspect/run
            println!("\n{} Build artifacts in: {:?}", "ℹ".cyan().bold(), temp_dir);
        }
        _ => {
            // Native targets can be run directly
            println!("\n{} the program...", "Executing".cyan().bold());
            let mut cmd = Command::new("cargo");
            cmd.arg("run").current_dir(&temp_dir);

            // Pass through any additional arguments
            if !args.is_empty() {
                cmd.arg("--");
                cmd.args(args);
            }

            let status = cmd.status()?;

            if !status.success() {
                anyhow::bail!("Program execution failed");
            }

            // Clean up temp directory for native builds
            if temp_dir.exists() {
                fs::remove_dir_all(&temp_dir)?;
            }
        }
    }

    Ok(())
}

pub fn interpret_file(file: &Path) -> Result<()> {
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
    let mut lex = lexer::Lexer::new(&source);
    let tokens = lex.tokenize_with_locations();
    let mut parse =
        parser::Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
    let program = parse
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    let mut interp = interpreter::Interpreter::new();
    match interp.run(&program) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{} {}", "Runtime error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

/// Run the Windjammerscript REPL (Read-Eval-Print Loop)
pub fn run_repl() -> Result<()> {
    use colored::*;
    use std::io::{self, BufRead, Write};

    println!(
        "{} {} {}",
        "Windjammerscript".cyan().bold(),
        "REPL".white().bold(),
        "(type 'exit' or Ctrl-D to quit)".dimmed()
    );
    println!(
        "{}",
        "Tip: Any code you write here can be compiled with `wj build` for production.".dimmed()
    );
    println!();

    let stdin = io::stdin();
    let mut accumulated_source = String::new();
    let mut line_buffer = String::new();
    let mut in_block = false;
    let mut brace_depth: i32 = 0;

    loop {
        // Print prompt
        if in_block {
            print!("{} ", "...".dimmed());
        } else {
            print!("{} ", "wj>".green().bold());
        }
        io::stdout().flush()?;

        line_buffer.clear();
        let bytes_read = stdin.lock().read_line(&mut line_buffer)?;
        if bytes_read == 0 {
            // EOF (Ctrl-D)
            println!();
            break;
        }

        let line = line_buffer.trim_end();

        if line == "exit" || line == "quit" {
            break;
        }

        // Track brace depth for multi-line input
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

        // Wrap in main() if it's a simple expression/statement
        let source = if accumulated_source.contains("fn main()") {
            accumulated_source.clone()
        } else {
            format!("fn main() {{\n{}\n}}", accumulated_source)
        };

        // Parse and interpret
        let mut lex = lexer::Lexer::new(&source);
        let tokens = lex.tokenize_with_locations();
        let mut parse = parser::Parser::new_with_source(tokens, "repl".to_string(), source.clone());

        match parse.parse() {
            Ok(program) => {
                let mut interp = interpreter::Interpreter::new();
                match interp.run(&program) {
                    Ok(val) => {
                        // Print non-unit return values
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
