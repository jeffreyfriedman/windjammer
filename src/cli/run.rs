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

    // Go / JavaScript / TypeScript: build a ready-to-run directory, then run it with the native toolchain.
    if matches!(target_str.to_lowercase().as_str(), "javascript" | "js" | "typescript" | "ts" | "go" | "golang") {
        crate::cli::build::execute(
            path,
            Some(output_dir),
            false,
            target_str,
            crate::cli::build::BuildOptions { tree_shake: false, source_maps: false },
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
            "Unknown target: {}. Use 'rust', 'go', 'javascript', 'typescript', or 'wasm'",
            target_str
        ),
    };

    crate::build_project(path, output_dir, target, true)?;

    // Run with cargo (shared target dir outside the temp project)
    let _ = crate::cargo_cache::reclaim();
    let mut cmd = Command::new("cargo");
    cmd.arg("run").current_dir(output_dir);
    crate::cargo_cache::configure_cargo_command(&mut cmd, crate::cargo_cache::TargetKind::Shared);

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
