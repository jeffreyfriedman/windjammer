// wj check - Type check without building
//
// This command wraps `cargo check` for fast error checking.

use anyhow::{bail, Result};
use colored::*;
use std::process::Command;

pub fn execute() -> Result<()> {
    println!("{} type checking", "Running".green().bold());

    let status = Command::new("cargo").arg("check").status()?;

    if !status.success() {
        bail!("Type check failed");
    }

    println!("{} No type errors found", "✓".green().bold());

    Ok(())
}

