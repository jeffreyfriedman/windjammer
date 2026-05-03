use anyhow::{Context, Result};
/// Plugin discovery and delegation
///
/// Enables `wj <plugin> <args>` to delegate to `wj-<plugin>` binaries.
/// Plugins are external binaries, not part of core compiler.
use std::path::PathBuf;
use std::process::Command;

/// Execute a plugin
pub fn execute_plugin(plugin_name: &str, args: &[String]) -> Result<i32> {
    let binary_name = format!("wj-{}", plugin_name);

    // Search order (TDD: support multiple plugin locations):
    // 1. ./wj-plugins/wj-<plugin>/target/release/wj-<plugin> (standard)
    // 2. ./wj-<plugin>/target/release/wj-<plugin> (alternate, e.g. windjammer-game repo)
    // 3. wj-<plugin> in $PATH (global)

    let locations = vec![
        PathBuf::from(".")
            .join("wj-plugins")
            .join(&binary_name)
            .join("target/release")
            .join(&binary_name),
        PathBuf::from(".")
            .join(&binary_name)
            .join("target/release")
            .join(&binary_name),
    ];

    for local_plugin in &locations {
        if local_plugin.exists() {
            // Execute local plugin
            let status = Command::new(local_plugin)
                .args(args)
                .status()
                .with_context(|| {
                    format!("Failed to execute local plugin: {}", local_plugin.display())
                })?;

            return Ok(status.code().unwrap_or(1));
        }
    }

    // Try $PATH as fallback
    let status = Command::new(&binary_name)
        .args(args)
        .status()
        .with_context(|| {
            format!(
                "Plugin '{}' not found!\n\
                 Tried:\n\
                 1. Local (standard): {}\n\
                 2. Local (alternate): {}\n\
                 3. Global: {} in $PATH\n\
                 \n\
                 Install with:\n\
                 cargo build --release --manifest-path wj-plugins/{}/Cargo.toml",
                plugin_name,
                locations[0].display(),
                locations[1].display(),
                binary_name,
                binary_name
            )
        })?;

    Ok(status.code().unwrap_or(1))
}
