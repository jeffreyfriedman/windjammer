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

pub fn execute() -> Result<()> {
    println!("{}", "Installing Windjammer toolchain...".cyan().bold());
    println!();

    let bin = bin_dir();
    std::fs::create_dir_all(&bin)
        .with_context(|| format!("Failed to create directory: {}", bin.display()))?;

    let installed = install_compiler_binary(&bin)?;
    let plugins = install_plugins(&bin)?;
    install_stdlib()?;

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

fn install_compiler_binary(bin: &Path) -> Result<usize> {
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

fn install_plugins(bin: &Path) -> Result<usize> {
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
                .join("windjammer-game/wj-plugins/wj-game/target/release"),
        ]
    } else {
        vec![]
    };

    let plugin_names = ["wj-game", "wj-lint"];

    for name in &plugin_names {
        for dir in &plugin_search_dirs {
            let source = dir.join(name);
            if source.exists() {
                let dest = bin.join(name);
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

fn install_stdlib() -> Result<()> {
    let current_exe =
        std::env::current_exe().context("Failed to determine current executable path")?;
    let repo_root = find_repo_root(&current_exe);

    if let Some(root) = repo_root {
        let std_src = root.join("std");
        if std_src.exists() {
            let std_dest = wj_home().join("std");
            copy_dir_recursive(&std_src, &std_dest)?;
            println!("  {} stdlib -> {}", "installed".green(), std_dest.display());
        }
    }

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

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
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
