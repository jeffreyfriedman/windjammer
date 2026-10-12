// wj run - Compile and execute Windjammer file
//
// This command builds the project to a temporary directory and then runs it.

use anyhow::{bail, Result};
use colored::*;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

pub fn execute(path: &Path, args: &[String], target_str: &str) -> Result<()> {
    let code = execute_status(path, args, target_str, false)?;
    if code != 0 {
        bail!("Program exited with error");
    }
    Ok(())
}

/// Build and run; returns the program's exit status (stdout carries only program output: the
/// banner and cargo's progress go to stderr).
pub fn execute_status(path: &Path, args: &[String], target_str: &str, release: bool) -> Result<i32> {
    eprintln!(
        "{} {} (target: {})",
        "Running".green().bold(),
        path.display(),
        target_str
    );

    // Create temporary build directory
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path();

    // Go / JavaScript / TypeScript: build a ready-to-run directory, then run it with the native toolchain.
    if matches!(target_str.to_lowercase().as_str(), "javascript" | "js" | "typescript" | "ts" | "go" | "golang") {
        crate::cli::build::execute(
            path,
            Some(output_dir),
            false,
            target_str,
            crate::cli::build::BuildOptions { minify: false, tree_shake: false, source_maps: false, polyfills: false, v8_optimize: false },
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
            true,  // enable_lint
            false, // no_generate_cargo_toml
            &[],   // metadata
            false, // json_output
            false, // ir_shadow_validate
        )?;
        let mut cmd = match target_str.to_lowercase().as_str() {
            "go" | "golang" => {
                let mut c = Command::new("go");
                c.arg("run").arg(".");
                c
            }
            "typescript" | "ts" => {
                let mut c = Command::new("node");
                c.arg("main.ts");
                c
            }
            _ => {
                let mut c = Command::new("node");
                c.arg("main.mjs");
                c
            }
        };
        cmd.current_dir(output_dir).args(args);
        let status = cmd.status()?;
        return Ok(status.code().unwrap_or(1));
    }

    // Build to temp directory (build_project handles both files and directories)
    let target = match target_str.to_lowercase().as_str() {
        "rust" => crate::CompilationTarget::Rust,
        "wasm" | "webassembly" => crate::CompilationTarget::Wasm,
        _ => bail!(
            "Unknown target: {}. Use 'rust', 'go', 'javascript', 'typescript', or 'wasm'",
            target_str
        ),
    };

    crate::build_project(path, output_dir, target, true)?;

    // Run with cargo (shared target dir outside the temp project)
    let _ = crate::cargo_cache::reclaim();
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet"]).current_dir(output_dir);
    if release {
        cmd.arg("--release");
    }
    crate::cargo_cache::configure_cargo_command(&mut cmd, crate::cargo_cache::TargetKind::Shared);

    // Forward arguments to the program
    if !args.is_empty() {
        cmd.arg("--").args(args);
    }

    let status = cmd.status()?;
    Ok(status.code().unwrap_or(1))
}
