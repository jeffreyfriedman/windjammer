//! `wj eject [PATH] -o DIR`: standalone Cargo project from a Windjammer project.
//! The implementation lives in `crate::ejector` (shared with the legacy `windjammer eject`).

use anyhow::Result;
use colored::*;
use std::path::Path;

pub fn execute(
    path: &Path,
    output: &Path,
    target: &str,
    library: bool,
    module_file: bool,
) -> Result<()> {
    let target = match target.to_lowercase().as_str() {
        "rust" => crate::CompilationTarget::Rust,
        "wasm" | "webassembly" => crate::CompilationTarget::Wasm,
        other => anyhow::bail!("wj eject supports --target rust or wasm (got {other})"),
    };
    let opts = crate::ejector::EjectOptions {
        target,
        library,
        module_file,
    };
    // Driver edge: environment -> explicit configuration (same as `wj build`).
    let config = crate::compiler_config::CompilerConfig::from_env();
    let report = crate::ejector::eject(path, output, &opts, &config)?;

    println!("{} {}", "Ejected".green().bold(), report.output.display());
    for f in &report.rust_files {
        let rel = f.strip_prefix(&report.output).unwrap_or(f);
        println!("  {}", rel.display());
    }
    for (name, dir) in &report.vendored {
        println!("  vendored {name} -> {}", dir.display());
    }
    println!();
    println!(
        "Standalone Cargo project. Next: cd {} && cargo build",
        output.display()
    );
    Ok(())
}
