//! Integer widths the compiled build would mix (ADR-0033, WJ0003), confirmed on the Rust `wj build` emits.
//!
//! Two readings with one walker (`check::widths::clashes`). The cheap one reads the Rust the checker generates
//! in-process for each file (milliseconds). It is only a trigger: that generator does not run the build's
//! multi-pass width inference, so its widths can differ from the build's. When it flags a file, the program is
//! generated exactly as `wj build --no-cargo --no-lint` does, and only clashes in that output are reported.
//! A clean program pays for the trigger alone; a flagged one pays one transpile (about a second).

use crate::check::{widths, Checked};
use crate::diagnostics::{Diagnostic, Report};
use std::path::{Path, PathBuf};

/// `checked` with the width predictions for the program at `path` added, when it has no error yet.
pub fn add_to(mut checked: Checked, path: &Path) -> Checked {
    if checked.report.diagnostics.iter().any(|d| d.is_error()) {
        return checked;
    }
    let found = predicted_widths(path);
    if found.is_empty() {
        return checked;
    }
    let old = checked.report.summary.clone();
    let mut all = checked.report.diagnostics.clone();
    all.extend(found);
    let mut report = Report::new(&checked.report.command, all);
    report.summary.files_checked = old.files_checked;
    report.summary.files_cached = old.files_cached;
    report.summary.elapsed_ms = old.elapsed_ms;
    checked.report = report;
    checked
}

/// WJ0003 width predictions for the program at `path` (a file or a project directory).
pub fn predicted_widths(path: &Path) -> Vec<Diagnostic> {
    let Ok(files) = crate::check::files::collect(path) else { return Vec::new() };
    let suspects: Vec<&PathBuf> =
        files.iter().filter(|f| widths::generated_rust(f).is_some_and(|r| !widths::clashes(&r).is_empty())).collect();
    if suspects.is_empty() {
        return Vec::new();
    }
    let Ok(out) = tempfile::tempdir() else { return Vec::new() };
    if !emit(path, out.path()) {
        return Vec::new();
    }
    let mut emitted = Vec::new();
    rust_files(out.path(), &mut emitted);
    let mut found = Vec::new();
    for src in suspects {
        let stem = src.file_stem().map(|s| s.to_string_lossy().to_string());
        let Some(rs) = emitted.iter().find(|r| r.file_stem().map(|s| s.to_string_lossy().to_string()) == stem) else { continue };
        let (Ok(rust), Ok(text)) = (std::fs::read_to_string(rs), std::fs::read_to_string(src)) else { continue };
        found.extend(widths::predict(&rust, &text, &src.display().to_string()));
    }
    found
}

/// Generate the program as `wj build --no-cargo` does. Inside the `wj` binary in a child process (the build
/// prints progress on stdout, which belongs to the program in an interpreted run); elsewhere in-process.
fn emit(path: &Path, out: &Path) -> bool {
    // a crate root `main.wj` is built as its source root, as `wj run --compiled` does (building the file alone drops
    // its sibling modules: docs/known-gaps.md, `tests/book_repros/lone_script_mod/`)
    let root_dir = (path.is_file() && path.file_name().is_some_and(|n| n == "main.wj")).then(|| {
        use crate::module_system::project;
        project::source_root_for(path, project::find_manifest_dir(path).as_deref())
    });
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let path = match &root_dir {
        Some(r) if path.parent().is_some_and(|d| canon(d) == canon(r)) => r.as_path(),
        _ => path,
    };
    let exe = std::env::current_exe().ok().filter(|e| e.file_stem().is_some_and(|s| s == "wj"));
    match exe {
        Some(exe) => std::process::Command::new(exe)
            .arg("build")
            .arg(path)
            .arg("--output")
            .arg(out)
            .args(["--no-cargo", "--no-lint"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success()),
        None => crate::build_project(path, out, crate::CompilationTarget::Rust, false).is_ok(),
    }
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && p.file_name().is_some_and(|n| n != "target") {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}
