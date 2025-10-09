// wj add - Add dependencies to wj.toml
use crate::config::{DependencySpec, WjConfig};
use std::path::Path;

pub fn execute(
    package: &str,
    version: Option<&str>,
    features: Option<&str>,
    path: Option<&str>,
) -> anyhow::Result<()> {
    // Load wj.toml from current directory
    let wj_toml_path = Path::new("wj.toml");

    if !wj_toml_path.exists() {
        anyhow::bail!("wj.toml not found in current directory. Are you in a Windjammer project?");
    }

    let mut config =
        WjConfig::load_from_file(wj_toml_path).map_err(|e| anyhow::anyhow!("{}", e))?;

    // Create dependency spec
    let spec = if let Some(features_str) = features {
        let features_vec: Vec<String> = features_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        DependencySpec::Detailed {
            version: version.map(String::from),
            features: Some(features_vec),
            path: path.map(String::from),
            git: None,
            branch: None,
        }
    } else if path.is_some() {
        DependencySpec::Detailed {
            version: version.map(String::from),
            features: None,
            path: path.map(String::from),
            git: None,
            branch: None,
        }
    } else {
        match version {
            Some(v) => DependencySpec::Simple(v.to_string()),
            None => {
                // Fetch latest version from crates.io (for now, use a placeholder)
                println!("Warning: No version specified, using latest from crates.io");
                DependencySpec::Simple("*".to_string())
            }
        }
    };

    // Add the dependency
    config.add_dependency(package.to_string(), spec);

    // Save wj.toml
    config
        .save_to_file(wj_toml_path)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Generate Cargo.toml
    let cargo_toml_content = config.to_cargo_toml();
    std::fs::write("Cargo.toml", cargo_toml_content)?;

    println!("✓ Added {} to wj.toml", package);
    println!("✓ Updated Cargo.toml");

    // Run cargo update if cargo is available
    if let Ok(output) = std::process::Command::new("cargo")
        .arg("update")
        .arg("-p")
        .arg(package)
        .output()
    {
        if output.status.success() {
            println!("✓ Updated Cargo.lock");
        }
    }

    Ok(())
}
