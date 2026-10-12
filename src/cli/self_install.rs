use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

fn wj_home() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .expect("Could not determine home directory (neither HOME nor USERPROFILE set)");
    PathBuf::from(home).join(".wj")
}

fn bin_dir() -> PathBuf {
    wj_home().join("bin")
}

pub fn execute(dry_run: bool) -> Result<()> {
    let verb = if dry_run { "Planning installation of" } else { "Installing" };
    println!("{}", format!("{verb} Windjammer toolchain...").cyan().bold());
    println!();

    let bin = bin_dir();
    if !dry_run {
        std::fs::create_dir_all(&bin)
            .with_context(|| format!("Failed to create directory: {}", bin.display()))?;
    }

    let installed = install_compiler_binary(&bin, dry_run)?;
    let plugins = install_plugins(&bin, dry_run)?;
    install_stdlib(dry_run)?;

    println!();
    check_path(&bin);

    println!();
    println!(
        "{}",
        format!(
            "Installed {} binary(ies) to {}",
            installed + plugins,
            bin.display()
        )
        .green()
        .bold()
    );

    Ok(())
}

fn install_compiler_binary(bin: &Path, dry_run: bool) -> Result<usize> {
    let current_exe =
        std::env::current_exe().context("Failed to determine current executable path")?;

    let repo_root = find_repo_root(&current_exe);

    let candidates: Vec<PathBuf> = if let Some(root) = &repo_root {
        vec![root.join("target/release/wj"), root.join("target/debug/wj")]
    } else {
        vec![current_exe.clone()]
    };

    let source = candidates
        .iter()
        .find(|p| p.exists())
        .unwrap_or(&current_exe);

    let dest = bin.join("wj");
    if dry_run {
        println!("  {} wj: {} -> {}", "would install".yellow(), source.display(), dest.display());
        return Ok(1);
    }
    if source == &dest {
        println!("  {} wj is already {}", "ok".green(), dest.display());
        return Ok(1);
    }
    std::fs::copy(source, &dest)
        .with_context(|| format!("Failed to copy {} -> {}", source.display(), dest.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }

    println!("  {} wj -> {}", "installed".green(), dest.display());

    Ok(1)
}

fn install_plugins(bin: &Path, dry_run: bool) -> Result<usize> {
    let current_exe =
        std::env::current_exe().context("Failed to determine current executable path")?;
    let repo_root = find_repo_root(&current_exe);
    let mut count = 0usize;

    let plugin_search_dirs: Vec<PathBuf> = if let Some(root) = &repo_root {
        vec![
            root.join("target/release"),
            root.join("target/debug"),
            root.parent()
                .unwrap_or(root)
                .join("game-host/wj-plugins/wj-game/target/release"),
            root.parent()
                .unwrap_or(root)
                .join("game-host/wj-plugins/wj-vgraph/target/release"),
        ]
    } else {
        vec![]
    };

    let plugin_names = ["wj-game", "wj-lint", "wj-vgraph"];

    for name in &plugin_names {
        for dir in &plugin_search_dirs {
            let source = dir.join(name);
            if source.exists() {
                let dest = bin.join(name);
                if dry_run {
                    println!("  {} {}: {} -> {}", "would install".yellow(), name, source.display(), dest.display());
                    count += 1;
                    break;
                }
                std::fs::copy(&source, &dest).with_context(|| {
                    format!("Failed to copy {} -> {}", source.display(), dest.display())
                })?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
                }

                println!("  {} {} -> {}", "installed".green(), name, dest.display());
                count += 1;
                break;
            }
        }
    }

    Ok(count)
}

/// Install `std/` from the copy embedded in the binary (`embedded_assets`), so an installed `wj`
/// needs no source checkout next to it.
fn install_stdlib(dry_run: bool) -> Result<()> {
    let std_dest = wj_home().join("std");
    let files = crate::embedded_assets::file_count(crate::embedded_assets::Asset::StdWj);
    if dry_run {
        println!(
            "  {} stdlib: {} embedded files -> {}",
            "would install".yellow(),
            files,
            std_dest.display()
        );
        return Ok(());
    }
    // replace, not merge: files removed from std/ must not linger from an older install
    if std_dest.exists() {
        std::fs::remove_dir_all(&std_dest)
            .with_context(|| format!("Failed to replace {}", std_dest.display()))?;
    }
    crate::embedded_assets::write_to(crate::embedded_assets::Asset::StdWj, &std_dest)
        .with_context(|| format!("Failed to install stdlib into {}", std_dest.display()))?;
    println!("  {} stdlib ({} files) -> {}", "installed".green(), files, std_dest.display());
    Ok(())
}

fn find_repo_root(exe_path: &Path) -> Option<PathBuf> {
    let mut dir = exe_path.parent()?;
    for _ in 0..10 {
        if dir.join("Cargo.toml").exists() && dir.join("src").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
    None
}

fn check_path(bin: &Path) {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let bin_str = bin.to_string_lossy();

    if path_var.split(':').any(|p| p == bin_str.as_ref()) {
        println!("  {} {} is already in PATH", "ok".green(), bin.display());
    } else {
        println!(
            "  {} {} is not in your PATH",
            "warning".yellow(),
            bin.display()
        );
        println!();
        println!("  Add this to your shell profile (~/.zshrc or ~/.bashrc):");
        println!();
        println!(
            "    {}",
            format!("export PATH=\"{}:$PATH\"", bin.display()).cyan()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wj_home_is_under_home() {
        let home = wj_home();
        assert!(home.ends_with(".wj"));
    }

    #[test]
    fn test_bin_dir_is_under_wj_home() {
        let bin = bin_dir();
        assert!(bin.ends_with(".wj/bin"));
    }
}
