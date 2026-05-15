//! Path discovery, transitive dependency propagation, and external crate resolution.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Convert path to string suitable for Cargo.toml (absolute, forward-slash, no Windows \\?\ prefix)
pub(crate) fn path_to_toml_string(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    sanitize_path_for_toml(&abs)
}

/// Sanitize a path for use in TOML string values:
/// - Strip Windows extended-length prefix (\\?\)
/// - Convert backslashes to forward slashes (valid in Cargo.toml on all platforms)
pub(crate) fn sanitize_path_for_toml(path: &Path) -> String {
    let s = path.display().to_string();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.replace('\\', "/")
}

/// Find windjammer-runtime path for Cargo.toml dependency
pub(crate) fn find_windjammer_runtime_path() -> PathBuf {
    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let original_cwd = current.clone();

    // Try current directory first (if we're in windjammer repo)
    if current
        .join("crates/windjammer-runtime/Cargo.toml")
        .exists()
    {
        return current.join("crates/windjammer-runtime");
    }

    // Also check windjammer/ subdirectory (running from workspace root)
    if current
        .join("windjammer/crates/windjammer-runtime/Cargo.toml")
        .exists()
    {
        return current.join("windjammer/crates/windjammer-runtime");
    }

    // Search upward (up to 5 levels)
    for _ in 0..5 {
        if let Some(parent) = current.parent() {
            if parent
                .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return parent.join("windjammer/crates/windjammer-runtime");
            }
            if parent.join("crates/windjammer-runtime/Cargo.toml").exists() {
                return parent.join("crates/windjammer-runtime");
            }
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Try the compiler's own location (the wj binary is in windjammer/target/release/)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut search = exe_dir.to_path_buf();
            for _ in 0..5 {
                if search.join("crates/windjammer-runtime/Cargo.toml").exists() {
                    return search.join("crates/windjammer-runtime");
                }
                if let Some(p) = search.parent() {
                    search = p.to_path_buf();
                } else {
                    break;
                }
            }
        }
    }

    // Fallback: relative paths
    let sibling = original_cwd.join("../windjammer/crates/windjammer-runtime");
    if sibling.join("Cargo.toml").exists() {
        return sibling;
    }
    PathBuf::from("./crates/windjammer-runtime")
}

/// Convert a `DependencySpec` into a Cargo.toml dependency line.
pub(crate) fn dep_spec_to_cargo_line(name: &str, spec: &crate::config::DependencySpec) -> String {
    match spec {
        crate::config::DependencySpec::Simple(version) => {
            format!("{} = \"{}\"", name, version)
        }
        crate::config::DependencySpec::Detailed {
            version,
            features,
            path,
            git,
            branch,
        } => {
            let mut parts = Vec::new();
            if let Some(v) = version {
                parts.push(format!("version = \"{}\"", v));
            }
            if let Some(f) = features {
                let feat_str: Vec<String> = f.iter().map(|s| format!("\"{}\"", s)).collect();
                parts.push(format!("features = [{}]", feat_str.join(", ")));
            }
            if let Some(p) = path {
                parts.push(format!("path = \"{}\"", p));
            }
            if let Some(g) = git {
                parts.push(format!("git = \"{}\"", g));
            }
            if let Some(b) = branch {
                parts.push(format!("branch = \"{}\"", b));
            }
            format!("{} = {{ {} }}", name, parts.join(", "))
        }
    }
}

/// Read source project's Cargo.toml and propagate dependencies that aren't already present.
/// This ensures FFI dependencies (wgpu, bytemuck, rapier3d, etc.) are available in the
/// generated build's Cargo.toml without requiring the user to manually copy them.
pub(crate) fn propagate_source_cargo_deps(
    source_dir: &Path,
    existing_deps: &[String],
) -> Vec<String> {
    use std::collections::HashSet;

    let skip_crates: HashSet<&str> = ["windjammer", "windjammer-runtime", "windjammer_runtime"]
        .into_iter()
        .collect();

    let candidates = [
        source_dir.join("Cargo.toml"),
        source_dir
            .parent()
            .map(|p| p.join("Cargo.toml"))
            .unwrap_or_default(),
    ];

    let cargo_path = candidates.iter().find(|p| p.exists());
    let cargo_path = match cargo_path {
        Some(p) => p,
        None => return Vec::new(),
    };

    let content = match fs::read_to_string(cargo_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut propagated = Vec::new();
    let mut in_deps = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }

        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let dep_name = match trimmed.split(['=', ' ']).next() {
            Some(n) => n.trim(),
            None => continue,
        };

        if dep_name.is_empty() {
            continue;
        }

        if skip_crates.contains(dep_name) {
            continue;
        }

        let already_present = existing_deps
            .iter()
            .any(|d| d.starts_with(dep_name) || d.starts_with(&dep_name.replace('-', "_")));

        if !already_present {
            propagated.push(trimmed.to_string());
        }
    }

    propagated
}

