//! `wj eject`: turn a Windjammer project into a standalone Cargo project.
//!
//! Ejection runs the exact `wj build` pipeline (same codegen, same `Cargo.toml` generation),
//! then makes the result self-contained:
//! - every `path = "..."` dependency (the embedded `windjammer-runtime`, path deps from
//!   `wj.toml`) is copied into `vendor/<name>/` and re-pointed with a relative path;
//! - build bookkeeping (`.wj-compiler-stamp`, `.wj-cache/`, `metadata.json`) is removed;
//! - a README and `.gitignore` are added.
//!
//! The generated `.rs` files are byte-identical to `wj build` output, so the ejected program
//! behaves exactly like the built one, and the tree builds anywhere with plain `cargo build`.

use crate::compiler_config::CompilerConfig;
use crate::CompilationTarget;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Options for [`eject`].
#[derive(Debug, Clone)]
pub struct EjectOptions {
    pub target: CompilationTarget,
    /// Library crate (`wj build --library`).
    pub library: bool,
    /// `--module-file`: generate `mod.rs` re-exports for multi-module libraries.
    pub module_file: bool,
}

impl Default for EjectOptions {
    fn default() -> Self {
        Self {
            target: CompilationTarget::Rust,
            library: false,
            module_file: false,
        }
    }
}

/// What [`eject`] produced.
#[derive(Debug)]
pub struct EjectReport {
    pub output: PathBuf,
    pub rust_files: Vec<PathBuf>,
    /// `(crate name, vendored dir relative to output)`.
    pub vendored: Vec<(String, PathBuf)>,
}

type Vendored = BTreeMap<PathBuf, (String, PathBuf)>;

/// Eject `input` (a `.wj` file or project directory) into `output`, which must be absent or empty.
pub fn eject(
    input: &Path,
    output: &Path,
    opts: &EjectOptions,
    config: &CompilerConfig,
) -> Result<EjectReport> {
    if !input.exists() {
        bail!("input {} does not exist", input.display());
    }
    if output.exists() && fs::read_dir(output)?.next().is_some() {
        bail!(
            "output directory {} is not empty; choose a new directory",
            output.display()
        );
    }
    fs::create_dir_all(output)?;
    let output = output.canonicalize()?;

    crate::build_project_with_config(input, &output, opts.target, true, opts.library, &[], config)?;
    crate::build_utils::apply_library_build_post_steps(
        input,
        &output,
        opts.library,
        opts.module_file,
    )?;

    let _ = fs::remove_file(output.join(".wj-compiler-stamp"));
    let _ = fs::remove_dir_all(output.join(".wj-cache"));
    // Cross-crate Windjammer metadata is meaningless to a plain Cargo project.
    let _ = fs::remove_file(output.join("metadata.json"));

    let manifest = output.join("Cargo.toml");
    if !manifest.is_file() {
        bail!("build produced no Cargo.toml in {}", output.display());
    }
    let mut vendored = Vendored::new();
    vendor_path_deps(&manifest, &output, &mut vendored)?;
    stamp_manifest_header(&manifest)?;

    write_if_absent(&output.join("README.md"), &readme(input))?;
    write_if_absent(&output.join(".gitignore"), "/target/\n")?;

    let mut rust_files = Vec::new();
    collect_rs(&output, &output.join("vendor"), &mut rust_files);
    rust_files.sort();
    Ok(EjectReport {
        output,
        rust_files,
        vendored: vendored.into_values().collect(),
    })
}

/// Rewrite every `path = "..."` dependency in `manifest` to a copy under `<root>/vendor/`.
/// Recurses into vendored manifests; `seen` maps canonical source dir -> (name, vendored rel path).
fn vendor_path_deps(manifest: &Path, root: &Path, seen: &mut Vendored) -> Result<()> {
    let text = fs::read_to_string(manifest)?;
    let parsed: toml::Value =
        toml::from_str(&text).with_context(|| format!("parse {}", manifest.display()))?;
    let manifest_dir = manifest.parent().unwrap_or(Path::new("."));
    let mut rewritten = text.clone();

    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(deps) = parsed.get(section).and_then(|v| v.as_table()) else {
            continue;
        };
        for (name, spec) in deps {
            let Some(path_str) = spec.get("path").and_then(|p| p.as_str()) else {
                continue;
            };
            let src = manifest_dir.join(path_str);
            let canon = src
                .canonicalize()
                .with_context(|| format!("path dependency {name} = {}", src.display()))?;
            if canon.starts_with(root) {
                continue; // already inside the ejected tree
            }
            let rel_from_root = match seen.get(&canon) {
                Some((_, rel)) => rel.clone(),
                None => {
                    let rel = PathBuf::from("vendor").join(unique_dir_name(name, seen));
                    seen.insert(canon.clone(), (name.clone(), rel.clone()));
                    copy_crate(&canon, &root.join(&rel))?;
                    let vendored_manifest = root.join(&rel).join("Cargo.toml");
                    strip_workspace_section(&vendored_manifest)?;
                    vendor_path_deps(&vendored_manifest, root, seen)?;
                    rel
                }
            };
            let new_path = relative_path(manifest_dir, &root.join(&rel_from_root), root);
            rewritten = rewritten.replace(
                &format!("path = \"{path_str}\""),
                &format!("path = \"{new_path}\""),
            );
        }
    }
    fs::write(manifest, rewritten)?;
    Ok(())
}

