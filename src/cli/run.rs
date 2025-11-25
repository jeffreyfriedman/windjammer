// wj run - Compile and execute Windjammer file
//
// This command builds the project to a temporary directory and then runs it.

use anyhow::{bail, Result};
use colored::*;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

pub fn execute(path: &Path, args: &[String], target_str: &str) -> Result<()> {
    println!(
        "{} {} (target: {})",
        "Running".green().bold(),
        path.display(),
        target_str
    );

    // Create temporary build directory
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path();

    // Check if target is JavaScript
    if target_str == "javascript" || target_str == "js" {
        // Build JavaScript to temp directory
        crate::cli::build::execute(
            path,
            Some(output_dir),
            false,
            target_str,
            crate::cli::build::BuildOptions {
                minify: false,
                tree_shake: false,
                source_maps: false,
                polyfills: false,
                v8_optimize: false,
            },
            false, // check
            false, // raw_errors
            false, // fix
            false, // verbose
            false, // quiet
            None,  // filter_file
            None,  // filter_type
            false, // library
            false, // module_file
            false, // run_cargo - run.rs handles execution itself
        )?;

        // Run with Node.js
        let output_file = output_dir.join("output.js");
        let mut cmd = Command::new("node");
        cmd.arg(&output_file).args(args);

        let status = cmd.status()?;
        if !status.success() {
            bail!("Program exited with error");
        }

        return Ok(());
    }

    // Build to temp directory (build_project handles both files and directories)
    let target = match target_str.to_lowercase().as_str() {
        "rust" => crate::CompilationTarget::Rust,
        "wasm" | "webassembly" => crate::CompilationTarget::Wasm,
        _ => bail!(
            "Unknown target: {}. Use 'rust', 'javascript', or 'wasm'",
            target_str
        ),
    };

    crate::build_project(path, output_dir, target)?;

    // Run with cargo
    let mut cmd = Command::new("cargo");
    cmd.arg("run").current_dir(output_dir);

    // Forward arguments to the program
    if !args.is_empty() {
        cmd.arg("--").args(args);
    }

    // Execute and capture status
    let status = cmd.status()?;

    if !status.success() {
        bail!("Program exited with error");
    }

    Ok(())
}
