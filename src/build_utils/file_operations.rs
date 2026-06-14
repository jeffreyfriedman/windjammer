//! File copy and transform helpers used during build output assembly.

use std::path::Path;

/// Copy top-level `*.rs` files and `*/mod.rs` module trees from `<project>/src/` into the output
/// directory when they are not already present. This keeps hand-written Rust under `<project>/src/`
/// discoverable in `out/` without pulling in unrelated trees (output is usually `out/`, not under
/// `src/`).
pub(crate) fn copy_project_src_tree_into_output(output_dir: &Path) -> std::io::Result<()> {
    use std::fs;
    let Some(root) = output_dir.parent() else {
        return Ok(());
    };
    let root = if root.as_os_str().is_empty() {
        Path::new(".")
    } else {
        root
    };
    let src_dir = root.join("src");
    if !src_dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(&src_dir)? {
        let entry = entry?;
        let p = entry.path();
        let name = entry.file_name();
        let dest = output_dir.join(&name);
        if p.is_file() {
            if p.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            // Skip `src/foo.rs` when `src/foo/` exists (Rust's split module layout: parent file +
            // subfolder is one logical module; copying only the `.rs` breaks `out/`).
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                if src_dir.join(stem).is_dir() {
                    continue;
                }
            }
            if !dest.exists() {
                fs::copy(&p, &dest)?;
            }
        } else if p.is_dir() && p.join("mod.rs").exists() {
            copy_dir_merge_shallow(&p, &dest)?;
        }
    }
    Ok(())
}

pub(crate) fn copy_dir_merge_shallow(src: &Path, dst: &Path) -> std::io::Result<()> {
    use std::fs;
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_merge_shallow(&from, &to)?;
        } else if !to.exists() {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Copy hand-written sibling `*.rs` from the parent directory into `output_dir` when the file
/// does not already exist in the output (Windjammer-emitted files take precedence).
///
/// This picks up:
/// - `components/platform.rs` next to `components/generated/` (same parent as `generated/`)
/// - `ffi.rs` (or other root modules) next to `out/` when building into a crate output folder
pub(crate) fn copy_sibling_rs_from_parent(output_dir: &Path) -> std::io::Result<()> {
    use std::fs;
    let Some(parent) = output_dir.parent() else {
        return Ok(());
    };
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        // Avoid hijacking Cargo's build script or entrypoints from the parent folder.
        if matches!(stem, "build" | "main" | "lib") {
            continue;
        }
        // Never copy wj test harness outputs into a library output dir (e.g. player_test.rs
        // transpiled next to lib/ would otherwise become a spurious lib module).
        if stem.ends_with("_test")
            || stem.ends_with("_tests")
            || stem.starts_with("test_")
            || stem == "tests"
        {
            continue;
        }
        let dest = output_dir.join(p.file_name().unwrap());
        if !dest.exists() {
            fs::copy(&p, &dest)?;
        }
    }
    Ok(())
}

/// Strip main() functions from generated Rust files (library mode)
pub fn strip_main_functions(output_dir: &Path) -> anyhow::Result<()> {
    use colored::*;
    use std::fs;

    let mut stripped_count = 0;

    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs") && file_name != "mod.rs" {
                    let content = fs::read_to_string(&path)?;

                    let mut new_lines = Vec::new();
                    let mut found_main = false;

                    for line in content.lines() {
                        let trimmed = line.trim();

                        if trimmed.starts_with("fn main()") || trimmed.starts_with("pub fn main()")
                        {
                            found_main = true;
                            stripped_count += 1;
                            break;
                        }

                        new_lines.push(line);
                    }

                    if found_main {
                        let new_content = new_lines.join("\n") + "\n";
                        fs::write(&path, new_content)?;
                    }
                }
            }
        }
    }

    if stripped_count > 0 {
        println!(
            "{} Stripped {} main() functions (library mode)",
            "✓".green(),
            stripped_count
        );
    }

    Ok(())
}
