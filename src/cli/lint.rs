// wj lint - Run linter (clippy)
//
// This command wraps `cargo clippy` for consistency.

use anyhow::{bail, Result};
use colored::*;
use std::process::Command;

pub fn execute(fix: bool) -> Result<()> {
    if fix {
        println!("{} and fixing linter warnings", "Running".green().bold());
    } else {
        println!("{} linter", "Running".green().bold());
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("clippy")
        .arg("--all-targets")
        .arg("--all-features")
        .arg("--")
        .arg("-D")
        .arg("warnings");

    if fix {
        cmd.arg("--fix");
    }

    let status = cmd.status()?;

    if !status.success() {
        bail!("Linter found issues");
    }

    println!("{} No linter issues found", "✓".green().bold());

    Ok(())
}
