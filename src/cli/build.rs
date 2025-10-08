// wj build - Build Windjammer project
//
// This command compiles Windjammer source files to Rust.

use anyhow::Result;
use colored::*;
use std::path::Path;

pub fn execute(path: &Path, output: Option<&Path>, _release: bool) -> Result<()> {
    let output_dir = output.unwrap_or_else(|| Path::new("./build"));

    println!(
        "{} Windjammer project from {:?}",
        "Building".green().bold(),
        path
    );
    println!("Output: {:?}", output_dir);

    // Use the existing build logic from main.rs
    let target = crate::CompilationTarget::Wasm;
    crate::build_project(path, output_dir, target)?;

    println!("\n{} Build complete!", "Success!".green().bold());
    println!("Run your project with:");
    println!("  cd {:?} && cargo run", output_dir);

    Ok(())
}

