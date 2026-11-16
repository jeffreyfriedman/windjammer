// wj optimize - what the post-codegen optimizer changes in a file's emitted Rust, and why each change is sound.
//
// Reporting only (docs/economics/COMMANDS.md): the compiler runs as a child process with the optimizer off
// (`WJ_OPTIMIZER=off`) to obtain the code generator's own output; the production optimizer is then applied here,
// so the report is exactly what `wj build` emits.

use crate::optimizer::{run, OptimizationConfig};
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

/// One optimizer pass: its name in `OptimizationOutput::applied`, default state, rewrite and soundness section.
pub struct PassInfo {
    pub name: &'static str,
    pub default_on: bool,
    pub rewrite: &'static str,
    pub soundness: &'static str,
}

const DOC: &str = "docs/optimization/PASSES.md";

/// Every pass of `src/optimizer/passes`, in PASSES.md order. `tests/optimize_cli_test.rs` checks that each has a
/// PASSES.md section and that `OptimizationConfig::production()` agrees with `default_on`.
pub const PASSES: &[PassInfo] = &[
    PassInfo { name: "temp_clone", default_on: true, rewrite: "`E.clone()` -> `E` on a freshly built temporary", soundness: "#p1-temp_clone-on" },
    PassInfo { name: "copy_clone", default_on: true, rewrite: "`x.clone()` -> `x` for a binding of a primitive Copy type", soundness: "#p2-copy_clone-on" },
    PassInfo { name: "same_type_cast", default_on: true, rewrite: "drop `as T` on a value already of type `T`", soundness: "#p3-same_type_cast-on" },
    PassInfo { name: "with_capacity", default_on: true, rewrite: "`Vec::new()` -> `Vec::with_capacity(n)` before an unconditional `0..n` push loop", soundness: "#p4-with_capacity-on" },
    PassInfo { name: "string_append", default_on: true, rewrite: "`s = format!(\"{}..\", s, ..)` -> in-place `write!`", soundness: "#p5-string_append-on" },
    PassInfo { name: "inline_policy", default_on: true, rewrite: "remove `#[inline]` from tests, `main` and large functions", soundness: "#p6-inline_policy-on" },
    PassInfo { name: "dead_helpers", default_on: true, rewrite: "remove file-private `#[repr(C)]` and unreferenced `to_bytes` helpers", soundness: "#p7-dead_helpers-on" },
    PassInfo { name: "scratch_reuse", default_on: false, rewrite: "hoist a per-iteration `String::new()`/`Vec::new()` out of its loop and `clear()` it (WJ_AUTO_ARENA=1)", soundness: "#p8-scratch_reuse-off-auto-arena-adr-0025" },
    PassInfo { name: "macro_arg_clone", default_on: true, rewrite: "`format!(\"{}\", x.clone())` -> `format!(\"{}\", x)` in the format family", soundness: "#p9-macro_arg_clone-on" },
];

fn pass(name: &str) -> Result<&'static PassInfo> {
    match PASSES.iter().find(|p| p.name == name) {
        Some(p) => Ok(p),
        None => bail!(
            "unknown pass `{name}`; known passes: {}",
            PASSES.iter().map(|p| p.name).collect::<Vec<_>>().join(", ")
        ),
    }
}

/// The code generator's output for `file`, before the optimizer (a child `wj build --no-cargo`).
fn unoptimized_rust(file: &Path) -> Result<String> {
    let exe = std::env::current_exe().context("locating the wj executable")?;
    let dir = tempfile::tempdir().context("creating a temporary directory")?;
    let out = Command::new(exe)
        .arg("build")
        .arg(file)
        .args(["--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(dir.path())
        .env("WJ_OPTIMIZER", "off")
        .output()
        .context("running wj build")?;
    if !out.status.success() {
        let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        bail!("{} does not compile:\n{}", file.display(), text.trim_end());
    }
    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("main");
    let rs = dir.path().join(format!("{stem}.rs"));
    let rs = if rs.exists() { rs } else { dir.path().join("main.rs") };
    std::fs::read_to_string(&rs).with_context(|| format!("reading {}", rs.display()))
}

fn clones(code: &str) -> usize {
    code.matches(".clone()").count()
}

/// `wj optimize --list`
pub fn list(json: bool) -> Result<()> {
    if json {
        let v: Vec<serde_json::Value> = PASSES
            .iter()
            .map(|p| serde_json::json!({"name": p.name, "default_on": p.default_on, "rewrite": p.rewrite, "soundness": format!("{DOC}{}", p.soundness)}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "passes": v }))?);
        return Ok(());
    }
    for p in PASSES {
        println!("{:<16} {:<4} {}", p.name, if p.default_on { "on" } else { "off" }, p.rewrite);
        println!("{:<21} soundness: {DOC}{}", "", p.soundness);
    }
    Ok(())
}

/// `wj optimize --explain FILE [--pass NAME] [--json]`
pub fn explain(file: &Path, only: Option<&str>, json: bool) -> Result<()> {
    if let Some(name) = only {
        pass(name)?;
    }
    let before = unoptimized_rust(file)?;
    let out = run(&OptimizationConfig::production(), &before);
    let mut rows: Vec<(&'static PassInfo, usize)> = Vec::new();
    for (name, n) in &out.applied {
        if only.is_some_and(|o| o != *name) {
            continue;
        }
        rows.push((pass(name)?, *n));
    }
    if json {
        let passes: Vec<serde_json::Value> = rows
            .iter()
            .map(|(p, n)| serde_json::json!({"name": p.name, "edits": n, "default_on": p.default_on, "soundness": format!("{DOC}{}", p.soundness)}))
            .collect();
        let doc = serde_json::json!({
            "file": file.display().to_string(),
            "passes": passes,
            "clones_before": clones(&before),
            "clones_after": clones(&out.code),
        });
        println!("{}", serde_json::to_string_pretty(&doc)?);
        return Ok(());
    }
    println!("{}", file.display());
    if rows.is_empty() {
        println!("  no pass changed this file");
    }
    for (p, n) in &rows {
        println!("  {}: {} edit(s)  {}", p.name, n, p.rewrite);
        println!("    soundness: {DOC}{}", p.soundness);
    }
    println!("  .clone() in the emitted Rust: {} before, {} after", clones(&before), clones(&out.code));
    Ok(())
}
