// wj run - Compile and execute Windjammer file
//
// This command builds the project to a temporary directory and then runs it.

use anyhow::{bail, Result};
use colored::*;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// How a run reports: the diagnostic format and whether build progress is shown. The CLI driver decides (`--error-format`,
/// `WJ_ERROR_FORMAT`, `WJ_PROGRESS`, whether stderr is a terminal; ADR-0037); library code never reads the environment.
#[derive(Debug, Clone, Copy, Default)]
pub struct Output {
    pub format: crate::diagnostics::render::Format,
    /// "Running", "Building Windjammer project", "Transpilation complete" on stderr.
    pub progress: bool,
}

impl Output {
    /// Progress on a terminal only; a pipe (a script, an agent) gets the program's output and diagnostics.
    pub fn for_stderr(format: crate::diagnostics::render::Format, progress_override: Option<bool>) -> Output {
        use std::io::IsTerminal;
        Output { format, progress: progress_override.unwrap_or_else(|| std::io::stderr().is_terminal()) }
    }
}

pub fn execute(path: &Path, args: &[String], target_str: &str) -> Result<()> {
    let code = execute_status(path, args, target_str, false, Output::for_stderr(Default::default(), None))?;
    if code != 0 {
        bail!("Program exited with error");
    }
    Ok(())
}

/// Build and run; returns the program's exit status (stdout carries only program output: the
/// banner and cargo's progress go to stderr).
pub fn execute_status(path: &Path, args: &[String], target_str: &str, release: bool, out: Output) -> Result<i32> {
    if out.progress {
        eprintln!("{} {} (target: {})", "Running".green().bold(), path.display(), target_str);
    }

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
            crate::cli::build::BuildOptions { tree_shake: false, source_maps: false, unchecked_arithmetic: false, locked: false },
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

    // Rust/wasm: a stable per-source directory keeps the program crate warm between runs.
    let (workdir, failed) = build_native(path, target_str, release, out)?;
    if let Some(code) = failed {
        return Ok(code);
    }
    let _tree = crate::cargo_cache::active::hold(crate::cargo_cache::TargetKind::Shared);
    let status = compiled_command(&workdir, release, args).status()?;
    Ok(status.code().unwrap_or(1))
}

/// The command that runs a program `build_native` built (cargo's own build is then a no-op), with its arguments.
pub fn compiled_command(workdir: &Path, release: bool, args: &[String]) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--manifest-path"]).arg(workdir.join("Cargo.toml"));
    if release {
        cmd.arg("--release");
    }
    if !release {
        crate::cargo_cache::build_profile::apply_fast_dev_env(&mut cmd);
    }
    // the caller holds the tree while the command runs (`cargo_cache::active::hold`)
    drop(crate::cargo_cache::configure_cargo_command(&mut cmd, crate::cargo_cache::TargetKind::Shared));
    // The crate was built by `compile_or_report`; cargo would replay rustc's warnings about the generated Rust
    // (unused runtime imports, extra parentheses) on every run. They are not about the `.wj` source and cost an
    // agent tokens on every call (ADR-0037), so the run step never shows them.
    cmd.env("CARGO_BUILD_WARNINGS", "allow");
    // Forward arguments to the program
    if !args.is_empty() {
        cmd.arg("--").args(args);
    }
    cmd
}

/// Transpile and `cargo build` the program into its stable per-source directory, without running it (`wj run
/// --compiled` and `wj run --verify`). Returns the directory and, when the build failed, the exit status to use (the
/// diagnostics are already printed). Records the compiled verdict (ADR-0033).
pub fn build_native(path: &Path, target_str: &str, release: bool, out: Output) -> Result<(std::path::PathBuf, Option<i32>)> {
    let workdir = crate::cargo_cache::build_profile::run_workdir(path);
    let output_dir = workdir.as_path();
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
        build_windjammer_dependencies(path, out)?;
    }
    // Concurrent runs of one source share this directory: serialize the transpile step so no process
    // compiles a half-written file (cargo's own build lock serializes the compile).
    let lock = std::fs::File::create(output_dir.join(".lock"))?;
    lock.lock()?;
    let built = build_quietly(path, output_dir, target, out);
    let _ = lock.unlock();
    built?;

    // Compile first with machine-readable output, so a rustc error reaches the user as a Windjammer
    // diagnostic on the `.wj` source (code, hint, location) and never as raw Rust compiler text.
    let _ = crate::cargo_cache::reclaim();
    let failed = compile_or_report(output_dir, release, path, out.format)?;
    Ok((workdir, failed))
}

