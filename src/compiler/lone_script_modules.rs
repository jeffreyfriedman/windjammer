//! A lone script (a file built alone, no `wj.toml` above it) is its own crate root: the modules it declares with
//! `mod name` (`name.wj` or `name/mod.wj` beside the declaring file, recursively) are part of its build, exactly as
//! a project's modules are. Each is generated as a module file next to the script's output.

use std::path::{Path, PathBuf};

/// The files a single-file build of `path` compiles: the modules the script declares (first, so the script reads
/// their current metadata), then the script. A file inside a project, or a library build, compiles `wj_files` only.
pub(super) fn lone_script_files(path: &Path, wj_files: &[PathBuf], library: bool) -> Vec<PathBuf> {
    if library || !path.is_file() || crate::module_system::project::find_manifest_dir(path).is_some() {
        return wj_files.to_vec();
    }
    let mut modules = Vec::new();
    collect_declared_modules(path, &mut modules);
    modules.retain(|m| !wj_files.contains(m));
    modules.extend(wj_files.iter().cloned());
    modules
}

/// Where a declared module of the script at `entry` is generated: its path relative to the script's directory,
/// under `output` (`dep.wj` -> `out/dep.rs`, `shapes/mod.wj` -> `out/shapes/mod.rs`).
pub(super) fn declared_module_output(entry: &Path, module: &Path, output: &Path) -> anyhow::Result<PathBuf> {
    let base = entry.parent().unwrap_or(Path::new("."));
    let rel = module.strip_prefix(base).unwrap_or(module);
    let out = output.join(rel).with_extension("rs");
    super::ensure_output_parent_dir(&out)?;
    Ok(out)
}

/// The module files `file` declares by `mod name` without a body, and theirs, depth first.
fn collect_declared_modules(file: &Path, out: &mut Vec<PathBuf>) {
    let Ok(source) = std::fs::read_to_string(file) else { return };
    let Ok((_parser, program)) = super::parse_wj_source(file, &source) else { return };
    let dir = file.parent().unwrap_or(Path::new("."));
    // `dep/mod.wj` declares its children beside itself; `dep.wj` declares them in `dep/`.
    let child_dir = if file.file_stem().is_some_and(|s| s == "mod") {
        dir.to_path_buf()
    } else {
        dir.join(file.file_stem().unwrap_or_default())
    };
    let names: Vec<String> = program
        .items
        .iter()
        .filter_map(|item| match item {
            crate::parser::Item::Mod { name, items, .. } if items.is_empty() => Some(name.clone()),
            _ => None,
        })
        .collect();
    for name in names {
        let found = [dir.join(format!("{name}.wj")), dir.join(&name).join("mod.wj"), child_dir.join(format!("{name}.wj"))]
            .into_iter()
            .find(|p| p.is_file() && p != file);
        if let Some(module) = found.filter(|m| !out.contains(m)) {
            collect_declared_modules(&module, out);
            out.push(module);
        }
    }
}
