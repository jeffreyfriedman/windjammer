//! The file a single-file build is given is the crate root of that build (a lone script, or a project's `main.wj`
//! built alone with `wj build src/main.wj`): the modules it declares with `mod name` (`name.wj` or `name/mod.wj`
//! beside the declaring file, recursively) are part of its build, exactly as in a project build. Each is generated
//! as a module file next to the entry's output; a generated `mod.rs` re-exports its public children
//! (`pub use self::child::*`), the rule the resolver and the interpreter follow (`lookup_child_reexports`).

use std::path::{Path, PathBuf};

/// The files a single-file build of `path` compiles: the modules the entry declares (first, so the entry reads
/// their current metadata), then the entry. A library build compiles `wj_files` only.
pub(super) fn lone_script_files(path: &Path, wj_files: &[PathBuf], library: bool, config: &crate::compiler_config::CompilerConfig) -> anyhow::Result<Vec<PathBuf>> {
    if library || !path.is_file() {
        return Ok(wj_files.to_vec());
    }
    let mut modules = Vec::new();
    let root = path.parent().unwrap_or(Path::new("."));
    collect_declared_modules(path, path, root, config.entry_use_paths_name_sibling_modules, &mut modules)?;
    modules.retain(|m| !wj_files.contains(m));
    modules.extend(wj_files.iter().cloned());
    Ok(modules)
}

/// The first segments of the `use crate::<module>::...` paths of `program` that name no item the program itself
/// declares: they name modules of the crate root, i.e. siblings of the entry file (decided by declaration: an item
/// the entry declares, `fn`/`struct`/`enum`/`trait`/`const`/`static`/`mod`, is not a module).
fn crate_root_modules_used(program: &crate::parser::Program<'_>) -> Vec<String> {
    use crate::parser::Item;
    let declared: Vec<&str> = program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Function { decl, .. } => Some(decl.name.as_str()),
            Item::Struct { decl, .. } => Some(decl.name.as_str()),
            Item::Enum { decl, .. } => Some(decl.name.as_str()),
            Item::Trait { decl, .. } => Some(decl.name.as_str()),
            Item::Const { name, .. } | Item::Static { name, .. } | Item::Mod { name, .. } => Some(name.as_str()),
            Item::TypeAlias { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let mut used: Vec<String> = Vec::new();
    for item in &program.items {
        if let Item::Use { path, .. } = item {
            if path.len() >= 2 && path[0] == "crate" && !declared.contains(&path[1].as_str()) && !used.contains(&path[1]) {
                used.push(path[1].clone());
            }
        }
    }
    used
}

/// The entry file's `mod name` declarations for the sibling modules its `use crate::name::...` paths reach, added
/// to `program` so the generated `main.rs` declares them (`mod adapters;`), as a project build's crate root does.
/// Only a name whose `name.wj` or `name/mod.wj` exists beside the entry is declared; any other `use crate::name` keeps
/// its path as written (an item of the file itself, or an extern/inline path), with no error.
pub(super) fn declare_used_crate_modules(program: &mut crate::parser::Program<'static>, root: &Path) {
    let used = crate_root_modules_used(program);
    for name in used {
        if !root.join(format!("{name}.wj")).is_file() && !root.join(&name).join("mod.wj").is_file() {
            continue;
        }
        let already = program.items.iter().any(|i| matches!(i, crate::parser::Item::Mod { name: n, .. } if *n == name));
        if !already {
            let location = program.items.iter().find_map(|i| match i {
                crate::parser::Item::Use { location, .. } => Some(location.clone()),
                _ => None,
            });
            if let Some(location) = location {
                program.items.insert(0, crate::parser::Item::Mod { name: name.clone(), items: Vec::new(), is_public: false, location });
            }
            // `use crate::util` imports the module itself; the `mod util;` just declared already binds that name in the
            // crate root, and keeping both is rustc E0255 (the name defined twice).
            program.items.retain(|i| {
                !matches!(i, crate::parser::Item::Use { path, alias: None, is_pub: false, .. } if path.len() == 2 && path[0] == "crate" && path[1] == name)
            });
        }
    }
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
/// `root` is the entry's directory, the crate root that `use crate::name` paths resolve against. A `use crate::name` path
/// names a module only when `name.wj` or `name/mod.wj` exists there (otherwise it is left as written).
fn collect_declared_modules(entry: &Path, file: &Path, root: &Path, use_paths: bool, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let Ok(source) = std::fs::read_to_string(file) else { return Ok(()) };
    let Ok((_parser, program)) = super::parse_wj_source(file, &source) else { return Ok(()) };
    let entry_program_used = crate_root_modules_used(&program);
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
            collect_declared_modules(entry, &module, root, use_paths, out)?;
            out.push(module);
        }
    }
    for name in entry_program_used.into_iter().filter(|_| use_paths) {
        if out.iter().any(|m| crate_module_name(root, m).as_deref() == Some(name.as_str())) {
            continue;
        }
        let candidates = [root.join(format!("{name}.wj")), root.join(&name).join("mod.wj")];
        match candidates.iter().find(|p| p.is_file() && p.as_path() != file) {
            Some(module) => {
                collect_declared_modules(entry, module, root, use_paths, out)?;
                if !out.contains(module) {
                    out.push(module.clone());
                }
            }
            None => {}
        }
    }
    Ok(())
}

/// The top-level module a file at `root/name.wj` or `root/name/...` belongs to.
fn crate_module_name(root: &Path, file: &Path) -> Option<String> {
    let rel = file.strip_prefix(root).ok()?;
    let first = rel.components().next()?.as_os_str().to_string_lossy().to_string();
    Some(first.strip_suffix(".wj").unwrap_or(&first).to_string())
}

/// After a single-file build generated the entry's declared modules: each directory module's `mod.rs` (`x/mod.wj`)
/// is written by the writer a project build uses (`pub mod` children and their `pub use self::child::*`
/// re-exports), so `wj build src/main.wj` emits what `wj build src` does for those modules.
pub(super) fn write_directory_module_files(path: &Path, wj_files: &[PathBuf], output: &Path, library: bool, config: &crate::compiler_config::CompilerConfig) -> anyhow::Result<()> {
    let base = path.parent().unwrap_or(Path::new("."));
    for module in lone_script_files(path, wj_files, library, config)? {
        if wj_files.contains(&module) || module.file_name().is_none_or(|n| n != "mod.wj") {
            continue;
        }
        let rel = module.parent().and_then(|d| d.strip_prefix(base).ok()).unwrap_or(Path::new(""));
        crate::build_utils::generate_directory_mod_file(&output.join(rel), Some((output, base)))?;
    }
    Ok(())
}

/// Parse one script of a lone-script build; a lone `main.wj` (a file given to `wj build`, not a project directory)
/// also declares the sibling modules its `use crate::` paths name.
pub(crate) fn parse_lone_script(
    file: &Path,
    source: &str,
    path: &Path,
    wj_files: &[PathBuf],
    library: bool,
    config: &crate::compiler_config::CompilerConfig,
) -> anyhow::Result<(crate::parser::Parser, crate::parser::Program<'static>)> {
    let (parser, mut program) = super::parse_wj_source(file, source)?;
    if config.entry_use_paths_name_sibling_modules && !library && path.is_file() && wj_files.iter().any(|f| f == file) {
        declare_used_crate_modules(&mut program, file.parent().unwrap_or(Path::new(".")));
    }
    Ok((parser, program))
}