fn unique_dir_name(name: &str, seen: &Vendored) -> String {
    let taken = |n: &str| seen.values().any(|(_, rel)| rel.ends_with(n));
    if !taken(name) {
        return name.to_string();
    }
    (2..)
        .map(|i| format!("{name}-{i}"))
        .find(|n| !taken(n))
        .unwrap()
}

/// `to` relative to `from`; both are inside `root` (vendored crates live at `root/vendor/<n>`).
fn relative_path(from: &Path, to: &Path, root: &Path) -> String {
    let from_rel = from.strip_prefix(root).unwrap_or(Path::new(""));
    let to_rel = to.strip_prefix(root).unwrap_or(to);
    let mut out = PathBuf::new();
    for _ in from_rel.components() {
        out.push("..");
    }
    out.push(to_rel);
    out.to_string_lossy().replace('\\', "/")
}

fn copy_crate(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let n = name.to_string_lossy();
        if matches!(
            n.as_ref(),
            "target" | ".git" | ".wj-cache" | "Cargo.lock" | ".complete"
        ) {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if entry.file_type()?.is_dir() {
            copy_crate(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// A vendored crate becomes a member of the ejected workspace, so it must not declare its own.
fn strip_workspace_section(manifest: &Path) -> Result<()> {
    let text = fs::read_to_string(manifest)?;
    let mut out = String::new();
    let mut skipping = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            skipping = t == "[workspace]" || t.starts_with("[workspace.");
        }
        if !skipping {
            out.push_str(line);
            out.push('\n');
        }
    }
    fs::write(manifest, out)?;
    Ok(())
}

fn stamp_manifest_header(manifest: &Path) -> Result<()> {
    let text = fs::read_to_string(manifest)?;
    let body = text
        .strip_prefix("# Auto-generated by Windjammer compiler - do not edit manually\n")
        .unwrap_or(&text);
    let header = format!(
        "# Ejected from Windjammer {} with `wj eject`. Standalone Cargo project: edit freely.\n",
        env!("CARGO_PKG_VERSION")
    );
    fs::write(manifest, format!("{header}{body}"))?;
    Ok(())
}

fn write_if_absent(path: &Path, content: &str) -> Result<()> {
    if !path.exists() {
        fs::write(path, content)?;
    }
    Ok(())
}

fn readme(input: &Path) -> String {
    let name = input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());
    format!(
        "# {name}\n\n\
         Standalone Rust project ejected from Windjammer {ver} with `wj eject`.\n\n\
         The Rust sources are exactly what `wj build` generates. Dependencies that lived outside\n\
         this directory (including `windjammer-runtime`) are vendored under `vendor/`, so the\n\
         project builds anywhere:\n\n\
         ```bash\ncargo build\ncargo run      # binaries\ncargo test\n```\n",
        ver = env!("CARGO_PKG_VERSION")
    )
}

fn collect_rs(dir: &Path, skip: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p == skip || p.file_name().is_some_and(|n| n == "target") {
            continue;
        }
        if p.is_dir() {
            collect_rs(&p, skip, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_from_root_and_vendor() {
        let root = Path::new("/o");
        assert_eq!(
            relative_path(root, &root.join("vendor/rt"), root),
            "vendor/rt"
        );
        assert_eq!(
            relative_path(&root.join("vendor/a"), &root.join("vendor/rt"), root),
            "../../vendor/rt"
        );
    }

    #[test]
    fn workspace_section_is_stripped() {
        let d = std::env::temp_dir().join(format!("wj-eject-ws-{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        let m = d.join("Cargo.toml");
        fs::write(
            &m,
            "[package]\nname = \"a\"\n\n[workspace]\nmembers = []\n\n[dependencies]\nx = \"1\"\n",
        )
        .unwrap();
        strip_workspace_section(&m).unwrap();
        let t = fs::read_to_string(&m).unwrap();
        assert!(!t.contains("[workspace]") && !t.contains("members"), "{t}");
        assert!(t.contains("[dependencies]\nx = \"1\""), "{t}");
        let _ = fs::remove_dir_all(&d);
    }
}
