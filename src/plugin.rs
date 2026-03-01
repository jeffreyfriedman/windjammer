/// Plugin discovery and delegation
///
/// Enables `wj <plugin> <args>` to delegate to `wj-<plugin>` binaries.
/// Plugins are external binaries, not part of core compiler.

use std::path::PathBuf;
use std::process::Command;
use anyhow::{Context, Result};

/// Execute a plugin
pub fn execute_plugin(plugin_name: &str, args: &[String]) -> Result<i32> {
    let binary_name = format!("wj-{}", plugin_name);
    
    // Search order:
    // 1. ./wj-plugins/wj-<plugin>/target/release/wj-<plugin> (local)
    // 2. wj-<plugin> in $PATH (global)
    
    let local_plugin = PathBuf::from(".")
        .join("wj-plugins")
        .join(&binary_name)
        .join("target/release")
        .join(&binary_name);
    
    if local_plugin.exists() {
        // Execute local plugin
        let status = Command::new(&local_plugin)
            .args(args)
            .status()
            .with_context(|| format!("Failed to execute local plugin: {}", local_plugin.display()))?;
        
        Ok(status.code().unwrap_or(1))
    } else {
        // Try $PATH
        let status = Command::new(&binary_name)
            .args(args)
            .status()
            .with_context(|| {
                format!(
                    "Plugin '{}' not found!\n\
                     Tried:\n\
                     1. Local: {}\n\
                     2. Global: {} in $PATH\n\
                     \n\
                     Install with:\n\
                     cargo build --release --manifest-path wj-plugins/{}/Cargo.toml",
                    plugin_name,
                    local_plugin.display(),
                    binary_name,
                    binary_name
                )
            })?;
        
        Ok(status.code().unwrap_or(1))
    }
}