/// Scan generated .rs files for `use <crate>::...` imports and resolve crate paths.
pub(crate) fn detect_external_crate_deps(output_dir: &Path, source_dir: &Path) -> Vec<String> {
    use std::collections::HashSet;

    let builtin_crates: HashSet<&str> = [
        "std",
        "core",
        "alloc",
        "crate",
        "self",
        "super",
        "windjammer_runtime",
        "windjammer",
        "serde",
        "serde_core",
        "smallvec",
        "glob",
        "typenum",
        "bytemuck",
    ]
    .into_iter()
    .collect();

    let output_pkg_name = {
        let p = output_dir.join("Cargo.toml");
        read_package_name(&p)
            .map(|n| n.replace('-', "_"))
            .unwrap_or_default()
    };

    let mut external_crates: HashSet<String> = HashSet::new();

    if let Ok(entries) = walk_rs_files(output_dir) {
        for path in entries {
            if let Ok(content) = fs::read_to_string(&path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if let Some(rest) = trimmed.strip_prefix("use ") {
                        if let Some(crate_name) = rest.split("::").next() {
                            let crate_name = crate_name.trim().trim_start_matches('{');
                            if !crate_name.is_empty()
                                && !builtin_crates.contains(crate_name)
                                && crate_name != output_pkg_name
                                && crate_name.chars().next().is_some_and(|c| c.is_alphabetic())
                            {
                                external_crates.insert(crate_name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    let mut deps = Vec::new();
    for crate_name in &external_crates {
        if let Some(dep_line) = resolve_crate_path(crate_name, source_dir, output_dir) {
            deps.push(dep_line);
        }
    }
    deps
}

pub(crate) fn walk_rs_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, files);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    files.push(path);
                }
            }
        }
    }
    walk(dir, &mut files);
    Ok(files)
}

/// Attempt to find the crate's path on disk for path-based dependency.
///
/// Searches upward from `source_dir` (up to 5 levels) and checks each
/// ancestor's subdirectories (up to 2 levels deep) for a matching
/// `Cargo.toml`. Uses absolute paths so relative `source_dir` values
/// (e.g. `"src"`) don't limit traversal depth. Skips any match that
/// points to `output_dir` itself to prevent cyclic self-dependencies.
pub(crate) fn resolve_crate_path(
    crate_name: &str,
    source_dir: &Path,
    output_dir: &Path,
) -> Option<String> {
    let hyphenated = crate_name.replace('_', "-");

    let abs_source = source_dir.canonicalize().unwrap_or_else(|_| {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(source_dir)
    });

    let mut current = abs_source.clone();
    for _ in 0..5 {
        if let Some(found) = try_find_crate_at(&current, &hyphenated, crate_name, output_dir) {
            return Some(found);
        }
        if !current.pop() {
            break;
        }
    }

    None
}

/// Check `root` and its immediate subdirectories for `<hyphenated>/Cargo.toml`.
pub(crate) fn try_find_crate_at(
    root: &Path,
    hyphenated: &str,
    crate_name: &str,
    output_dir: &Path,
) -> Option<String> {
    // Direct child: <root>/<hyphenated>/Cargo.toml
    if let Some(dep) = check_cargo_toml(&root.join(hyphenated), crate_name, output_dir) {
        return Some(dep);
    }
    // Compiled output variant: <root>/<hyphenated>/src/Cargo.toml
    let src_variant = root.join(hyphenated).join("src");
    if let Some(dep) = check_cargo_toml(&src_variant, crate_name, output_dir) {
        return Some(dep);
    }
    // One level deeper: <root>/*/<hyphenated>/Cargo.toml
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() {
                if let Some(dep) = check_cargo_toml(&child.join(hyphenated), crate_name, output_dir)
                {
                    return Some(dep);
                }
            }
        }
    }
    None
}

/// If `crate_dir/Cargo.toml` exists, produce a dependency line for it.
/// `output_dir` is the directory we're generating into — we must never add
/// a dependency that points back to it (that would create a cyclic package).
pub(crate) fn check_cargo_toml(
    crate_dir: &Path,
    crate_name: &str,
    output_dir: &Path,
) -> Option<String> {
    let cargo_toml = crate_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return None;
    }
    let abs = crate_dir
        .canonicalize()
        .unwrap_or_else(|_| crate_dir.to_path_buf());
    let abs_output = output_dir
        .canonicalize()
        .unwrap_or_else(|_| output_dir.to_path_buf());
    if abs == abs_output {
        return None;
    }
    let actual_pkg = read_package_name(&cargo_toml).unwrap_or_else(|| crate_name.to_string());
    let hyphenated = crate_name.replace('_', "-");
    let path_str = sanitize_path_for_toml(&abs);
    if actual_pkg.replace('-', "_") != crate_name {
        Some(format!(
            "{} = {{ path = \"{}\", package = \"{}\" }}",
            crate_name, path_str, actual_pkg
        ))
    } else {
        Some(format!("{} = {{ path = \"{}\" }}", hyphenated, path_str))
    }
}

pub(crate) fn read_package_name(cargo_toml_path: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_toml_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") {
            return trimmed.split('"').nth(1).map(|s| s.to_string());
        }
    }
    None
}
