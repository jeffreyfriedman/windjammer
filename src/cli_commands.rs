//! Command dispatch for the legacy `windjammer` binary.

use crate::build_utils;
use crate::cargo_integration;
use crate::cli_execution;
use crate::ejector;
use crate::error_handling;
use crate::lexer;
use crate::parser;
use crate::test_runner;

use crate::cli_args::{Cli, Commands, CompilationTarget};
use crate::cli_project_build::find_wj_files;
use anyhow::Result;
use clap::Parser;
use colored::*;
use std::path::Path;

/// Main CLI entry point (called from bin/wj.rs after plugin discovery)
#[allow(dead_code)]
pub fn run_main_cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            path,
            output,
            target,
            check,
            raw_errors,
            library,
            module_file,
            no_lint,
        } => {
            crate::cli_project_build::build_project(&path, &output, target, !no_lint)?;

            // Generate mod.rs if requested
            if module_file {
                build_utils::generate_mod_file(&output)?;
            }

            // Strip main() functions if library mode
            if library {
                build_utils::strip_main_functions(&output)?;
            }

            if check {
                cargo_integration::check_with_cargo(&output, raw_errors)?;
            }
        }
        Commands::Check {
            path,
            output,
            target,
            raw_errors,
        } => {
            crate::cli_project_build::build_project(&path, &output, target, true)?;
            cargo_integration::check_with_cargo(&output, raw_errors)?;
        }
        Commands::Lint {
            path,
            max_function_length,
            max_file_length,
            max_complexity,
            no_unused,
            no_style,
            errors_only,
            json,
            fix,
        } => {
            error_handling::lint_project(
                &path,
                max_function_length,
                max_file_length,
                max_complexity,
                !no_unused,
                !no_style,
                errors_only,
                json,
                fix,
            )?;
        }
        Commands::Eject {
            path,
            output,
            target,
            format,
            comments,
            no_cargo_toml,
        } => {
            eject_project(&path, &output, target, format, comments, !no_cargo_toml)?;
        }
        Commands::Run {
            file,
            target,
            interpret,
            args,
        } => {
            if interpret {
                cli_execution::interpret_file(&file)?;
            } else {
                cli_execution::run_file(&file, target, &args)?;
            }
        }
        Commands::Repl {} => {
            cli_execution::run_repl()?;
        }
        Commands::Test {
            path,
            filter,
            nocapture,
            parallel,
            json,
        } => {
            test_runner::run_tests(
                path.as_deref(),
                filter.as_deref(),
                nocapture,
                parallel,
                json,
            )?;
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn check_project(path: &Path) -> Result<()> {
    println!(
        "{} Windjammer project: {:?}",
        "Checking".cyan().bold(),
        path
    );

    let wj_files = find_wj_files(path)?;

    if wj_files.is_empty() {
        println!("{} No .wj files found", "Warning:".yellow().bold());
        return Ok(());
    }

    println!("Found {} file(s) to check", wj_files.len());

    // For now, just parse all files to check for syntax errors
    let mut has_errors = false;

    for file in &wj_files {
        let file_name = file.file_name().unwrap().to_str().unwrap();
        print!("  Checking {:?}... ", file_name);

        let source = std::fs::read_to_string(file)?;
        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = parser::Parser::new(tokens);

        match parser.parse() {
            Ok(_) => println!("{}", "✓".green()),
            Err(e) => {
                println!("{}", "✗".red());
                println!("    Error: {}", e);
                has_errors = true;
            }
        }
    }

    if !has_errors {
        println!(
            "\n{} All files passed syntax check!",
            "Success!".green().bold()
        );
    } else {
        println!("\n{} Some files have errors", "Error:".red().bold());
    }

    Ok(())
}

#[allow(dead_code)]
fn check_file(file_path: &Path) -> Result<()> {
    let source = std::fs::read_to_string(file_path)?;

    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();

    let mut parser = parser::Parser::new(tokens);
    let _program = parser
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    Ok(())
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn eject_project(
    path: &Path,
    output: &Path,
    target: CompilationTarget,
    format: bool,
    comments: bool,
    generate_cargo_toml: bool,
) -> Result<()> {
    let config = ejector::EjectConfig {
        format,
        comments,
        generate_cargo_toml,
        target,
    };

    let mut ejector = ejector::Ejector::new(config);
    ejector.eject_project(path, output)?;

    Ok(())
}
