//! The file a single-file build is given is the crate root of that build (a lone script, or a project's `main.wj`
//! built alone with `wj build src/main.wj`): the modules it declares with `mod name` (`name.wj` or `name/mod.wj`
//! beside the declaring file, recursively) are part of its build, exactly as in a project build. Each is generated
//! as a module file next to the entry's output; a generated `mod.rs` re-exports its public children
//! (`pub use self::child::*`), the rule the resolver and the interpreter follow (`lookup_child_reexports`).

use std::path::{Path, PathBuf};

/// The files a single-file build of `path` compiles: the modules the entry declares (first, so the entry reads
/// their current metadata), then the entry. A library build compiles `wj_files` only.
pub(super) fn lone_script_files(path: &Path, wj_files: &[PathBuf], library: bool) -> Vec<PathBuf> {
    if library || !path.is_file() {
        return wj_files.to_vec();
    }
    let mut modules = Vec::new();
    collect_declared_modules(path, &mut modules);
    modules.retain(|m| !wj_files.contains(m));
    modules.extend(wj_files.iter().cloned());
    modules
}

/// Where a declared module of the entry at `entry` is generated: its path relative to the entry's directory,
/// under `output` (`dep.wj` -> `out/dep.rs`, `shapes/mod.wj` -> `out/shapes/mod.rs`).
/// The generator learns that position (a declared `x/mod.wj` is generated as `mod.rs`, which re-exports its children).
pub(super) fn declared_module_output(
    entry: &Path,
    module: &Path,
    output: &Path,
    codegen: &mut crate::codegen::rust::CodeGenerator<'_>,
) -> anyhow::Result<PathBuf> {
    let base = entry.parent().unwrap_or(Path::new("."));
    let rel = module.strip_prefix(base).unwrap_or(module);
    let out = output.join(rel).with_extension("rs");
    // A directory module's own items go to `_mod_items.rs`, as in a project build; the shared writer then produces
    // its `mod.rs` (`write_directory_module_files`).
    let out = if out.file_name().is_some_and(|n| n == "mod.rs") { out.with_file_name("_mod_items.rs") } else { out };
    super::ensure_output_parent_dir(&out)?;
    codegen.set_output_position(out.clone(), output.to_path_buf(), base.to_path_buf());
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

/// After a single-file build generated the entry's declared modules: each directory module's `mod.rs` (`x/mod.wj`)
/// is written by the writer a project build uses (`pub mod` children and their `pub use self::child::*`
/// re-exports), so `wj build src/main.wj` emits what `wj build src` does for those modules.
pub(super) fn write_directory_module_files(path: &Path, wj_files: &[PathBuf], output: &Path, library: bool) -> anyhow::Result<()> {
    let base = path.parent().unwrap_or(Path::new("."));
    for module in lone_script_files(path, wj_files, library) {
        if wj_files.contains(&module) || module.file_name().is_none_or(|n| n != "mod.wj") {
            continue;
        }
        let rel = module.parent().and_then(|d| d.strip_prefix(base).ok()).unwrap_or(Path::new(""));
        crate::build_utils::generate_directory_mod_file(&output.join(rel), Some((output, base)))?;
    }
    Ok(())
}
