// wj test - Run tests
//
// This command wraps `cargo test` for a better user experience.

use anyhow::{bail, Result};
use colored::*;
use std::process::Command;

pub fn execute(filter: Option<&str>) -> Result<()> {
    println!("{} tests", "Running".green().bold());

    let mut cmd = Command::new("cargo");
    cmd.arg("test");

    if let Some(f) = filter {
        cmd.arg(f);
    }

    let status = cmd.status()?;

    if !status.success() {
        bail!("Tests failed");
    }

    Ok(())
}
