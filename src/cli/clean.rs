use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

pub fn execute(all: bool) -> Result<()> {
    println!("{}", "Cleaning Windjammer build artifacts...".cyan().bold());
    println!();

    let mut total_freed = 0u64;

    total_freed += clean_temp_artifacts()?;

    if all {
        total_freed += clean_local_target()?;
    }

    println!();
    if total_freed > 0 {
        println!(
            "  {} Freed approximately {}",
            "done".green().bold(),
            format_size(total_freed)
        );
    } else {
        println!("  {} Nothing to clean", "ok".green());
    }

    Ok(())
}

fn clean_temp_artifacts() -> Result<u64> {
    let mut freed = 0u64;
    let temp = std::env::temp_dir();

    let windjammer_test = temp.join("windjammer-test");
    if windjammer_test.exists() {
        let size = dir_size(&windjammer_test);
        std::fs::remove_dir_all(&windjammer_test)?;
        println!(
            "  {} removed {} ({})",
            "cleaned".green(),
            windjammer_test.display(),
            format_size(size)
        );
        freed += size;
    }

    let stale_prefixes = ["wj_", "wj-", "windjammer_", "windjammer-"];
    if let Ok(entries) = std::fs::read_dir(&temp) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if stale_prefixes.iter().any(|p| name_str.starts_with(p)) && entry.path().is_dir() {
                let size = dir_size(&entry.path());
                if let Err(e) = std::fs::remove_dir_all(entry.path()) {
                    eprintln!(
                        "  {} failed to remove {}: {}",
                        "warning".yellow(),
                        entry.path().display(),
                        e
                    );
                } else {
                    println!(
                        "  {} removed {} ({})",
                        "cleaned".green(),
                        entry.path().display(),
                        format_size(size)
                    );
                    freed += size;
                }
            }
        }
    }

    Ok(freed)
}

fn clean_local_target() -> Result<u64> {
    let mut freed = 0u64;

    let cwd = std::env::current_dir()?;
    let target = cwd.join("target");
    if target.exists() {
        let size = dir_size(&target);
        println!(
            "  {} removing {} ({})",
            "cleaning".yellow(),
            target.display(),
            format_size(size)
        );
        std::fs::remove_dir_all(&target)?;
        freed += size;
    }

    let nested_targets = find_nested_targets(&cwd, 3);
    for nested in nested_targets {
        if nested == target {
            continue;
        }
        let size = dir_size(&nested);
        if size > 0 {
            println!(
                "  {} removing {} ({})",
                "cleaning".yellow(),
                nested.display(),
                format_size(size)
            );
            std::fs::remove_dir_all(&nested)?;
            freed += size;
        }
    }

    Ok(freed)
}

fn find_nested_targets(dir: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut results = Vec::new();
    find_nested_targets_recursive(dir, max_depth, 0, &mut results);
    results
}

fn find_nested_targets_recursive(
    dir: &Path,
    max_depth: usize,
    current_depth: usize,
    results: &mut Vec<PathBuf>,
) {
    if current_depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str == "target" && path.join("release").exists() || path.join("debug").exists() {
            results.push(path);
        } else if name_str != "node_modules" && !name_str.starts_with('.') {
            find_nested_targets_recursive(&path, max_depth, current_depth + 1, results);
        }
    }
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} bytes", bytes)
    }
}
