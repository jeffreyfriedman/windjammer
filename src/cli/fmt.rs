// wj fmt - Format Windjammer code
//
// This command wraps `cargo fmt` for consistency.

use anyhow::{bail, Result};
use colored::*;
use std::process::Command;

pub fn execute(check: bool) -> Result<()> {
    if check {
        println!("{} code formatting", "Checking".green().bold());
    } else {
        println!("{} code", "Formatting".green().bold());
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("fmt").arg("--all");

    if check {
        cmd.arg("--").arg("--check");
    }

    let status = cmd.status()?;

    if !status.success() {
        bail!("Formatting check failed");
    }

    if check {
        println!("{} Code is properly formatted", "✓".green().bold());
    } else {
        println!("{} Code formatted successfully", "✓".green().bold());
    }

    Ok(())
}

