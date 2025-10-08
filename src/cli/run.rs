// wj run - Compile and execute Windjammer file
//
// This command builds the project to a temporary directory and then runs it.

use anyhow::{bail, Result};
use colored::*;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

pub fn execute(path: &Path, args: &[String]) -> Result<()> {
    println!("{} {}", "Running".green().bold(), path.display());

    // Create temporary build directory
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path();

    // Build to temp directory (build_project handles both files and directories)
    let target = crate::CompilationTarget::Wasm;
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

