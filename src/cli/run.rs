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

    if matches!(target, crate::CompilationTarget::Rust) {
        build_windjammer_dependencies(path)?;
    }
    build_quietly(path, output_dir, target)?;

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

/// Transpile with the compiler's progress output on stderr, so `wj run --compiled` keeps stdout for the program
/// (the interpreted and compiled runs must print the same bytes). Inside the `wj` binary the build runs as a
/// child `wj build --no-cargo` with stdout redirected to stderr; elsewhere (library callers) in-process.
fn build_quietly(path: &Path, output_dir: &Path, target: crate::CompilationTarget) -> Result<()> {
    let exe = std::env::current_exe().ok().filter(|e| e.file_stem().is_some_and(|s| s == "wj"));
    let (Some(exe), crate::CompilationTarget::Rust) = (exe, target) else {
        return crate::build_project(path, output_dir, target, true);
    };
    let status = Command::new(exe)
        .arg("build")
        .arg(path)
        .arg("--output")
        .arg(output_dir)
        .arg("--no-cargo")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .output()?;
    use std::io::Write;
    std::io::stderr().write_all(&status.stdout)?;
    if !status.status.success() {
        bail!("build failed");
    }
    Ok(())
}

/// Build every Windjammer source dependency (same `package::graph` the interpreter loads) into its `gen/`
/// crate, which the generated `Cargo.toml` then links (ADR-0010: dependencies build before dependents).
/// Skipped when `gen/` is newer than every source file of the dependency.
fn build_windjammer_dependencies(path: &Path) -> Result<()> {
    let Some(root) = crate::module_system::project::find_manifest_dir(path) else { return Ok(()) };
    let nodes = crate::package::graph::resolve(&root).map_err(|e| anyhow::anyhow!(e))?;
    for n in nodes.into_iter().filter(|n| n.kind == crate::package::graph::DepKind::Windjammer) {
        let (Some(dep_root), Some(src)) = (n.root.clone(), n.source_root()) else { continue };
        let gen = dep_root.join("gen");
        let built = std::fs::metadata(gen.join("Cargo.toml")).and_then(|m| m.modified()).ok();
        let newest = crate::module_system::sources::wj_files_under(&src)
            .unwrap_or_default()
            .iter()
            .filter_map(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok())
            .max();
        if built.is_some() && built >= newest {
            continue;
        }
        eprintln!("{} dependency {} ({})", "Building".green().bold(), n.name, src.display());
        build_quietly(&src, &gen, crate::CompilationTarget::Rust)?;
    }
    Ok(())
}
