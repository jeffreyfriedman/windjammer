// wj remove - Remove dependencies from wj.toml
use crate::config::WjConfig;
use std::path::Path;

pub fn execute(package: &str) -> anyhow::Result<()> {
    // Load wj.toml from current directory
    let wj_toml_path = Path::new("wj.toml");

    if !wj_toml_path.exists() {
        anyhow::bail!("wj.toml not found in current directory. Are you in a Windjammer project?");
    }

    let mut config =
        WjConfig::load_from_file(wj_toml_path).map_err(|e| anyhow::anyhow!("{}", e))?;

    // Remove the dependency
    let removed = config.remove_dependency(package);

    if !removed {
        anyhow::bail!("Dependency '{}' not found in wj.toml", package);
    }

    // Save wj.toml
    config
        .save_to_file(wj_toml_path)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Generate Cargo.toml
    let cargo_toml_content = config.to_cargo_toml();
    std::fs::write("Cargo.toml", cargo_toml_content)?;

    println!("✓ Removed {} from wj.toml", package);
    println!("✓ Updated Cargo.toml");

    // Run cargo update if cargo is available
    if let Ok(output) = std::process::Command::new("cargo").arg("update").output() {
        if output.status.success() {
            println!("✓ Updated Cargo.lock");
        }
    }

    Ok(())
}