/// Transpile with the compiler's progress output on stderr, so `wj run --compiled` keeps stdout for the program
/// (the interpreted and compiled runs must print the same bytes). Inside the `wj` binary the build runs as a
/// child `wj build --no-cargo` with stdout redirected to stderr; elsewhere (library callers) in-process.
fn build_quietly(path: &Path, output_dir: &Path, target: crate::CompilationTarget, out: Output) -> Result<()> {
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
        // Line maps let rustc errors land on the right `.wj` line (diagnostics::rustc_map).
        .arg("--source-maps")
        .stdout(std::process::Stdio::piped())
        .stderr(if out.progress { std::process::Stdio::inherit() } else { std::process::Stdio::piped() })
        .output()?;
    use std::io::Write;
    // Without progress, the child's incremental-cache notes (`⚡ ...`, `⟳ ...`) are dropped; its diagnostics are kept.
    let kept: String = String::from_utf8_lossy(&status.stderr)
        .lines()
        .filter(|l| !is_cache_note(l))
        .map(|l| format!("{l}\n"))
        .collect();
    // The agent format (ADR-0037) is one line per diagnostic: the child prints lints in the human layout.
    let cwd = std::env::current_dir().ok().map(|d| d.to_string_lossy().to_string());
    let kept = if out.format == crate::diagnostics::render::Format::Agent {
        crate::diagnostics::render::agent_lint_text(&kept, cwd.as_deref())
    } else {
        crate::diagnostics::render::collapse_repeated_lint_blocks(&kept, cwd.as_deref())
    };
    std::io::stderr().write_all(kept.as_bytes())?;
    // Progress only: the "run cargo build manually" hints do not apply to `wj run`; shown when `out.progress` (ADR-0037).
    if out.progress {
        let progress: String = String::from_utf8_lossy(&status.stdout)
            .lines()
            .filter(|l| !l.contains("cargo build"))
            .map(|l| format!("{l}\n"))
            .collect();
        std::io::stderr().write_all(progress.as_bytes())?;
    }
    if !status.status.success() {
        bail!("build failed");
    }
    Ok(())
}

/// An incremental-compilation status line of `wj build` (not a diagnostic).
fn is_cache_note(line: &str) -> bool {
    line.starts_with('\u{26a1}') || line.starts_with('\u{27f3}')
}

/// Build every Windjammer source dependency (same `package::graph` the interpreter loads) into its `gen/`
/// crate, which the generated `Cargo.toml` then links (ADR-0010: dependencies build before dependents).
/// Skipped when `gen/` is newer than every source file of the dependency.
fn build_windjammer_dependencies(path: &Path, out: Output) -> Result<()> {
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
        build_quietly(&src, &gen, crate::CompilationTarget::Rust, out)?;
    }
    Ok(())
}

/// `cargo build` the generated crate; on failure print translated diagnostics and return exit status 1.
fn compile_or_report(output_dir: &Path, release: bool, source: &Path, format: crate::diagnostics::render::Format) -> Result<Option<i32>> {
    let mut build = Command::new("cargo");
    build.args(["build", "--quiet", "--message-format=json"]).current_dir(output_dir);
    if release {
        build.arg("--release");
    }
    let _tree = crate::cargo_cache::configure_cargo_command(&mut build, crate::cargo_cache::TargetKind::Shared);
    let out = build.output()?;
    if out.status.success() {
        crate::cli::verdict::record(source, crate::cli::verdict::Outcome::Accepted, release);
        return Ok(None);
    }
    let text = format!("{}{}", String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout));
    let diags = crate::cli::build_json::map_cargo_failure_in(output_dir, &text, Some(if source.is_dir() { source } else { source.parent().unwrap_or(Path::new(".")) }));
    let only_unparsed = diags.len() == 1 && diags[0].message.contains("could not be parsed");
    if !only_unparsed && !diags.is_empty() {
        // a compile error is a property of the content and the compiler: later interpreted runs mention it (ADR-0033)
        let errors = diags.iter().filter(|d| d.is_error()).count().max(1);
        let first = diags.first().map(crate::cli::verdict::summary);
        crate::cli::verdict::record(source, crate::cli::verdict::Outcome::Rejected { errors, first }, release);
    }
    for d in &diags {
        eprint!("{}", crate::diagnostics::render::render_as(d, &|f| std::fs::read_to_string(f).ok(), format));
    }
    if only_unparsed {
        // Not a compile error (for example a dependency could not be fetched): cargo's own text is the diagnostic.
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(Some(1))
}
