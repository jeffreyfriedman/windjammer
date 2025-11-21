// Update command implementation
//
// Checks for new releases on GitHub and updates the wj binary using cargo install

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Deserialize;
use std::process::Command;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/jeffreyfriedman/windjammer/releases/latest";
const CRATE_NAME: &str = "windjammer";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[allow(dead_code)]
    name: String,
    html_url: String,
    body: Option<String>,
}

pub fn execute(check_only: bool, force: bool) -> Result<()> {
    println!("{}", "Checking for updates...".cyan());

    // Get current version
    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: {}", current_version.green());

    // Check latest release from GitHub
    let latest_release = get_latest_release()?;
    let latest_version = latest_release.tag_name.trim_start_matches('v');

    println!("Latest version:  {}", latest_version.green());

    // Compare versions
    if is_newer_version(latest_version, current_version) {
        println!(
            "\n{}",
            format!(
                "📦 New version available: {} → {}",
                current_version, latest_version
            )
            .yellow()
            .bold()
        );

        if let Some(body) = &latest_release.body {
            println!("\n{}", "Release notes:".cyan());
            // Print first 5 lines of release notes
            for line in body.lines().take(5) {
                println!("  {}", line);
            }
            if body.lines().count() > 5 {
                println!("  ...");
            }
        }

        println!(
            "\n{}",
            format!("View full release: {}", latest_release.html_url).cyan()
        );

        if check_only {
            println!(
                "\n{}",
                "Run 'wj update' to install the latest version.".yellow()
            );
            return Ok(());
        }

        // Perform update
        println!("\n{}", "Installing update...".cyan());
        install_update()?;

        println!(
            "\n{}",
            format!("✅ Successfully updated to version {}", latest_version)
                .green()
                .bold()
        );
    } else if force {
        println!("\n{}", "Already up to date. Forcing reinstall...".yellow());
        install_update()?;
        println!("\n{}", "✅ Successfully reinstalled".green().bold());
    } else {
        println!("\n{}", "✅ Already up to date!".green().bold());
    }

    Ok(())
}

fn get_latest_release() -> Result<GithubRelease> {
    // Use curl to fetch latest release info
    let output = Command::new("curl")
        .args([
            "-s",
            "-H",
            "Accept: application/vnd.github.v3+json",
            GITHUB_API_URL,
        ])
        .output()
        .context("Failed to check for updates. Is curl installed?")?;

    if !output.status.success() {
        anyhow::bail!("Failed to fetch release information from GitHub");
    }

    let response =
        String::from_utf8(output.stdout).context("Invalid UTF-8 in GitHub API response")?;

    serde_json::from_str(&response).context("Failed to parse GitHub API response")
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    // Simple version comparison (assumes semver)
    let parse_version =
        |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse::<u32>().ok()).collect() };

    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);

    for (l, c) in latest_parts.iter().zip(current_parts.iter()) {
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }

    // If all equal, check if latest has more parts (e.g., 1.0.1 > 1.0)
    latest_parts.len() > current_parts.len()
}

fn install_update() -> Result<()> {
    println!("Running: cargo install {} --force", CRATE_NAME);

    let status = Command::new("cargo")
        .args(["install", CRATE_NAME, "--force"])
        .status()
        .context("Failed to run cargo install. Is cargo in your PATH?")?;

    if !status.success() {
        anyhow::bail!("cargo install failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("0.20.1", "0.20.0"));
        assert!(is_newer_version("0.21.0", "0.20.0"));
        assert!(is_newer_version("1.0.0", "0.20.0"));
        assert!(!is_newer_version("0.20.0", "0.20.0"));
        assert!(!is_newer_version("0.19.0", "0.20.0"));
    }
}
